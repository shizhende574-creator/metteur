//! Checkpoint serialization for the running interpreter.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::execution::checkpoint::{CheckpointSink, ExecutionCheckpoint, RunStatus};
use crate::execution::context::ExecutionContext;

use super::Interpreter;

impl Interpreter {
    /// Persists a checkpoint of the current run state, if a sink is set.
    pub(crate) fn write_checkpoint(&self, ctx: &ExecutionContext) {
        let Some(sink) = &self.checkpoint else {
            return;
        };
        self.persist(sink, ctx, RunStatus::Running, None);
    }

    /// Persists the terminal checkpoint for a finished run.
    pub(crate) fn write_terminal_checkpoint(
        &self,
        ctx: &ExecutionContext,
        status: RunStatus,
        error: Option<String>,
    ) {
        let Some(sink) = &self.checkpoint else {
            return;
        };
        self.persist(sink, ctx, status, error);
    }

    /// Serializes and writes a checkpoint through the sink.
    fn persist(
        &self,
        sink: &Arc<dyn CheckpointSink>,
        ctx: &ExecutionContext,
        status: RunStatus,
        error: Option<String>,
    ) {
        let checkpoint = ExecutionCheckpoint {
            run_id: sink.run_id(),
            blueprint_id: self.blueprint_id,
            status,
            started_at: self.started_at,
            updated_at: now_millis(),
            call_stack: self.state.call_stack.clone(),
            data_values: self.state.data_values.clone(),
            executed: self.scheduler.executed_list(),
            pending: self.scheduler.pending_list(),
            triggered: self.scheduler.triggered_list(),
            transaction_log: ctx.transaction_log.entries().to_vec(),
            executed_order: self.scheduler.order_list(),
            attempt_counts: self.state.attempt_counts.clone(),
            validation_marks: self.state.validation_marks.clone(),
            variables: ctx.variables.clone(),
            foreach_stack: self.state.foreach_stack.clone(),
            todos: ctx.todos.clone(),
            exec_tree: self.tree.clone(),
            frame_trees: self.frame_trees.clone(),
            current_tree: self.current_tree.clone(),
            circuit_failures: self.circuit_failures,
            error,
        };
        if let Err(err) = sink.write(&checkpoint) {
            tracing::error!("failed to write checkpoint: {err}");
        }
    }
}

/// Returns the current time in milliseconds since the Unix epoch.
pub(crate) fn now_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}
