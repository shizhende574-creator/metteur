use std::sync::Arc;
use std::time::Duration;

use metteur_daemon::sandbox::approval::{ApprovalBroker, Decision, Scope};
use metteur_daemon::sandbox::{authorize, authorize_write};

use super::common::{answer, context, grants};

#[tokio::test]
async fn once_decisions_reprompt_for_both_commands_and_writes() {
    for decision in [Decision::Allow, Decision::Deny] {
        let (ctx, mut events) = context();
        let broker = ctx.approvals.as_ref().unwrap();
        let store = grants(&ctx);
        let path = ctx.workspace_root.join("file.txt");
        for _ in 0..2 {
            let (result, _) = tokio::join!(
                authorize(&ctx, "echo scoped"),
                answer(&mut events, broker, &store, decision, Scope::Once),
            );
            assert_eq!(result.unwrap(), decision == Decision::Allow);
            let (result, _) = tokio::join!(
                authorize_write(&ctx, &path, "file.txt", "write bytes"),
                answer(&mut events, broker, &store, decision, Scope::Once),
            );
            assert_eq!(result.unwrap(), decision == Decision::Allow);
        }
    }
}

#[tokio::test]
async fn run_decisions_cover_repeats_but_not_a_new_run() {
    for decision in [Decision::Allow, Decision::Deny] {
        let (mut ctx, mut events) = context();
        let store = grants(&ctx);
        let path = ctx.workspace_root.join("file.txt");
        for _ in 0..2 {
            let broker = ctx.approvals.as_ref().unwrap();
            let (result, _) = tokio::join!(
                authorize(&ctx, "echo scoped"),
                answer(&mut events, broker, &store, decision, Scope::Run),
            );
            assert_eq!(result.unwrap(), decision == Decision::Allow);
            let (result, _) = tokio::join!(
                authorize_write(&ctx, &path, "file.txt", "write bytes"),
                answer(&mut events, broker, &store, decision, Scope::Run),
            );
            assert_eq!(result.unwrap(), decision == Decision::Allow);
            assert_eq!(
                tokio::time::timeout(Duration::from_millis(100), authorize(&ctx, "echo scoped"))
                    .await
                    .unwrap()
                    .unwrap(),
                decision == Decision::Allow
            );
            assert_eq!(
                tokio::time::timeout(
                    Duration::from_millis(100),
                    authorize_write(&ctx, &path, "file.txt", "different bytes")
                )
                .await
                .unwrap()
                .unwrap(),
                decision == Decision::Allow
            );
            assert!(events.try_recv().is_err());
            broker.close();
            ctx.approvals = Some(Arc::new(ApprovalBroker::new()));
        }
    }
}

#[tokio::test]
async fn persistent_scopes_survive_a_new_broker_and_keep_workspace_boundaries() {
    for scope in [Scope::Workspace, Scope::Global] {
        for decision in [Decision::Allow, Decision::Deny] {
            let (mut ctx, mut events) = context();
            let store = grants(&ctx);
            let (result, _) = tokio::join!(
                authorize(&ctx, "echo scoped"),
                answer(&mut events, ctx.approvals.as_ref().unwrap(), &store, decision, scope),
            );
            assert_eq!(result.unwrap(), decision == Decision::Allow);
            ctx.approvals.as_ref().unwrap().close();
            ctx.approvals = Some(Arc::new(ApprovalBroker::new()));
            assert_eq!(authorize(&ctx, "echo scoped").await.unwrap(), decision == Decision::Allow);
            assert!(events.try_recv().is_err());
            ctx.workspace_db = Some(super::common::db("other-workspace"));
            if scope == Scope::Global {
                assert_eq!(
                    authorize(&ctx, "echo scoped").await.unwrap(),
                    decision == Decision::Allow
                );
                assert!(events.try_recv().is_err());
            } else {
                let other_store = grants(&ctx);
                let (result, _) = tokio::join!(
                    authorize(&ctx, "echo scoped"),
                    answer(
                        &mut events,
                        ctx.approvals.as_ref().unwrap(),
                        &other_store,
                        Decision::Deny,
                        Scope::Once
                    ),
                );
                assert!(!result.unwrap());
            }
        }
    }
}

#[tokio::test]
async fn timeout_and_channel_loss_do_not_cache_a_user_denial() {
    let (ctx, mut events) = context();
    assert!(!authorize(&ctx, "echo scoped").await.unwrap());
    let expired = events.recv().await.unwrap();
    let metteur_daemon::execution::ExecutionEvent::ApprovalRequested {
        request_id,
        ..
    } = expired
    else {
        panic!("expected approval")
    };
    let broker = ctx.approvals.as_ref().unwrap();
    let store = grants(&ctx);
    assert!(broker.respond(&request_id, Decision::Allow, Scope::Global, &store).is_err());
    let (result, _) = tokio::join!(
        authorize(&ctx, "echo scoped"),
        answer(&mut events, broker, &store, Decision::Allow, Scope::Once),
    );
    assert!(result.unwrap());
    drop(events);
    assert!(!authorize(&ctx, "echo scoped").await.unwrap());
    assert!(broker.pending_ids().is_empty());
}

#[tokio::test]
async fn aborting_a_wait_withdraws_its_request() {
    let (ctx, mut events) = context();
    let broker = ctx.approvals.as_ref().unwrap().clone();
    let store = grants(&ctx);
    let task = tokio::spawn(async move { authorize(&ctx, "echo scoped").await });
    let event = events.recv().await.unwrap();
    let metteur_daemon::execution::ExecutionEvent::ApprovalRequested {
        request_id,
        ..
    } = event
    else {
        panic!("expected approval")
    };
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(broker.pending_ids().is_empty());
    assert!(broker.respond(&request_id, Decision::Allow, Scope::Global, &store).is_err());
}

#[tokio::test]
async fn file_persistent_scopes_reuse_the_existing_normalized_subject_key() {
    for scope in [Scope::Workspace, Scope::Global] {
        for decision in [Decision::Allow, Decision::Deny] {
            let (mut ctx, mut events) = context();
            let store = grants(&ctx);
            let path = ctx.workspace_root.join("file.txt");
            let (result, _) = tokio::join!(
                authorize_write(&ctx, &path, "file.txt", "first content"),
                answer(&mut events, ctx.approvals.as_ref().unwrap(), &store, decision, scope),
            );
            assert_eq!(result.unwrap(), decision == Decision::Allow);
            ctx.approvals.as_ref().unwrap().close();
            ctx.approvals = Some(Arc::new(ApprovalBroker::new()));
            assert_eq!(
                store.lookup(metteur_daemon::sandbox::command_hash("file.txt")),
                Some(decision == Decision::Allow)
            );
            assert_eq!(
                authorize_write(&ctx, &path, "file.txt", "new content").await.unwrap(),
                decision == Decision::Allow
            );
            assert!(events.try_recv().is_err());
        }
    }
}

#[tokio::test]
async fn explicit_approval_nodes_bind_scoped_decisions_to_original_content() {
    use metteur_daemon::replan::await_approval;
    for scope in [Scope::Once, Scope::Run, Scope::Workspace, Scope::Global] {
        let (mut ctx, mut events) = context();
        let broker = ctx.approvals.as_ref().unwrap().clone();
        let store = grants(&ctx);
        let detail = serde_json::json!({"request_type":"replan_proposal", "edits":["original"]});
        let (result, _) = tokio::join!(
            await_approval(&mut ctx, "replan_proposal", "review edit", detail.clone()),
            answer(&mut events, &broker, &store, Decision::Allow, scope),
        );
        assert!(result.unwrap());
        if scope == Scope::Once {
            let (result, _) = tokio::join!(
                await_approval(&mut ctx, "replan_proposal", "review edit", detail.clone()),
                answer(&mut events, &broker, &store, Decision::Deny, Scope::Once),
            );
            assert!(!result.unwrap());
        } else {
            assert!(
                tokio::time::timeout(
                    Duration::from_millis(100),
                    await_approval(&mut ctx, "replan_proposal", "review edit", detail)
                )
                .await
                .unwrap()
                .unwrap()
            );
            assert!(events.try_recv().is_err());
        }
        let (result, _) = tokio::join!(
            await_approval(
                &mut ctx,
                "replan_proposal",
                "review edit",
                serde_json::json!({"edits":["changed"]})
            ),
            answer(&mut events, &broker, &store, Decision::Deny, Scope::Once),
        );
        assert!(!result.unwrap());
        ctx.cancel_requested.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(
            await_approval(&mut ctx, "replan_proposal", "cancelled", serde_json::Value::Null)
                .await
                .is_err()
        );
    }
}
