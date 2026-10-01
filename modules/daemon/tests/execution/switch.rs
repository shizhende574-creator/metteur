//! Interpreter switch tests.

use crate::common::*;

/// Builds Start(Case) -> Switch -> WriteFile(hit.txt) per branch -> End.
fn switch_blueprint(case_value: &str) -> (Blueprint, PathBuf) {
    let workspace = std::env::temp_dir().join(format!("metteur-switch-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace).unwrap();
    let (start, switch, write_a, write_b, write_d, end) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let pin = |p_id: Uuid, name: &str, pin_type: PinType, data_type: DataType| Pin {
        id: p_id,
        name: name.to_string(),
        pin_type,
        data_type,
        ..Default::default()
    };
    let write_node = |id: Uuid, content: &str| {
        let (exin, exout, path, text) =
            (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        Node {
            id,
            node_type: NodeType::Function,
            kind: "Tool".to_string(),
            position: (0.0, 0.0),
            pins: vec![
                pin(exin, "Exec", PinType::ExecInput, DataType::Void),
                pin(exout, "Exec", PinType::ExecOutput, DataType::Void),
                pin_with_default(
                    path,
                    "path",
                    PinType::DataInput,
                    DataType::String,
                    serde_json::json!("hit.txt"),
                ),
                pin_with_default(
                    text,
                    "content",
                    PinType::DataInput,
                    DataType::String,
                    serde_json::json!(content),
                ),
                pin(Uuid::new_v4(), "Result", PinType::DataOutput, DataType::String),
            ],
            data: serde_json::json!({ "tool_name": "WriteFile" }),
        }
    };
    let (s_ex, s_case) = (Uuid::new_v4(), Uuid::new_v4());
    let (sw_exin, sw_case, sw_result, sw_a, sw_b, sw_d) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let e_exin = Uuid::new_v4();
    let write_a_node = write_node(write_a, "a");
    let write_b_node = write_node(write_b, "b");
    let write_d_node = write_node(write_d, "d");
    let a_exin = write_a_node
        .pins
        .iter()
        .find(|p| p.name == "Exec" && p.pin_type == PinType::ExecInput)
        .unwrap()
        .id;
    let b_exin = write_b_node
        .pins
        .iter()
        .find(|p| p.name == "Exec" && p.pin_type == PinType::ExecInput)
        .unwrap()
        .id;
    let d_exin = write_d_node
        .pins
        .iter()
        .find(|p| p.name == "Exec" && p.pin_type == PinType::ExecInput)
        .unwrap()
        .id;
    let a_exout = write_a_node
        .pins
        .iter()
        .find(|p| p.name == "Exec" && p.pin_type == PinType::ExecOutput)
        .unwrap()
        .id;
    let b_exout = write_b_node
        .pins
        .iter()
        .find(|p| p.name == "Exec" && p.pin_type == PinType::ExecOutput)
        .unwrap()
        .id;
    let d_exout = write_d_node
        .pins
        .iter()
        .find(|p| p.name == "Exec" && p.pin_type == PinType::ExecOutput)
        .unwrap()
        .id;
    let blueprint = Blueprint {
        id: Uuid::new_v4(),
        name: "switch".to_string(),
        nodes: vec![
            Node {
                id: start,
                node_type: NodeType::Event,
                kind: "Start".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(s_ex, "Exec", PinType::ExecOutput, DataType::Void),
                    pin(s_case, "Case", PinType::DataOutput, DataType::String),
                ],
                data: serde_json::json!({ "Case": case_value }),
            },
            Node {
                id: switch,
                node_type: NodeType::Control,
                kind: "Switch".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(sw_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin(sw_case, "Case", PinType::DataInput, DataType::String),
                    pin(sw_result, "Result", PinType::DataOutput, DataType::Any),
                    pin(sw_a, "Case_a", PinType::ExecOutput, DataType::Void),
                    pin(sw_b, "Case_b", PinType::ExecOutput, DataType::Void),
                    pin(sw_d, "Default", PinType::ExecOutput, DataType::Void),
                ],
                data: serde_json::json!({ "cases": ["a", "b"] }),
            },
            write_a_node,
            write_b_node,
            write_d_node,
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
                target_node: switch,
                target_pin: sw_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: s_case,
                target_node: switch,
                target_pin: sw_case,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: switch,
                source_pin: sw_a,
                target_node: write_a,
                target_pin: a_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: switch,
                source_pin: sw_b,
                target_node: write_b,
                target_pin: b_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: switch,
                source_pin: sw_d,
                target_node: write_d,
                target_pin: d_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: write_a,
                source_pin: a_exout,
                target_node: end,
                target_pin: e_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: write_b,
                source_pin: b_exout,
                target_node: end,
                target_pin: e_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: write_d,
                source_pin: d_exout,
                target_node: end,
                target_pin: e_exin,
            },
        ],
        entry_node_id: start,
    };
    (blueprint, workspace)
}

#[tokio::test]
async fn switch_routes_to_matching_case() {
    let (blueprint, workspace) = switch_blueprint("b");
    let mut interpreter = Interpreter::new(
        Arc::new(Registry::with_builtins()),
        LlmClientFactory::new(),
        workspace.clone(),
    );
    interpreter.run(&shared(blueprint), None).await.unwrap();
    assert_eq!(std::fs::read_to_string(workspace.join("hit.txt")).unwrap(), "b");
}

#[tokio::test]
async fn switch_falls_back_to_default() {
    let (blueprint, workspace) = switch_blueprint("zzz");
    let mut interpreter = Interpreter::new(
        Arc::new(Registry::with_builtins()),
        LlmClientFactory::new(),
        workspace.clone(),
    );
    interpreter.run(&shared(blueprint), None).await.unwrap();
    assert_eq!(std::fs::read_to_string(workspace.join("hit.txt")).unwrap(), "d");
}
