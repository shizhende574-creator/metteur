//! Receipt queries do not resume an execution or restore old permissions.
use super::{
    super::proto::{ResourceRequest, ResourceResponse},
    DaemonService,
    resources::response,
};
use crate::storage::{
    delivery::{DeliveryStore, MAX_LEDGER_BYTES, MAX_RECEIPTS},
    resources::{Result, fault},
};
use serde::Deserialize;
use tonic::{Request, Response, Status};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Query {
    Capabilities,
    ReconcilePending,
    Read {
        id: Uuid,
    },
    Retire {
        id: Uuid,
    },
    List {
        after: Option<Uuid>,
    },
    SessionRevision {
        session: Uuid,
    },
}

impl DaemonService {
    pub(crate) async fn chat_delivery(
        &self,
        request: Request<ResourceRequest>,
    ) -> std::result::Result<Response<ResourceResponse>, Status> {
        let subject =
            super::super::acl::subject_from_request(&request).unwrap_or_else(|| "local".into());
        let req = request.into_inner();
        if req.schema_version != 1 || req.payload_json.len() > 4096 {
            return Ok(response(Err(fault("unsupported_request", "Unsupported delivery query"))));
        }
        let ws = self
            .state
            .workspaces
            .get(std::path::Path::new(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("Workspace not open"))?;
        let _guard = ws.admit().await.map_err(super::to_status)?;
        let result = (|| -> Result<serde_json::Value> {
            let query: Query = serde_json::from_str(&req.payload_json)
                .map_err(|_| fault("invalid_request", "Invalid delivery query fields"))?;
            let store = DeliveryStore::new(ws.db.clone(), &ws.root().to_string_lossy(), &subject)?;
            match query {
                Query::Capabilities => Ok(
                    serde_json::json!({"schema":1,"durable_manual_send":true,"pending_reconciliation":true,"durable_queue":false,"max_receipts":MAX_RECEIPTS,"ledger_bytes":MAX_LEDGER_BYTES,"ordinary_disconnect":"cancel","automatic_replay":false}),
                ),
                Query::ReconcilePending => {
                    let recovery = crate::storage::recovery::RecoveryStore::new(
                        ws.db.clone(),
                        &format!("project:{}", ws.root().to_string_lossy()),
                        &subject,
                    )?;
                    let now = chrono::Utc::now().timestamp_millis() as u64;
                    let mut items = Vec::new();
                    for id in recovery.pending_submissions()? {
                        items.push(store.retire(id, now)?);
                    }
                    Ok(serde_json::json!({"items":items,"raw_copies_unchanged":true}))
                }
                Query::Read {
                    id,
                } => Ok(serde_json::json!({"receipt":store.get(id)?})),
                Query::Retire {
                    id,
                } => Ok(
                    serde_json::json!({"receipt":store.retire(id,chrono::Utc::now().timestamp_millis() as u64)?}),
                ),
                Query::List {
                    after,
                } => {
                    let items = store.list(after)?;
                    let next = if items.len() == 50 {
                        items.last().map(|r| r.id)
                    } else {
                        None
                    };
                    Ok(serde_json::json!({"items":items,"next":next}))
                }
                Query::SessionRevision {
                    session,
                } => {
                    if !self
                        .state
                        .acl_store
                        .read()
                        .unwrap()
                        .is_allowed(&subject, "/metteur.Daemon/GetChatSession")
                    {
                        return Err(fault(
                            "permission_denied",
                            "Current conversation read permission is required",
                        ));
                    }
                    let exists = crate::chat::session::load_thread(&ws.db, &session)
                        .map_err(|_| {
                            fault("storage_unavailable", "Conversation could not be read")
                        })?
                        .is_some();
                    if !exists {
                        return Err(fault(
                            "not_found",
                            "Conversation is missing; it will not be recreated",
                        ));
                    }
                    Ok(
                        serde_json::json!({"session":session,"revision":ws.db.chat_revision(session.as_bytes()).map_err(|_|fault("storage_unavailable","Conversation revision could not be read"))?}),
                    )
                }
            }
        })();
        Ok(response(result))
    }
}
