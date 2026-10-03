//! Interpreter approval tests.

use crate::common::*;

/// Builds Start -> RequestApproval -> WriteFile per outcome -> End.
fn approval_blueprint() -> (Blueprint, PathBuf) {
    let workspace = std::env::temp_dir().join(format!("metteur-approval-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace).unwrap();
    let (start, approval, write_a, write_d, end) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
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
    let (s_ex, a_exin, a_msg, a_allowed, a_ok, a_no) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let e_exin = Uuid::new_v4();
    let write_a_node = write_node(write_a, "approved");
    let write_d_node = write_node(write_d, "denied");
    let a_in = |n: &Node| {
        n.pins.iter().find(|p| p.name == "Exec" && p.pin_type == PinType::ExecInput).unwrap().id
    };
    let a_out = |n: &Node| {
        n.pins
            .iter()
            .find(|p| p.name == "Exec" && p.pin_type == PinType::ExecOutput)
            .unwrap()
            .id
    };
    let (wa_in, wa_out, wd_in, wd_out) =
        (a_in(&write_a_node), a_out(&write_a_node), a_in(&write_d_node), a_out(&write_d_node));
    let blueprint = Blueprint {
        id: Uuid::new_v4(),
        name: "approval".to_string(),
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
                id: approval,
                node_type: NodeType::Control,
                kind: "RequestApproval".to_string(),
                position: (0.0, 0.0),
                pins: vec![
                    pin(a_exin, "Exec", PinType::ExecInput, DataType::Void),
                    pin_with_default(
                        a_msg,
                        "Message",
                        PinType::DataInput,
                        DataType::String,
                        serde_json::json!("proceed?"),
                    ),
                    pin(a_allowed, "Allowed", PinType::DataOutput, DataType::Bool),
                    pin(a_ok, "Approved", PinType::ExecOutput, DataType::Void),
                    pin(a_no, "Denied", PinType::ExecOutput, DataType::Void),
                ],
                data: serde_json::Value::Null,
            },
            write_a_node,
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
                target_node: approval,
                target_pin: a_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: approval,
                source_pin: a_ok,
                target_node: write_a,
                target_pin: wa_in,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: approval,
                source_pin: a_no,
                target_node: write_d,
                target_pin: wd_in,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: write_a,
                source_pin: wa_out,
                target_node: end,
                target_pin: e_exin,
            },
            Edge {
                id: Uuid::new_v4(),
                source_node: write_d,
                source_pin: wd_out,
                target_node: end,
                target_pin: e_exin,
            },
        ],
        entry_node_id: start,
    };
    (blueprint, workspace)
}

#[tokio::test]
async fn request_approval_routes_both_outcomes() {
    use metteur_daemon::sandbox::approval::{ApprovalBroker, Decision, Scope};
    for (decision, expected) in [(Decision::Allow, "approved"), (Decision::Deny, "denied")] {
        let (blueprint, workspace) = approval_blueprint();
        let broker = Arc::new(ApprovalBroker::new());
        let mut interpreter = Interpreter::new(
            Arc::new(Registry::with_builtins()),
            LlmClientFactory::new(),
            workspace.clone(),
        )
        .with_approvals(broker.clone());
        let blueprint_arc = Arc::new(PLock::new(blueprint));
        let task = tokio::spawn(async move { interpreter.run(&blueprint_arc, None).await });
        loop {
            let ids = broker.pending_ids();
            if !ids.is_empty() {
                broker.respond(&ids[0], decision, Scope::Once, &Default::default()).unwrap();
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        task.await.unwrap().unwrap();
        assert!(broker.is_closed());
        assert!(broker.pending_ids().is_empty());
        assert_eq!(std::fs::read_to_string(workspace.join("hit.txt")).unwrap(), expected);
    }
}
