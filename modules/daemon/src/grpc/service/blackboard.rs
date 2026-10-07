//! Workspace-scoped read-only blackboard query; the standard RPC ACL applies.
use super::super::proto::{BlackboardProjection, GetBlackboardRequest};
use super::{DaemonService, to_status};
use crate::execution::{
    DbCheckpointSink,
    blackboard::{self, Query},
};
use crate::observability::{anon::Anonymizer, audit::AuditWriter};
use std::path::PathBuf;
use tonic::{Request, Response, Status};

impl DaemonService {
    pub(crate) async fn get_blackboard(
        &self,
        request: Request<GetBlackboardRequest>,
    ) -> Result<Response<BlackboardProjection>, Status> {
        let subject =
            super::super::acl::subject_from_request(&request).unwrap_or_else(|| "local".into());
        let req = request.into_inner();
        let run = uuid::Uuid::parse_str(&req.run_id)
            .map_err(|_| Status::invalid_argument("invalid run id"))?;
        if req.query_json.len() > 16_384 {
            return Err(Status::invalid_argument("query too large"));
        }
        let query: Query = if req.query_json.trim().is_empty() {
            Query::default()
        } else {
            serde_json::from_str(&req.query_json)
                .map_err(|_| Status::invalid_argument("invalid blackboard query"))?
        };
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let checkpoint = DbCheckpointSink::load(&ws.db, run)
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found("blueprint run not found in this workspace"))?;
        // Always apply built-ins at this boundary, plus configured patterns.
        // No deanonymization map or full context is exposed to the caller.
        let patterns = ws.config.read().await.anonymize.extra_patterns.clone();
        let projection = blackboard::project(
            &run.to_string(),
            &checkpoint.view,
            &checkpoint.exec_tree,
            &query,
            &Anonymizer::new(&patterns),
        )
        .await;
        if query.entry_id.is_some() && projection.entries.is_empty() {
            return Err(Status::not_found("blackboard entry not found"));
        }
        // Do not copy keyword text (which may itself contain a secret) to audit.
        AuditWriter::new(ws.db.clone()).record(&subject, "blackboard.query", serde_json::json!({
            "run_id": run, "returned_entries": projection.entries.len(), "evidence_lookup": query.entry_id.is_some()
        })).map_err(to_status)?;
        let projection_json = serde_json::to_string(&projection)
            .map_err(|_| Status::internal("cannot serialize blackboard"))?;
        Ok(Response::new(BlackboardProjection {
            projection_json,
        }))
    }
}
