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
