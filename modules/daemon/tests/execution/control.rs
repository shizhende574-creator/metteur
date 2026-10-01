//! Interpreter control tests.

use crate::common::*;

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
