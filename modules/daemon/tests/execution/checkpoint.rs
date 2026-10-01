//! Interpreter checkpoint tests.

use crate::common::*;

/// A checkpoint taken after a node completes must already list that node's
/// successors as pending.
///
/// Writing the checkpoint before the execution edges fire recorded the
/// just-finished node as executed while its successors were in neither
/// `executed` nor `pending`. Resuming such a checkpoint drained an empty
/// queue, reported the run as completed, and silently skipped the rest of
/// the graph.
#[tokio::test]
async fn checkpoint_records_successors_of_finished_node() {
    let blueprint = build_blueprint();
    let sink = Arc::new(RecordSink::new());
    let mut interpreter = new_interpreter().with_checkpoint_sink(sink.clone());
    interpreter.run(&shared(blueprint.clone()), None).await.unwrap();

    let checkpoints = sink.checkpoints.lock().unwrap();
    // Checkpoint 0 is the pre-run seed; checkpoint 1 follows Start.
    let after_start = checkpoints
        .get(1)
        .expect("a checkpoint after the first node must exist");
    assert!(
        !after_start.pending.is_empty(),
        "a checkpoint taken after a node finished must carry its successors, \
         otherwise a resume silently drops them (executed={:?}, pending=[])",
        after_start.executed,
    );
}

/// Resuming from a checkpoint the run itself produced must still execute
/// every node that was left outstanding.
#[tokio::test]
async fn resume_from_live_checkpoint_finishes_outstanding_nodes() {
    let blueprint = build_blueprint();
    let sink = Arc::new(RecordSink::new());
    let mut interpreter = new_interpreter().with_checkpoint_sink(sink.clone());
    interpreter.run(&shared(blueprint.clone()), None).await.unwrap();
    let captured = {
        let checkpoints = sink.checkpoints.lock().unwrap();
        let cp = checkpoints
            .get(1)
            .expect("a checkpoint after the first node must exist")
            .clone();
        assert!(
            !cp.pending.is_empty(),
            "resuming this checkpoint would skip the rest of the graph"
        );
        cp
    };
    let expected_pending: Vec<uuid::Uuid> = captured.pending.clone();

    let mut resumed = new_interpreter();
    let events = resumed
        .resume_with_control(
            &shared(blueprint),
            captured,
            None,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .await
        .unwrap();
    // Every node the checkpoint left outstanding must actually run.
    for node_id in expected_pending {
        assert!(
            events.iter().any(|e| matches!(
                e,
                ExecutionEvent::NodeStarted { node_id: n } if *n == node_id
            )),
            "node {node_id} was pending at checkpoint time but never ran after resume"
        );
    }
}


#[tokio::test]
async fn resume_continues_from_checkpoint() {
    let blueprint = build_blueprint();
    let checkpoint = checkpoint_after_start(&blueprint);
    let mut interpreter = new_interpreter();
    let events = interpreter
        .resume_with_control(
            &shared(blueprint),
            checkpoint,
            None,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .await
        .unwrap();
    // Add and Judge are the remaining nodes (started + finished + data).
    assert_eq!(events.len(), 6);
}

#[tokio::test]
async fn resume_rejects_non_resumable_run() {
    let blueprint = build_blueprint();
    let mut checkpoint = checkpoint_after_start(&blueprint);
    checkpoint.status = RunStatus::Completed;
    let mut interpreter = new_interpreter();
    let result = interpreter
        .resume_with_control(
            &shared(blueprint),
            checkpoint,
            None,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn writes_checkpoints_through_sink() {
    let blueprint = build_blueprint();
    let sink = Arc::new(MemorySink::new());
    let mut interpreter = new_interpreter().with_checkpoint_sink(sink.clone());
    interpreter.run(&shared(blueprint), None).await.unwrap();
    let checkpoints = sink.checkpoints.lock().unwrap();
    // Initial + one per node (3) + terminal.
    assert_eq!(checkpoints.len(), 5);
    assert_eq!(checkpoints.last().unwrap().status, RunStatus::Completed);
}
