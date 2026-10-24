//! Bounded discovery from the same authoring snapshot used by DraftBlueprint.
use super::Args;
use crate::{
    error::{DaemonError, DaemonResult},
    execution::context::ExecutionContext,
    registry::tool::Tool,
};
use async_trait::async_trait;
use metteur_shared::{PinType, Value};
use serde_json::{Map, json};

pub struct QueryBlueprintNodes;
#[async_trait]
impl Tool for QueryBlueprintNodes {
    fn name(&self) -> &str {
        "QueryBlueprintNodes"
    }
    fn description(&self) -> &str {
        "Discover actual blueprint node kinds before authoring with DraftBlueprint. Search with query and pagination; describe an exact kind to obtain canonical input/output names, types, defaults, choices and a compact JSON node example. For CallFunction pass configuration.function. Tool aliases use their exact current tool argument names. This only reads the current registry; it does not save, run, install or grant permissions."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({"type":"object","additionalProperties":false,"properties":{
            "query":{"type":"string","maxLength":128},"kind":{"type":"string","maxLength":128},
            "configuration":{"type":"object","description":"Instance configuration, such as function for CallFunction."},
            "after":{"type":"string","maxLength":128},"limit":{"type":"integer","minimum":1,"maximum":20}
        }})
    }
    fn read_only(&self) -> bool {
        true
    }
    fn lifetime(&self) -> metteur_shared::llm::ToolResultLifetime {
        metteur_shared::llm::ToolResultLifetime::Persistent
    }
    fn max_result_bytes(&self) -> usize {
        16 * 1024
    }
    async fn call(&self, args: &[Value], ctx: &mut ExecutionContext) -> DaemonResult<Value> {
        let a = Args::new(args);
        let query = a.string("query", 0).unwrap_or_default();
        let kind = a.string("kind", 1).unwrap_or_default();
        let after = a.string("after", 3).unwrap_or_default();
        let limit = a.int("limit", 4).unwrap_or(12);
        if query.len() > 128 || kind.len() > 128 || after.len() > 128 || !(1..=20).contains(&limit)
        {
            return Err(DaemonError::Execution(
                "Use bounded query/kind/after and limit 1..20".into(),
            ));
        }
        let catalog = ctx.registry.authoring_catalog();
        let visible = |name: &str, signature: &metteur_shared::node_catalog::NodeSignature| {
            signature.executor_kind != "Tool"
                || name == "Tool"
                || crate::registry::capabilities::allowed(ctx, name)
        };
        let revision = crate::storage::resources::digest(
            &serde_json::to_vec(&catalog).map_err(|e| DaemonError::Execution(e.to_string()))?,
        );
        let value = if kind.is_empty() {
            let needle = query.to_lowercase();
            let found: Vec<_> = catalog
                .iter()
                .filter(|(name, s)| {
                    visible(name, s)
                        && name.as_str() > after.as_str()
                        && (name.to_lowercase().contains(&needle)
                            || s.description.to_lowercase().contains(&needle))
                })
                .take(limit as usize + 1)
                .collect();
            let next = (found.len() > limit as usize).then(|| found[limit as usize - 1].0.clone());
            json!({"source":"current_authoring_registry","revision":revision,"nodes":found.iter().take(limit as usize).map(|(name,s)|json!({"kind":name,"executor_kind":s.executor_kind,"description":s.description,"dynamic_pins":s.dynamic_pins})).collect::<Vec<_>>(),"next":next,"usage":"Describe selected kinds, then use DraftBlueprint with compact JSON. No capability is granted by discovery."})
        } else {
            let configuration = match a.get("configuration", 2) {
                Some(Value::Json(v)) if v.is_object() => v,
                None => json!({}),
                _ => return Err(DaemonError::Execution("configuration must be an object".into())),
            };
            let signature=catalog.resolve(&kind,&configuration).filter(|s|visible(&kind,s)).ok_or_else(||DaemonError::Execution(format!("Node kind '{kind}' is unavailable for this role. Search QueryBlueprintNodes for current kinds.")))?;
            if kind == "CallFunction"
                && let Some(name) = configuration["function"].as_str()
                && !catalog.functions.contains_key(name)
            {
                return Err(DaemonError::Execution(
                    "Unknown current function; inspect available_functions".into(),
                ));
            }
            let mut example = Map::from_iter([("kind".into(), json!(kind))]);
            if kind == "Start" {
                example.insert("filePath".into(), json!("example.txt"));
            }
            for pin in &signature.pins {
                if pin.pin_type != PinType::DataInput {
                    continue;
                }
                if let Some(default) = &pin.default {
                    example.insert(
                        pin.key.clone(),
                        serde_json::to_value(default).unwrap_or(serde_json::Value::Null),
                    );
                }
            }
            if let Some(function) = configuration.get("function") {
                example.insert("function".into(), function.clone());
            }
            json!({"source":"current_authoring_registry","revision":revision,"signature":signature,"draft_node":example,"available_functions":if kind=="CallFunction"{catalog.functions.keys().take(20).cloned().collect::<Vec<_>>()}else{vec![]},"functions_limited":kind=="CallFunction"&&catalog.functions.len()>20,"syntax":{"control":"start -> action -> end; branch.True -> success","data":"target.Input <- source.Output or a $source.Output value","literal":"Use direct keys in draft_node, e.g. {kind: Start, filePath: example.txt}. Do not wrap these in data, pins, or configuration. A Start constant creates a same-name output. Tool aliases compile to Tool with tool_name; author them using the discovered alias. Use canonical names/keys and ordinary JSON values; do not invent UUIDs or DSL."}})
        };
        let text =
            serde_json::to_string(&value).map_err(|e| DaemonError::Execution(e.to_string()))?;
        if text.len() > 15 * 1024 {
            return Err(DaemonError::Execution(
                "Signature exceeds discovery result limit; use a narrower kind/query".into(),
            ));
        }
        Ok(Value::String(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn live_discovery_is_bounded_role_filtered_and_compiles_with_exact_tool_keys() {
        let registry = std::sync::Arc::new(crate::Registry::with_builtins());
        let mut ctx = ExecutionContext::new(
            registry.clone(),
            crate::llm::LlmClientFactory::new(),
            std::env::temp_dir(),
        );
        let described = QueryBlueprintNodes
            .call(&[Value::Json(json!({"kind":"ExecuteCommand"}))], &mut ctx)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(described.as_str().unwrap()).unwrap();
        assert!(
            value["signature"]["pins"].as_array().unwrap().iter().any(|p| p["name"] == "command")
        );
        let graph=metteur_shared::dsl::compile_draft_value_with_catalog(&json!({"name":"discovered","nodes":{"start":{"kind":"Start"},"cmd":{"kind":"ExecuteCommand","command":"echo literal"},"end":{"kind":"End"}},"flow":["start -> cmd -> end"]}),&registry.authoring_catalog()).unwrap();
        assert!(graph.nodes.iter().any(|n| n.data["command"] == "echo literal"));
        assert!(
            QueryBlueprintNodes.call(&[Value::Json(json!({"limit":21}))], &mut ctx).await.is_err()
        );
        assert!(
            QueryBlueprintNodes
                .call(&[Value::Json(json!({"kind":"InventedNode"}))], &mut ctx)
                .await
                .is_err()
        );
        ctx.read_only_tools = true;
        assert!(
            QueryBlueprintNodes
                .call(&[Value::Json(json!({"kind":"WriteFile"}))], &mut ctx)
                .await
                .is_err()
        );
    }
}
