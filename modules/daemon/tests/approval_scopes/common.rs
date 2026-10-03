use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use metteur_daemon::execution::{ExecutionEvent, context::ExecutionContext};
use metteur_daemon::llm::LlmClientFactory;
use metteur_daemon::registry::Registry;
use metteur_daemon::sandbox::PermissionMode;
use metteur_daemon::sandbox::approval::{ApprovalBroker, Decision, Scope};
use metteur_daemon::sandbox::grant::GrantStore;
use metteur_daemon::storage::persistence::Db;
use metteur_shared::config::{Config, SandboxConfig};
use tokio::sync::{RwLock, mpsc};

pub fn root(tag: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("metteur-approval-{tag}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

pub fn db(tag: &str) -> Db {
    Db::open(&root(tag)).unwrap()
}

pub fn context() -> (ExecutionContext, mpsc::UnboundedReceiver<ExecutionEvent>) {
    let mut ctx = ExecutionContext::new(
        Arc::new(Registry::with_builtins()),
        LlmClientFactory::new(),
        root("run"),
    );
    ctx.permission_mode = PermissionMode::Ask;
    ctx.config = Some(Arc::new(RwLock::new(Config {
        sandbox: SandboxConfig {
            enabled: true,
            approval_timeout_secs: 1,
            ..Default::default()
        },
        ..Default::default()
    })));
    ctx.approvals = Some(Arc::new(ApprovalBroker::new()));
    ctx.workspace_db = Some(db("workspace"));
    ctx.global_db = Some(db("global"));
    let (tx, rx) = mpsc::unbounded_channel();
    ctx.events = Some(tx);
    (ctx, rx)
}

pub fn grants(ctx: &ExecutionContext) -> GrantStore {
    GrantStore::new(ctx.workspace_db.clone(), ctx.global_db.clone())
}

pub async fn answer(
    events: &mut mpsc::UnboundedReceiver<ExecutionEvent>,
    broker: &ApprovalBroker,
    store: &GrantStore,
    decision: Decision,
    scope: Scope,
) -> String {
    let event = tokio::time::timeout(Duration::from_secs(5), events.recv()).await.unwrap().unwrap();
    let ExecutionEvent::ApprovalRequested {
        request_id,
        ..
    } = event
    else {
        panic!("expected approval request, got {event:?}");
    };
    broker.respond(&request_id, decision, scope, store).unwrap();
    request_id
}
