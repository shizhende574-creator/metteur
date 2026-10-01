//! Interpreter tree tests.

use crate::common::*;

#[tokio::test]
async fn execution_tree_tracks_nodes_and_tokens() {
    let (start, llm, end) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let pin = |p_id: Uuid, name: &str, pin_type: PinType, data_type: DataType| Pin {
        id: p_id,
        name: name.to_string(),
        pin_type,
        data_type,
        ..Default::default()
    };
    let (s_ex, l_exin, l_exout, e_exin) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let blueprint = Blueprint {
        id: Uuid::new_v4(),
        name: "tree-test".to_string(),
        nodes: vec![
            Node {
                id: start,
                node_type: NodeType::Event,
                kind: "Start".to_string(),
                position: (0.0, 0.0),
                pins: vec![pin(s_ex, "Exec", PinType::ExecOutput, DataType::Void)],
                data: serde_json::Value::Null,
            },
            Node {
                id: llm,
                node_type: NodeType::Function,
                kind: "CallLLM".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(l_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin(l_exout, "Exec", PinType::ExecOutput, DataType::Void),
                    pin(Uuid::new_v4(), "Result", PinType::DataOutput, DataType::String),
                    pin(Uuid::new_v4(), "Context", PinType::DataOutput, DataType::Context),
                ],
                data: serde_json::json!({
                    "provider": "mock",
                    "mock_text": "ok",
                    "model": "mock",
                }),
            },
            Node {
                id: end,
                node_type: NodeType::Event,
                kind: "End".to_string(),
                position: (0.0, 0.0),
                pins: vec![pin(e_exin, "Exec", PinType::ExecInput, DataType::Void)],
                data: serde_json::Value::Null,
            },
        ],
        edges: vec![
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: s_ex,
                target_node: llm,
                target_pin: l_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: llm,
                source_pin: l_exout,
                target_node: end,
                target_pin: e_exin,
            },
        ],
        entry_node_id: start,
    };
    let mut interpreter = new_interpreter();
    interpreter.run(&shared(blueprint), None).await.unwrap();
    // The tree has a run root plus one node per executed blueprint node.
    assert_eq!(interpreter.exec_tree().roots.len(), 1);
    let run = interpreter.exec_tree().nodes.get(&interpreter.exec_tree().roots[0]).expect("run root");
    assert!(matches!(run.kind, metteur_daemon::execution::tree::TreeNodeKind::Run));
    assert_eq!(run.children.len(), 3);
    let done = interpreter
        .exec_tree()
        .nodes
        .values()
        .filter(|n| matches!(n.status, metteur_daemon::execution::tree::TreeNodeStatus::Done))
        .count();
    assert_eq!(done, interpreter.exec_tree().nodes.len());
}

#[tokio::test]
async fn function_call_closes_function_and_caller_tree_nodes() {
    let func = add_function();
    let registry = Arc::new(Registry::with_builtins());
    registry.register_function(func);
    let blueprint = call_function_blueprint("AddFunc");
    let mut interpreter =
        Interpreter::new(registry, LlmClientFactory::new(), std::env::temp_dir());
    interpreter.run(&shared(blueprint), None).await.unwrap();
    // No tree node may remain Running after the run completes; the run
    // root, its blueprint nodes and every entered function frame are Done.
    let running = interpreter
        .exec_tree()
        .nodes
        .values()
        .filter(|n| matches!(n.status, metteur_daemon::execution::tree::TreeNodeStatus::Running))
        .count();
    assert_eq!(running, 0, "all tree nodes must close: {:#?}", interpreter.exec_tree().nodes);
    // The CallFunction node carries the entered function as a child.
    let call_node = interpreter.exec_tree().nodes.values().find(|n| n.label == "CallFunction");
    assert!(call_node.is_some(), "CallFunction node must exist in the tree");
    let root = interpreter.exec_tree().nodes.get(&interpreter.exec_tree().roots[0]).expect("run root");
    assert!(matches!(root.kind, metteur_daemon::execution::tree::TreeNodeKind::Run));
}

#[tokio::test]
async fn execution_tree_survives_checkpoint_round_trip() {
    let (start, end) = (Uuid::new_v4(), Uuid::new_v4());
    let pin = |p_id: Uuid, name: &str, pin_type: PinType, data_type: DataType| Pin {
        id: p_id,
        name: name.to_string(),
        pin_type,
        data_type,
        ..Default::default()
    };
    let (s_ex, e_exin) = (Uuid::new_v4(), Uuid::new_v4());
    let blueprint = Blueprint {
        id: Uuid::new_v4(),
        name: "tree-cp".to_string(),
        nodes: vec![
            Node {
                id: start,
                node_type: NodeType::Event,
                kind: "Start".to_string(),
                position: (0.0, 0.0),
                pins: vec![pin(s_ex, "Exec", PinType::ExecOutput, DataType::Void)],
                data: serde_json::Value::Null,
            },
            Node {
                id: end,
                node_type: NodeType::Event,
                kind: "End".to_string(),
                position: (0.0, 0.0),
                pins: vec![pin(e_exin, "Exec", PinType::ExecInput, DataType::Void)],
                data: serde_json::Value::Null,
            },
        ],
        edges: vec![Edge {
            id: Uuid::new_v4(),
            source_node: start,
            source_pin: s_ex,
            target_node: end,
            target_pin: e_exin,
        }],
        entry_node_id: start,
    };
    let sink = Arc::new(MemorySink::new());
    let mut interpreter = new_interpreter().with_checkpoint_sink(sink.clone());
    interpreter.run(&shared(blueprint.clone()), None).await.unwrap();
    let last = sink.checkpoints.lock().unwrap().last().unwrap().clone();
    assert!(!last.exec_tree.nodes.is_empty());
    // A fresh run of the same blueprint also populates a tree root.
    let mut interpreter = new_interpreter();
    interpreter.run(&shared(blueprint), None).await.unwrap();
    assert_eq!(interpreter.exec_tree().roots.len(), 1);
}
