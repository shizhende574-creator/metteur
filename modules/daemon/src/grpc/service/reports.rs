//! Read-only report projection; identities and usage come from durable records.
use super::super::proto::{OversightReports, OversightReportsRequest};
use super::{DaemonService, to_status};
use crate::{
    execution::DbCheckpointSink,
    oversight::{budget, scheduler},
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
        DbCheckpointSink::load(&ws.db, run)
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found("Blueprint run not found"))?;
        let schedule = scheduler::load(&ws.db, run).map_err(to_status)?;
        let calls = budget::load(&ws.db, run).map_err(to_status)?.calls;
        let cancel_result = schedule.as_ref().and_then(|s| s.cancel_result.clone());
        let reports: Vec<_> = schedule
            .into_iter()
            .flat_map(|s| s.reviews)
            .map(|r| {
                let mut value = serde_json::to_value(&r).expect("serializable report");
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
            reports_json: serde_json::json!({"run_id":run,"reports":reports}).to_string(),
        }))
    }
}
