//! Trusted proposal state. Neither identity, lineage nor decisions are model input.
use super::{
    requests::{self, State},
    scheduler::{self, Status},
};
use crate::{
    DaemonError, DaemonResult,
    execution::context::ExecutionContext,
    storage::persistence::{Db, cf},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub proposal_id: Uuid,
    pub review_id: Uuid,
    pub run_id: Uuid,
    pub source_request_ids: Vec<Uuid>,
    pub kind: String,
    pub state: State,
    pub binding: Value,
    pub result_refs: Vec<String>,
    pub reason: String,
}
fn error(text: &str) -> DaemonError {
    DaemonError::Execution(text.into())
}
fn encode(value: &impl Serialize) -> DaemonResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| DaemonError::Serialization(e.to_string()))
}
fn save(db: &Db, run: Uuid, s: &scheduler::Schedule, q: &requests::Queue) -> DaemonResult<()> {
    db.put_pair_durable(
        cf::EXECUTION_STATE,
        format!("oversight:schedule:{run}").as_bytes(),
        &encode(s)?,
        format!("oversight:requests:{run}").as_bytes(),
        &encode(q)?,
    )?;
    db.oversight_notify.notify_one();
    Ok(())
}
pub(crate) fn enabled(ctx: &ExecutionContext) -> DaemonResult<()> {
    let config = ctx
        .config
        .as_ref()
        .ok_or_else(|| error("Oversight actions require configuration"))?
        .try_read()
        .map_err(|_| error("Configuration is changing"))?;
    let settings = metteur_shared::config::oversight::OversightConfig::from_config(&config)
        .map_err(|_| error("Invalid oversight settings"))?;
    if settings.mode == "off" {
        return Err(error("Oversight actions are disabled in off mode"));
    }
    if ctx.cancel_requested.load(std::sync::atomic::Ordering::SeqCst)
        || ctx.approvals.as_ref().is_none_or(|b| b.is_closed())
    {
        return Err(error("Run is closing"));
    }
    Ok(())
}
pub(crate) fn register(
    ctx: &ExecutionContext,
    review: Uuid,
    id: Uuid,
    kind: &str,
    binding: Value,
) -> DaemonResult<Guard> {
    enabled(ctx)?;
    let db = ctx.workspace_db.as_ref().ok_or_else(|| error("Workspace database unavailable"))?;
    let run = ctx.run_id;
    let _gate = db.oversight_gate.lock().map_err(|_| error("Oversight lock poisoned"))?;
    let mut s = scheduler::load(db, run)?.ok_or_else(|| error("Review unavailable"))?;
    if s.closed {
        return Err(error("Run is closed"));
    }
    let r = s
        .reviews
        .iter_mut()
        .find(|r| r.review_id == review && r.status == Status::Running)
        .ok_or_else(|| error("Review is not running"))?;
    // One concrete action per review keeps confirmation and results unambiguous.
    if !r.proposals.is_empty() {
        return Err(error("This review already proposed an action"));
    }
    let mut q = requests::load(db, run)?;
    if q.closed {
        return Err(error("Requests closed"));
    }
    for id in &r.source_request_ids {
        if !q.requests.iter().any(|q| {
            q.request_id == *id
                && q.review_id == Some(review)
                && q.state == State::Reviewing
                && q.source == "concierge_forwarded"
        }) {
            return Err(error("Request lineage changed"));
        }
    }
    let p = Proposal {
        proposal_id: id,
        review_id: review,
        run_id: run,
        source_request_ids: r.source_request_ids.iter().copied().collect(),
        kind: kind.into(),
        state: State::AwaitingConfirmation,
        binding,
        result_refs: vec![],
        reason: String::new(),
    };
    for request in &mut q.requests {
        if r.source_request_ids.contains(&request.request_id) {
            request.state = State::AwaitingConfirmation;
            request.revision += 1;
            request.proposals.push(requests::ProposalResult {
                proposal_id: id,
                state: State::AwaitingConfirmation,
                result_refs: vec![],
            });
        }
    }
    r.proposals.push(p);
    save(db, run, &s, &q)?;
    Ok(Guard {
        db: db.clone(),
        run,
        id,
        armed: true,
    })
}
pub(crate) fn transition(
    db: &Db,
    run: Uuid,
    id: Uuid,
    state: State,
    refs: Vec<String>,
    reason: &str,
) -> DaemonResult<()> {
    let _gate = db.oversight_gate.lock().map_err(|_| error("Oversight lock poisoned"))?;
    let mut s = scheduler::load(db, run)?.ok_or_else(|| error("Review unavailable"))?;
    if s.closed {
        return Err(error("Run is closed"));
    }
    let r = s
        .reviews
        .iter_mut()
        .find(|r| r.proposals.iter().any(|p| p.proposal_id == id))
        .ok_or_else(|| error("Proposal unavailable"))?;
    let p = r.proposals.iter_mut().find(|p| p.proposal_id == id).expect("selected proposal");
    let valid = matches!(
        (p.state, state),
        (
            State::AwaitingConfirmation,
            State::ApprovedPendingApply | State::Rejected | State::Failed
        ) | (State::ApprovedPendingApply, State::Applied | State::Failed)
    );
    if !valid || (state == State::Applied && refs.is_empty()) {
        return Err(error("Stale or invalid proposal transition"));
    }
    let mut q = requests::load(db, run)?;
    if q.closed {
        return Err(error("Requests closed"));
    }
    p.state = state;
    p.result_refs = refs.clone();
    p.reason = reason.into();
    for request in &mut q.requests {
        if p.source_request_ids.contains(&request.request_id) {
            let child = request
                .proposals
                .iter_mut()
                .find(|p| p.proposal_id == id)
                .ok_or_else(|| error("Proposal lineage missing"))?;
            child.state = state;
            child.result_refs = refs.clone();
            request.state = state;
            request.result_refs = refs.clone();
            request.revision += 1;
        }
    }
    if state == State::Applied {
        r.actual_action_refs.extend(refs);
        if r.status == Status::Completed {
            r.verdict = Some("action_taken".into());
        }
    }
    save(db, run, &s, &q)
}
pub(crate) struct Guard {
    db: Db,
    run: Uuid,
    id: Uuid,
    armed: bool,
}
impl Guard {
    pub(crate) async fn confirm(&mut self, ctx: &ExecutionContext) -> DaemonResult<bool> {
        enabled(ctx)?;
        let s = scheduler::load(&self.db, self.run)?.ok_or_else(|| error("Review unavailable"))?;
        let p = s
            .reviews
            .iter()
            .flat_map(|r| &r.proposals)
            .find(|p| p.proposal_id == self.id)
            .ok_or_else(|| error("Proposal unavailable"))?;
        let queue = requests::load(&self.db, self.run)?;
        let original:Vec<_>=queue.requests.iter().filter(|r|p.source_request_ids.contains(&r.request_id)).map(|r|json!({"request_id":r.request_id,"original_text":r.original_text,"concierge_note":r.concierge_note})).collect();
        let mut detail = p.binding.clone();
        detail["proposal_id"] = json!(p.proposal_id);
        detail["run_id"] = json!(p.run_id);
        detail["review_id"] = json!(p.review_id);
        detail["source_request_ids"] = json!(p.source_request_ids);
        detail["original_requests"] = json!(original);
        detail["confirmation_required"] = json!(true);
        let subject = format!("Confirm concrete oversight proposal {}", self.id);
        // Deliberately bypass all cached grants. Only this fresh broker request can approve.
        let response = crate::sandbox::request_user_approval(
            ctx,
            crate::sandbox::command_hash(&format!("{subject}\n{detail}")),
            &subject,
            detail,
            120,
        )
        .await?;
        if response.decision == crate::sandbox::approval::Decision::Deny {
            transition(
                &self.db,
                self.run,
                self.id,
                State::Rejected,
                vec![],
                "User rejected this proposal",
            )?;
            self.armed = false;
            return Ok(false);
        }
        if response.scope != crate::sandbox::approval::Scope::Once {
            return Err(error("Oversight proposals require a per-proposal AllowOnce confirmation"));
        }
        enabled(ctx)?;
        transition(
            &self.db,
            self.run,
            self.id,
            State::ApprovedPendingApply,
            vec![],
            "Confirmed; application is not complete",
        )?;
        Ok(true)
    }
    pub(crate) fn staged(&mut self) {
        self.armed = false;
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if self.armed {
            let _ = transition(
                &self.db,
                self.run,
                self.id,
                State::Failed,
                vec![],
                "Proposal expired, was interrupted, or became stale; no completed action is implied",
            );
        }
    }
}
