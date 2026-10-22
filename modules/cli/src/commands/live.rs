//! Resolve a concrete control target, with no latest-owner fallback.
use super::status;
use metteur_proto::proto::daemon_client::DaemonClient;
use tonic::transport::Channel;

pub(crate) struct LiveTarget {
    pub kind: String,
    pub run: String,
    pub session: String,
    pub turn: String,
}
/// Resolve once; later RPCs carry this identity and cannot follow a new owner.
pub(crate) async fn live_target(
    client: &mut DaemonClient<Channel>,
    workspace: &str,
    require_targets: bool,
) -> anyhow::Result<LiveTarget> {
    if require_targets {
        let result = client
            .chat_delivery(metteur_proto::proto::ResourceRequest {
                workspace_path: workspace.into(),
                schema_version: 1,
                payload_json: r#"{"operation":"capabilities"}"#.into(),
                ..Default::default()
            })
            .await
            .map_err(status)?
            .into_inner();
        if !result.error_json.is_empty() {
            anyhow::bail!("Live control capability unavailable: {}", result.error_json);
        }
        let data: serde_json::Value = serde_json::from_str(&result.payload_json)?;
        if data["targeted_controls"] != true {
            anyhow::bail!("Host does not verify concrete live control targets");
        }
    }
    let result = client
        .chat_delivery(metteur_proto::proto::ResourceRequest {
            workspace_path: workspace.into(),
            schema_version: 1,
            payload_json: r#"{"operation":"control_target"}"#.into(),
            ..Default::default()
        })
        .await
        .map_err(status)?
        .into_inner();
    if !result.error_json.is_empty() {
        anyhow::bail!("Live target unavailable: {}", result.error_json);
    }
    let data: serde_json::Value = serde_json::from_str(&result.payload_json)?;
    let field = |name: &str| data[name].as_str().unwrap_or_default().to_owned();
    let target = LiveTarget {
        kind: field("state"),
        run: field("run"),
        session: field("session"),
        turn: field("turn"),
    };
    if !(target.kind == "blueprint" && !target.run.is_empty()
        || target.kind == "chat" && !target.session.is_empty() && !target.turn.is_empty())
    {
        anyhow::bail!("No confirmed live target; no control was sent");
    }
    Ok(target)
}
