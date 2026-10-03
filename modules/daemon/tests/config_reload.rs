use std::{path::PathBuf, sync::Arc};

use metteur_daemon::{
    execution::ExecutionContext,
    grpc::{
        AppState, DaemonService,
        proto::{SetConfigRequest, daemon_server::Daemon},
    },
    integration::mcp::McpHost,
    llm::LlmClientFactory,
    observability::metrics::Metrics,
    registry::Registry,
    workspace::WorkspaceManager,
};
use metteur_shared::config::Config;
use serde_json::json;
use tonic::Request;

async fn setup() -> (DaemonService, Arc<AppState>, PathBuf) {
    let root = std::env::temp_dir().join(format!("metteur-config-reload-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let registry = Arc::new(Registry::with_builtins());
    let host = McpHost::new(registry.clone(), Arc::new(Metrics::default()));
    let state = Arc::new(
        AppState::new(
            WorkspaceManager::new().with_global_config_path(root.join("global.toml")),
            registry,
            Config::default(),
        )
        .with_mcp_host(host),
    );
    state.workspaces.open(&root).await.unwrap();
    (DaemonService::new(state.clone()), state, root)
}

async fn save(
    service: &DaemonService,
    path: &str,
    value: serde_json::Value,
) -> Result<(), tonic::Status> {
    Daemon::set_config(
        service,
        Request::new(SetConfigRequest {
            workspace_path: path.into(),
            config_json: value.to_string(),
        }),
    )
    .await
    .map(|_| ())
}

#[tokio::test]
async fn global_and_workspace_lsp_reload_reach_existing_and_child_contexts() {
    let (service, state, root) = setup().await;
    let ws = state.workspaces.get(&root).await.unwrap();
    save(&service, &root.to_string_lossy(), json!({"config_version":2})).await.unwrap();
    let mut ctx =
        ExecutionContext::new(state.registry.clone(), LlmClientFactory::new(), root.clone())
            .with_config(ws.config.clone());
    ctx.lsp_source = Some(ws.lsp_manager.clone());
    let child = ctx.child_nested();
    assert!(ctx.lsp_manager().is_none());
    save(&service, "", json!({"config_version":2,"llm":{"default_model":"updated"},"sandbox":{"mode":"ask"},"lsp":{"enabled":true,"debounce_ms":71,"languages":[{"id":"rust","command":["missing-test-language-server"],"extensions":["rs"]}]}})).await.unwrap();
    assert_eq!(
        ctx.config.as_ref().unwrap().read().await.llm.default_model.as_deref(),
        Some("updated")
    );
    assert_eq!(ctx.config.as_ref().unwrap().read().await.sandbox.mode, "ask");
    assert_eq!(child.lsp_manager().unwrap().debounce().as_millis(), 71);
    let old = ctx.lsp_manager().unwrap();
    save(&service, &root.to_string_lossy(), json!({"config_version":2,"lsp":{"debounce_ms":0}}))
        .await
        .unwrap();
    let current = ctx.lsp_manager().unwrap();
    assert!(!Arc::ptr_eq(&old, &current));
    assert_eq!(current.debounce().as_millis(), 0);
    assert_eq!(child.lsp_manager().unwrap().debounce().as_millis(), 0);
    let error = current.client_for_extension("rs").await.err().unwrap();
    assert!(error.to_string().contains("failed to start rust"));
    save(&service, &root.to_string_lossy(), json!({"config_version":2,"lsp":{"enabled":false}}))
        .await
        .unwrap();
    assert!(ctx.lsp_manager().is_none());
    assert!(child.lsp_manager().is_none());
}

#[tokio::test]
async fn failed_workspace_merge_does_not_partially_save_global_config() {
    let (service, state, root) = setup().await;
    save(&service, "", json!({"config_version":2,"llm":{"default_model":"before"}})).await.unwrap();
    let before = std::fs::read(root.join("global.toml")).unwrap();
    std::fs::write(root.join(".metteur/config.toml"), "[invalid").unwrap();
    assert!(
        save(&service, "", json!({"config_version":2,"llm":{"default_model":"after"}}))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(root.join("global.toml")).unwrap(), before);
    assert_eq!(state.global_config.read().await.llm.default_model.as_deref(), Some("before"));
}

#[tokio::test]
async fn mcp_failure_is_visible_and_explicit_workspace_alias_wins() {
    let (service, state, root) = setup().await;
    let bad = json!({"config_version":2,"mcp":{"servers":{"broken":{"enabled":true,"command":["missing-test-mcp-server"]}}}});
    let error = save(&service, &root.to_string_lossy(), bad).await.unwrap_err();
    assert!(error.message().contains("Configuration saved, but not fully applied"));
    assert!(error.message().contains("broken"));
    assert!(
        std::fs::read_to_string(root.join(".metteur/config.toml"))
            .unwrap()
            .contains("missing-test-mcp-server")
    );
    save(&service, &root.to_string_lossy(), json!({"config_version":2,"mcp":{"servers":{"same":{"enabled":false,"command":["workspace"]}}}})).await.unwrap();
    save(&service, "", json!({"config_version":2,"mcp":{"servers":{"same":{"enabled":false,"command":["global"]}}}})).await.unwrap();
    let other = root.join("other");
    std::fs::create_dir(&other).unwrap();
    state.workspaces.open(&other).await.unwrap();
    assert_eq!(state.merged_mcp_config().await.unwrap().servers["same"].command, vec!["workspace"]);
    state.resync_mcp().await.unwrap();
}

#[tokio::test]
async fn restart_boundaries_remain_visible_on_repeated_saves() {
    let (service, _, root) = setup().await;
    let config = json!({"config_version":2,"daemon":{"wake":true}});
    for _ in 0..2 {
        assert!(
            save(&service, "", config.clone())
                .await
                .unwrap_err()
                .message()
                .contains("require restart")
        );
    }
    assert!(std::fs::read_to_string(root.join("global.toml")).unwrap().contains("wake = true"));
    let invalid =
        json!({"config_version":2,"lsp":{"enabled":true,"languages":[{"id":"rust","command":[]}]}});
    let before = std::fs::read(root.join("global.toml")).unwrap();
    assert_eq!(save(&service, "", invalid).await.unwrap_err().code(), tonic::Code::InvalidArgument);
    assert_eq!(std::fs::read(root.join("global.toml")).unwrap(), before);
}

#[tokio::test]
async fn ambiguous_legacy_extensions_keep_raw_format_available_for_explicit_migration() {
    let (service, _, root) = setup().await;
    save(&service, "", json!({"oversight":{"triggers":{"interval_ms":500,"on_validation_failed":true}}})).await.unwrap();
    save(&service, &root.to_string_lossy(), json!({"oversight":{"triggers":{"interval_ms":0}}})).await.unwrap();
    let before = std::fs::read(root.join(".metteur/config.toml")).unwrap();
    let response = Daemon::get_config(&service, Request::new(metteur_daemon::grpc::proto::GetConfigRequest { workspace_path:root.to_string_lossy().into() })).await.unwrap().into_inner();
    assert!(response.legacy_format);
    assert!(response.overrides_json.is_empty());
    let raw: serde_json::Value = serde_json::from_str(&response.config_json).unwrap();
    assert_eq!(raw["oversight"]["triggers"], json!({"interval_ms":0}));
    assert_eq!(before, std::fs::read(root.join(".metteur/config.toml")).unwrap());
}
