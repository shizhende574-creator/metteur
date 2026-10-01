//! The worklist execution loop and run completion handling.

use metteur_shared::{CALL_FUNCTION_KIND, FUNCTION_ENTRY_KIND, FUNCTION_EXIT_KIND};

use crate::error::{DaemonError, DaemonResult};
use crate::execution::checkpoint::RunStatus;
use crate::execution::context::ExecutionContext;
use crate::execution::interrupt::InterruptPriority;
use crate::execution::tree::TreeNodeStatus;

use super::checkpointing::now_millis;
use super::{ExecutionEvent, Interpreter, SharedBlueprint};

impl Interpreter {
    /// Resolves whether a cancelled run should roll its mutations back,
    /// preferring the live config over the value captured at build time.
    fn should_rollback_on_cancel(&self, ctx: &ExecutionContext) -> bool {
        ctx.config
            .as_ref()
            .and_then(|c| c.try_read().ok())
            .map(|cfg| cfg.execution.rollback_on_cancel)
            .unwrap_or(self.rollback_on_cancel)
    }

    /// Runs the worklist loop until the queue is drained.
    ///
    /// A checkpoint is written after each completed node and one final
    /// checkpoint with the terminal status when the run finishes or fails.
    pub(crate) async fn execute(
        &mut self,
        blueprint: &SharedBlueprint,
        ctx: &mut ExecutionContext,
    ) -> DaemonResult<Vec<ExecutionEvent>> {
        let bp_id = blueprint.read().id;
        ctx.audit(
            "execution.start",
            serde_json::json!({
                "run_id": ctx.run_id.to_string(),
                "blueprint_id": bp_id.to_string(),
            }),
        );
        if let Some(sink) = &self.checkpoint {
            tracing::info!("run {} started", sink.run_id());
        }

        let result = self.run_loop(blueprint, ctx).await;
        if let Some(root) = self.tree_root.clone() {
            let (status, now) = match &result {
                Ok(()) => (TreeNodeStatus::Done, now_millis()),
                Err(err) => (TreeNodeStatus::Failed(err.to_string()), now_millis()),
            };
            self.tree.finish(&root, status, now);
        }
        // A cancelled run is not a failure: record it under its own status so a
        // client can tell an abandoned run from a broken one.
        let cancelled = matches!(&result, Err(DaemonError::Interrupted(_)));
        let rollback_on_cancel = self.should_rollback_on_cancel(ctx);
        if cancelled && rollback_on_cancel {
            // Undo the file mutations this run made, so cancelling leaves the
            // workspace as it was found. Command side effects are outside the
            // WAL and are not undone.
            match ctx.transaction_log.rollback_after(0) {
                Ok(undone) => ctx.audit(
                    "execution.cancel_rollback",
                    serde_json::json!({
                        "run_id": ctx.run_id.to_string(),
                        "undone": undone,
                    }),
                ),
                Err(err) => ctx.audit(
                    "execution.cancel_rollback_failed",
                    serde_json::json!({
                        "run_id": ctx.run_id.to_string(),
                        "error": err.to_string(),
                    }),
                ),
            }
        }
        let status = match &result {
            Ok(()) => {
                ctx.audit(
                    "execution.end",
                    serde_json::json!({
                        "run_id": ctx.run_id.to_string(),
                        "blueprint_id": bp_id.to_string(),
                    }),
                );
                RunStatus::Completed
            }
            Err(_) if cancelled => {
                ctx.audit(
                    "execution.cancelled",
                    serde_json::json!({
                        "run_id": ctx.run_id.to_string(),
                        "blueprint_id": bp_id.to_string(),
                        "rolled_back": rollback_on_cancel,
                    }),
                );
                RunStatus::Cancelled
            }
            Err(err) => {
                ctx.audit(
                    "execution.failed",
                    serde_json::json!({
                        "run_id": ctx.run_id.to_string(),
                        "error": err.to_string(),
                    }),
                );
                RunStatus::Failed
            }
        };
        self.write_terminal_checkpoint(ctx, status, result.as_ref().err().map(|e| e.to_string()));
        result.map(|()| self.events.clone())
    }

    /// The main execution loop.
    ///
    /// Execution uses a worklist-based dataflow scheduler: a node runs once it
    /// has been triggered by an execution edge and all of its data inputs are
    /// available. This allows diamond-shaped blueprints where a node consumes
    /// data produced by multiple branches.
    ///
    /// The root blueprint is re-read at every node boundary so a replan can
    /// hot-apply data-level edits mid-run. No blueprint lock is held across
    /// `.await`, so a replan write lock can never deadlock against a run.
    ///
    /// Function frames: a `CallFunction` node pushes a frame carrying its own
    /// scheduler; when that frame's queue drains the exit node values are
    /// mapped back onto the caller's output pins and the frame is popped.
    async fn run_loop(
        &mut self,
        root: &SharedBlueprint,
        ctx: &mut ExecutionContext,
    ) -> DaemonResult<()> {
        loop {
            // Honor cancellation and pause requests between nodes.
            crate::execution::control::gate(ctx).await?;

            // Resolve the active frame: the innermost entered function body,
            // or the root blueprint when no function is on the stack.
            let (active_bp, is_function) = match self.active_function() {
                Some(fn_id) => {
                    let entry = self.registry.function_by_id(fn_id).ok_or_else(|| {
                        DaemonError::Execution(format!("function {fn_id} not in registry"))
                    })?;
                    (entry.body, true)
                }
                None => (root.read().clone(), false),
            };

            let node_id = match self.active_scheduler_mut().pop() {
                Some(id) => id,
                None => {
                    if is_function {
                        // A drained function body first advances an unfinished
                        // ForEach loop of its own frame, if any.
                        if self.advance_foreach(&active_bp, ctx)? {
                            continue;
                        }
                        // The function body drained: pop the frame, map its exit
                        // values back to the caller, and continue.
                        let frame = self.state.call_stack.pop().expect("function frame on top");
                        // Discard the function body's variable frame with it.
                        ctx.variables.pop();
                        // Close the function body's tree node, rewinding the
                        // current position to the caller (CallFunction) node so
                        // `finish_function` closes that one too.
                        if let Some(fn_tree) = self.frame_trees.pop() {
                            self.tree.finish(&fn_tree, TreeNodeStatus::Done, now_millis());
                            if self.current_tree.as_deref() == Some(fn_tree.as_str()) {
                                self.current_tree =
                                    self.tree.nodes.get(&fn_tree).and_then(|n| n.parent.clone());
                            }
                        }
                        self.finish_function(&frame, &active_bp, ctx).await?;
                        continue;
                    }
                    // A drained root frame advances an unfinished ForEach loop,
                    // if any; otherwise the run is complete.
                    if self.advance_foreach(&active_bp, ctx)? {
                        continue;
                    }
                    break;
                }
            };

            // Skip nodes that already ran (e.g. re-queued by data producers).
            if self.active_scheduler().is_executed(node_id) {
                continue;
            }
            // Snapshot the node and its inputs; no blueprint lock is held
            // across the executor's `.await`.
            let (node, inputs) = {
                // Wait until all data inputs are available; the node is
                // re-enqueued when a producer fires an edge or produces data.
                if !self.data_inputs_ready(&active_bp, node_id) {
                    continue;
                }
                let node = active_bp
                    .node(node_id)
                    .cloned()
                    .ok_or_else(|| DaemonError::Execution(format!("unknown node {node_id}")))?;
                let inputs = self.gather_inputs(&active_bp, node_id)?;
                (node, inputs)
            };

            self.emit(ExecutionEvent::NodeStarted {
                node_id,
            });
            ctx.current_node = node_id;
            ctx.audit(
                "node.started",
                serde_json::json!({ "node_id": node_id.to_string(), "kind": node.kind }),
            );
            self.tree_begin(&node);

            // A CallFunction node hands control to the called body.
            if node.kind == CALL_FUNCTION_KIND {
                self.enter_function(node_id, &node, &inputs, ctx).await?;
                continue;
            }

            // A ForEach node is driven by the interpreter loop, not by its
            // executor: entering starts the iteration, draining the body
            // advances it (see the queue-empty branch above).
            if node.kind == "ForEach" {
                self.enter_foreach(node_id, &node, &inputs, &active_bp, ctx)?;
                continue;
            }

            // Entry/exit nodes of a function body execute implicitly: their
            // values are already wired by enter_function / finish_function.
            if (node.kind == FUNCTION_ENTRY_KIND && is_function)
                || (node.kind == FUNCTION_EXIT_KIND && is_function)
            {
                self.active_scheduler_mut().mark_executed(node_id);
                self.emit(ExecutionEvent::NodeFinished {
                    node_id,
                });
                ctx.audit(
                    "node.finished",
                    serde_json::json!({ "node_id": node_id.to_string(), "kind": node.kind }),
                );
                self.tree_end(ctx, TreeNodeStatus::Done);
                self.write_checkpoint(ctx);
                // Drive the body forward from the entry node (exit nodes
                // typically have no successors, which is harmless).
                self.fire_edges(&active_bp, node_id)?;
                continue;
            }

            if let Some(sink) = &self.checkpoint {
                tracing::info!("run {} executing node {node_id}", sink.run_id());
            }

            let executor = self.registry.node_executor(&node.kind).ok_or_else(|| {
                DaemonError::Execution(format!("no executor for node kind '{}'", node.kind))
            })?;
            let outputs = executor.execute(&node, &inputs, ctx).await?;
            self.state.data_values.extend(outputs.iter().map(|(id, v)| (*id, v.clone())));
            let function = self.active_function();
            self.emit(ExecutionEvent::NodeData {
                node_id,
                outputs: outputs.iter().map(|(id, v)| (*id, v.clone())).collect(),
                function,
            });
            self.active_scheduler_mut().mark_executed(node_id);
            if let Some(root_frame) = self.state.call_stack.first_mut()
                && root_frame.function.is_none()
            {
                root_frame.node_id = node_id;
            }
            self.emit(ExecutionEvent::NodeFinished {
                node_id,
            });
            ctx.audit(
                "node.finished",
                serde_json::json!({ "node_id": node_id.to_string(), "kind": node.kind }),
            );
            self.tree_end(ctx, TreeNodeStatus::Done);

            // Defer normal interrupts until the next Call LLM node.
            if let Some(bus) = &ctx.interrupts {
                for msg in bus.drain(InterruptPriority::Normal) {
                    ctx.pending_normal.push_back(msg);
                }
            }

            // Validation retry: a failed validator with retry budget rolls
            // back its segment and re-queues it before any successor can be
            // enqueued with stale data. It persists its own checkpoint.
            if self.maybe_retry(node_id, &node, &outputs, ctx)? {
                continue;
            }

            // Fire execution output edges and wake data consumers.
            self.fire_edges(&active_bp, node_id)?;

            // Circuit breaker: repeated validation failures trigger a
            // user-approved replan and re-run this node under the new plan.
            self.maybe_circuit_break(node_id, &node, &outputs, ctx).await?;

            // Persist only once the successors are queued. A checkpoint taken
            // before `fire_edges` would record this node as executed while its
            // successors are in neither `executed` nor `pending`; a crash in
            // that window would resume into a drained queue and report the run
            // as completed with the remaining branch silently skipped.
            self.write_checkpoint(ctx);
        }
        Ok(())
    }
}
