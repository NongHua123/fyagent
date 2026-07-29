//! In-memory installation job coordination.
//!
//! `JobStore` owns the one active desktop installation slot for the current
//! process.  It deliberately has no persistence: a restarted application must
//! not resume a partially completed installation job.  Every mutation happens
//! under one short synchronous mutex, while callers perform network, file and
//! platform work outside this module.

use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

use uuid::Uuid;

use super::{
    error::{InstallerError, InstallerErrorCode},
    types::{InstallResult, JobProgress, JobSnapshot, JobStage, RemoteReleaseStatus},
};

/// Narrow event boundary used by the Tauri integration layer.
///
/// The store gives the sink an owned, complete snapshot only after it releases
/// its state mutex.  Emitting must therefore not mutate the store or block on
/// installation work.
pub trait JobEventSink: Send + Sync {
    fn emit_snapshot(&self, snapshot: JobSnapshot);
}

impl<F> JobEventSink for F
where
    F: Fn(JobSnapshot) + Send + Sync,
{
    fn emit_snapshot(&self, snapshot: JobSnapshot) {
        self(snapshot);
    }
}

#[derive(Default)]
struct NoopJobEventSink;

impl JobEventSink for NoopJobEventSink {
    fn emit_snapshot(&self, _snapshot: JobSnapshot) {}
}

/// Read-only cancellation handle for long-running workers.
///
/// The handle intentionally carries no job ID or mutation methods.  Workers
/// can observe a cancellation request without being able to move a job into a
/// different state.
#[derive(Clone)]
pub struct JobCancellation {
    requested: Arc<AtomicBool>,
}

impl JobCancellation {
    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
}

impl super::cancellation::Cancellation for JobCancellation {
    fn is_cancelled(&self) -> bool {
        self.is_requested()
    }
}

/// The state and cancellation signal for one job.  The fields stay private so
/// all externally visible changes keep the snapshot sequence invariant.
pub struct JobController {
    snapshot: JobSnapshot,
    cancellation: JobCancellation,
}

impl JobController {
    fn new(snapshot: JobSnapshot) -> Self {
        Self {
            snapshot,
            cancellation: JobCancellation {
                requested: Arc::new(AtomicBool::new(false)),
            },
        }
    }

    fn snapshot(&self) -> JobSnapshot {
        self.snapshot.clone()
    }

    fn cancellation_handle(&self) -> JobCancellation {
        self.cancellation.clone()
    }

    fn is_terminal(&self) -> bool {
        self.snapshot.stage.is_terminal()
    }

    fn bump_sequence(&mut self, timestamp: String) -> Result<(), InstallerError> {
        self.snapshot.sequence =
            self.snapshot.sequence.checked_add(1).ok_or_else(|| {
                internal_error("job snapshot sequence exhausted the supported range")
            })?;
        self.snapshot.updated_at = timestamp;
        Ok(())
    }

    /// Marks the request immediately so I/O can stop, but deliberately keeps
    /// the job non-terminal until its worker has acknowledged cancellation and
    /// removed the job-scoped temporary data.
    fn request_cancellation(&mut self, timestamp: String) -> Result<JobSnapshot, InstallerError> {
        debug_assert!(self.snapshot.stage.is_cancellable());

        self.cancellation.requested.store(true, Ordering::Release);
        self.snapshot.cancellable = false;
        self.snapshot.progress = None;
        self.bump_sequence(timestamp)?;
        Ok(self.snapshot())
    }

    /// Publishes the terminal snapshot only after the worker has stopped
    /// cancellable I/O and completed its cleanup acknowledgement.
    fn transition_to_cancelled(
        &mut self,
        timestamp: String,
    ) -> Result<JobSnapshot, InstallerError> {
        debug_assert!(self.cancellation.is_requested());
        debug_assert!(self.snapshot.stage.is_cancellable());
        debug_assert!(self.snapshot.stage.can_transition_to(JobStage::Cancelled));

        self.snapshot.stage = JobStage::Cancelled;
        self.snapshot.progress = None;
        self.snapshot.result = None;
        self.snapshot.error = None;
        self.bump_sequence(timestamp)?;
        Ok(self.snapshot())
    }
}

#[derive(Default)]
struct JobStoreState {
    current: Option<JobController>,
    // A successful application-restart request owns the same mutex as
    // `try_start`. It prevents a new worker from appearing in the delay before
    // the process re-execs, while preserving any active worker unchanged when
    // the restart claim is refused.
    restart_pending: bool,
    // The set is intentionally process-local.  It makes the "never reuse a
    // job id" invariant exact even if a UUID generator were ever to collide.
    issued_job_ids: HashSet<String>,
}

/// Process-local, single-job coordinator.
///
/// A terminal snapshot remains available until another job is started.  A new
/// start replaces only a terminal controller; it never replaces a running one.
#[derive(Clone)]
pub struct JobStore {
    state: Arc<Mutex<JobStoreState>>,
    event_sink: Arc<dyn JobEventSink>,
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl JobStore {
    pub fn new() -> Self {
        Self::with_event_sink(Arc::new(NoopJobEventSink))
    }

    pub fn with_event_sink(event_sink: Arc<dyn JobEventSink>) -> Self {
        Self {
            state: Arc::new(Mutex::new(JobStoreState::default())),
            event_sink,
        }
    }

    /// Returns the latest snapshot, including a terminal snapshot that has not
    /// yet been superseded by another job.
    pub fn get(&self) -> Result<Option<JobSnapshot>, InstallerError> {
        Ok(self
            .lock_state()?
            .current
            .as_ref()
            .map(JobController::snapshot))
    }

    /// Atomically claims the only non-terminal job slot and publishes its
    /// initial `Checking` snapshot.
    pub fn try_start(
        &self,
        release: RemoteReleaseStatus,
        timestamp: impl Into<String>,
    ) -> Result<JobSnapshot, InstallerError> {
        let snapshot = {
            let mut state = self.lock_state()?;
            if state.restart_pending {
                return Err(restart_pending_error());
            }
            if let Some(current) = state.current.as_ref() {
                if !current.is_terminal() {
                    return Err(InstallerError::new(InstallerErrorCode::JobAlreadyRunning)
                        .with_context("job_id", &current.snapshot.job_id)
                        .with_diagnostic_message("a desktop installation job is already active"));
                }
            }

            let job_id = loop {
                let candidate = Uuid::new_v4().to_string();
                if state.issued_job_ids.insert(candidate.clone()) {
                    break candidate;
                }
            };
            let snapshot = JobSnapshot::checking(job_id, release, timestamp);
            let controller = JobController::new(snapshot.clone());
            state.current = Some(controller);
            snapshot
        };

        self.publish(snapshot.clone());
        Ok(snapshot)
    }

    /// Atomically reserves the job slot for an application restart. The claim
    /// is valid only when there is no job or the retained snapshot is terminal;
    /// a cancellation request remains non-terminal until its worker cleanup
    /// acknowledgement, so it intentionally blocks restart here.
    pub fn claim_restart(&self) -> Result<(), InstallerError> {
        let mut state = self.lock_state()?;
        if state.restart_pending {
            return Err(restart_pending_error());
        }
        if let Some(current) = state.current.as_ref() {
            if !restart_claim_is_allowed(Some(current.snapshot.stage)) {
                return Err(InstallerError::new(InstallerErrorCode::JobAlreadyRunning)
                    .with_context("job_id", &current.snapshot.job_id)
                    .with_diagnostic_message(
                        "a non-terminal desktop installation job blocks application restart",
                    ));
            }
        }
        state.restart_pending = true;
        Ok(())
    }

    /// Obtains the cancellation signal for the current job.  Service workers
    /// should keep this handle while they await I/O so cancellation remains
    /// observable even after the terminal snapshot has been superseded.
    pub fn cancellation_handle(&self, job_id: &str) -> Result<JobCancellation, InstallerError> {
        let state = self.lock_state()?;
        let controller = current_job(&state, job_id)?;
        Ok(controller.cancellation_handle())
    }

    /// Transitions a running job to a non-terminal stage.
    ///
    /// In particular, callers must inspect the returned snapshot before
    /// invoking a platform installer: if a competing cancellation won the
    /// shared lock, this leaves the job at its prior cancellable stage and
    /// lets the worker publish `Cancelled` only after cleanup.
    pub fn update_stage(
        &self,
        job_id: &str,
        next_stage: JobStage,
        timestamp: impl Into<String>,
    ) -> Result<JobSnapshot, InstallerError> {
        let timestamp = timestamp.into();
        self.mutate(job_id, move |controller| {
            if controller.is_terminal() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }

            if controller.cancellation.is_requested() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }

            if next_stage.is_terminal() {
                return Err(invalid_transition(
                    controller.snapshot.stage,
                    next_stage,
                    "terminal stages are owned by succeed, fail, and request_cancel",
                ));
            }

            let current_stage = controller.snapshot.stage;
            if !current_stage.can_transition_to(next_stage) {
                return Err(invalid_transition(
                    current_stage,
                    next_stage,
                    "the requested stage is not reachable from the current stage",
                ));
            }

            controller.snapshot.stage = next_stage;
            controller.snapshot.cancellable = next_stage.is_cancellable();
            controller.snapshot.progress = None;
            controller.snapshot.result = None;
            controller.snapshot.error = None;
            controller.bump_sequence(timestamp)?;
            Ok(JobMutation::changed(controller.snapshot()))
        })
    }

    /// Replaces progress for a running job and publishes the complete
    /// snapshot.  The progress DTO itself owns byte/percentage normalization.
    pub fn update_progress(
        &self,
        job_id: &str,
        progress: JobProgress,
        timestamp: impl Into<String>,
    ) -> Result<JobSnapshot, InstallerError> {
        let timestamp = timestamp.into();
        self.mutate(job_id, move |controller| {
            if controller.is_terminal() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }

            if controller.cancellation.is_requested() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }

            if !stage_accepts_progress(controller.snapshot.stage, progress.phase) {
                return Err(internal_error(
                    "job progress does not match the active installation stage",
                ));
            }

            controller.snapshot.progress = Some(progress);
            controller.bump_sequence(timestamp)?;
            Ok(JobMutation::changed(controller.snapshot()))
        })
    }

    /// Completes a verified installation.  Success is legal only after the
    /// post-install verification stage, so a platform command completion alone
    /// cannot publish success.
    pub fn succeed(
        &self,
        job_id: &str,
        result: InstallResult,
        timestamp: impl Into<String>,
    ) -> Result<JobSnapshot, InstallerError> {
        let timestamp = timestamp.into();
        self.mutate(job_id, move |controller| {
            if controller.is_terminal() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }

            let current_stage = controller.snapshot.stage;
            if !current_stage.can_transition_to(JobStage::Succeeded) {
                return Err(invalid_transition(
                    current_stage,
                    JobStage::Succeeded,
                    "success requires completed post-install verification",
                ));
            }

            controller.snapshot.stage = JobStage::Succeeded;
            controller.snapshot.cancellable = false;
            controller.snapshot.progress = None;
            controller.snapshot.result = Some(result);
            controller.snapshot.error = None;
            controller.bump_sequence(timestamp)?;
            Ok(JobMutation::changed(controller.snapshot()))
        })
    }

    /// Completes a no-download path after the service has freshly verified and
    /// launched an equal-or-newer local Stable application. This is deliberately
    /// narrower than `succeed`: only the cancellable Checking stage may take
    /// this path, so an install can never be reported as complete without its
    /// normal post-install verification.
    pub fn succeed_after_launch(
        &self,
        job_id: &str,
        result: InstallResult,
        timestamp: impl Into<String>,
    ) -> Result<JobSnapshot, InstallerError> {
        let timestamp = timestamp.into();
        self.mutate(job_id, move |controller| {
            if controller.is_terminal() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }
            if controller.snapshot.stage != JobStage::Checking {
                return Err(invalid_transition(
                    controller.snapshot.stage,
                    JobStage::Succeeded,
                    "launch-only success is allowed only while checking local state",
                ));
            }
            if controller.cancellation.is_requested() {
                return Ok(JobMutation::changed(
                    controller.transition_to_cancelled(timestamp)?,
                ));
            }

            controller.snapshot.stage = JobStage::Succeeded;
            controller.snapshot.cancellable = false;
            controller.snapshot.progress = None;
            controller.snapshot.result = Some(result);
            controller.snapshot.error = None;
            controller.bump_sequence(timestamp)?;
            Ok(JobMutation::changed(controller.snapshot()))
        })
    }

    /// Fails a running job while preserving one stable, stage-tagged error in
    /// its terminal snapshot.
    pub fn fail(
        &self,
        job_id: &str,
        error: InstallerError,
        timestamp: impl Into<String>,
    ) -> Result<JobSnapshot, InstallerError> {
        let timestamp = timestamp.into();
        self.mutate(job_id, move |controller| {
            if controller.is_terminal() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }

            let current_stage = controller.snapshot.stage;
            if !current_stage.can_transition_to(JobStage::Failed) {
                return Err(invalid_transition(
                    current_stage,
                    JobStage::Failed,
                    "failure is not reachable from the current stage",
                ));
            }

            controller.snapshot.stage = JobStage::Failed;
            controller.snapshot.cancellable = false;
            controller.snapshot.progress = None;
            controller.snapshot.result = None;
            controller.snapshot.error = Some(error.with_stage(current_stage).to_dto());
            controller.bump_sequence(timestamp)?;
            Ok(JobMutation::changed(controller.snapshot()))
        })
    }

    /// Requests cancellation at the documented reversible boundary.
    ///
    /// The request and entering `Installing` share the same mutex. If
    /// installing wins first, this is a no-op; if cancellation wins first, a
    /// later `Installing` request cannot start a platform operation. The slot
    /// remains occupied until `complete_cancellation` acknowledges worker
    /// cleanup, so a terminal snapshot always means cleanup has finished.
    pub fn request_cancel(
        &self,
        job_id: &str,
        timestamp: impl Into<String>,
    ) -> Result<JobSnapshot, InstallerError> {
        let timestamp = timestamp.into();
        self.mutate(job_id, move |controller| {
            if controller.is_terminal() || !controller.snapshot.stage.is_cancellable() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }

            let snapshot = controller.request_cancellation(timestamp)?;
            Ok(JobMutation::changed(snapshot))
        })
    }

    /// Acknowledges that the worker observed an accepted cancellation request,
    /// stopped its cancellable I/O, and finished cleanup. Only this transition
    /// releases the single-job slot for a later installation.
    pub fn complete_cancellation(
        &self,
        job_id: &str,
        timestamp: impl Into<String>,
    ) -> Result<JobSnapshot, InstallerError> {
        let timestamp = timestamp.into();
        self.mutate(job_id, move |controller| {
            if controller.is_terminal() {
                return Ok(JobMutation::unchanged(controller.snapshot()));
            }
            if !controller.cancellation.is_requested() {
                return Err(internal_error(
                    "cancellation completion requires an accepted cancellation request",
                ));
            }

            let snapshot = controller.transition_to_cancelled(timestamp)?;
            Ok(JobMutation::changed(snapshot))
        })
    }

    fn mutate<F>(&self, job_id: &str, operation: F) -> Result<JobSnapshot, InstallerError>
    where
        F: FnOnce(&mut JobController) -> Result<JobMutation, InstallerError>,
    {
        let mutation = {
            let mut state = self.lock_state()?;
            let controller = current_job_mut(&mut state, job_id)?;
            operation(controller)?
        };

        if mutation.changed {
            self.publish(mutation.snapshot.clone());
        }
        Ok(mutation.snapshot)
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, JobStoreState>, InstallerError> {
        self.state
            .lock()
            .map_err(|_| internal_error("job store synchronization is unavailable"))
    }

    fn publish(&self, snapshot: JobSnapshot) {
        self.event_sink.emit_snapshot(snapshot);
    }
}

struct JobMutation {
    snapshot: JobSnapshot,
    changed: bool,
}

impl JobMutation {
    fn changed(snapshot: JobSnapshot) -> Self {
        Self {
            snapshot,
            changed: true,
        }
    }

    fn unchanged(snapshot: JobSnapshot) -> Self {
        Self {
            snapshot,
            changed: false,
        }
    }
}

fn current_job<'a>(
    state: &'a JobStoreState,
    job_id: &str,
) -> Result<&'a JobController, InstallerError> {
    let Some(controller) = state.current.as_ref() else {
        return Err(job_not_found(job_id));
    };
    if controller.snapshot.job_id != job_id {
        return Err(job_not_found(job_id));
    }
    Ok(controller)
}

fn current_job_mut<'a>(
    state: &'a mut JobStoreState,
    job_id: &str,
) -> Result<&'a mut JobController, InstallerError> {
    let Some(controller) = state.current.as_mut() else {
        return Err(job_not_found(job_id));
    };
    if controller.snapshot.job_id != job_id {
        return Err(job_not_found(job_id));
    }
    Ok(controller)
}

fn stage_accepts_progress(stage: JobStage, phase: super::types::ProgressPhase) -> bool {
    use super::types::ProgressPhase;

    matches!(
        (stage, phase),
        (JobStage::Downloading, ProgressPhase::Download)
            | (JobStage::VerifyingDownload, ProgressPhase::Verification)
            | (JobStage::Installing, ProgressPhase::Installation)
            | (JobStage::VerifyingInstallation, ProgressPhase::Verification)
    )
}

fn job_not_found(job_id: &str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::JobNotFound)
        .with_context("job_id", job_id)
        .with_diagnostic_message("the desktop installation job is not current")
}

/// Pure restart policy: only a missing job or a fully terminal snapshot may
/// claim a process restart. In particular, `cancellable == false` does not
/// make a cancellable-stage job terminal; it can still be awaiting cleanup.
fn restart_claim_is_allowed(job_stage: Option<JobStage>) -> bool {
    match job_stage {
        None => true,
        Some(stage) => stage.is_terminal(),
    }
}

fn restart_pending_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::JobAlreadyRunning)
        .with_diagnostic_message("application restart already owns the desktop installation slot")
}

fn invalid_transition(current: JobStage, next: JobStage, message: &'static str) -> InstallerError {
    internal_error(message)
        .with_context("source", "job_state_machine")
        .with_diagnostic_message(format!("{message}: {current:?} -> {next:?}"))
}

fn internal_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::InternalError).with_diagnostic_message(message)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Barrier, Mutex},
        thread,
    };

    use uuid::{Uuid, Version};

    use super::*;
    use crate::codex_desktop::{
        error::InstallerErrorCode,
        types::{CpuArchitecture, InstalledApplicationSummary, PlatformVersion, ProgressPhase},
    };

    #[derive(Default)]
    struct RecordingSink {
        snapshots: Mutex<Vec<JobSnapshot>>,
    }

    impl RecordingSink {
        fn snapshots(&self) -> Vec<JobSnapshot> {
            self.snapshots.lock().unwrap().clone()
        }
    }

    impl JobEventSink for RecordingSink {
        fn emit_snapshot(&self, snapshot: JobSnapshot) {
            self.snapshots.lock().unwrap().push(snapshot);
        }
    }

    fn release() -> RemoteReleaseStatus {
        RemoteReleaseStatus {
            release_id: "v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_owned(),
            display_version: "1.2.3.4".to_owned(),
            platform_version: PlatformVersion::WindowsMsix {
                major: 1,
                minor: 2,
                build: 3,
                revision: 4,
            },
            expected_size: 1024,
            checked_at: "2026-07-29T00:00:00Z".to_owned(),
        }
    }

    fn advance_to_verifying_download(store: &JobStore, job_id: &str) {
        store
            .update_stage(job_id, JobStage::Preflight, "t1")
            .unwrap();
        store
            .update_stage(job_id, JobStage::Downloading, "t2")
            .unwrap();
        store
            .update_stage(job_id, JobStage::VerifyingDownload, "t3")
            .unwrap();
    }

    fn advance_to_verifying_installation(store: &JobStore, job_id: &str) {
        advance_to_verifying_download(store, job_id);
        let installing = store
            .update_stage(job_id, JobStage::Installing, "t4")
            .unwrap();
        assert_eq!(installing.stage, JobStage::Installing);
        store
            .update_stage(job_id, JobStage::VerifyingInstallation, "t5")
            .unwrap();
    }

    #[test]
    fn cancellation_keeps_the_slot_until_worker_cleanup_acknowledges_it() {
        let store = JobStore::new();
        let first = store.try_start(release(), "t0").unwrap();
        let parsed = Uuid::parse_str(&first.job_id).expect("job ids use UUID syntax");
        assert_eq!(parsed.get_version(), Some(Version::Random));

        let conflict = store.try_start(release(), "t1").unwrap_err();
        assert_eq!(conflict.code(), InstallerErrorCode::JobAlreadyRunning);

        let cancellation_requested = store.request_cancel(&first.job_id, "t2").unwrap();
        assert_eq!(cancellation_requested.stage, JobStage::Checking);
        assert!(!cancellation_requested.cancellable);

        let still_running = store.try_start(release(), "t3").unwrap_err();
        assert_eq!(still_running.code(), InstallerErrorCode::JobAlreadyRunning);

        let cancelled = store.complete_cancellation(&first.job_id, "t4").unwrap();
        assert_eq!(cancelled.stage, JobStage::Cancelled);

        let second = store.try_start(release(), "t5").unwrap();
        assert_ne!(first.job_id, second.job_id);
        assert_eq!(second.sequence, 0);
    }

    #[test]
    fn restart_claim_allows_only_missing_or_terminal_jobs() {
        assert!(restart_claim_is_allowed(None));
        for terminal in [JobStage::Succeeded, JobStage::Failed, JobStage::Cancelled] {
            assert!(restart_claim_is_allowed(Some(terminal)));
        }
        for active in [
            JobStage::Checking,
            JobStage::Preflight,
            JobStage::Downloading,
            JobStage::VerifyingDownload,
            JobStage::Installing,
            JobStage::VerifyingInstallation,
        ] {
            assert!(!restart_claim_is_allowed(Some(active)));
        }
    }

    #[test]
    fn restart_claim_rejects_cancellation_pending_and_installing_without_modifying_them() {
        let cancellation_pending_store = JobStore::new();
        let cancellation_pending_job = cancellation_pending_store
            .try_start(release(), "t0")
            .unwrap();
        let cancellation_pending = cancellation_pending_store
            .request_cancel(&cancellation_pending_job.job_id, "t1")
            .unwrap();

        let cancellation_error = cancellation_pending_store.claim_restart().unwrap_err();
        assert_eq!(
            cancellation_error.code(),
            InstallerErrorCode::JobAlreadyRunning
        );
        let after_cancellation_claim = cancellation_pending_store.get().unwrap().unwrap();
        assert_eq!(after_cancellation_claim.stage, JobStage::Checking);
        assert!(!after_cancellation_claim.cancellable);
        assert_eq!(
            after_cancellation_claim.sequence,
            cancellation_pending.sequence
        );

        let installing_store = JobStore::new();
        let installing_job = installing_store.try_start(release(), "t2").unwrap();
        advance_to_verifying_download(&installing_store, &installing_job.job_id);
        let installing = installing_store
            .update_stage(&installing_job.job_id, JobStage::Installing, "t6")
            .unwrap();

        let installing_error = installing_store.claim_restart().unwrap_err();
        assert_eq!(
            installing_error.code(),
            InstallerErrorCode::JobAlreadyRunning
        );
        let after_installing_claim = installing_store.get().unwrap().unwrap();
        assert_eq!(after_installing_claim.stage, JobStage::Installing);
        assert!(!after_installing_claim.cancellable);
        assert_eq!(after_installing_claim.sequence, installing.sequence);
    }

    #[test]
    fn restart_claim_allows_empty_or_terminal_slots_and_blocks_later_starts() {
        let empty_store = JobStore::new();
        empty_store.claim_restart().unwrap();
        let empty_start_error = empty_store.try_start(release(), "t0").unwrap_err();
        assert_eq!(
            empty_start_error.code(),
            InstallerErrorCode::JobAlreadyRunning
        );

        let terminal_store = JobStore::new();
        let terminal_job = terminal_store.try_start(release(), "t1").unwrap();
        let terminal = terminal_store
            .fail(
                &terminal_job.job_id,
                InstallerError::new(InstallerErrorCode::SourceUnavailable),
                "t2",
            )
            .unwrap();
        assert_eq!(terminal.stage, JobStage::Failed);

        terminal_store.claim_restart().unwrap();
        let terminal_start_error = terminal_store.try_start(release(), "t3").unwrap_err();
        assert_eq!(
            terminal_start_error.code(),
            InstallerErrorCode::JobAlreadyRunning
        );
        assert_eq!(
            terminal_store.get().unwrap().unwrap().stage,
            JobStage::Failed
        );
    }

    #[test]
    fn restart_claim_and_start_compete_for_one_atomic_slot() {
        let store = Arc::new(JobStore::new());
        let barrier = Arc::new(Barrier::new(3));

        let claim_store = store.clone();
        let claim_barrier = barrier.clone();
        let claim = thread::spawn(move || {
            claim_barrier.wait();
            claim_store.claim_restart()
        });

        let start_store = store.clone();
        let start_barrier = barrier.clone();
        let start = thread::spawn(move || {
            start_barrier.wait();
            start_store.try_start(release(), "start")
        });

        barrier.wait();
        let claim_result = claim.join().unwrap();
        let start_result = start.join().unwrap();

        assert_ne!(claim_result.is_ok(), start_result.is_ok());
        if let Err(error) = claim_result {
            assert_eq!(error.code(), InstallerErrorCode::JobAlreadyRunning);
        }
        if let Err(error) = start_result {
            assert_eq!(error.code(), InstallerErrorCode::JobAlreadyRunning);
        }
    }

    #[test]
    fn illegal_transition_does_not_change_the_snapshot() {
        let store = JobStore::new();
        let job = store.try_start(release(), "t0").unwrap();

        let error = store
            .update_stage(&job.job_id, JobStage::Installing, "t1")
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::InternalError);

        let current = store.get().unwrap().unwrap();
        assert_eq!(current.stage, JobStage::Checking);
        assert_eq!(current.sequence, 0);
    }

    #[test]
    fn snapshots_and_events_have_strictly_increasing_sequences() {
        let sink = Arc::new(RecordingSink::default());
        let store = JobStore::with_event_sink(sink.clone());
        let job = store.try_start(release(), "t0").unwrap();
        store
            .update_stage(&job.job_id, JobStage::Preflight, "t1")
            .unwrap();
        store
            .update_stage(&job.job_id, JobStage::Downloading, "t2")
            .unwrap();
        store
            .update_progress(
                &job.job_id,
                JobProgress::new(ProgressPhase::Download, Some(512), Some(1024)),
                "t3",
            )
            .unwrap();
        let failed = store
            .fail(
                &job.job_id,
                InstallerError::new(InstallerErrorCode::DownloadFailed),
                "t4",
            )
            .unwrap();

        assert_eq!(failed.sequence, 4);
        assert_eq!(
            failed.error.as_ref().and_then(|error| error.stage),
            Some(JobStage::Downloading)
        );
        assert_eq!(
            sink.snapshots()
                .iter()
                .map(|snapshot| snapshot.sequence)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn terminal_snapshots_are_immutable_and_do_not_reemit() {
        let sink = Arc::new(RecordingSink::default());
        let store = JobStore::with_event_sink(sink.clone());
        let job = store.try_start(release(), "t0").unwrap();
        let terminal = store
            .fail(
                &job.job_id,
                InstallerError::new(InstallerErrorCode::SourceUnavailable),
                "t1",
            )
            .unwrap();
        let event_count = sink.snapshots().len();

        let after_progress = store
            .update_progress(
                &job.job_id,
                JobProgress::new(ProgressPhase::Download, Some(1), Some(1)),
                "t2",
            )
            .unwrap();
        let after_cancel = store.request_cancel(&job.job_id, "t3").unwrap();

        assert_eq!(after_progress.stage, JobStage::Failed);
        assert_eq!(after_progress.sequence, terminal.sequence);
        assert_eq!(after_cancel.stage, JobStage::Failed);
        assert_eq!(after_cancel.sequence, terminal.sequence);
        assert_eq!(sink.snapshots().len(), event_count);
    }

    #[test]
    fn cancellation_and_entering_installing_are_decided_by_one_boundary() {
        let store = Arc::new(JobStore::new());
        let job = store.try_start(release(), "t0").unwrap();
        advance_to_verifying_download(&store, &job.job_id);
        let cancellation = store.cancellation_handle(&job.job_id).unwrap();

        let barrier = Arc::new(Barrier::new(3));
        let install_store = store.clone();
        let install_job_id = job.job_id.clone();
        let install_barrier = barrier.clone();
        let install = thread::spawn(move || {
            install_barrier.wait();
            install_store.update_stage(&install_job_id, JobStage::Installing, "install")
        });
        let cancel_store = store.clone();
        let cancel_job_id = job.job_id.clone();
        let cancel_barrier = barrier.clone();
        let cancel = thread::spawn(move || {
            cancel_barrier.wait();
            cancel_store.request_cancel(&cancel_job_id, "cancel")
        });

        barrier.wait();
        let install_snapshot = install.join().unwrap().unwrap();
        let cancel_snapshot = cancel.join().unwrap().unwrap();
        let final_snapshot = store.get().unwrap().unwrap();

        assert!(matches!(
            final_snapshot.stage,
            JobStage::Installing | JobStage::VerifyingDownload
        ));
        assert_eq!(install_snapshot.stage, final_snapshot.stage);
        assert_eq!(cancel_snapshot.stage, final_snapshot.stage);
        assert_eq!(
            cancellation.is_requested(),
            final_snapshot.stage == JobStage::VerifyingDownload
        );

        if final_snapshot.stage == JobStage::VerifyingDownload {
            let cancelled = store.complete_cancellation(&job.job_id, "cleanup").unwrap();
            assert_eq!(cancelled.stage, JobStage::Cancelled);
        }
    }

    #[test]
    fn superseded_job_id_is_rejected_without_touching_the_new_job() {
        let store = JobStore::new();
        let old_job = store.try_start(release(), "t0").unwrap();
        store.request_cancel(&old_job.job_id, "t1").unwrap();
        store.complete_cancellation(&old_job.job_id, "t2").unwrap();
        let new_job = store.try_start(release(), "t3").unwrap();

        let error = store
            .update_stage(&old_job.job_id, JobStage::Preflight, "t4")
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::JobNotFound);

        let current = store.get().unwrap().unwrap();
        assert_eq!(current.job_id, new_job.job_id);
        assert_eq!(current.stage, JobStage::Checking);
    }

    #[test]
    fn only_verified_post_install_state_can_succeed() {
        let store = JobStore::new();
        let job = store.try_start(release(), "t0").unwrap();
        let result = InstallResult {
            installed: InstalledApplicationSummary {
                stable_identity: "fixture.identity".to_owned(),
                display_version: Some("1.2.3.4".to_owned()),
                platform_version: PlatformVersion::WindowsMsix {
                    major: 1,
                    minor: 2,
                    build: 3,
                    revision: 4,
                },
                architecture: CpuArchitecture::X86_64,
            },
            warnings: Vec::new(),
        };

        let error = store
            .succeed(&job.job_id, result.clone(), "t1")
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::InternalError);

        advance_to_verifying_installation(&store, &job.job_id);
        let succeeded = store.succeed(&job.job_id, result, "t6").unwrap();
        assert_eq!(succeeded.stage, JobStage::Succeeded);
        assert!(succeeded.result.is_some());
        assert!(succeeded.error.is_none());
    }

    #[test]
    fn launch_only_success_is_limited_to_checking_and_honors_cancellation() {
        let result = InstallResult {
            installed: InstalledApplicationSummary {
                stable_identity: "fixture.identity".to_owned(),
                display_version: Some("1.2.3.4".to_owned()),
                platform_version: PlatformVersion::WindowsMsix {
                    major: 1,
                    minor: 2,
                    build: 3,
                    revision: 4,
                },
                architecture: CpuArchitecture::X86_64,
            },
            warnings: Vec::new(),
        };

        let store = JobStore::new();
        let job = store.try_start(release(), "t0").unwrap();
        let succeeded = store
            .succeed_after_launch(&job.job_id, result.clone(), "t1")
            .unwrap();
        assert_eq!(succeeded.stage, JobStage::Succeeded);
        assert_eq!(succeeded.result, Some(result.clone()));

        let cancelled_store = JobStore::new();
        let cancelled_job = cancelled_store.try_start(release(), "t0").unwrap();
        cancelled_store
            .request_cancel(&cancelled_job.job_id, "t1")
            .unwrap();
        let cancelled = cancelled_store
            .succeed_after_launch(&cancelled_job.job_id, result, "t2")
            .unwrap();
        assert_eq!(cancelled.stage, JobStage::Cancelled);
        assert!(cancelled.result.is_none());
    }

    #[test]
    fn progress_is_limited_to_its_matching_stage() {
        let store = JobStore::new();
        let job = store.try_start(release(), "t0").unwrap();

        let error = store
            .update_progress(
                &job.job_id,
                JobProgress::new(ProgressPhase::Download, Some(1), Some(1)),
                "t1",
            )
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::InternalError);
    }
}
