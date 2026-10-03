//! Interpreter circuit tests.

use crate::common::*;

#[tokio::test]
async fn circuit_breaker_aborts_when_user_denies() {
    use metteur_daemon::sandbox::approval::{ApprovalBroker, Decision, Scope};
    use metteur_shared::config::{Config, ExecutionConfig};

    let broker = Arc::new(ApprovalBroker::new());
    let config = Arc::new(RwLock::new(Config {
        execution: ExecutionConfig {
            circuit_break_after: 1,
            validation_max_attempts: 1,
            foreach_max_iterations: 1000,
            ..Default::default()
        },
        ..Default::default()
    }));

    // Start(A=0, Expected=5) -> Validator(mode eq): A != Expected always.
    let (start, validator) = (Uuid::new_v4(), Uuid::new_v4());
    let (start_ex, start_a, start_exp, v_exin, v_actual, v_expected, v_passed) = (
        Uuid::new_v4(),
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
    let blueprint = Blueprint {
        id: Uuid::new_v4(),
        name: "breaker".to_string(),
        nodes: vec![
            Node {
                id: start,
                node_type: NodeType::Event,
                kind: "Start".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(start_ex, "Exec", PinType::ExecOutput, DataType::Void),
                    pin(start_a, "A", PinType::DataOutput, DataType::Float),
                    pin(start_exp, "Expected", PinType::DataOutput, DataType::Float),
                ],
                data: serde_json::json!({ "A": 0, "Expected": 5 }),
            },
            Node {
                id: validator,
                node_type: NodeType::Function,
                kind: "Validator".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(v_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin(v_actual, "Actual", PinType::DataInput, DataType::Float),
                    pin(v_expected, "Expected", PinType::DataInput, DataType::Float),
                    pin(v_passed, "Passed", PinType::DataOutput, DataType::Bool),
                ],
                data: serde_json::json!({ "mode": "eq" }),
            },
        ],
        edges: vec![
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_ex,
                target_node: validator,
                target_pin: v_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_a,
                target_node: validator,
                target_pin: v_actual,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_exp,
                target_node: validator,
                target_pin: v_expected,
            },
        ],
        entry_node_id: start,
    };

    let mut interpreter = Interpreter::new(
        Arc::new(Registry::with_builtins()),
        LlmClientFactory::new(),
        std::env::temp_dir(),
    )
    .with_config(config)
    .with_approvals(broker.clone());
    let blueprint_arc = Arc::new(PLock::new(blueprint));
    let task = tokio::spawn(async move { interpreter.run(&blueprint_arc, None).await });
    // Deny the circuit-tripped approval as soon as it appears.
    loop {
        let ids = broker.pending_ids();
        if !ids.is_empty() {
            broker.respond(&ids[0], Decision::Deny, Scope::Once, &Default::default()).unwrap();
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let result = task.await.unwrap();
    assert!(matches!(
        result,
        Err(DaemonError::Execution(msg)) if msg.contains("aborted by user")
    ));
}
