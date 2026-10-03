//! Durable file intent coverage using the workspace DB and the storage seam.
use metteur_daemon::error::{DaemonError, DaemonResult};
use metteur_daemon::execution::file_journal::*;
use metteur_daemon::execution::{ExecutionContext, TransactionLog};
use metteur_daemon::llm::LlmClientFactory;
use metteur_daemon::registry::{
    Registry, Tool,
    tools::{edit::EditFile, fs_tools::WriteFile},
};
use metteur_daemon::storage::persistence::Db;
use metteur_shared::Value;
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

fn fixture() -> (PathBuf, Db, FileJournal) {
    let root = std::env::temp_dir().join(format!("metteur-intents-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db = Db::open(&root.join(".metteur/db")).unwrap();
    let journal = FileJournal::new(root.clone(), Arc::new(DbFileJournal(db.clone())));
    (root, db, journal)
}

fn origin() -> FileOrigin {
    FileOrigin {
        run_id: Uuid::new_v4(),
        node_id: Uuid::new_v4(),
        attempt: 2,
        wal_position: 0,
    }
}

#[derive(Debug)]
struct FailingStore {
    inner: DbFileJournal,
    phase: Option<FilePhase>,
    blob: bool,
}
impl FileJournalStore for FailingStore {
    fn save(&self, op: &FileOperation) -> DaemonResult<()> {
        if self.phase == Some(op.phase) {
            return Err(DaemonError::Internal("injected save failure".into()));
        }
        self.inner.save(op)
    }
    fn operations(&self) -> DaemonResult<Vec<FileOperation>> {
        self.inner.operations()
    }
    fn put_blob(&self, bytes: &[u8]) -> DaemonResult<String> {
        if self.blob {
            return Err(DaemonError::Internal("injected blob failure".into()));
        }
        self.inner.put_blob(bytes)
    }
    fn blob(&self, hash: &str) -> DaemonResult<Vec<u8>> {
        self.inner.blob(hash)
    }
}

#[test]
fn intent_and_blob_failures_leave_file_unchanged() {
    for (phase, blob) in [(Some(FilePhase::Prepared), false), (None, true)] {
        let (root, db, journal) = fixture();
        let path = root.join("file");
        std::fs::write(&path, b"before").unwrap();
        let log = TransactionLog::new().with_journal(FileJournal::new(
            root.clone(),
            Arc::new(FailingStore {
                inner: DbFileJournal(db),
                phase,
                blob,
            }),
        ));
        assert!(matches!(
            log.mutate_file(&root, &path, Some(b"after"), origin(), None),
            Err(DaemonError::Persistence(_))
        ));
        assert_eq!(std::fs::read(path).unwrap(), b"before");
        assert!(journal.store.operations().unwrap().is_empty());
        assert!(log.ensure_healthy().is_err());
    }
}

#[test]
fn failed_applied_save_retains_preimage_and_stops_later_mutations() {
    let (root, db, journal) = fixture();
    let path = root.join("file");
    std::fs::write(&path, b"before").unwrap();
    let log = TransactionLog::new().with_journal(FileJournal::new(
        root.clone(),
        Arc::new(FailingStore {
            inner: DbFileJournal(db),
            phase: Some(FilePhase::Applied),
            blob: false,
        }),
    ));
    assert!(log.mutate_file(&root, &path, Some(b"after"), origin(), None).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"after");
    let records = journal.store.operations().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].phase, FilePhase::Prepared);
    assert_eq!(journal.store.blob(records[0].before.as_deref().unwrap()).unwrap(), b"before");
    let next = root.join("next");
    assert!(log.mutate_file(&root, &next, Some(b"never"), origin(), None).is_err());
    assert!(!next.exists());
    // A fresh run also cannot overwrite the unresolved operation.
    assert!(journal.prepare(&path, Some(b"never"), origin(), None).is_err());
}

#[test]
fn create_overwrite_delete_have_stable_identity_order_and_shared_blobs() {
    let (root, db, journal) = fixture();
    let path = root.join("file");
    let log = TransactionLog::new().with_journal(journal.clone());
    let origin = origin();
    for after in [Some(b"one".as_slice()), Some(b"two".as_slice()), None, Some(b"three".as_slice())]
    {
        log.mutate_file(&root, &path, after, origin.clone(), None).unwrap();
    }
    let mut records = journal.store.operations().unwrap();
    records.sort_by_key(|op| op.origin.wal_position);
    assert_eq!(records.len(), 4);
    for (index, op) in records.iter().enumerate() {
        assert_eq!(op.origin.wal_position, index);
        assert_eq!(op.origin.run_id, origin.run_id);
        assert_eq!(op.origin.attempt, 2);
        assert_eq!(op.phase, FilePhase::Applied);
    }
    assert_eq!(
        records.iter().map(|op| op.operation_id).collect::<std::collections::HashSet<_>>().len(),
        4
    );
    assert_eq!(records[0].after, records[1].before);
    assert_eq!(records[1].after, records[2].before);
    assert_eq!(records[2].after, records[3].before);
    let snapshot = metteur_daemon::storage::versioning::VersionManager::new(db, root.clone())
        .create_snapshot("same blobs")
        .unwrap();
    assert_eq!(snapshot.files["file"], records[3].after.clone().unwrap());
    assert!(serde_json::to_string(&log.entries()).unwrap().contains("operation_id"));
    assert!(!serde_json::to_string(&log.entries()).unwrap().contains("three"));
}

#[tokio::test]
async fn write_and_edit_tools_use_the_same_durable_journal() {
    let (root, db, journal) = fixture();
    let mut ctx = ExecutionContext::new(
        Arc::new(Registry::with_builtins()),
        LlmClientFactory::new(),
        root.clone(),
    );
    ctx.workspace_db = Some(db);
    ctx.run_id = Uuid::new_v4();
    WriteFile
        .call(&[Value::Json(serde_json::json!({"path":"file", "content":"alpha"}))], &mut ctx)
        .await
        .unwrap();
    EditFile.call(&[Value::Json(serde_json::json!({"path":"file", "edits":[{"old_string":"alpha", "new_string":"beta"}]}))], &mut ctx).await.unwrap();
    assert_eq!(std::fs::read(root.join("file")).unwrap(), b"beta");
    assert_eq!(journal.store.operations().unwrap().len(), 2);
    let mut child = ctx.child_nested();
    WriteFile
        .call(&[Value::Json(serde_json::json!({"path":"child", "content":"shared"}))], &mut child)
        .await
        .unwrap();
    assert_eq!(ctx.transaction_log.entries().len(), 3);
    ctx.transaction_log.rollback().unwrap();
    assert!(!root.join("file").exists());
    assert!(!root.join("child").exists());
}

#[test]
fn directories_and_metadata_paths_are_rejected() {
    let (root, _, journal) = fixture();
    assert!(journal.prepare(&root.join(".metteur/secret"), Some(b"x"), origin(), None).is_err());
    let dir = root.join("dir");
    std::fs::create_dir_all(&dir).unwrap();
    assert!(journal.prepare(&dir, None, origin(), None).is_err());
    assert!(journal.store.operations().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn symlinks_and_hardlinks_are_rejected_before_intent_or_write() {
    let (root, _, journal) = fixture();
    let original = root.join("original");
    std::fs::write(&original, b"safe").unwrap();
    let link = root.join("link");
    std::os::unix::fs::symlink(&original, &link).unwrap();
    assert!(journal.prepare(&link, Some(b"x"), origin(), None).is_err());
    std::fs::hard_link(&original, root.join("hard")).unwrap();
    assert!(journal.prepare(&original, Some(b"x"), origin(), None).is_err());
    assert_eq!(std::fs::read(&original).unwrap(), b"safe");
    assert!(journal.store.operations().unwrap().is_empty());
}

#[tokio::test]
async fn persistence_failure_stops_the_remaining_react_tool_batch() {
    use metteur_daemon::execution::react::{ReactOptions, run_react};
    use metteur_daemon::llm::MockStep;
    use metteur_shared::llm::{ContextManager, ToolCall};
    let (root, db, _) = fixture();
    let mut ctx = ExecutionContext::new(
        Arc::new(Registry::with_builtins()),
        LlmClientFactory::new(),
        root.clone(),
    );
    ctx.transaction_log = TransactionLog::new().with_journal(FileJournal::new(
        root.clone(),
        Arc::new(FailingStore {
            inner: DbFileJournal(db),
            phase: Some(FilePhase::Prepared),
            blob: false,
        }),
    ));
    let calls = ["first", "second"]
        .into_iter()
        .map(|name| ToolCall {
            id: name.into(),
            name: "WriteFile".into(),
            arguments: serde_json::json!({"path": name, "content":"never"}),
        })
        .collect();
    let opts = ReactOptions {
        provider: "mock".into(),
        mock_steps: Some(vec![MockStep::Tools(calls)]),
        ..Default::default()
    };
    assert!(matches!(
        run_react(&mut ctx, ContextManager::new_from_prompt(vec![], "write"), &opts).await,
        Err(DaemonError::Persistence(_))
    ));
    assert!(!root.join("first").exists());
    assert!(!root.join("second").exists());
    assert_eq!(ctx.transaction_log.entries().len(), 1, "only the first tool was started");
}
