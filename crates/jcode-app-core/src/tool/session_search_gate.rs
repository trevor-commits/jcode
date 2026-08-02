use anyhow::{Result, anyhow};
use std::sync::{Arc, OnceLock};

static SESSION_SEARCH_GATE: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();

pub(super) async fn acquire_session_search_permit() -> Result<tokio::sync::OwnedSemaphorePermit> {
    SESSION_SEARCH_GATE
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(1)))
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| anyhow!("session_search concurrency gate closed"))
}
