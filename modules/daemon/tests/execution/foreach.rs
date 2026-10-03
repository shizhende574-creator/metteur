//! Interpreter foreach tests.

use crate::common::*;

/// Builds Start -> ForEach([1,2,3]) -> Body: Set(last=Iteration) ->
/// Completed -> Get(last) -> Validator(eq 3) -> End.
pub(crate) fn foreach_blueprint(list: serde_json::Value) -> Blueprint {
    let (start, fe, set, get, validator, end) = (
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
    let (s_ex, fe_exin, fe_list, fe_iter, fe_idx, fe_body, fe_done) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let (set_exin, set_exout, set_name, set_val, set_out) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let (get_exin, get_exout, get_name, get_val) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let (v_exin, v_exout, v_actual, v_expected, v_passed) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let e_exin = Uuid::new_v4();
    Blueprint {
        id: Uuid::new_v4(),
        name: "foreach".to_string(),
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
                id: fe,
                node_type: NodeType::Control,
                kind: "ForEach".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(fe_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin_with_default(fe_list, "List", PinType::DataInput, DataType::Any, list),
                    pin(fe_iter, "Iteration", PinType::DataOutput, DataType::Any),
                    pin(fe_idx, "Index", PinType::DataOutput, DataType::Int),
                    pin(fe_body, "Body", PinType::ExecOutput, DataType::Void),
                    pin(fe_done, "Completed", PinType::ExecOutput, DataType::Void),
                ],
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
                        serde_json::json!("last"),
                    ),
                    pin(set_val, "Value", PinType::DataInput, DataType::Any),
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
                        serde_json::json!("last"),
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
                        serde_json::json!(3),
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
                target_node: fe,
                target_pin: fe_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: fe,
                source_pin: fe_body,
                target_node: set,
                target_pin: set_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: fe,
                source_pin: fe_done,
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
                source_node: fe,
                source_pin: fe_iter,
                target_node: set,
                target_pin: set_val,
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
async fn foreach_iterates_in_order() {
    let blueprint = foreach_blueprint(serde_json::json!([1, 2, 3]));
    let mut interpreter = new_interpreter();
    interpreter.run(&shared(blueprint), None).await.unwrap();
}

#[tokio::test]
async fn foreach_empty_list_skips_body() {
    // The body would set `flag`; reading it after an empty run must fail.
    let blueprint = foreach_blueprint(serde_json::json!([]));
    let mut interpreter = new_interpreter();
    let result = interpreter.run(&shared(blueprint), None).await;
    assert!(matches!(
        result,
        Err(DaemonError::Execution(msg)) if msg.contains("undefined variable")
    ));
}

#[tokio::test]
async fn foreach_respects_iteration_limit() {
    use metteur_shared::config::{Config, ExecutionConfig};
    let blueprint = foreach_blueprint(serde_json::json!([1, 2, 3]));
    let config = Arc::new(RwLock::new(Config {
        execution: ExecutionConfig {
            circuit_break_after: 0,
            validation_max_attempts: 1,
            foreach_max_iterations: 2,
            ..Default::default()
        },
        ..Default::default()
    }));
    let mut interpreter = new_interpreter().with_config(config);
    let result = interpreter.run(&shared(blueprint), None).await;
    assert!(matches!(
        result,
        Err(DaemonError::Execution(msg)) if msg.contains("exceeded 2 iterations")
    ));
}
