//! Bounded node activity, separate from execution checkpoints and never replayed.
use super::{DbCheckpointSink, ExecutionContext, ExecutionEvent, react::ReactEvent};
use crate::{
    error::{DaemonError, DaemonResult},
    storage::persistence::{Db, cf},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};

const MAX_TEXT: usize = 16 * 1024;
const MAX_RUN: usize = 2 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeActivity {
    pub node_id: uuid::Uuid,
    pub scope: String,
    pub frame: Vec<String>,
    pub invocation: u64,
    pub attempt: u32,
    pub messages: Vec<serde_json::Value>,
    pub truncated: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunOutput {
    pub schema_version: u32,
    pub run_id: uuid::Uuid,
    pub revision: u64,
    pub nodes: Vec<NodeActivity>,
    pub truncated: bool,
}
fn key(run: uuid::Uuid) -> Vec<u8> {
    format!("dw/output/v1/{run}").into_bytes()
}
pub fn read(db: &Db, run: uuid::Uuid) -> DaemonResult<RunOutput> {
    match db.get(cf::EXECUTION_STATE, &key(run))? {
        Some(bytes) => {
            let value: RunOutput = serde_json::from_slice(&bytes)
                .map_err(|e| DaemonError::Serialization(e.to_string()))?;
            if value.schema_version != 1 || value.run_id != run {
                return Err(DaemonError::Persistence("Invalid node output identity".into()));
            }
            Ok(value)
        }
        None => Ok(RunOutput {
            schema_version: 1,
            run_id: run,
            revision: 0,
            nodes: vec![],
            truncated: false,
        }),
    }
}
fn bounded(text: &str) -> String {
    let mut end = text.len().min(MAX_TEXT);
    while !text.is_char_boundary(end) {
        end -= 1
    }
    text[..end].into()
}
pub struct Recorder {
    db: Option<Db>,
    run: uuid::Uuid,
    tx: Option<tokio::sync::mpsc::UnboundedSender<ExecutionEvent>>,
    pub activity: NodeActivity,
    open: Option<usize>,
    last: Instant,
    failure: Option<String>,
}
impl Recorder {
    pub fn new(ctx: &ExecutionContext, node: uuid::Uuid) -> DaemonResult<Self> {
        let invocation = ctx
            .workspace_db
            .as_ref()
            .map(|db| DbCheckpointSink::load(db, ctx.run_id))
            .transpose()?
            .flatten()
            .and_then(|c| c.view.invocations.into_iter().rev().find(|i| i.node_id == node));
        Ok(Self {
            db: ctx.workspace_db.clone(),
            run: ctx.run_id,
            tx: ctx.events.clone(),
            activity: NodeActivity {
                node_id: node,
                scope: invocation.as_ref().map(|i| i.scope.clone()).unwrap_or_default(),
                frame: invocation.as_ref().map(|i| i.frame.clone()).unwrap_or_default(),
                invocation: invocation.as_ref().map(|i| i.sequence).unwrap_or(0),
                attempt: invocation.as_ref().map(|i| i.attempt).unwrap_or(1),
                messages: vec![],
                truncated: false,
            },
            open: None,
            last: Instant::now() - Duration::from_secs(1),
            failure: None,
        })
    }
    fn assistant(&mut self) -> Option<usize> {
        if let Some(i) = self.open {
            return Some(i);
        }
        if self.activity.messages.len() >= 256 {
            self.activity.truncated = true;
            return None;
        }
        let i = self.activity.messages.len();
        self.activity.messages.push(json!({"id":format!("{}-a{i}",self.activity.invocation),"role":"assistant","content":"","reasoning":"","createdAt":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,"pending":true}));
        self.open = Some(i);
        Some(i)
    }
    pub fn delta(&mut self, delta: crate::llm::StreamDelta, hide_thinking: bool) {
        let (field, text) = match delta {
            crate::llm::StreamDelta::Text(t) => ("content", t),
            crate::llm::StreamDelta::Reasoning(t) if !hide_thinking => ("reasoning", t),
            _ => return,
        };
        let mut first = false;
        if let Some(i) = self.assistant() {
            let old = self.activity.messages[i][field].as_str().unwrap_or("");
            first = old.is_empty();
            let joined = format!("{old}{text}");
            if joined.len() > MAX_TEXT {
                self.activity.truncated = true
            }
            self.activity.messages[i][field] = json!(bounded(&joined));
        }
        // The first content can immediately follow reasoning, then stall. Persist
        // each field's first delta even inside the regular coalescing interval.
        self.flush(first);
    }
    pub fn event(&mut self, event: ReactEvent) {
        match event {
            ReactEvent::Assistant {
                text,
                reasoning,
            } => {
                if let Some(i) = self.assistant() {
                    self.activity.truncated |= text.len() > MAX_TEXT || reasoning.len() > MAX_TEXT;
                    self.activity.messages[i]["content"] = json!(bounded(&text));
                    self.activity.messages[i]["reasoning"] = json!(bounded(&reasoning));
                    self.activity.messages[i]["pending"] = json!(false);
                    self.open = None;
                }
            }
            ReactEvent::ToolStart {
                call_id,
                name,
                summary,
            } => {
                self.open = None;
                if self.activity.messages.len() < 256 {
                    self.activity.messages.push(json!({"id":format!("{}-t{call_id}",self.activity.invocation),"role":"tool","actor":name,"content":"","createdAt":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,"pending":true,"detail":{"callId":call_id,"summary":bounded(&summary),"running":true}}));
                } else {
                    self.activity.truncated = true
                }
            }
            ReactEvent::ToolProgress {
                call_id,
                tail,
            } => {
                if let Some(row) = self
                    .activity
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|r| r["detail"]["callId"] == call_id)
                {
                    row["content"] = json!(bounded(&tail));
                }
            }
            ReactEvent::Tool {
                call_id,
                name,
                ok,
                elapsed_ms,
                content,
            } => {
                self.activity.truncated |= content.len() > MAX_TEXT;
                if let Some(row) = self
                    .activity
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|r| r["detail"]["callId"] == call_id)
                {
                    row["actor"] = json!(name);
                    row["content"] = json!(bounded(&content));
                    row["pending"] = json!(false);
                    row["detail"]["ok"] = json!(ok);
                    row["detail"]["elapsedMs"] = json!(elapsed_ms);
                    row["detail"]["running"] = json!(false);
                }
            }
        }
        self.flush(true);
    }
    fn flush(&mut self, force: bool) {
        if self.failure.is_some() || (!force && self.last.elapsed() < Duration::from_millis(250)) {
            return;
        }
        self.last = Instant::now();
        let result = (|| -> DaemonResult<u64> {
            let Some(db) = &self.db else {
                return Ok(0);
            };
            let _guard = db
                .resource_gate
                .lock()
                .map_err(|_| DaemonError::Persistence("Output lock unavailable".into()))?;
            let mut run = read(db, self.run)?;
            let previous = run.nodes.iter().position(|n| {
                n.invocation == self.activity.invocation
                    && n.node_id == self.activity.node_id
                    && n.scope == self.activity.scope
                    && n.frame == self.activity.frame
            });
            let before = run.nodes.clone();
            if let Some(i) = previous {
                run.nodes[i] = self.activity.clone()
            } else if run.nodes.len() < 128 {
                run.nodes.push(self.activity.clone())
            } else {
                run.truncated = true
            }
            run.revision += 1;
            if serde_json::to_vec(&run)
                .map_err(|e| DaemonError::Serialization(e.to_string()))?
                .len()
                > MAX_RUN
            {
                run.nodes = before;
                run.truncated = true;
            }
            db.put_output_durable(
                &key(self.run),
                &serde_json::to_vec(&run).map_err(|e| DaemonError::Serialization(e.to_string()))?,
            )?;
            Ok(run.revision)
        })();
        match result {
            Ok(revision) => {
                if let Some(tx) = &self.tx {
                    let _ = tx.send(ExecutionEvent::NodeOutput {
                        node_id: self.activity.node_id,
                        revision,
                    });
                }
            }
            Err(error) => self.failure = Some(error.to_string()),
        }
    }
    pub fn finish(&mut self) -> DaemonResult<()> {
        for row in &mut self.activity.messages {
            row["pending"] = json!(false)
        }
        self.open = None;
        self.flush(true);
        match &self.failure {
            Some(error) => Err(DaemonError::Persistence(format!(
                "Node output could not be durably recorded: {error}"
            ))),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_limits_and_output_notifications_do_not_rewrite_or_reload_execution_state() {
        let path = std::env::temp_dir().join(format!("iu-output-{}", uuid::Uuid::new_v4()));
        let db = Db::open(&path).unwrap();
        let changes = db.subscribe_changes();
        let mut ctx = ExecutionContext::new(
            std::sync::Arc::new(crate::Registry::with_builtins()),
            crate::llm::LlmClientFactory::new(),
            path.clone(),
        );
        ctx.workspace_db = Some(db.clone());
        let run = ctx.run_id;
        let mut recorder = Recorder::new(&ctx, uuid::Uuid::new_v4()).unwrap();
        recorder.delta(crate::llm::StreamDelta::Reasoning("thinking".into()), false);
        recorder.delta(crate::llm::StreamDelta::Text("partial".into()), false);
        assert_eq!(
            read(&db, run).unwrap().nodes[0].messages[0]["content"],
            "partial",
            "Partial content must persist even before the next provider delta"
        );
        recorder.delta(crate::llm::StreamDelta::Text("中文".repeat(20000)), false);
        recorder.finish().unwrap();
        let record = read(&db, run).unwrap();
        assert!(record.nodes[0].truncated);
        assert!(record.nodes[0].messages[0]["content"].as_str().unwrap().len() <= MAX_TEXT);
        assert!(!record.nodes[0].messages[0]["pending"].as_bool().unwrap());
        assert!(
            !changes.has_changed().unwrap(),
            "Streaming output has a dedicated event and must not reload the workspace"
        );
        assert!(
            DbCheckpointSink::list(&db).unwrap().is_empty(),
            "Output records are not execution checkpoints"
        );
        drop(recorder);
        drop(ctx);
        drop(db);
        let reopened = Db::open(&path).unwrap();
        assert_eq!(read(&reopened, run).unwrap().revision, record.revision);
    }
}
