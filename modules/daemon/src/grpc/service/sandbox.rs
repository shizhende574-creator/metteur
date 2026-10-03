//! Sandbox RPCs: approval responses.

use std::path::PathBuf;

use tonic::{Request, Response, Status};

use super::super::proto::{ApprovalDecisionRequest, Empty};
use super::*;

impl DaemonService {
    pub(crate) async fn respond_approval(
        &self,
        request: Request<ApprovalDecisionRequest>,
    ) -> Result<Response<Empty>, Status> {
        use crate::sandbox::approval::parse_decision;
        use crate::sandbox::grant::GrantStore;

        let req = request.into_inner();
        let Some((decision, scope)) = parse_decision(&req.decision) else {
            return Err(Status::invalid_argument(format!("invalid decision '{}'", req.decision)));
        };
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let broker = self
            .approval_broker_for(ws.root())
            .await
            .ok_or_else(|| Status::not_found("no running execution or chat"))?;
        let grants = GrantStore::new(Some(ws.db.clone()), self.state.global_db.clone());
        broker.respond(&req.request_id, decision, scope, &grants).map_err(to_status)?;
        Ok(Response::new(Empty {}))
    }
}
