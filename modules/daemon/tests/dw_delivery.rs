//! Actual RPC adapters, workspace RocksDB and the existing local mock runner.
use metteur_daemon::{
    AppState, DaemonService, Registry, WorkspaceManager, chat::session::load_thread,
};
use metteur_proto::proto::{ResourceRequest, SendChatRequest, daemon_server::Daemon};
use metteur_shared::config::Config;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_stream::StreamExt;
use tonic::Request;
use uuid::Uuid;

async fn setup() -> (DaemonService, Arc<AppState>, PathBuf) {
    let root = std::env::temp_dir().join(format!("dw-delivery-rpc-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let state = Arc::new(AppState::new(
        WorkspaceManager::new().with_global_config_path(root.join(".metteur/global.toml")),
        Arc::new(Registry::with_builtins()),
        Config::default(),
    ));
    state.workspaces.open(&root).await.unwrap();
    (DaemonService::new(state.clone()), state, root)
}
fn command(root: &Path, id: Uuid, session: &str, expected: u64) -> SendChatRequest {
    SendChatRequest{workspace_path:root.to_string_lossy().into(),message:"manual input\r\n中文".into(),session_id:session.into(),history_json:"[]".into(),options_json:serde_json::json!({"provider":"mock","mock_text":"one answer","mock_delay_ms":150,"max_tokens":128,"delivery":{"id":id,"expected_revision":expected}}).to_string()}
}
async fn receipt(service: &DaemonService, root: &Path, id: Uuid) -> serde_json::Value {
    let result = Daemon::chat_delivery(
        service,
        Request::new(ResourceRequest {
            workspace_path: root.to_string_lossy().into(),
            schema_version: 1,
            payload_json: serde_json::json!({"operation":"read","id":id}).to_string(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(result.error_json.is_empty(), "{}", result.error_json);
    serde_json::from_str::<serde_json::Value>(&result.payload_json).unwrap()["receipt"].clone()
}

async fn admission(service: &DaemonService, root: &Path) -> serde_json::Value {
    let result = Daemon::chat_delivery(
        service,
        Request::new(ResourceRequest {
            workspace_path: root.to_string_lossy().into(),
            schema_version: 1,
            payload_json: serde_json::json!({"operation":"admission"}).to_string(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(result.error_json.is_empty(), "{}", result.error_json);
    serde_json::from_str(&result.payload_json).unwrap()
}

#[tokio::test]
async fn foreground_snapshot_uses_the_running_chat_identity_and_releases_only_when_finished() {
    let (service, _state, root) = setup().await;
    assert_eq!(admission(&service, &root).await["state"], "idle");
    let id = Uuid::new_v4();
    let mut request = command(&root, id, "", 0);
    let mut options: serde_json::Value = serde_json::from_str(&request.options_json).unwrap();
    options["mock_delay_ms"] = 1000.into();
    request.options_json = options.to_string();
    let mut stream = Daemon::send_chat(&service, Request::new(request)).await.unwrap().into_inner();
    let accepted = receipt(&service, &root, id).await;
    let owner = admission(&service, &root).await;
    assert_eq!(owner["state"], "chat");
    assert_eq!(owner["session"], accepted["session"]);
    assert_eq!(owner["turn"], accepted["turn"]);
    while let Some(event) = stream.next().await {
        event.unwrap();
    }
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while admission(&service, &root).await["state"] != "idle" {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn blueprint_pause_keeps_foreground_admission_and_legacy_send_is_rejected() {
    let (service, _state, root) = setup().await;
    let graph=metteur_shared::dsl::compile_draft_value_with_catalog(&serde_json::json!({"name":"foreground fixture","nodes":{"s":{"kind":"Start"},"d":{"kind":"Delay","Ms":1500},"e":{"kind":"End"}},"flow":["s -> d -> e"]}),&Registry::with_builtins().authoring_catalog()).unwrap();
    let file =
        String::from_utf8(metteur_daemon::storage::blueprint_files::encode_native(&graph).unwrap())
            .unwrap();
    Daemon::write_file(
        &service,
        Request::new(metteur_proto::proto::WriteFileRequest {
            workspace_path: root.to_string_lossy().into(),
            path: "foreground.blueprint".into(),
            content: file.clone(),
            ..Default::default()
        }),
    )
    .await
    .unwrap();
    let mut stream = Daemon::execute_blueprint(
        &service,
        Request::new(metteur_proto::proto::ExecuteBlueprintRequest {
            workspace_path: root.to_string_lossy().into(),
            blueprint_id: graph.id.to_string(),
            blueprint_json: file,
        }),
    )
    .await
    .unwrap()
    .into_inner();
    let owner = admission(&service, &root).await;
    assert_eq!(owner["state"], "blueprint");
    let run = owner["run"].as_str().unwrap().to_owned();
    Daemon::pause_execution(
        &service,
        Request::new(metteur_proto::proto::PauseRequest {
            workspace_path: root.to_string_lossy().into(),
            run_id: run.clone(),
        }),
    )
    .await
    .unwrap();
    assert_eq!(admission(&service, &root).await["reason"], "Blueprint is paused");
    let mut legacy = command(&root, Uuid::new_v4(), "", 0);
    legacy.options_json = r#"{"provider":"mock"}"#.into();
    assert_eq!(
        Daemon::send_chat(&service, Request::new(legacy)).await.err().unwrap().code(),
        tonic::Code::FailedPrecondition
    );
    Daemon::cancel_execution(
        &service,
        Request::new(metteur_proto::proto::CancelRequest {
            workspace_path: root.to_string_lossy().into(),
            run_id: run,
        }),
    )
    .await
    .unwrap();
    assert_eq!(admission(&service, &root).await["state"], "blueprint");
    let mut aborted = false;
    while let Some(event) = stream.next().await {
        if let Err(error) = event {
            assert_eq!(error.code(), tonic::Code::Aborted);
            aborted = true;
        }
    }
    assert!(aborted, "Cancel must have its real interrupted outcome");
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while admission(&service, &root).await["state"] != "idle" {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn accepted_input_is_durable_before_the_first_stream_event_and_duplicate_never_runs_again() {
    let (service, state, root) = setup().await;
    let id = Uuid::new_v4();
    let request = command(&root, id, "", 0);
    let mut stream =
        Daemon::send_chat(&service, Request::new(request.clone())).await.unwrap().into_inner();
    let accepted = receipt(&service, &root, id).await;
    let session = Uuid::parse_str(accepted["session"].as_str().unwrap()).unwrap();
    let ws = state.workspaces.get(&root).await.unwrap();
    let persisted = load_thread(&ws.db, &session).unwrap().unwrap();
    assert_eq!(persisted.context.messages.last().unwrap().text_content(), "manual input\r\n中文");
    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.kind, "delivery");
    let mut replay =
        Daemon::send_chat(&service, Request::new(request.clone())).await.unwrap().into_inner();
    let mut replay_kinds = vec![];
    while let Some(event) = replay.next().await {
        replay_kinds.push(event.unwrap().kind);
    }
    assert_eq!(replay_kinds, vec!["delivery", "delivery_replay"]);
    while let Some(event) = stream.next().await {
        assert_ne!(event.unwrap().kind, "error");
    }
    let finished = receipt(&service, &root, id).await;
    assert_eq!(finished["phase"], "finished");
    assert_eq!(finished["turn"], accepted["turn"]);
    assert_eq!(load_thread(&ws.db, &session).unwrap().unwrap().turns, 1);
    let mut late = Daemon::send_chat(&service, Request::new(request)).await.unwrap().into_inner();
    assert_eq!(late.next().await.unwrap().unwrap().kind, "delivery");
    assert_eq!(load_thread(&ws.db, &session).unwrap().unwrap().turns, 1);
}

#[tokio::test]
async fn changed_input_and_stale_revision_are_rejected_without_advancing_the_session() {
    let (service, state, root) = setup().await;
    let id = Uuid::new_v4();
    let request = command(&root, id, "", 0);
    let mut stream =
        Daemon::send_chat(&service, Request::new(request.clone())).await.unwrap().into_inner();
    while let Some(event) = stream.next().await {
        event.unwrap();
    }
    let accepted = receipt(&service, &root, id).await;
    let session = Uuid::parse_str(accepted["session"].as_str().unwrap()).unwrap();
    let ws = state.workspaces.get(&root).await.unwrap();
    let revision = ws.db.chat_revision(session.as_bytes()).unwrap();
    let mut changed = request;
    changed.message = "different input".into();
    assert!(
        Daemon::send_chat(&service, Request::new(changed))
            .await
            .err()
            .unwrap()
            .message()
            .contains("request_conflict")
    );
    let rejected = Uuid::new_v4();
    assert!(
        Daemon::send_chat(
            &service,
            Request::new(command(&root, rejected, &session.to_string(), 0))
        )
        .await
        .is_err()
    );
    assert!(receipt(&service, &root, rejected).await.is_null());
    assert_eq!(ws.db.chat_revision(session.as_bytes()).unwrap(), revision);
}

#[tokio::test]
async fn accepted_input_and_continuation_expected_revision_commit_together() {
    let (service, _state, root) = setup().await;
    let id = Uuid::new_v4();
    let mut request = command(&root, id, "", 0);
    let mut options: serde_json::Value = serde_json::from_str(&request.options_json).unwrap();
    options["continuation_policy"] = serde_json::json!({"detached":false,"max_turns":2,"token_limit":100000,"expires_ms":chrono::Utc::now().timestamp_millis()+60000,"purpose":"bounded fixture"});
    request.options_json = options.to_string();
    let mut stream = Daemon::send_chat(&service, Request::new(request)).await.unwrap().into_inner();
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        assert_ne!(event.kind, "error", "{}", event.content);
    }
    assert_eq!(receipt(&service, &root, id).await["phase"], "finished");
}

#[tokio::test]
async fn unreceived_identity_retirement_fences_a_late_send_without_creating_a_session() {
    let (service, state, root) = setup().await;
    let id = Uuid::new_v4();
    let result = Daemon::chat_delivery(
        &service,
        Request::new(ResourceRequest {
            workspace_path: root.to_string_lossy().into(),
            schema_version: 1,
            payload_json: serde_json::json!({"operation":"retire","id":id}).to_string(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(result.error_json.is_empty());
    assert!(
        Daemon::send_chat(&service, Request::new(command(&root, id, "", 0)))
            .await
            .err()
            .unwrap()
            .message()
            .contains("delivery_retired")
    );
    let ws = state.workspaces.get(&root).await.unwrap();
    assert!(metteur_daemon::chat::session::list_threads(&ws.db).unwrap().is_empty());
}

#[tokio::test]
async fn new_chat_identity_is_bound_before_acceptance_and_invalid_model_is_not_accepted() {
    let (service, _state, root) = setup().await;
    let id = Uuid::new_v4();
    let session = Uuid::new_v4();
    let mut request = command(&root, id, "", 0);
    let mut options: serde_json::Value = serde_json::from_str(&request.options_json).unwrap();
    options["delivery"]["new_session"] = serde_json::json!(session);
    request.options_json = options.to_string();
    let mut stream = Daemon::send_chat(&service, Request::new(request)).await.unwrap().into_inner();
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        assert_ne!(event.kind, "error", "{}", event.content);
    }
    assert_eq!(receipt(&service, &root, id).await["session"], session.to_string());
    let rejected = Uuid::new_v4();
    let mut request = command(&root, rejected, "", 0);
    let mut options: serde_json::Value = serde_json::from_str(&request.options_json).unwrap();
    options["provider"] = serde_json::json!("unconfigured-provider");
    options["model"] = serde_json::json!("deleted-model");
    request.options_json = options.to_string();
    assert!(
        Daemon::send_chat(&service, Request::new(request))
            .await
            .err()
            .unwrap()
            .message()
            .contains("Selected model is no longer configured")
    );
    assert!(receipt(&service, &root, rejected).await.is_null());
}

#[tokio::test]
async fn host_backed_pending_inputs_are_reconciled_before_another_browser_can_resubmit() {
    let (service, state, root) = setup().await;
    let ws = state.workspaces.get(&root).await.unwrap();
    let recovery = metteur_daemon::storage::recovery::RecoveryStore::new(
        ws.db.clone(),
        &format!("project:{}", ws.root().to_string_lossy()),
        "local",
    )
    .unwrap();
    let unreceived = Uuid::new_v4();
    let received = Uuid::new_v4();
    let mut stream = Daemon::send_chat(&service, Request::new(command(&root, received, "", 0)))
        .await
        .unwrap()
        .into_inner();
    while let Some(event) = stream.next().await {
        assert_ne!(event.unwrap().kind, "error");
    }
    let original_receipt = receipt(&service, &root, received).await;
    let mut copies = vec![];
    for (kind, submission) in
        [("chat", unreceived), ("new_chat", received), ("supervision", Uuid::new_v4())]
    {
        let draft = metteur_daemon::storage::recovery::DraftInput {
            id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            kind: kind.into(),
            entity: Uuid::new_v4().to_string(),
            edit_sequence: 1,
            raw: "unmodified raw\r\n中文".into(),
            baseline_identity: None,
            selections: vec![],
            pending_submission: Some(submission),
            selected_model: None,
            view: None,
        };
        copies.push(draft.id);
        recovery.backup(draft, 0, Uuid::new_v4(), 1).unwrap();
    }
    let result = Daemon::chat_delivery(
        &service,
        Request::new(ResourceRequest {
            workspace_path: root.to_string_lossy().into(),
            schema_version: 1,
            payload_json: serde_json::json!({"operation":"reconcile_pending"}).to_string(),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(result.error_json.is_empty(), "{}", result.error_json);
    let result: serde_json::Value = serde_json::from_str(&result.payload_json).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 2);
    assert_eq!(receipt(&service, &root, unreceived).await["phase"], "retired");
    assert_eq!(receipt(&service, &root, received).await["turn"], original_receipt["turn"]);
    assert_eq!(receipt(&service, &root, received).await["phase"], "finished");
    assert!(
        Daemon::send_chat(&service, Request::new(command(&root, unreceived, "", 0))).await.is_err()
    );
    for id in copies {
        assert_eq!(recovery.get(id).unwrap().unwrap().input.raw, "unmodified raw\r\n中文");
    }
}

#[tokio::test]
async fn receipt_reconciliation_reports_a_deleted_source_without_recreating_it() {
    let (service, state, root) = setup().await;
    let id = Uuid::new_v4();
    let mut stream = Daemon::send_chat(&service, Request::new(command(&root, id, "", 0)))
        .await
        .unwrap()
        .into_inner();
    while let Some(event) = stream.next().await {
        assert_ne!(event.unwrap().kind, "error");
    }
    let query = || ResourceRequest {
        workspace_path: root.to_string_lossy().into(),
        schema_version: 1,
        payload_json: serde_json::json!({"operation":"read","id":id}).to_string(),
        ..Default::default()
    };
    let before = Daemon::chat_delivery(&service, Request::new(query())).await.unwrap().into_inner();
    let before: serde_json::Value = serde_json::from_str(&before.payload_json).unwrap();
    assert_eq!(before["source_available"], true);
    let session = before["receipt"]["session"].as_str().unwrap();
    Daemon::delete_chat_session(
        &service,
        Request::new(metteur_proto::proto::DeleteChatSessionRequest {
            workspace_path: root.to_string_lossy().into(),
            session_id: session.into(),
        }),
    )
    .await
    .unwrap();
    let after = Daemon::chat_delivery(&service, Request::new(query())).await.unwrap().into_inner();
    let after: serde_json::Value = serde_json::from_str(&after.payload_json).unwrap();
    assert_eq!(after["source_available"], false);
    assert_eq!(after["receipt"]["turn"], before["receipt"]["turn"]);
    let ws = state.workspaces.get(&root).await.unwrap();
    assert!(load_thread(&ws.db, &Uuid::parse_str(session).unwrap()).unwrap().is_none());
}
