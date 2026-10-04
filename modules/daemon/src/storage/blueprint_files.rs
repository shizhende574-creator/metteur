//! Authoritative blueprint files; database records are only mirrors and bindings.
use metteur_shared::Blueprint;
use serde_json::{Value, json};
use uuid::Uuid;

use super::persistence::{Db, cf};
use super::versioning::{VersionManager, VersionRef};
use crate::error::{DaemonError, DaemonResult};

fn binding_key(id: Uuid) -> Vec<u8> {
    format!("blueprint-file:{id}").into_bytes()
}

pub fn binding(db: &Db, id: Uuid) -> DaemonResult<Option<VersionRef>> {
    db.get(cf::BLUEPRINTS, &binding_key(id))?
        .map(|bytes| {
            serde_json::from_slice(&bytes).map_err(|e| DaemonError::Serialization(e.to_string()))
        })
        .transpose()
}

/// Decode both existing native/CLI JSON and the editor's canvas representation.
pub fn decode(bytes: &[u8]) -> DaemonResult<Blueprint> {
    let mut doc: Value =
        serde_json::from_slice(bytes).map_err(|e| DaemonError::Serialization(e.to_string()))?;
    if doc.get("entry_node_id").is_none() {
        let mut nodes = Vec::new();
        for node in doc["nodes"]
            .as_array()
            .ok_or_else(|| DaemonError::Execution("blueprint nodes are missing".into()))?
        {
            if node["type"] == "FileReference" {
                continue;
            }
            let mut data = node.get("data").cloned().unwrap_or(json!({}));
            let mut pins = Vec::new();
            for field in ["inputs", "outputs"] {
                for pin in node[field].as_array().into_iter().flatten() {
                    let kind = pin["kind"].as_str().unwrap_or("");
                    let id = pin["id"].as_str().unwrap_or("");
                    let key = pin["key"].as_str().unwrap_or("");
                    let name = pin["name"].as_str().filter(|s| !s.is_empty()).unwrap_or(key);
                    let ty = pin["type"].as_str().unwrap_or(if kind.starts_with("exec") {
                        "void"
                    } else {
                        "any"
                    });
                    if kind == "data-in"
                        && let Some(raw) = node["values"].get(id).and_then(Value::as_str)
                    {
                        let object = data.as_object_mut().ok_or_else(|| {
                            DaemonError::Execution("node data must be an object".into())
                        })?;
                        object.remove(name);
                        object.remove(key);
                        object.remove(id);
                        if !raw.is_empty() {
                            let value = match ty {
                                "int" | "float" => raw
                                    .parse::<f64>()
                                    .ok()
                                    .filter(|v| v.is_finite())
                                    .map(|v| {
                                        if v.fract() == 0.0
                                            && v >= i64::MIN as f64
                                            && v < i64::MAX as f64
                                        {
                                            json!(v as i64)
                                        } else {
                                            json!(v)
                                        }
                                    })
                                    .unwrap_or(json!(raw)),
                                "bool" => json!(raw == "true"),
                                _ if ty == "any"
                                    || ty == "json"
                                    || ty.starts_with("list")
                                    || ty.starts_with("object") =>
                                {
                                    serde_json::from_str(raw).unwrap_or(json!(raw))
                                }
                                _ => json!(raw),
                            };
                            object.insert(name.into(), value);
                        }
                    }
                    pins.push(json!({"id":id,"key": if key.is_empty() { None } else { Some(key) },"name":name,
                        "pin_type":match kind { "exec-in"=>"ExecInput","exec-out"=>"ExecOutput","data-out"=>"DataOutput",_=>"DataInput" },
                        "data_type":ty,"default":pin.get("default"),"optional":pin["optional"].as_bool().unwrap_or(false),
                        "choices":pin.get("choices").cloned().unwrap_or(json!([])),"description":pin["description"].as_str().filter(|v| !v.is_empty())}));
                }
            }
            nodes.push(json!({"id":node["id"],"kind":if node["type"] == "Arithmetic" { json!("Add") } else {node["type"].clone()},
                "node_type":node["nodeType"],"position":[node["position"]["x"],node["position"]["y"]],"pins":pins,"data":data}));
        }
        let ids: Vec<_> = nodes.iter().filter_map(|n| n["id"].as_str()).collect();
        let entry = doc["entryNodeId"]
            .as_str()
            .filter(|id| ids.contains(id))
            .or_else(|| ids.first().copied())
            .unwrap_or("");
        let edges: Vec<_> = doc["edges"].as_array().into_iter().flatten().filter(|e| ids.contains(&e["source"].as_str().unwrap_or("")) && ids.contains(&e["target"].as_str().unwrap_or(""))).map(|e| json!({"id":e["id"],"source_node":e["source"],"source_pin":e["sourceHandle"],"target_node":e["target"],"target_pin":e["targetHandle"]})).collect();
        doc = json!({"id":doc["id"],"name":doc["name"],"entry_node_id":entry,"nodes":nodes,"edges":edges});
    }
    serde_json::from_value(doc).map_err(|e| DaemonError::Serialization(e.to_string()))
}

pub fn save(
    db: &Db,
    versions: &VersionManager,
    blueprint: &Blueprint,
    uri: &str,
    bytes: &[u8],
    expected: Option<&VersionRef>,
) -> DaemonResult<VersionRef> {
    let _guard = versions.blueprint_gate.lock();
    if decode(bytes)? != *blueprint {
        return Err(DaemonError::Execution("blueprint file and executable graph disagree".into()));
    }
    let path = versions.blueprint_path(uri)?;
    if let Some(current) = binding(db, blueprint.id)?
        && versions.blueprint_path(&current.blueprint_uri)? != path
    {
        return Err(DaemonError::Execution(
            "blueprint already belongs to another file; use a new id for a copy".into(),
        ));
    }
    for (key, value) in db.scan(cf::BLUEPRINTS)? {
        if key.starts_with(b"blueprint-file:") && key != binding_key(blueprint.id) {
            let other: VersionRef = serde_json::from_slice(&value)
                .map_err(|e| DaemonError::Serialization(e.to_string()))?;
            if versions.blueprint_path(&other.blueprint_uri)? == path {
                return Err(DaemonError::Execution(
                    "file already belongs to another blueprint".into(),
                ));
            }
        }
    }
    let version = versions.write_blueprint_file(uri, bytes, expected)?;
    let graph =
        serde_json::to_vec(blueprint).map_err(|e| DaemonError::Serialization(e.to_string()))?;
    let metadata =
        serde_json::to_vec(&version).map_err(|e| DaemonError::Serialization(e.to_string()))?;
    db.put_pair(
        cf::BLUEPRINTS,
        blueprint.id.as_bytes(),
        &graph,
        &binding_key(blueprint.id),
        &metadata,
    )?;
    Ok(version)
}
