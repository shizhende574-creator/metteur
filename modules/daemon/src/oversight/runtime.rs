//! A run-scoped lifetime; no workspace polling task survives execution.
use super::scheduler;
use crate::{execution::context::ExecutionContext, storage::persistence::Db};
use metteur_shared::config::oversight::OversightConfig;
use uuid::Uuid;

pub struct Runtime {
    db: Db,
    run: Uuid,
    timer: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
        if let Err(error) = scheduler::close(&self.db, self.run) {
            tracing::warn!(%error,"could not close review scheduler");
        }
    }
}
pub async fn start(ctx: &ExecutionContext) -> Option<Runtime> {
    let db = ctx.workspace_db.as_ref()?;
    if ctx.run_id.is_nil() {
        return None;
    }
    let config = match &ctx.config {
        Some(config) => config.read().await.clone(),
        None => Default::default(),
    };
    let settings = match OversightConfig::from_config(&config) {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!(%error,"review settings unavailable");
            return None;
        }
    };
    if let Err(error) = scheduler::initialize(db, ctx.run_id, settings.clone()) {
        tracing::warn!(%error,"review scheduler unavailable");
        return None;
    }
    let timer = if settings.triggers.interval_ms > 0 {
        let db = db.clone();
        let run = ctx.run_id;
        Some(tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(settings.triggers.interval_ms))
                    .await;
                if scheduler::trigger(&db, run, "interval", scheduler::now()).is_err() {
                    break;
                }
            }
        }))
    } else {
        None
    };
    Some(Runtime {
        db: db.clone(),
        run: ctx.run_id,
        timer,
    })
}
