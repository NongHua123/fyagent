//! Cooperative cancellation shared by metadata resolution and artifact download.
//!
//! A job controller owns the concrete token. Core I/O code only observes this
//! narrow trait, so dropping a pending HTTP future can stop work before the
//! irreversible platform-install boundary.

use std::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use super::error::{InstallerError, InstallerErrorCode};

const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub trait Cancellation: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

impl Cancellation for AtomicBool {
    fn is_cancelled(&self) -> bool {
        self.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NeverCancelled;

impl Cancellation for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CancellationObserved;

/// Resolve only after a cancellation request is observed. The small polling
/// interval also makes this usable with an AtomicBool without adding a new
/// async cancellation dependency.
pub(crate) async fn wait_until_cancelled(cancellation: &dyn Cancellation) {
    while !cancellation.is_cancelled() {
        tokio::time::sleep(CANCELLATION_POLL_INTERVAL).await;
    }
}

/// Race any pending operation with cooperative cancellation. When cancellation
/// wins, the operation future is dropped immediately; reqwest then aborts its
/// request/body stream rather than keeping a job in a waiting stage.
pub(crate) async fn race_with_cancellation<T, F>(
    operation: F,
    cancellation: &dyn Cancellation,
) -> Result<T, CancellationObserved>
where
    F: Future<Output = T>,
{
    if cancellation.is_cancelled() {
        return Err(CancellationObserved);
    }

    tokio::select! {
        output = operation => Ok(output),
        _ = wait_until_cancelled(cancellation) => Err(CancellationObserved),
    }
}

pub(crate) fn cancellation_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::DownloadCancelled)
        .with_diagnostic_message("cancellation was observed before the install boundary")
}

#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicBool, Arc};

    use futures::future;

    use super::*;

    #[tokio::test]
    async fn dropping_a_pending_operation_observes_cancellation_promptly() {
        let cancellation = Arc::new(AtomicBool::new(false));
        let cancellation_for_task = Arc::clone(&cancellation);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancellation_for_task.store(true, Ordering::Release);
        });

        assert_eq!(
            race_with_cancellation(future::pending::<()>(), cancellation.as_ref()).await,
            Err(CancellationObserved)
        );
    }
}
