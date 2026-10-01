//! Interpreter basics tests.

use crate::common::*;

#[tokio::test]
async fn executes_arithmetic_chain() {
    let blueprint = build_blueprint();
    let mut interpreter = new_interpreter();
    let events = interpreter.run(&shared(blueprint), None).await.unwrap();
    // Start, Add, Judge each emit started + finished + node_data.
    assert_eq!(events.len(), 9);
    assert!(matches!(events[0], ExecutionEvent::NodeStarted { .. }));
}

#[tokio::test]
async fn run_resets_state_on_reuse() {
    let blueprint = build_blueprint();
    let mut interpreter = new_interpreter();
    let first = interpreter.run(&shared(blueprint.clone()), None).await.unwrap();
    let second = interpreter.run(&shared(blueprint), None).await.unwrap();
    assert_eq!(first.len(), second.len());
    assert_eq!(first.len(), 9);
}

#[tokio::test]
async fn detects_execution_cycle() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let a_exec_out = Uuid::new_v4();
    let a_exec_in = Uuid::new_v4();
    let b_exec_out = Uuid::new_v4();
    let b_exec_in = Uuid::new_v4();

    let blueprint = Blueprint {
        id: Uuid::new_v4(),
        name: "cycle".to_string(),
        nodes: vec![
            Node {
                id: a,
                node_type: NodeType::Function,
                kind: "CallLLM".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    Pin {
                        id: a_exec_in,
                        name: "Exec".to_string(),
                        pin_type: PinType::ExecInput,
                        data_type: DataType::Void,
                        ..Default::default()
                    },
                    Pin {
                        id: a_exec_out,
                        name: "Exec".to_string(),
                        pin_type: PinType::ExecOutput,
                        data_type: DataType::Void,
                        ..Default::default()
                    },
                ],
                data: serde_json::Value::Null,
            },
            Node {
                id: b,
                node_type: NodeType::Function,
                kind: "CallLLM".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    Pin {
                        id: b_exec_in,
                        name: "Exec".to_string(),
                        pin_type: PinType::ExecInput,
                        data_type: DataType::Void,
                        ..Default::default()
                    },
                    Pin {
                        id: b_exec_out,
                        name: "Exec".to_string(),
                        pin_type: PinType::ExecOutput,
                        data_type: DataType::Void,
                        ..Default::default()
                    },
                ],
                data: serde_json::Value::Null,
            },
        ],
        edges: vec![
            Edge {
                id: Uuid::new_v4(),
                source_node: a,
                source_pin: a_exec_out,
                target_node: b,
                target_pin: b_exec_in,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: b,
                source_pin: b_exec_out,
                target_node: a,
                target_pin: a_exec_in,
            },
        ],
        entry_node_id: a,
    };

    let mut interpreter = new_interpreter();
    let result = interpreter.run(&shared(blueprint), None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn executes_diamond_blueprint() {
    // Start branches to Add1 and Add2, which merge into Add3. Add3
    // consumes data produced by both branches, so it must wait for both.
    let start = Uuid::new_v4();
    let add1 = Uuid::new_v4();
    let add2 = Uuid::new_v4();
    let add3 = Uuid::new_v4();

    let start_exec = Uuid::new_v4();
    let start_a = Uuid::new_v4();
    let start_b = Uuid::new_v4();

    let add1_exec_in = Uuid::new_v4();
    let add1_exec_out = Uuid::new_v4();
    let add1_a = Uuid::new_v4();
    let add1_b = Uuid::new_v4();
    let add1_result = Uuid::new_v4();

    let add2_exec_in = Uuid::new_v4();
    let add2_exec_out = Uuid::new_v4();
    let add2_a = Uuid::new_v4();
    let add2_b = Uuid::new_v4();
    let add2_result = Uuid::new_v4();

    let add3_exec_in = Uuid::new_v4();
    let add3_exec_out = Uuid::new_v4();
    let add3_a = Uuid::new_v4();
    let add3_b = Uuid::new_v4();
    let add3_result = Uuid::new_v4();

    let add_node =
        |id: Uuid, exec_in: Uuid, exec_out: Uuid, a: Uuid, b: Uuid, result: Uuid| Node {
            id,
            node_type: NodeType::Pure,
            kind: "Add".to_string(),
            position: (0.0, 0.0),
            pins: vec![
                Pin {
                    id: exec_in,
                    name: "Exec".to_string(),
                    pin_type: PinType::ExecInput,
                    data_type: DataType::Void,
                    ..Default::default()
                },
                Pin {
                    id: exec_out,
                    name: "Exec".to_string(),
                    pin_type: PinType::ExecOutput,
                    data_type: DataType::Void,
                    ..Default::default()
                },
                Pin {
                    id: a,
                    name: "A".to_string(),
                    pin_type: PinType::DataInput,
                    data_type: DataType::Float,
                    ..Default::default()
                },
                Pin {
                    id: b,
                    name: "B".to_string(),
                    pin_type: PinType::DataInput,
                    data_type: DataType::Float,
                    ..Default::default()
                },
                Pin {
                    id: result,
                    name: "Result".to_string(),
                    pin_type: PinType::DataOutput,
                    data_type: DataType::Float,
                    ..Default::default()
                },
            ],
            data: serde_json::Value::Null,
        };

    let blueprint = Blueprint {
        id: Uuid::new_v4(),
        name: "diamond".to_string(),
        nodes: vec![
            Node {
                id: start,
                node_type: NodeType::Event,
                kind: "Start".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    Pin {
                        id: start_exec,
                        name: "Exec".to_string(),
                        pin_type: PinType::ExecOutput,
                        data_type: DataType::Void,
                        ..Default::default()
                    },
                    Pin {
                        id: start_a,
                        name: "A".to_string(),
                        pin_type: PinType::DataOutput,
                        data_type: DataType::Float,
                        ..Default::default()
                    },
                    Pin {
                        id: start_b,
                        name: "B".to_string(),
                        pin_type: PinType::DataOutput,
                        data_type: DataType::Float,
                        ..Default::default()
                    },
                ],
                data: serde_json::json!({ "A": 2, "B": 3 }),
            },
            add_node(add1, add1_exec_in, add1_exec_out, add1_a, add1_b, add1_result),
            add_node(add2, add2_exec_in, add2_exec_out, add2_a, add2_b, add2_result),
            add_node(add3, add3_exec_in, add3_exec_out, add3_a, add3_b, add3_result),
        ],
        edges: vec![
            // Exec: Start -> Add1, Start -> Add2, Add1 -> Add3, Add2 -> Add3.
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_exec,
                target_node: add1,
                target_pin: add1_exec_in,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_exec,
                target_node: add2,
                target_pin: add2_exec_in,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: add1,
                source_pin: add1_exec_out,
                target_node: add3,
                target_pin: add3_exec_in,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: add2,
                source_pin: add2_exec_out,
                target_node: add3,
                target_pin: add3_exec_in,
            },
            // Data: Start -> Add1/Add2, Add1/Add2 -> Add3.
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_a,
                target_node: add1,
                target_pin: add1_a,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_b,
                target_node: add1,
                target_pin: add1_b,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_a,
                target_node: add2,
                target_pin: add2_a,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: start,
                source_pin: start_b,
                target_node: add2,
                target_pin: add2_b,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: add1,
                source_pin: add1_result,
                target_node: add3,
                target_pin: add3_a,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: add2,
                source_pin: add2_result,
                target_node: add3,
                target_pin: add3_b,
            },
        ],
        entry_node_id: start,
    };

    let mut interpreter = new_interpreter();
    let events = interpreter.run(&shared(blueprint), None).await.unwrap();
    // Start, Add1, Add2, Add3 each emit started + finished + node_data.
    assert_eq!(events.len(), 12);
    // Add3 must have executed, which requires both branches to have run.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ExecutionEvent::NodeFinished { node_id } if *node_id == add3))
    );
}
