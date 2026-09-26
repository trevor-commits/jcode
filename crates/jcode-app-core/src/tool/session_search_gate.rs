use anyhow::{Result, anyhow};
use std::sync::{Arc, OnceLock};

static SESSION_SEARCH_GATE: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();

fn gate() -> Arc<tokio::sync::Semaphore> {
    SESSION_SEARCH_GATE
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(1)))
        .clone()
}

pub(super) async fn acquire_session_search_permit() -> Result<tokio::sync::OwnedSemaphorePermit> {
    gate()
        .acquire_owned()
        .await
        .map_err(|_| anyhow!("session_search concurrency gate closed"))
}

/// Run blocking session-search work while holding the gate permit for the full
/// blocking lifetime. The permit is moved into the `spawn_blocking` closure so
/// cancellation of the awaiting task cannot release it while work still runs.
pub(super) async fn spawn_blocking_with_session_search_permit<R, F>(f: F) -> Result<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let permit = acquire_session_search_permit().await?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        f()
    })
    .await
    .map_err(|err| anyhow!("session_search blocking task failed: {err}"))
}
