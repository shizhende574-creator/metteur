//! Read-only report projection; identities and usage come from durable records.
use super::super::proto::{OversightReports, OversightReportsRequest};
use super::{DaemonService, to_status};
use crate::{
    execution::DbCheckpointSink,
    oversight::{budget, scheduler, diagnostic::{Category, Diagnostic, Stage}},
};
use tonic::{Request, Response, Status};
impl DaemonService {
    pub(crate) async fn list_oversight_reports(
        &self,
        request: Request<OversightReportsRequest>,
    ) -> Result<Response<OversightReports>, Status> {
        let req = request.into_inner();
        let run = uuid::Uuid::parse_str(&req.run_id)
            .map_err(|_| Status::invalid_argument("Invalid run id"))?;
        let ws = self
            .state
            .workspaces
            .get(&std::path::PathBuf::from(req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("Workspace not open"))?;
        let checkpoint = DbCheckpointSink::load(&ws.db, run)
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found("Blueprint run not found"))?;
        let mut closing = crate::oversight::recovery::closing(&ws.db, run).map_err(to_status)?;
        if let Some(receipt) = &mut closing {
            let active = self
                .state
                .running
                .read()
                .await
                .get(&ws.root)
                .is_some_and(|entry| entry.run_id == run);
            receipt["checkpoint_status"] = serde_json::json!(checkpoint.status);
            receipt["checkpoint_error"] = serde_json::json!(checkpoint.error);
            receipt["recovery_required"] =
                serde_json::json!(checkpoint.status.resumable() && !active);
        }
        let schedule = scheduler::load(&ws.db, run).map_err(to_status)?;
        let calls = budget::load(&ws.db, run).map_err(to_status)?.calls;
        let cancel_result = schedule.as_ref().and_then(|s| s.cancel_result.clone());
        let reports: Vec<_> = schedule
            .into_iter()
            .flat_map(|s| s.reviews)
            .map(|r| {
                let mut value = serde_json::to_value(&r).expect("serializable report");
                if r.diagnostic.is_none() && matches!(r.status, scheduler::Status::Failed | scheduler::Status::TimedOut | scheduler::Status::BudgetExhausted | scheduler::Status::Cancelled) {
                    value["diagnostic"] = serde_json::json!(Diagnostic::new(Category::UnknownLegacy, Stage::Unknown, run, r.review_id));
                }
                for proposal in value["proposals"].as_array_mut().into_iter().flatten() {
                    if proposal["diagnostic"].is_null() && matches!(proposal["state"].as_str(), Some("failed" | "rejected" | "closed_unhandled")) {
                        let mut detail = Diagnostic::new(Category::UnknownLegacy, Stage::Unknown, run, r.review_id);
                        detail.proposal_id = proposal["proposal_id"].as_str().and_then(|id| uuid::Uuid::parse_str(id).ok());
                        proposal["diagnostic"] = serde_json::json!(detail);
                    }
                }
                if r.proposals.iter().any(|p| p.kind == "CancelRun") {
                    value["cancel_result"] = serde_json::json!(cancel_result);
                }
                value["usage"] = serde_json::json!(
                    calls.iter().filter(|c| r.work.call_ids.contains(&c.id)).collect::<Vec<_>>()
                );
                value
            })
            .collect();
        Ok(Response::new(OversightReports {
            reports_json: serde_json::json!({"run_id":run,"reports":reports,"closing":closing})
                .to_string(),
        }))
    }
}
