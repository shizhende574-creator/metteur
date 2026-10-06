use super::*;
use crate::sandbox::approval::{ApprovalBroker, Decision, Scope};
use crate::storage::versioning::VersionManager;
use std::sync::Arc;

fn setup() -> (ExecutionContext, DbCheckpointSink, ExecutionCheckpoint) {
    let root = std::env::temp_dir().join(format!("metteur-apply-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db = Db::open(&root.join(".metteur/db")).unwrap();
    let versions = Arc::new(VersionManager::new(db.clone(), root.clone()));
    let registry = Arc::new(crate::registry::Registry::with_builtins());
    let graph = metteur_shared::dsl::compile_draft_value_with_catalog(&serde_json::json!({
        "name":"apply", "nodes":{"s":{"kind":"Start"},"llm":{"kind":"CallLLM","prompt":"old"},"e":{"kind":"End"}},"flow":["s -> llm -> e"]
    }), &registry.authoring_catalog()).unwrap();
    let version = blueprint_files::save(
        &db,
        &versions,
        &graph,
        "plan.blueprint",
        &serde_json::to_vec(&graph).unwrap(),
        None,
    )
    .unwrap();
    let mut cp = ExecutionCheckpoint::running(Uuid::new_v4(), graph.id, 1);
    cp.blueprint_version = Some(version);
    cp.pending = vec![graph.entry_node_id];
    let sink = DbCheckpointSink::new(db.clone(), cp.run_id);
    sink.write(&cp).unwrap();
    let mut ctx = ExecutionContext::new(registry, crate::llm::LlmClientFactory::new(), root)
        .with_run(cp.run_id, 1);
    ctx.blueprint = Some(Arc::new(parking_lot::RwLock::new(graph)));
    ctx.version_manager = Some(versions);
    ctx.workspace_db = Some(db);
    ctx.approvals = Some(Arc::new(ApprovalBroker::new()));
    attach(&ctx, None).unwrap();
    (ctx, sink, cp)
}
fn edits() -> serde_json::Value {
    serde_json::json!([{"op":"set_pin","match":{"kind":"CallLLM"},"pin":"prompt","value":"new"}])
}
async fn answer(broker: Arc<ApprovalBroker>, decision: Decision) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if let Some(id) = broker.pending_ids().first() {
                broker.respond(id, decision, Scope::Once, &Default::default()).unwrap();
                assert!(broker.respond(id, decision, Scope::Once, &Default::default()).is_err());
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
async fn approved(ctx: &mut ExecutionContext) {
    let broker = ctx.approvals.clone().unwrap();
    let script = edits();
    let (result, ()) = tokio::join!(
        approve(ctx, "change prompt", &script, Source::ModelTool),
        answer(broker, Decision::Allow)
    );
    assert!(result.unwrap().contains("pending"));
}

#[tokio::test]
async fn approval_stages_once_and_commits_file_graph_checkpoint_together() {
    let (mut ctx, sink, mut cp) = setup();
    let before = ctx.blueprint.as_ref().unwrap().read().clone();
    approved(&mut ctx).await;
    assert_eq!(*ctx.blueprint.as_ref().unwrap().read(), before);
    let (events, mut rx) = tokio::sync::mpsc::unbounded_channel();
    ctx.events = Some(events);
    commit_boundary(&ctx, &mut cp, &sink).unwrap();
    let after = ctx.blueprint.as_ref().unwrap().read().clone();
    assert_ne!(before, after);
    assert_eq!(
        blueprint_files::decode(&std::fs::read(ctx.workspace_root.join("plan.blueprint")).unwrap())
            .unwrap(),
        after
    );
    assert_eq!(
        DbCheckpointSink::load(db(&ctx).unwrap(), ctx.run_id).unwrap().unwrap().blueprint_version,
        cp.blueprint_version
    );
    assert_eq!(
        cp.blueprint_version,
        blueprint_files::binding(db(&ctx).unwrap(), after.id).unwrap()
    );
    assert!(
        matches!(rx.try_recv().unwrap(),crate::execution::ExecutionEvent::Message { message,.. } if message.starts_with("replan.applied"))
    );
    let count = ctx.version_manager.as_ref().unwrap().list_snapshots().unwrap().len();
    commit_boundary(&ctx, &mut cp, &sink).unwrap();
    assert!(rx.try_recv().is_err());
    assert_eq!(ctx.version_manager.as_ref().unwrap().list_snapshots().unwrap().len(), count);
    ensure_resolved(db(&ctx).unwrap()).unwrap();
    assert_eq!(DbCheckpointSink::list(db(&ctx).unwrap()).unwrap().len(), 1);
    ctx.attach_file_journal();
    ctx.transaction_log.validate_resume(ctx.run_id).unwrap();
    assert_eq!(ctx.transaction_log.rollback_after(0).unwrap(), 0);
    assert_eq!(
        blueprint_files::decode(&std::fs::read(ctx.workspace_root.join("plan.blueprint")).unwrap())
            .unwrap(),
        after
    );
}

#[tokio::test]
async fn stale_file_graph_and_execution_state_invalidate_waiting_approval() {
    for mode in 0..3 {
        let (mut ctx, sink, mut cp) = setup();
        let graph = ctx.blueprint.clone().unwrap();
        let broker = ctx.approvals.clone().unwrap();
        let path = ctx.workspace_root.join("plan.blueprint");
        let script = edits();
        let (result, ()) =
            tokio::join!(approve(&mut ctx, "change", &script, Source::ModelTool), async {
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while broker.pending_ids().is_empty() {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    }
                })
                .await
                .unwrap();
                match mode {
                    0 => std::fs::write(path, "external edit").unwrap(),
                    1 => graph.write().name = "concurrent graph".into(),
                    _ => {
                        cp.pending.clear();
                        sink.write(&cp).unwrap();
                    }
                }
                answer(broker, Decision::Allow).await;
            });
        assert!(result.is_err());
        assert!(ctx.blueprint_apply.lock().pending.is_none());
        assert!(
            db(&ctx)
                .unwrap()
                .scan(cf::EXECUTION_STATE)
                .unwrap()
                .iter()
                .all(|(key, _)| !key.starts_with(INTENT_PREFIX))
        );
    }
}

#[tokio::test]
async fn denied_and_completed_node_proposals_have_no_effects() {
    let (mut ctx, sink, mut cp) = setup();
    let original = std::fs::read(ctx.workspace_root.join("plan.blueprint")).unwrap();
    let broker = ctx.approvals.clone().unwrap();
    let script = edits();
    let (result, ()) = tokio::join!(
        approve(&mut ctx, "change", &script, Source::ModelTool),
        answer(broker, Decision::Deny)
    );
    assert!(result.is_err());
    cp.executed.push(
        ctx.blueprint
            .as_ref()
            .unwrap()
            .read()
            .nodes
            .iter()
            .find(|n| n.kind == "CallLLM")
            .unwrap()
            .id,
    );
    sink.write(&cp).unwrap();
    assert!(approve(&mut ctx, "change", &script, Source::ModelTool).await.is_err());
    assert_eq!(std::fs::read(ctx.workspace_root.join("plan.blueprint")).unwrap(), original);
}

struct FailingSink(Uuid);
impl CheckpointSink for FailingSink {
    fn run_id(&self) -> Uuid {
        self.0
    }
    fn write(&self, _: &ExecutionCheckpoint) -> DaemonResult<()> {
        Err(DaemonError::Persistence("injected checkpoint failure".into()))
    }
}
#[tokio::test]
async fn checkpoint_failure_leaves_recovery_intent_and_never_reports_applied() {
    let (mut ctx, sink, mut cp) = setup();
    approved(&mut ctx).await;
    let (events, mut rx) = tokio::sync::mpsc::unbounded_channel();
    ctx.events = Some(events);
    assert!(commit_boundary(&ctx, &mut cp, &FailingSink(ctx.run_id)).is_err());
    assert!(ensure_resolved(db(&ctx).unwrap()).is_err());
    assert!(commit_boundary(&ctx, &mut cp, &sink).is_err());
    assert!(rx.try_recv().is_err());
    assert_ne!(
        DbCheckpointSink::load(db(&ctx).unwrap(), ctx.run_id).unwrap().unwrap().blueprint_version,
        cp.blueprint_version
    );
}

#[tokio::test]
async fn file_storage_failure_blocks_without_publishing_a_new_graph() {
    let (mut ctx, sink, mut cp) = setup();
    approved(&mut ctx).await;
    let original = ctx.blueprint.as_ref().unwrap().read().clone();
    let next = ctx.blueprint_apply.lock().pending.as_ref().unwrap().file.clone();
    let hash = crate::storage::versioning::hash_content(&next);
    db(&ctx).unwrap().put(cf::FILE_BLOBS, hash.as_bytes(), b"injected collision").unwrap();
    assert!(commit_boundary(&ctx, &mut cp, &sink).is_err());
    assert_eq!(*ctx.blueprint.as_ref().unwrap().read(), original);
    assert!(ensure_resolved(db(&ctx).unwrap()).is_err());
}

#[tokio::test]
async fn circuit_mock_uses_the_same_pending_and_commit_protocol() {
    let (mut ctx, sink, mut cp) = setup();
    let broker = ctx.approvals.clone().unwrap();
    let script = edits().to_string();
    let (result, ()) = tokio::join!(
        crate::replan::trip_and_replan_mock(&mut ctx, Uuid::nil(), 3, script),
        async {
            answer(broker.clone(), Decision::Allow).await;
            answer(broker, Decision::Allow).await;
        }
    );
    result.unwrap();
    assert!(ctx.blueprint_apply.lock().pending.is_some());
    commit_boundary(&ctx, &mut cp, &sink).unwrap();
    ensure_resolved(db(&ctx).unwrap()).unwrap();
}

#[tokio::test]
async fn interpreter_applies_before_the_next_node_reads_its_inputs() {
    let (ctx, _, _) = setup();
    let graph = metteur_shared::dsl::compile_draft_value_with_catalog(&serde_json::json!({
        "name":"boundary", "nodes":{
            "s":{"kind":"Start"},
            "r":{"kind":"ReplanBlueprint","summary":"change A","edits":[{"op":"set_pin","match":{"kind":"Add"},"pin":"A","value":9}]},
            "a":{"kind":"Add","A":2,"B":3},"e":{"kind":"End"}},
        "flow":["s -> r -> a -> e"]
    }),&ctx.registry.authoring_catalog()).unwrap();
    let db = db(&ctx).unwrap().clone();
    let versions = ctx.version_manager.clone().unwrap();
    blueprint_files::save(
        &db,
        &versions,
        &graph,
        "boundary.blueprint",
        &serde_json::to_vec(&graph).unwrap(),
        None,
    )
    .unwrap();
    let sink = Arc::new(DbCheckpointSink::new(db.clone(), Uuid::new_v4()));
    let broker = ctx.approvals.clone().unwrap();
    let shared = Arc::new(parking_lot::RwLock::new(graph));
    let mut interpreter = crate::execution::Interpreter::new(
        ctx.registry.clone(),
        ctx.llm_factory.clone(),
        ctx.workspace_root.clone(),
    )
    .with_checkpoint_sink(sink.clone())
    .with_workspace_db(db.clone())
    .with_version_manager(versions)
    .with_approvals(broker.clone());
    let (result, ()) = tokio::join!(
        async {
            let r = interpreter.run(&shared, None).await;
            assert!(r.is_ok(), "{r:?}");
            r
        },
        answer(broker, Decision::Allow)
    );
    let events = result.unwrap();
    assert!(events.iter().any(|e| matches!(e,crate::execution::ExecutionEvent::NodeData { outputs,.. } if outputs.iter().any(|(_,v)| v.as_float() == Some(12.0)))));
    let cp = DbCheckpointSink::load(&db, sink.run_id()).unwrap().unwrap();
    assert_eq!(cp.status, RunStatus::Completed);
    assert_eq!(cp.blueprint_version, blueprint_files::binding(&db, shared.read().id).unwrap());
}
