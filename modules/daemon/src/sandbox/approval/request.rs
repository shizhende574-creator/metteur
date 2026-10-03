//! A request handle withdraws its pending slot on every exit path.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::oneshot;
use tokio::time::Instant;

use super::{ApprovalFailure, BrokerState, Outcome};

/// The sole receiver of one immutable approval request.
pub struct PendingApproval {
    request_id: String,
    state: Arc<Mutex<BrokerState>>,
    rx: oneshot::Receiver<Outcome>,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}

impl PendingApproval {
    pub(super) fn new(
        request_id: String,
        state: Arc<Mutex<BrokerState>>,
        rx: oneshot::Receiver<Outcome>,
        deadline: Instant,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            request_id,
            state,
            rx,
            deadline,
            cancelled,
        }
    }

    /// The id shown to the user and echoed by `RespondApproval`.
    pub fn id(&self) -> &str {
        &self.request_id
    }

    /// Waits for a response, cancellation, closure or expiry.
    pub async fn wait(mut self) -> Outcome {
        let failure = tokio::select! {
            biased;
            response = &mut self.rx => {
                return self.check_cancelled(response.unwrap_or(Err(ApprovalFailure::Closed)));
            }
            _ = tokio::time::sleep_until(self.deadline) => ApprovalFailure::Expired,
            _ = wait_cancelled(&self.cancelled) => ApprovalFailure::Cancelled,
        };
        {
            let mut state = self.state.lock().unwrap();
            if let Some(pending) = state.pending.remove(&self.request_id) {
                let _ = pending.tx.send(Err(failure));
            }
        }
        // If respond won the lock before expiry, it already committed a valid
        // response. Preserve that ordering rather than inventing a timeout.
        let response = (&mut self.rx).await.unwrap_or(Err(ApprovalFailure::Closed));
        self.check_cancelled(response)
    }

    fn check_cancelled(&self, outcome: Outcome) -> Outcome {
        if self.cancelled.load(Ordering::SeqCst) {
            Err(ApprovalFailure::Cancelled)
        } else if self.state.lock().unwrap().closed {
            Err(ApprovalFailure::Closed)
        } else {
            outcome
        }
    }
}

impl Drop for PendingApproval {
    fn drop(&mut self) {
        // Keep rx alive until withdrawal has serialized with respond. It is
        // dropped only after this method, when no pending sender can remain.
        self.state.lock().unwrap().pending.remove(&self.request_id);
    }
}

async fn wait_cancelled(cancelled: &AtomicBool) {
    while !cancelled.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
