use super::blueprint_nodes;
use crate::execution::ExecutionCheckpoint;
use metteur_shared::Blueprint;
use serde_json::json;
use uuid::Uuid;

fn graph() -> Blueprint {
    metteur_shared::dsl::compile_draft_value_with_catalog(
        &json!({"name":"Inspection","nodes":{"s":{"kind":"Start"},"d":{"kind":"Delay","Ms":1000},"e":{"kind":"End"}},"flow":["s -> d -> e"]}),
        &crate::registry::Registry::with_builtins().authoring_catalog(),
    ).unwrap()
}

#[test]
fn supervisor_can_inspect_the_root_before_the_first_node_runs() {
    let bp = graph();
    let mut cp = ExecutionCheckpoint::running(Uuid::new_v4(), bp.id, 0);
    cp.view.root = Some(bp.clone());
    assert!(cp.view.graphs.is_empty());
    let nodes = blueprint_nodes(&cp);
    assert_eq!(nodes.len(), bp.nodes.len());
    for (actual, expected) in nodes.iter().zip(&bp.nodes) {
        assert_eq!(actual["scope"], json!(bp.id));
        assert_eq!(actual["node"], json!(expected));
    }
}

#[test]
fn supervisor_reads_the_effective_root_instead_of_its_previous_invocation_image() {
    let previous = graph();
    let mut current = previous.clone();
    current.nodes.iter_mut().find(|n| n.kind == "Delay").unwrap().data = json!({"Ms":2000});
    let mut nested = graph();
    nested.id = Uuid::new_v4();
    let mut cp = ExecutionCheckpoint::running(Uuid::new_v4(), current.id, 0);
    cp.view.root = Some(current.clone());
    cp.view.graphs.insert(previous.id.to_string(), previous);
    cp.view.graphs.insert(nested.id.to_string(), nested.clone());
    let nodes = blueprint_nodes(&cp);
    assert_eq!(nodes.len(), current.nodes.len() + nested.nodes.len());
    let root: Vec<_> = nodes
        .iter()
        .filter(|n| n["scope"] == json!(current.id))
        .map(|n| n["node"].clone())
        .collect();
    assert_eq!(root, current.nodes.iter().map(|n| json!(n)).collect::<Vec<_>>());
    assert!(nodes.iter().any(|n| n["scope"] == json!(nested.id)));
}
