//! File RPCs: list/read/write/stat/create/remove/rename and workspace watching.

use std::path::PathBuf;

use tonic::{Request, Response, Status};

use super::super::proto::{
    CreateDirRequest, Empty, FileEntry, FileInfo, FileList, ListFilesRequest, ReadFileRequest,
    ReadFileResponse, RemoveFileRequest, RenameFileRequest, RevealInExplorerRequest,
    StatFileRequest, WatchEvent, WatchWorkspaceRequest, WriteFileRequest,
};
use super::*;

fn resolve_ws_entry(root: &std::path::Path, relative: &str) -> Result<PathBuf, Status> {
    let path = resolve_ws_path(root, relative)?;
    if path == root {
        return Err(Status::invalid_argument("Choose a file or folder inside the workspace"));
    }
    Ok(path)
}

impl DaemonService {
    pub(crate) async fn watch_workspace_changes(
        &self,
        request: Request<WatchWorkspaceRequest>,
    ) -> Result<
        Response<tokio_stream::wrappers::ReceiverStream<Result<WorkspaceChange, Status>>>,
        Status,
    > {
        let root = PathBuf::from(request.into_inner().workspace_path);
        let ws = self
            .state
            .workspaces
            .get(&root)
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let mut changes = ws.db.subscribe_changes();
        let mut personal = self.state.global_db.as_ref().map(|db| db.subscribe_changes());
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            let mut sequence = 0_u64;
            loop {
                changes.borrow_and_update();
                if let Some(personal) = personal.as_mut() {
                    personal.borrow_and_update();
                }
                if tx
                    .send(Ok(WorkspaceChange {
                        sequence,
                    }))
                    .await
                    .is_err()
                {
                    break;
                }
                tokio::select! {
                    _ = tx.closed() => break,
                    change = changes.changed() => if change.is_err() { break; },
                    change = async { match personal.as_mut() { Some(personal) => personal.changed().await, None => std::future::pending().await } } => if change.is_err() { break; },
                }
                sequence = sequence.wrapping_add(1);
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
    pub(crate) async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<FileList>, Status> {
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let dir = resolve_ws_path(ws.root(), &req.dir)?;
        let meta = std::fs::metadata(&dir).map_err(io_status)?;
        if !meta.is_dir() {
            return Err(Status::invalid_argument("not a directory"));
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(io_status)? {
            let entry = entry.map_err(io_status)?;
            let name = entry.file_name().to_string_lossy().to_string();
            let rel = entry
                .path()
                .strip_prefix(ws.root())
                .map_err(|_| Status::internal("entry outside workspace"))?
                .to_string_lossy()
                .replace('\\', "/");
            // The workspace metadata directory stays invisible to clients.
            if std::path::Path::new(&rel).starts_with(crate::workspace::METADATA_DIR) {
                continue;
            }
            let is_dir = entry.file_type().map_err(io_status)?.is_dir();
            entries.push(FileEntry {
                name,
                path: rel,
                is_dir,
            });
        }
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
        Ok(Response::new(FileList {
            entries,
        }))
    }

    pub(crate) async fn read_file(
        &self,
        request: Request<ReadFileRequest>,
    ) -> Result<Response<ReadFileResponse>, Status> {
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let path = resolve_ws_path(ws.root(), &req.path)?;
        let bytes = std::fs::read(&path).map_err(io_status)?;
        let content = String::from_utf8(bytes)
            .map_err(|_| Status::invalid_argument("file is not valid utf-8"))?;
        Ok(Response::new(ReadFileResponse {
            content,
        }))
    }

    pub(crate) async fn write_file(
        &self,
        request: Request<WriteFileRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let path = resolve_ws_entry(ws.root(), &req.path)?;
        let _admission = ws.admit().await.map_err(to_status)?;
        if req.create_only && path.exists() {
            return Err(Status::already_exists("file already exists"));
        }
        if let Some(expected) = &req.expected_content {
            if std::fs::read(&path).map_err(io_status)? != expected.as_bytes() {
                return Err(Status::failed_precondition(
                    "File changed on disk; reload or explicitly overwrite it",
                ));
            }
        }
        let bound = crate::storage::blueprint_files::bindings(&ws.db)
            .map_err(to_status)?
            .into_iter()
            .any(|(_, v)| {
                ws.version_manager.blueprint_path(&v.blueprint_uri).is_ok_and(|p| p == path)
            });
        if bound || path.extension().is_some_and(|ext| ext == "blueprint") {
            let graph = crate::storage::blueprint_files::decode(req.content.as_bytes())
                .map_err(to_status)?;
            let registry = self.state.registry_for(Some(ws.root()), false).await?;
            super::blueprint::ensure_valid(&graph, &registry)?;
            self.ensure_blueprint_idle(&ws.root, graph.id).await?;
            crate::replan::application::ensure_resolved(&ws.db).map_err(to_status)?;
            crate::storage::blueprint_files::save(
                &ws.db,
                &ws.version_manager,
                &graph,
                &req.path,
                req.content.as_bytes(),
                None,
            )
            .map_err(to_status)?;
        } else {
            mutate_regular_file(
                &ws,
                &path,
                Some(req.content.as_bytes()),
                req.expected_content.as_deref().map(str::as_bytes),
            )
            .map_err(to_status)?;
        }
        Ok(Response::new(Empty {}))
    }

    pub(crate) async fn stat_file(
        &self,
        request: Request<StatFileRequest>,
    ) -> Result<Response<FileInfo>, Status> {
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let path = resolve_ws_path(ws.root(), &req.path)?;
        let meta = std::fs::metadata(&path).map_err(io_status)?;
        Ok(Response::new(FileInfo {
            path: req.path,
            is_dir: meta.is_dir(),
            len: meta.len() as i64,
        }))
    }

    pub(crate) async fn create_dir(
        &self,
        request: Request<CreateDirRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let path = resolve_ws_entry(ws.root(), &req.path)?;
        let _admission = ws.admit().await.map_err(to_status)?;
        std::fs::create_dir_all(&path).map_err(io_status)?;
        Ok(Response::new(Empty {}))
    }

    pub(crate) async fn remove_file(
        &self,
        request: Request<RemoveFileRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let path = resolve_ws_entry(ws.root(), &req.path)?;
        let meta = std::fs::symlink_metadata(&path).map_err(io_status)?;
        let _admission = ws.admit().await.map_err(to_status)?;
        protect_blueprint_identity(&ws, &path)?;
        if meta.is_dir() {
            std::fs::remove_dir_all(&path).map_err(io_status)?;
        } else {
            mutate_regular_file(&ws, &path, None, None).map_err(to_status)?;
        }
        Ok(Response::new(Empty {}))
    }

    pub(crate) async fn rename_file(
        &self,
        request: Request<RenameFileRequest>,
    ) -> Result<Response<Empty>, Status> {
        let subject = super::super::acl::subject_from_request(&request).unwrap_or_else(|| "local".into());
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let from = resolve_ws_entry(ws.root(), &req.from)?;
        let to = resolve_ws_entry(ws.root(), &req.to)?;
        let _admission = ws.admit().await.map_err(to_status)?;
        protect_blueprint_identity(&ws, &from)?;
        protect_blueprint_identity(&ws, &to)?;
        if from == to {
            return Ok(Response::new(Empty {}));
        }
        if to.exists() {
            return Err(Status::already_exists("Rename destination already exists"));
        }
        std::fs::rename(&from, &to).map_err(io_status)?;
        let identity = format!("project:{}", ws.root().to_string_lossy());
        let store = crate::storage::recovery::RecoveryStore::new(ws.db.clone(), &identity, &subject)
            .map_err(|_| Status::internal("File renamed; recovery copies require reconciliation"))?;
        let from_relative = from.strip_prefix(ws.root()).map_err(|_| Status::internal("Rename source unavailable"))?.to_string_lossy().replace('\\', "/");
        let to_relative = to.strip_prefix(ws.root()).map_err(|_| Status::internal("Rename destination unavailable"))?.to_string_lossy().replace('\\', "/");
        store.rename_entity(&from_relative, &to_relative, chrono::Utc::now().timestamp_millis() as u64)
            .map_err(|_| Status::internal("File renamed; recovery transfer failed. Original recovery copies were retained for manual review"))?;
        Ok(Response::new(Empty {}))
    }

    /// Opens the system file manager with the entry selected (best effort).
    pub(crate) async fn reveal_in_explorer(
        &self,
        request: Request<RevealInExplorerRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let path = resolve_ws_path(ws.root(), &req.path)?;
        reveal(&path);
        Ok(Response::new(Empty {}))
    }

    /// Streams live file changes (created / modified / removed) for a workspace.
    ///
    /// The stream stays open until the client disconnects or the workspace is
    /// closed; events are dropped with no back-pressure when the client lags.
    pub(crate) async fn watch_workspace(
        &self,
        request: Request<WatchWorkspaceRequest>,
    ) -> Result<Response<tokio_stream::wrappers::ReceiverStream<Result<WatchEvent, Status>>>, Status>
    {
        let req = request.into_inner();
        let ws = self
            .state
            .workspaces
            .get(&PathBuf::from(&req.workspace_path))
            .await
            .ok_or_else(|| Status::not_found("workspace not open"))?;
        let mut rx = ws
            .watch_files()
            .ok_or_else(|| Status::unavailable("workspace file watcher unavailable"))?;
        let root = ws.root().to_path_buf();
        let (tx, rx_stream) = tokio::sync::mpsc::channel(64);
        tokio::spawn(async move {
            loop {
                let event = tokio::select! {
                    _ = tx.closed() => break,
                    event = rx.recv() => event,
                };
                match event {
                    Ok(ev) => {
                        let rel = ev.path.strip_prefix(&root).unwrap_or(&ev.path);
                        let rel_str = rel.to_string_lossy().replace('\\', "/");
                        let kind = match ev.kind {
                            crate::storage::versioning::watcher::WatchKind::Created => "created",
                            crate::storage::versioning::watcher::WatchKind::Modified => "modified",
                            crate::storage::versioning::watcher::WatchKind::Removed => "removed",
                        };
                        if tx
                            .send(Ok(WatchEvent {
                                path: rel_str,
                                kind: kind.to_string(),
                            }))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    // Reconnect forces a full resync; silently dropping a removal loses editor state.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let _ = tx
                            .send(Err(Status::resource_exhausted(
                                "File events overflowed; resubscribe and resync",
                            )))
                            .await;
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx_stream)))
    }
}

/// User file RPCs receive their own operation identity, outside an agent run.
fn mutate_regular_file(
    ws: &crate::workspace::manager::Workspace,
    path: &std::path::Path,
    after: Option<&[u8]>,
    expected: Option<&[u8]>,
) -> crate::error::DaemonResult<()> {
    use crate::execution::file_journal::{DbFileJournal, FileJournal, FileOrigin};
    let log = crate::execution::TransactionLog::new().with_journal(FileJournal::new(
        ws.root.clone(),
        std::sync::Arc::new(DbFileJournal(ws.db.clone())),
    ));
    log.mutate_file(
        ws.root(),
        path,
        after,
        FileOrigin {
            run_id: uuid::Uuid::new_v4(),
            node_id: uuid::Uuid::nil(),
            attempt: 1,
            wal_position: 0,
        },
        expected,
    )
}

/// Opens the OS file manager with `path` selected, fire-and-forget.
fn reveal(path: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn();
    }
    #[cfg(not(windows))]
    {
        // Unix has no portable "reveal"; open the parent directory instead.
        let parent = path.parent().unwrap_or(path);
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("xdg-open '{}' >/dev/null 2>&1 &", parent.display()))
            .spawn();
    }
}

/// A path binding is stable. Moving/deleting an authority needs an explicit
/// identity migration, rather than silently leaving a stale database mirror.
fn protect_blueprint_identity(
    ws: &crate::workspace::manager::Workspace,
    path: &std::path::Path,
) -> Result<(), Status> {
    for (_, version) in crate::storage::blueprint_files::bindings(&ws.db).map_err(to_status)? {
        if ws
            .version_manager
            .blueprint_path(&version.blueprint_uri)
            .map_err(to_status)?
            .starts_with(path)
        {
            return Err(Status::failed_precondition(
                "path contains an authoritative blueprint; identity migration is not supported",
            ));
        }
    }
    Ok(())
}
