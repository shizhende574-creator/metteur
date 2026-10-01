//! Interpreter variables tests.

use crate::common::*;

/// Builds Start -> VariableSet(x) -> VariableGet(x) -> Validator(eq 1) -> End.
fn variable_round_trip_blueprint() -> Blueprint {
    let (start, set, get, validator, end) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let pin = |p_id: Uuid, name: &str, pin_type: PinType, data_type: DataType| Pin {
        id: p_id,
        name: name.to_string(),
        pin_type,
        data_type,
        ..Default::default()
    };
    let (s_ex, set_exin, set_exout, set_name, set_val, set_out) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let (get_exin, get_exout, get_name, get_val) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let (v_exin, v_exout, v_actual, v_expected, v_passed) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let e_exin = Uuid::new_v4();
    Blueprint {
        id: Uuid::new_v4(),
        name: "vars".to_string(),
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
                id: set,
                node_type: NodeType::Function,
                kind: "VariableSet".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(set_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin(set_exout, "Exec", PinType::ExecOutput, DataType::Void),
                    pin_with_default(
                        set_name,
                        "Name",
                        PinType::DataInput,
                        DataType::String,
                        serde_json::json!("x"),
                    ),
                    pin_with_default(
                        set_val,
                        "Value",
                        PinType::DataInput,
                        DataType::Any,
                        serde_json::json!(1),
                    ),
                    pin(set_out, "Value", PinType::DataOutput, DataType::Any),
                ],
                data: serde_json::Value::Null,
            },
            Node {
                id: get,
                node_type: NodeType::Pure,
                kind: "VariableGet".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(get_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin(get_exout, "Exec", PinType::ExecOutput, DataType::Void),
                    pin_with_default(
                        get_name,
                        "Name",
                        PinType::DataInput,
                        DataType::String,
                        serde_json::json!("x"),
                    ),
                    pin(get_val, "Value", PinType::DataOutput, DataType::Any),
                ],
                data: serde_json::Value::Null,
            },
            Node {
                id: validator,
                node_type: NodeType::Function,
                kind: "Validator".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(v_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin(v_exout, "Exec", PinType::ExecOutput, DataType::Void),
                    pin(v_actual, "Actual", PinType::DataInput, DataType::Any),
                    pin_with_default(
                        v_expected,
                        "Expected",
                        PinType::DataInput,
                        DataType::Any,
                        serde_json::json!(1),
                    ),
                    pin(v_passed, "Passed", PinType::DataOutput, DataType::Bool),
                ],
                data: serde_json::json!({ "mode": "eq" }),
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
                target_node: set,
                target_pin: set_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: set,
                source_pin: set_exout,
                target_node: get,
                target_pin: get_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: get,
                source_pin: get_exout,
                target_node: validator,
                target_pin: v_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: validator,
                source_pin: v_exout,
                target_node: end,
                target_pin: e_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: get,
                source_pin: get_val,
                target_node: validator,
                target_pin: v_actual,
            },
        ],
        entry_node_id: start,
    }
}

#[tokio::test]
async fn variables_round_trip_within_frame() {
    let blueprint = variable_round_trip_blueprint();
    let mut interpreter = new_interpreter();
    interpreter.run(&shared(blueprint), None).await.unwrap();
}

#[tokio::test]
async fn variable_get_undefined_fails() {
    let (start, get, end) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let pin = |p_id: Uuid, name: &str, pin_type: PinType, data_type: DataType| Pin {
        id: p_id,
        name: name.to_string(),
        pin_type,
        data_type,
        ..Default::default()
    };
    let (s_ex, g_exin, g_name, g_val, e_exin) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let blueprint = Blueprint {
        id: Uuid::new_v4(),
        name: "undef".to_string(),
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
                id: get,
                node_type: NodeType::Pure,
                kind: "VariableGet".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(g_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin_with_default(
                        g_name,
                        "Name",
                        PinType::DataInput,
                        DataType::String,
                        serde_json::json!("missing"),
                    ),
                    pin(g_val, "Value", PinType::DataOutput, DataType::Any),
                ],
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
            target_node: get,
            target_pin: g_exin,
        }],
        entry_node_id: start,
    };
    let mut interpreter = new_interpreter();
    let result = interpreter.run(&shared(blueprint), None).await;
    assert!(matches!(
        result,
        Err(DaemonError::Execution(msg)) if msg.contains("undefined variable")
    ));
}
