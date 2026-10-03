//! Persistence failures must stop scheduling and retain uncertain outcomes.
use crate::common::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct FailSink {
    records: MemorySink,
    writes: AtomicUsize,
    fail_at: usize,
    persistent: bool,
}

impl CheckpointSink for FailSink {
    fn run_id(&self) -> Uuid {
        self.records.run_id()
    }
    fn write(&self, cp: &ExecutionCheckpoint) -> DaemonResult<()> {
        let n = self.writes.fetch_add(1, Ordering::SeqCst);
        if n == self.fail_at || (self.persistent && n > self.fail_at) {
            return Err(DaemonError::Internal("injected storage failure".into()));
        }
        self.records.write(cp)
    }
}

struct Effect(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl metteur_daemon::registry::Tool for Effect {
    fn name(&self) -> &str {
        "Effect"
    }
    fn description(&self) -> &str {
        "records a test effect"
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({})
    }
    async fn call(&self, _: &[Value], _: &mut ExecutionContext) -> DaemonResult<Value> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Bool(true))
    }
}

fn fixture(sink: Arc<dyn CheckpointSink>, calls: Arc<AtomicUsize>) -> (Interpreter, Blueprint) {
    let mut bp = build_blueprint();
    for node in &mut bp.nodes {
        node.pins.retain(|p| matches!(p.pin_type, PinType::ExecInput | PinType::ExecOutput));
        if node.kind != "Start" {
            node.kind = "Tool".into();
            node.data = serde_json::json!({"tool_name": "Effect"});
            node.pins.push(Pin::data(
                "Result",
                PinType::DataOutput,
                DataType::String,
                Uuid::new_v4(),
            ));
        }
    }
    let pins: Vec<_> = bp.nodes.iter().flat_map(|n| n.pins.iter().map(|p| p.id)).collect();
    bp.edges.retain(|e| pins.contains(&e.source_pin) && pins.contains(&e.target_pin));
    let registry = Arc::new(Registry::with_builtins());
    registry.try_register_tool(Arc::new(Effect(calls))).unwrap();
    (
        Interpreter::new(registry, LlmClientFactory::new(), std::env::temp_dir())
            .with_checkpoint_sink(sink),
        bp,
    )
}

#[tokio::test]
async fn failed_seed_or_executor_fence_prevents_effects() {
    for fail_at in [0, 1, 2, 3] {
        let sink = Arc::new(FailSink {
            records: MemorySink::new(),
            writes: AtomicUsize::new(0),
            fail_at,
            persistent: true,
        });
        let calls = Arc::new(AtomicUsize::new(0));
        let (mut runner, bp) = fixture(sink, calls.clone());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        runner = runner.with_event_tx(tx);
        let err = runner.run(&shared(bp), None).await.unwrap_err();
        assert!(matches!(err, DaemonError::Persistence(_)));
        assert_eq!(err.to_status(), tonic::Code::FailedPrecondition);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        drop(runner);
        let mut diagnostic = false;
        while let Some(event) = rx.recv().await {
            if let ExecutionEvent::Message {
                message,
                ..
            } = event
            {
                diagnostic |= message.contains("checkpoint failed");
            }
        }
        assert!(diagnostic);
    }
}

#[tokio::test]
async fn failed_outcome_commit_stops_successor_and_refuses_ambiguous_replay() {
    let sink = Arc::new(FailSink {
        records: MemorySink::new(),
        writes: AtomicUsize::new(0),
        fail_at: 4,
        persistent: true,
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let (mut runner, bp) = fixture(sink.clone(), calls.clone());
    assert!(matches!(
        runner.run(&shared(bp.clone()), None).await,
        Err(DaemonError::Persistence(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let saved = sink.records.checkpoints.lock().unwrap().last().unwrap().clone();
    assert!(saved.in_flight.is_some());
    assert_ne!(saved.status, RunStatus::Completed);
    let output = Arc::new(MemorySink::new());
    let (mut resumed, _) = fixture(output.clone(), calls.clone());
    let error = resumed
        .resume_with_control(
            &shared(bp),
            saved,
            None,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("manual recovery required"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(output.checkpoints.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failed_terminal_commit_never_reports_success() {
    let sink = Arc::new(FailSink {
        records: MemorySink::new(),
        writes: AtomicUsize::new(0),
        fail_at: 7,
        persistent: false,
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let (mut runner, bp) = fixture(sink.clone(), calls.clone());
    assert!(matches!(runner.run(&shared(bp), None).await, Err(DaemonError::Persistence(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let records = sink.records.checkpoints.lock().unwrap();
    assert_eq!(records.last().unwrap().status, RunStatus::Failed);
    assert!(records.iter().all(|c| c.status != RunStatus::Completed));
    assert!(
        runner.exec_tree().nodes.values().any(|n| matches!(
            n.status,
            metteur_daemon::execution::tree::TreeNodeStatus::Failed(_)
        ))
    );
}
