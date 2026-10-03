//! Reconcile interrupted file operations without replaying unknown effects.

use super::file_journal::{FileJournal, FileOperation, FilePhase, read_optional, validate_path};
use crate::error::{DaemonError, DaemonResult};

#[derive(Debug, Default)]
pub struct RecoveryReport {
    pub reconciled: usize,
    pub conflicts: Vec<std::path::PathBuf>,
}

impl RecoveryReport {
    pub fn ensure_safe(self) -> DaemonResult<Self> {
        if self.conflicts.is_empty() {
            Ok(self)
        } else {
            Err(DaemonError::Persistence(format!(
                "file recovery conflicts require manual resolution: {}",
                self.conflicts
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
    }
}

#[derive(PartialEq)]
enum ContentState {
    Before,
    After,
    Conflict,
}

impl FileJournal {
    fn content_state(&self, op: &FileOperation) -> DaemonResult<ContentState> {
        validate_path(&self.root, &op.path)?;
        // Load both images before any write, including an undo's before-image.
        let load = |image: &Option<String>| {
            image.as_deref().map(|h| self.store.blob(h)).transpose().map_err(|e| {
                DaemonError::Persistence(format!("file recovery image unavailable: {e}"))
            })
        };
        let before = load(&op.before)?;
        let after = load(&op.after)?;
        let current = read_optional(&op.path)?;
        Ok(if current == before {
            ContentState::Before
        } else if current == after {
            ContentState::After
        } else {
            ContentState::Conflict
        })
    }

    /// Call only with no active run, before admitting new workspace effects.
    /// Settled Applied records are history: later writes may supersede them.
    pub fn reconcile(&self) -> DaemonResult<RecoveryReport> {
        let mut report = RecoveryReport::default();
        let mut operations = self.store.operations()?;
        operations.sort_by_key(|op| (op.origin.run_id, std::cmp::Reverse(op.origin.wal_position)));
        for mut op in operations {
            if matches!(op.phase, FilePhase::Applied | FilePhase::Reverted) {
                continue;
            }
            match self.content_state(&op)? {
                ContentState::Before => op.phase = FilePhase::Reverted,
                ContentState::After if op.phase == FilePhase::Reverting => {
                    self.restore(&op.path, &op.before)?;
                    op.phase = FilePhase::Reverted;
                }
                ContentState::After => op.phase = FilePhase::Applied,
                ContentState::Conflict => {
                    op.phase = FilePhase::Conflict;
                    report.conflicts.push(op.path.clone());
                }
            }
            self.save(&op)?;
            report.reconciled += 1;
        }
        Ok(report)
    }

    /// Returns whether this call settled an outstanding undo. A durable undo
    /// fence makes crashes before/after restoring content distinguishable.
    pub fn rollback_operation(&self, op: &mut FileOperation) -> DaemonResult<bool> {
        if op.phase == FilePhase::Reverted {
            return Ok(false);
        }
        match self.content_state(op)? {
            ContentState::Before => {}
            ContentState::After => {
                op.phase = FilePhase::Reverting;
                self.save(op)?;
                self.restore(&op.path, &op.before)?;
            }
            ContentState::Conflict => {
                op.phase = FilePhase::Conflict;
                self.save(op)?;
                return Err(DaemonError::Execution(format!(
                    "file conflict at {}; later content was preserved",
                    op.path.display()
                )));
            }
        }
        op.phase = FilePhase::Reverted;
        self.save(op)?;
        Ok(true)
    }
}
