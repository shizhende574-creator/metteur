//! Interpreter control tests.

use crate::common::*;

struct WriteThenCancel {
    conflict: bool,
}
#[async_trait::async_trait]
impl metteur_daemon::registry::Tool for WriteThenCancel {
    fn name(&self) -> &str {
        "WriteThenCancel"
    }
    fn description(&self) -> &str {
        "writes files and cancels for a recovery boundary test"
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({})
    }
    async fn call(&self, _: &[Value], ctx: &mut ExecutionContext) -> DaemonResult<Value> {
        ctx.write_file(&ctx.workspace_root.join("first"), b"agent", None)?;
        ctx.write_file(&ctx.workspace_root.join("second"), b"agent", None)?;
        if self.conflict {
            std::fs::write(ctx.workspace_root.join("first"), b"user edit")?;
        }
        ctx.cancel_requested.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(Value::Bool(true))
    }
}

#[tokio::test]
async fn cancelled_run_reports_complete_or_partial_file_rollback_accurately() {
    for conflict in [false, true] {
        let root = std::env::temp_dir().join(format!("metteur-cancel-files-{}", Uuid::new_v4()));
        let db = metteur_daemon::storage::persistence::Db::open(&root.join(".metteur/db")).unwrap();
        let sink = Arc::new(MemorySink::new());
        let registry = Arc::new(Registry::with_builtins());
        registry
            .try_register_tool(Arc::new(WriteThenCancel {
                conflict,
            }))
            .unwrap();
        let mut bp = build_blueprint();
        bp.nodes[0].kind = "Tool".into();
        bp.nodes[0].data = serde_json::json!({"tool_name":"WriteThenCancel"});
        bp.nodes[0].pins.push(Pin::data(
            "Result",
            PinType::DataOutput,
            DataType::String,
            Uuid::new_v4(),
        ));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut runner = Interpreter::new(registry, LlmClientFactory::new(), root.clone())
            .with_checkpoint_sink(sink.clone())
            .with_workspace_db(db)
            .with_event_tx(tx);
        let error = runner.run(&shared(bp), None).await.unwrap_err();
        assert!(matches!(error, DaemonError::Interrupted(_)), "{error}");
        assert!(!root.join("second").exists());
        let cp = sink.checkpoints.lock().unwrap().last().unwrap().clone();
        assert_eq!(cp.status, RunStatus::Cancelled);
        let expected = if conflict {
            "rollback incomplete: restored 1"
        } else {
            "file rollback complete (2"
        };
        let mut messages = Vec::new();
        while let Ok(event) = rx.try_recv() {
            if let ExecutionEvent::Message {
                message,
                ..
            } = event
            {
                messages.push(message);
            }
        }
        assert!(messages.iter().any(|m| m.contains(expected)), "{messages:?}");
        if conflict {
            assert_eq!(std::fs::read(root.join("first")).unwrap(), b"user edit");
            assert!(cp.error.unwrap().contains("rollback incomplete"));
        } else {
            assert!(!root.join("first").exists());
        }
    }
}

#[tokio::test]
async fn cancel_marks_run_cancelled() {
    let blueprint = build_blueprint();
    let sink = Arc::new(MemorySink::new());
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut interpreter = new_interpreter().with_checkpoint_sink(sink.clone());
    let result = interpreter
        .run_with_control(
            &shared(blueprint),
            None,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cancel,
        )
        .await;
    assert!(result.is_err());
    let checkpoints = sink.checkpoints.lock().unwrap();
    // A user cancel is not a failure: it gets its own terminal status so a
    // client can tell an abandoned run from a broken one.
    assert_eq!(checkpoints.last().unwrap().status, RunStatus::Cancelled);
}

/// A paused run must stay cancellable. The pause loop parks between nodes,
/// so without a cancel re-check on every tick a cancel issued while paused
/// would never be observed and the run would hold its workspace slot
/// forever.
#[tokio::test]
async fn cancel_while_paused_terminates() {
    let blueprint = Arc::new(build_blueprint());
    let pause = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    // The task owns the interpreter and the shared blueprint so the future
    // is 'static.
    let handle = {
        let blueprint = blueprint.clone();
        let pause = pause.clone();
        let cancel = cancel.clone();
        tokio::spawn(async move {
            let mut interpreter = new_interpreter();
            let result = interpreter
                .run_with_control(&shared((*blueprint).clone()), None, pause, cancel)
                .await;
            (result, interpreter)
        })
    };
    // Let the loop reach the pause gate, then cancel.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    let (result, _) = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .expect("a paused run must observe a cancel request")
        .expect("run task must not panic");
    assert!(result.is_err());
}
