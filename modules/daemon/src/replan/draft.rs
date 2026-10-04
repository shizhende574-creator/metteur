//! Model-authored saves are proposals, distinct from a direct user Save RPC.
use crate::error::{DaemonError, DaemonResult};
use crate::execution::context::ExecutionContext;
use crate::storage::{
    blueprint_files,
    versioning::{ExpectedFile, VersionRef},
};
use metteur_shared::Blueprint;
use sha2::{Digest, Sha256};

pub(crate) async fn save(
    ctx: &mut ExecutionContext,
    graph: &Blueprint,
) -> DaemonResult<VersionRef> {
    let db = ctx.workspace_db.clone().ok_or_else(|| {
        DaemonError::Execution("saving a blueprint needs a workspace database".into())
    })?;
    if ctx.blueprint.as_ref().is_some_and(|bp| bp.read().id == graph.id) {
        return Err(DaemonError::Execution(
            "use ReplanBlueprint to change an executing plan".into(),
        ));
    }
    let versions = ctx.version_manager.clone().unwrap_or_else(|| {
        std::sync::Arc::new(crate::storage::versioning::VersionManager::new(
            db.clone(),
            ctx.workspace_root.clone(),
        ))
    });
    super::application::ensure_resolved(&db)?;
    let uri = blueprint_files::binding(&db, graph.id)?
        .map(|v| v.blueprint_uri)
        .unwrap_or_else(|| format!("blueprints/{}.blueprint", graph.id));
    let path = versions.blueprint_path(&uri)?;
    let (bytes, base) = if path.exists() {
        let original = std::fs::read(&path)?;
        let base = versions.capture_blueprint(&uri)?;
        if crate::storage::versioning::hash_content(&original) != base.blob_hash {
            return Err(DaemonError::Execution("draft file changed while preparing".into()));
        }
        (blueprint_files::encode_changes(&original, graph)?, Some(base))
    } else {
        (blueprint_files::encode_native(graph)?, None)
    };
    let id = uuid::Uuid::new_v4();
    let digest: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    let allowed = super::await_approval(
        ctx,
        "blueprint_save",
        "Save this model-authored blueprint?",
        serde_json::json!({
            "request_type":"blueprint_save","tool":"DraftBlueprint","source":"model_draft",
            "proposal_id":id,"run_id":ctx.run_id,"blueprint_id":graph.id,"path":uri,"base":base,
            "content_digest":digest,"blueprint":graph,
        }),
    )
    .await?;
    if !allowed {
        return Err(DaemonError::PermissionDenied("blueprint save denied".into()));
    }
    if ctx.cancel_requested.load(std::sync::atomic::Ordering::SeqCst)
        || ctx.approvals.as_ref().is_none_or(|b| b.is_closed())
    {
        return Err(DaemonError::PermissionDenied(
            "blueprint save approval is no longer valid".into(),
        ));
    }
    super::application::ensure_resolved(&db)?;
    let expected = base.as_ref().map(ExpectedFile::Version).unwrap_or(ExpectedFile::Absent);
    let (version, _) =
        blueprint_files::save_with_origin(&db, &versions, graph, &uri, &bytes, expected, None)?;
    ctx.audit(
        "blueprint.saved",
        serde_json::json!({"proposal_id":id,"source":"model_draft","before":base,"after":version}),
    );
    Ok(version)
}
