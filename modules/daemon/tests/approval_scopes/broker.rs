use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use metteur_daemon::sandbox::approval::{
    ApprovalBroker, ApprovalFailure, Decision, Scope, parse_decision,
};
use metteur_daemon::sandbox::grant::GrantStore;

use super::common::db;

#[test]
fn parses_exactly_the_eight_user_choices() {
    for (prefix, decision) in [("Allow", Decision::Allow), ("Deny", Decision::Deny)] {
        for (suffix, scope) in [
            ("Once", Scope::Once),
            ("Run", Scope::Run),
            ("Workspace", Scope::Workspace),
            ("Global", Scope::Global),
        ] {
            assert_eq!(parse_decision(&format!("{prefix}{suffix}")), Some((decision, scope)));
        }
    }
    for invalid in ["", "allow", "AllowSometimes", "AllowOnce Run", "Deny", "AllowOnce "] {
        assert_eq!(parse_decision(invalid), None);
    }
}

#[tokio::test]
async fn every_scope_preserves_response_and_only_its_own_grant() {
    for decision in [Decision::Allow, Decision::Deny] {
        for scope in [Scope::Once, Scope::Run, Scope::Workspace, Scope::Global] {
            let broker = ApprovalBroker::new();
            let ws = db("scope-ws");
            let global = db("scope-global");
            let store = GrantStore::new(Some(ws.clone()), Some(global.clone()));
            let request = broker
                .open_request(
                    7,
                    "git status",
                    Duration::from_secs(5),
                    Arc::new(AtomicBool::new(false)),
                )
                .unwrap();
            let id = request.id().to_string();
            assert_eq!(id.len(), 36);
            broker.respond(&id, decision, scope, &store).unwrap();
            let response = request.wait().await.unwrap();
            assert_eq!(response.request_id, id);
            assert_eq!(response.decision, decision);
            assert_eq!(response.scope, scope);
            let allow = decision == Decision::Allow;
            assert_eq!(broker.run_grant(7), (scope == Scope::Run).then_some(allow));
            assert_eq!(
                GrantStore::new(Some(ws), None).lookup(7),
                (scope == Scope::Workspace).then_some(allow)
            );
            assert_eq!(
                GrantStore::new(None, Some(global)).lookup(7),
                (scope == Scope::Global).then_some(allow)
            );
            assert_eq!(ApprovalBroker::new().run_grant(7), None);
            // A changed choice for the same consumed id cannot widen its scope.
            assert!(broker.respond(&id, Decision::Allow, Scope::Global, &store).is_err());
        }
    }
}

#[tokio::test]
async fn duplicate_or_changed_responses_do_not_replace_a_consumed_decision() {
    let broker = ApprovalBroker::new();
    let store = GrantStore::new(Some(db("duplicate")), None);
    let request = broker
        .open_request(7, "first content", Duration::from_secs(5), Arc::new(AtomicBool::new(false)))
        .unwrap();
    let id = request.id().to_string();
    broker.respond(&id, Decision::Deny, Scope::Once, &store).unwrap();
    assert!(broker.respond(&id, Decision::Allow, Scope::Workspace, &store).is_err());
    let changed = broker
        .open_request(
            7,
            "changed content",
            Duration::from_secs(5),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
    assert_ne!(changed.id(), id);
    assert_eq!(request.wait().await.unwrap().decision, Decision::Deny);
    assert_eq!(store.lookup(7), None);
    assert_eq!(broker.run_grant(7), None);
    assert_eq!(broker.pending_ids(), vec![changed.id().to_string()]);
}

#[tokio::test]
async fn expiry_rejects_late_persistent_responses_even_before_wait_is_polled() {
    let broker = ApprovalBroker::new();
    let store = GrantStore::new(Some(db("expired")), None);
    let request =
        broker.open_request(7, "x", Duration::ZERO, Arc::new(AtomicBool::new(false))).unwrap();
    assert!(broker.respond(request.id(), Decision::Allow, Scope::Workspace, &store).is_err());
    assert_eq!(request.wait().await, Err(ApprovalFailure::Expired));
    assert_eq!(store.lookup(7), None);
    assert_eq!(broker.run_grant(7), None);
}

#[tokio::test]
async fn a_wait_timeout_withdraws_the_request_without_a_denial_grant() {
    let broker = ApprovalBroker::new();
    let request = broker
        .open_request(7, "x", Duration::from_millis(1), Arc::new(AtomicBool::new(false)))
        .unwrap();
    let id = request.id().to_string();
    assert_eq!(request.wait().await, Err(ApprovalFailure::Expired));
    assert!(broker.pending_ids().is_empty());
    assert!(broker.respond(&id, Decision::Allow, Scope::Run, &Default::default()).is_err());
    assert_eq!(broker.run_grant(7), None);
}

#[tokio::test]
async fn dropped_cancelled_and_closed_requests_never_persist() {
    for kind in ["drop", "cancel", "close"] {
        let broker = ApprovalBroker::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        let store = GrantStore::new(None, Some(db(kind)));
        let request =
            broker.open_request(7, "x", Duration::from_secs(5), cancelled.clone()).unwrap();
        let id = request.id().to_string();
        match kind {
            "drop" => drop(request),
            "cancel" => {
                cancelled.store(true, Ordering::SeqCst);
                assert!(broker.respond(&id, Decision::Allow, Scope::Global, &store).is_err());
                assert_eq!(request.wait().await, Err(ApprovalFailure::Cancelled));
            }
            _ => {
                broker.record_run_grant(8, true);
                broker.close();
                assert_eq!(request.wait().await, Err(ApprovalFailure::Closed));
                assert!(broker.open_request(9, "new", Duration::from_secs(5), cancelled).is_err());
                assert_eq!(broker.run_grant(8), None);
            }
        }
        assert!(broker.respond(&id, Decision::Allow, Scope::Global, &store).is_err());
        assert_eq!(store.lookup(7), None);
    }
}

#[tokio::test]
async fn failed_persistence_does_not_wake_an_allowed_waiter_or_cache_a_grant() {
    for scope in [Scope::Workspace, Scope::Global] {
        let broker = ApprovalBroker::new();
        let request = broker
            .open_request(7, "x", Duration::from_secs(5), Arc::new(AtomicBool::new(false)))
            .unwrap();
        let id = request.id().to_string();
        assert!(broker.respond(&id, Decision::Allow, scope, &GrantStore::default()).is_err());
        assert_eq!(request.wait().await, Err(ApprovalFailure::Persistence));
        assert_eq!(broker.run_grant(7), None);
        assert!(broker.respond(&id, Decision::Allow, Scope::Once, &Default::default()).is_err());
    }
}

#[tokio::test]
async fn run_guard_invalidates_even_a_response_already_delivered() {
    let broker = Arc::new(ApprovalBroker::new());
    let guard = broker.close_on_drop();
    let request = broker
        .open_request(7, "x", Duration::from_secs(5), Arc::new(AtomicBool::new(false)))
        .unwrap();
    broker.respond(request.id(), Decision::Allow, Scope::Run, &Default::default()).unwrap();
    drop(guard);
    assert_eq!(request.wait().await, Err(ApprovalFailure::Closed));
    assert_eq!(broker.run_grant(7), None);
}

#[tokio::test]
async fn workspace_precedence_and_global_visibility_are_unchanged() {
    let ws = db("precedence");
    let global = db("global-visibility");
    let first = GrantStore::new(Some(ws), Some(global.clone()));
    let other = GrantStore::new(Some(db("other-ws")), Some(global));
    first.store(Scope::Global, 7, Decision::Allow, "git status").unwrap();
    first.store(Scope::Workspace, 7, Decision::Deny, "git status").unwrap();
    assert_eq!(first.lookup(7), Some(false));
    assert_eq!(other.lookup(7), Some(true));
    assert_eq!(other.lookup(8), None);
}
