//! Orchestration for the Codex desktop installer.
//!
//! This layer deliberately owns no Tauri state and accepts no installer paths,
//! URLs, scopes, identities, or checksum values from IPC. It coordinates the
//! already-constrained core adapters and exposes only the fixed V1 operations
//! that the Tauri command shell delegates to.

use std::{
    panic::AssertUnwindSafe,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use chrono::Utc;
use futures::FutureExt;

use crate::codex_desktop::{
    cancellation::{cancellation_error, NeverCancelled},
    download::{download_release, DownloadProgressSink, DownloadProgressUpdate, HttpTransport},
    error::{InstallerError, InstallerErrorCode},
    jobs::{JobCancellation, JobEventSink, JobStore},
    platform::{
        installed_application_matches_release, CodexDesktopPlatform, PlatformProgressReporter,
        PlatformProgressSink,
    },
    source::{CacheMode, ReleaseSource},
    temp::JobTempDir,
    types::{
        CpuArchitecture, DesktopPlatform, InstallResult, InstalledApplication,
        InstallerWarningCode, JobProgress, JobSnapshot, JobStage, LocalInstallStatus,
        ProgressPhase, ReleaseDescriptor, RemoteReleaseStatus, StartInstallRequest,
        UnsupportedReason,
    },
    verify::{ensure_required_disk_space, DiskSpaceProbe},
};

/// Fixed event name used by the Tauri integration adapter.
pub const JOB_UPDATED_EVENT: &str = "codex-desktop-installer://job-updated";

const PROGRESS_MINIMUM_INTERVAL: Duration = Duration::from_millis(100);
const PROGRESS_MINIMUM_BYTE_DELTA: u64 = 1024 * 1024;

/// Time boundary for deterministic service tests and complete job snapshots.
pub trait InstallerClock: Send + Sync {
    fn now_rfc3339(&self) -> String;
}

#[derive(Debug, Default)]
struct SystemInstallerClock;

impl InstallerClock for SystemInstallerClock {
    fn now_rfc3339(&self) -> String {
        Utc::now().to_rfc3339()
    }
}

/// Opens only the application-owned log directory selected during service
/// construction. The integration layer provides the platform/Tauri adapter;
/// the IPC command never accepts a path.
pub trait LogDirectoryOpener: Send + Sync {
    fn open(&self, directory: &Path) -> Result<(), InstallerError>;
}

impl<F> LogDirectoryOpener for F
where
    F: Fn(&Path) -> Result<(), InstallerError> + Send + Sync,
{
    fn open(&self, directory: &Path) -> Result<(), InstallerError> {
        self(directory)
    }
}

/// Dependencies that can perform I/O or observe host state. Construction is
/// inert: no metadata request, disk probe, temporary-directory creation, or
/// local installer inspection happens until a caller invokes an operation.
pub struct CodexDesktopServiceDependencies {
    source: Arc<dyn ReleaseSource>,
    platform: Arc<dyn CodexDesktopPlatform>,
    transport: Arc<dyn HttpTransport>,
    disk_space_probe: Arc<dyn DiskSpaceProbe>,
    temp_root: PathBuf,
    log_directory: PathBuf,
}

impl CodexDesktopServiceDependencies {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: Arc<dyn ReleaseSource>,
        platform: Arc<dyn CodexDesktopPlatform>,
        transport: Arc<dyn HttpTransport>,
        disk_space_probe: Arc<dyn DiskSpaceProbe>,
        temp_root: PathBuf,
        log_directory: PathBuf,
    ) -> Self {
        Self {
            source,
            platform,
            transport,
            disk_space_probe,
            temp_root,
            log_directory,
        }
    }
}

#[derive(Clone)]
struct CheckedRelease {
    descriptor: ReleaseDescriptor,
    status: RemoteReleaseStatus,
}

/// Process-local V1 installer service.
///
/// Its `JobStore` is intentionally in memory only. A restart does not revive
/// an old worker or attempt to reuse an old temporary directory.
#[derive(Clone)]
pub struct CodexDesktopService {
    source: Arc<dyn ReleaseSource>,
    platform: Arc<dyn CodexDesktopPlatform>,
    transport: Arc<dyn HttpTransport>,
    disk_space_probe: Arc<dyn DiskSpaceProbe>,
    temp_root: PathBuf,
    log_directory: PathBuf,
    clock: Arc<dyn InstallerClock>,
    job_store: JobStore,
    checked_release: Arc<Mutex<Option<CheckedRelease>>>,
    event_sink: Arc<ForwardingJobEventSink>,
    log_directory_opener: Arc<Mutex<Option<Arc<dyn LogDirectoryOpener>>>>,
}

enum InstallFlowOutcome {
    Installed(InstalledApplication),
    LaunchedExisting(InstalledApplication),
}

impl CodexDesktopService {
    pub fn new(dependencies: CodexDesktopServiceDependencies) -> Self {
        Self::with_clock(dependencies, Arc::new(SystemInstallerClock))
    }

    #[cfg(test)]
    pub(crate) fn with_clock(
        dependencies: CodexDesktopServiceDependencies,
        clock: Arc<dyn InstallerClock>,
    ) -> Self {
        Self::build(dependencies, clock)
    }

    #[cfg(not(test))]
    fn with_clock(
        dependencies: CodexDesktopServiceDependencies,
        clock: Arc<dyn InstallerClock>,
    ) -> Self {
        Self::build(dependencies, clock)
    }

    fn build(
        dependencies: CodexDesktopServiceDependencies,
        clock: Arc<dyn InstallerClock>,
    ) -> Self {
        let event_sink = Arc::new(ForwardingJobEventSink::default());
        let job_store = JobStore::with_event_sink(event_sink.clone());

        Self {
            source: dependencies.source,
            platform: dependencies.platform,
            transport: dependencies.transport,
            disk_space_probe: dependencies.disk_space_probe,
            temp_root: dependencies.temp_root,
            log_directory: dependencies.log_directory,
            clock,
            job_store,
            checked_release: Arc::new(Mutex::new(None)),
            event_sink,
            log_directory_opener: Arc::new(Mutex::new(None)),
        }
    }

    /// Attaches the integration-owned best-effort event publisher. Replacing
    /// the sink is useful during app setup/tests and does not affect job state.
    pub fn attach_job_event_sink(&self, sink: Arc<dyn JobEventSink>) {
        *recover_lock(&self.event_sink.sink) = Some(sink);
    }

    /// Attaches the trusted log-directory opener after the Tauri `AppHandle`
    /// exists. Until then, `open_log_directory` fails closed.
    pub fn attach_log_directory_opener(&self, opener: Arc<dyn LogDirectoryOpener>) {
        *recover_lock(&self.log_directory_opener) = Some(opener);
    }

    /// Performs a local-only status inspection. It never resolves metadata or
    /// looks at the currently cached remote release.
    pub async fn get_local_status(&self) -> Result<LocalInstallStatus, InstallerError> {
        self.platform.inspect_local().await
    }

    /// Resolves the latest trusted descriptor and remembers it as the only
    /// release an IPC caller may subsequently request for installation.
    pub async fn check_latest(&self, force: bool) -> Result<RemoteReleaseStatus, InstallerError> {
        let descriptor = self
            .resolve_latest(
                if force {
                    CacheMode::ForceRefresh
                } else {
                    CacheMode::UseCache
                },
                &NeverCancelled,
            )
            .await?;
        let status = descriptor.remote_status(self.clock.now_rfc3339());
        *recover_lock(&self.checked_release) = Some(CheckedRelease {
            descriptor,
            status: status.clone(),
        });
        Ok(status)
    }

    pub fn get_job(&self) -> Result<Option<JobSnapshot>, InstallerError> {
        self.job_store.get()
    }

    /// Reserves the process-local installation slot for an approved application
    /// restart. A restart never cancels or replaces an active worker; callers
    /// must wait for it to reach a terminal snapshot before claiming the slot.
    pub fn claim_restart(&self) -> Result<(), InstallerError> {
        self.job_store.claim_restart()
    }

    /// Atomically claims the process-local job slot and starts the worker only
    /// after returning the initial `Checking` snapshot to the caller.
    pub fn start_install(
        &self,
        request: StartInstallRequest,
    ) -> Result<JobSnapshot, InstallerError> {
        request.validate()?;

        let checked = recover_lock(&self.checked_release)
            .clone()
            .filter(|release| release.descriptor.release_id() == request.expected_release_id)
            .ok_or_else(|| {
                InstallerError::new(InstallerErrorCode::MetadataChanged).with_diagnostic_message(
                    "the requested release was not checked in this application session",
                )
            })?;

        let snapshot = self
            .job_store
            .try_start(checked.status, self.clock.now_rfc3339())?;
        let cancellation = self.job_store.cancellation_handle(&snapshot.job_id)?;
        let service = self.clone();
        let job_id = snapshot.job_id.clone();
        let expected_release_id = request.expected_release_id;

        tokio::spawn(async move {
            service
                .run_job(job_id, expected_release_id, cancellation)
                .await;
        });

        Ok(snapshot)
    }

    pub fn cancel_install(&self, job_id: &str) -> Result<JobSnapshot, InstallerError> {
        self.job_store
            .request_cancel(job_id, self.clock.now_rfc3339())
    }

    /// Re-detects a trusted local install on every call. Remote metadata is not
    /// consulted, so a mirror outage cannot prevent launching an installed app.
    pub async fn launch(&self) -> Result<(), InstallerError> {
        match self.platform.inspect_local().await? {
            LocalInstallStatus::Installed { application } => {
                self.platform.launch(&application).await
            }
            LocalInstallStatus::Unsupported { reason } => Err(unsupported_status_error(reason)),
            LocalInstallStatus::Ambiguous { .. } => Err(InstallerError::new(
                InstallerErrorCode::MacMultipleInstallations,
            )
            .with_diagnostic_message("local Codex installations are ambiguous")),
            LocalInstallStatus::NotInstalled { .. } => {
                Err(InstallerError::new(InstallerErrorCode::LaunchFailed)
                    .with_diagnostic_message("a supported Codex installation was not found"))
            }
        }
    }

    /// Opens the fixed, application-owned log directory through an integration
    /// adapter. No user-provided path reaches this operation.
    pub fn open_log_directory(&self) -> Result<(), InstallerError> {
        if !self.log_directory.is_dir() {
            return Err(InstallerError::new(InstallerErrorCode::InternalError)
                .with_diagnostic_message("the application log directory is unavailable"));
        }

        let opener = recover_lock(&self.log_directory_opener)
            .clone()
            .ok_or_else(|| {
                InstallerError::new(InstallerErrorCode::InternalError)
                    .with_diagnostic_message("the log directory opener is not attached")
            })?;
        opener.open(&self.log_directory)
    }

    async fn run_job(
        &self,
        job_id: String,
        expected_release_id: String,
        cancellation: JobCancellation,
    ) {
        let mut temporary_directory = None;
        let outcome = AssertUnwindSafe(self.run_install_flow(
            &job_id,
            &expected_release_id,
            &cancellation,
            &mut temporary_directory,
        ))
        .catch_unwind()
        .await;

        match outcome {
            Ok(Ok(outcome)) => {
                let (application, launched_existing) = match outcome {
                    InstallFlowOutcome::Installed(application) => (application, false),
                    InstallFlowOutcome::LaunchedExisting(application) => (application, true),
                };
                let warnings = temporary_directory
                    .as_ref()
                    .is_some_and(|directory| self.cleanup_temporary_directory(directory))
                    .then_some(InstallerWarningCode::TempCleanupFailed)
                    .into_iter()
                    .collect();
                let result = InstallResult {
                    installed: (&application).into(),
                    warnings,
                };
                if launched_existing {
                    self.settle_launched_existing(&job_id, result);
                } else {
                    self.settle_success(&job_id, result);
                }
            }
            Ok(Err(error)) => {
                if let Some(directory) = temporary_directory.as_ref() {
                    self.cleanup_temporary_directory(directory);
                }
                if cancellation.is_requested() {
                    self.settle_cancellation(&job_id);
                } else {
                    self.settle_failure(&job_id, error);
                }
            }
            Err(_) => {
                if let Some(directory) = temporary_directory.as_ref() {
                    self.cleanup_temporary_directory(directory);
                }
                // A caught worker panic is not a cancellation acknowledgement.
                // Preserve the fail-closed cleanup boundary, then make the
                // still-current job terminal so it cannot block future work.
                log::error!("Codex desktop installer worker flow panicked");
                self.settle_failure(
                    &job_id,
                    InstallerError::new(InstallerErrorCode::InternalError)
                        .with_diagnostic_message("the desktop installation worker panicked"),
                );
            }
        }
    }

    async fn run_install_flow(
        &self,
        job_id: &str,
        expected_release_id: &str,
        cancellation: &JobCancellation,
        temporary_directory: &mut Option<JobTempDir>,
    ) -> Result<InstallFlowOutcome, InstallerError> {
        let release = self
            .resolve_latest(CacheMode::ForceRefresh, cancellation)
            .await?;
        if release.release_id() != expected_release_id {
            return Err(InstallerError::new(InstallerErrorCode::MetadataChanged)
                .with_diagnostic_message(
                    "the release changed after the installation request was created",
                ));
        }

        // Treat a direct IPC invocation exactly like the renderer's version
        // decision: an already-installed equal or newer Stable app is launched
        // instead of reaching download or deployment. The platform adapter
        // re-detects only trusted Stable identity before returning this status.
        match self.platform.inspect_local().await? {
            LocalInstallStatus::Installed { application } => {
                if application
                    .platform_version
                    .is_at_least(&release.platform_version)?
                {
                    self.ensure_not_cancelled(cancellation)?;
                    self.platform.launch(&application).await?;
                    self.ensure_not_cancelled(cancellation)?;
                    return Ok(InstallFlowOutcome::LaunchedExisting(application));
                }
            }
            LocalInstallStatus::Unsupported { reason } => {
                return Err(unsupported_status_error(reason));
            }
            LocalInstallStatus::Ambiguous { .. } => {
                return Err(
                    InstallerError::new(InstallerErrorCode::MacMultipleInstallations)
                        .with_diagnostic_message("local Codex installations are ambiguous"),
                );
            }
            LocalInstallStatus::NotInstalled { .. } => {}
        }
        self.ensure_not_cancelled(cancellation)?;

        self.transition_to(job_id, JobStage::Preflight, cancellation)?;
        *temporary_directory = Some(JobTempDir::create(&self.temp_root, job_id)?);
        let plan = self
            .platform
            .preflight(
                &release,
                temporary_directory
                    .as_ref()
                    .expect("job temporary directory is assigned before preflight")
                    .path(),
            )
            .await?;
        self.ensure_not_cancelled(cancellation)?;

        let disk_paths = std::iter::once(
            temporary_directory
                .as_ref()
                .expect("job temporary directory is assigned before disk preflight")
                .path(),
        )
        .chain(plan.additional_disk_paths().iter().map(PathBuf::as_path));
        ensure_required_disk_space(
            self.disk_space_probe.as_ref(),
            disk_paths,
            release.expected_size,
        )?;
        self.ensure_not_cancelled(cancellation)?;

        self.transition_to(job_id, JobStage::Downloading, cancellation)?;
        let download_progress = DownloadJobProgressBridge::new(
            self.job_store.clone(),
            self.clock.clone(),
            job_id.to_owned(),
        );
        let artifact = match download_release(
            self.transport.as_ref(),
            &release,
            temporary_directory
                .as_ref()
                .expect("job temporary directory is assigned before downloading"),
            cancellation,
            &download_progress,
        )
        .await
        {
            Ok(artifact) => artifact,
            Err(download_error)
                if download_error.code() == InstallerErrorCode::ChecksumMismatch =>
            {
                // The artifact is already removed by `download_release` before
                // this branch runs. A short-lived CDN redirect may have moved
                // after metadata was locked, so refresh only to classify that
                // fail-closed mismatch; never retry the artifact blindly.
                let refreshed_release = self
                    .resolve_latest(CacheMode::ForceRefresh, cancellation)
                    .await?;
                if refreshed_release.release_id() != release.release_id() {
                    return Err(InstallerError::new(InstallerErrorCode::MetadataChanged)
                        .with_diagnostic_message(
                            "the release metadata changed after the downloaded artifact failed checksum validation",
                        ));
                }
                return Err(download_error);
            }
            Err(download_error) => return Err(download_error),
        };
        download_progress.take_error()?;
        self.ensure_not_cancelled(cancellation)?;

        // The downloader normally enters this stage through its verification
        // progress callback. Keep the explicit transition as a fail-closed
        // fallback should a future downloader omit that callback.
        self.transition_to(job_id, JobStage::VerifyingDownload, cancellation)?;
        let package = self.platform.verify_package(&release, &artifact).await?;
        if !package.belongs_to(&release) {
            return Err(
                InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                    .with_diagnostic_message(
                        "platform validation returned package evidence for a different release",
                    ),
            );
        }
        self.ensure_not_cancelled(cancellation)?;

        // `JobStore::update_stage` arbitrates cancellation and Installing under
        // one mutex. Only call the irreversible platform installer after this
        // method has confirmed the actual stage is `Installing`.
        self.transition_to(job_id, JobStage::Installing, cancellation)?;
        let installation_progress = Arc::new(InstallationProgressBridge::new(
            self.job_store.clone(),
            self.clock.clone(),
            job_id.to_owned(),
        ));
        let sink: PlatformProgressSink = installation_progress.clone();
        self.platform.install_current_user(&package, sink).await?;
        installation_progress.take_error()?;

        self.transition_to(job_id, JobStage::VerifyingInstallation, cancellation)?;
        self.publish_verification_progress(job_id)?;
        let status = self.platform.inspect_local().await?;
        let LocalInstallStatus::Installed { application } = status else {
            return Err(
                InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                    .with_diagnostic_message(
                        "post-install inspection did not find one matching Codex application",
                    ),
            );
        };
        if !installed_application_matches_release(&application, &release)? {
            return Err(InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                .with_diagnostic_message(
                    "post-install application identity, platform, architecture, or version did not match",
                ));
        }

        Ok(InstallFlowOutcome::Installed(application))
    }

    async fn resolve_latest(
        &self,
        cache_mode: CacheMode,
        cancellation: &dyn crate::codex_desktop::cancellation::Cancellation,
    ) -> Result<ReleaseDescriptor, InstallerError> {
        let (platform, architecture) = self.platform_target()?;
        self.source
            .resolve_latest(platform, architecture, cache_mode, cancellation)
            .await
    }

    fn platform_target(&self) -> Result<(DesktopPlatform, CpuArchitecture), InstallerError> {
        self.platform
            .platform()
            .map(|platform| (platform, self.platform.architecture()))
            .ok_or_else(|| {
                InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                    .with_diagnostic_message("the current host has no V1 desktop installer")
            })
    }

    fn transition_to(
        &self,
        job_id: &str,
        next_stage: JobStage,
        cancellation: &JobCancellation,
    ) -> Result<(), InstallerError> {
        if cancellation.is_requested() {
            return Err(cancellation_error());
        }
        let current = self.current_job(job_id)?;
        if current.stage == next_stage {
            return Ok(());
        }
        if current.stage == JobStage::Cancelled {
            return Err(cancellation_error());
        }
        if current.stage.is_terminal() {
            return Err(InstallerError::new(InstallerErrorCode::InternalError)
                .with_diagnostic_message("a terminal installation job cannot advance"));
        }

        let updated = self
            .job_store
            .update_stage(job_id, next_stage, self.clock.now_rfc3339())?;
        if updated.stage == next_stage {
            return Ok(());
        }
        if cancellation.is_requested() || updated.stage == JobStage::Cancelled {
            return Err(cancellation_error());
        }

        Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("the installation job did not enter its requested stage"))
    }

    fn publish_verification_progress(&self, job_id: &str) -> Result<(), InstallerError> {
        self.job_store
            .update_progress(
                job_id,
                JobProgress::new(ProgressPhase::Verification, None, None),
                self.clock.now_rfc3339(),
            )
            .map(|_| ())
    }

    fn current_job(&self, job_id: &str) -> Result<JobSnapshot, InstallerError> {
        let snapshot = self.job_store.get()?.ok_or_else(|| {
            InstallerError::new(InstallerErrorCode::JobNotFound)
                .with_diagnostic_message("the desktop installation job is not current")
        })?;
        if snapshot.job_id == job_id {
            Ok(snapshot)
        } else {
            Err(InstallerError::new(InstallerErrorCode::JobNotFound)
                .with_diagnostic_message("the desktop installation job is not current"))
        }
    }

    fn ensure_not_cancelled(&self, cancellation: &JobCancellation) -> Result<(), InstallerError> {
        if cancellation.is_requested() {
            Err(cancellation_error())
        } else {
            Ok(())
        }
    }

    fn cleanup_temporary_directory(&self, directory: &JobTempDir) -> bool {
        match directory.cleanup() {
            Ok(()) => false,
            Err(error) => {
                // `JobTempDir::cleanup` is fail-closed and does not recurse.
                // Do not replace a primary terminal error or log a local path.
                log::warn!(
                    "Codex desktop installer temporary cleanup failed with {:?}",
                    error.code()
                );
                true
            }
        }
    }

    fn settle_success(&self, job_id: &str, result: InstallResult) {
        if let Err(error) = self
            .job_store
            .succeed(job_id, result, self.clock.now_rfc3339())
        {
            log::warn!(
                "Codex desktop installer could not publish success with {:?}",
                error.code()
            );
        }
    }

    fn settle_launched_existing(&self, job_id: &str, result: InstallResult) {
        if let Err(error) =
            self.job_store
                .succeed_after_launch(job_id, result, self.clock.now_rfc3339())
        {
            log::warn!(
                "Codex desktop installer could not publish launched-existing success with {:?}",
                error.code()
            );
        }
    }

    fn settle_failure(&self, job_id: &str, error: InstallerError) {
        if let Err(settlement_error) = self.job_store.fail(job_id, error, self.clock.now_rfc3339())
        {
            if settlement_error.code() != InstallerErrorCode::JobNotFound {
                log::warn!(
                    "Codex desktop installer could not publish failure with {:?}",
                    settlement_error.code()
                );
            }
        }
    }

    fn settle_cancellation(&self, job_id: &str) {
        if let Err(error) = self
            .job_store
            .complete_cancellation(job_id, self.clock.now_rfc3339())
        {
            if error.code() != InstallerErrorCode::JobNotFound {
                log::warn!(
                    "Codex desktop installer could not publish cancellation with {:?}",
                    error.code()
                );
            }
        }
    }
}

#[derive(Default)]
struct ForwardingJobEventSink {
    sink: Mutex<Option<Arc<dyn JobEventSink>>>,
}

impl JobEventSink for ForwardingJobEventSink {
    fn emit_snapshot(&self, snapshot: JobSnapshot) {
        let sink = recover_lock(&self.sink).clone();
        if let Some(sink) = sink {
            sink.emit_snapshot(snapshot);
        }
    }
}

struct DownloadJobProgressBridge {
    job_store: JobStore,
    clock: Arc<dyn InstallerClock>,
    job_id: String,
    throttle: Mutex<ProgressThrottle>,
    error: Mutex<Option<InstallerError>>,
}

impl DownloadJobProgressBridge {
    fn new(job_store: JobStore, clock: Arc<dyn InstallerClock>, job_id: String) -> Self {
        Self {
            job_store,
            clock,
            job_id,
            throttle: Mutex::new(ProgressThrottle::default()),
            error: Mutex::new(None),
        }
    }

    fn take_error(&self) -> Result<(), InstallerError> {
        recover_lock(&self.error).take().map_or(Ok(()), Err)
    }

    fn publish(&self, update: DownloadProgressUpdate) -> Result<(), InstallerError> {
        let snapshot = current_job_snapshot(&self.job_store, &self.job_id)?;
        if snapshot.stage.is_terminal() {
            return Ok(());
        }

        match update.phase {
            ProgressPhase::Download => {
                if snapshot.stage != JobStage::Downloading {
                    return Err(progress_stage_error());
                }
            }
            ProgressPhase::Verification => {
                if snapshot.stage == JobStage::Downloading {
                    let transitioned = self.job_store.update_stage(
                        &self.job_id,
                        JobStage::VerifyingDownload,
                        self.clock.now_rfc3339(),
                    )?;
                    if transitioned.stage == JobStage::Cancelled {
                        return Ok(());
                    }
                    if transitioned.stage != JobStage::VerifyingDownload {
                        return Err(progress_stage_error());
                    }
                } else if snapshot.stage != JobStage::VerifyingDownload {
                    return Err(progress_stage_error());
                }
            }
            ProgressPhase::Installation => return Err(progress_stage_error()),
        }

        let progress = JobProgress::new(
            update.phase,
            Some(update.completed_bytes),
            Some(update.total_bytes),
        );
        if !recover_lock(&self.throttle).should_emit(&progress, Some(update.attempt)) {
            return Ok(());
        }
        self.job_store
            .update_progress(&self.job_id, progress, self.clock.now_rfc3339())?;
        Ok(())
    }

    fn record_error(&self, error: InstallerError) {
        let mut stored = recover_lock(&self.error);
        if stored.is_none() {
            *stored = Some(error);
        }
    }
}

impl DownloadProgressSink for DownloadJobProgressBridge {
    fn emit(&self, update: DownloadProgressUpdate) {
        if let Err(error) = self.publish(update) {
            self.record_error(error);
        }
    }
}

struct InstallationProgressBridge {
    job_store: JobStore,
    clock: Arc<dyn InstallerClock>,
    job_id: String,
    throttle: Mutex<ProgressThrottle>,
    error: Mutex<Option<InstallerError>>,
}

impl InstallationProgressBridge {
    fn new(job_store: JobStore, clock: Arc<dyn InstallerClock>, job_id: String) -> Self {
        Self {
            job_store,
            clock,
            job_id,
            throttle: Mutex::new(ProgressThrottle::default()),
            error: Mutex::new(None),
        }
    }

    fn take_error(&self) -> Result<(), InstallerError> {
        recover_lock(&self.error).take().map_or(Ok(()), Err)
    }

    fn record_error(&self, error: InstallerError) {
        let mut stored = recover_lock(&self.error);
        if stored.is_none() {
            *stored = Some(error);
        }
    }
}

impl PlatformProgressReporter for InstallationProgressBridge {
    fn report_progress(&self, progress: JobProgress) {
        let result = (|| {
            let snapshot = current_job_snapshot(&self.job_store, &self.job_id)?;
            if snapshot.stage.is_terminal() {
                return Ok(());
            }
            if snapshot.stage != JobStage::Installing
                || progress.phase != ProgressPhase::Installation
            {
                return Err(progress_stage_error());
            }
            if !recover_lock(&self.throttle).should_emit(&progress, None) {
                return Ok(());
            }
            self.job_store
                .update_progress(&self.job_id, progress, self.clock.now_rfc3339())?;
            Ok(())
        })();
        if let Err(error) = result {
            self.record_error(error);
        }
    }
}

#[derive(Default)]
struct ProgressThrottle {
    last_phase: Option<ProgressPhase>,
    last_attempt: Option<u8>,
    last_completed_bytes: Option<u64>,
    last_emitted_at: Option<Instant>,
}

impl ProgressThrottle {
    fn should_emit(&mut self, progress: &JobProgress, attempt: Option<u8>) -> bool {
        let now = Instant::now();
        let is_complete = matches!(
            (progress.completed_bytes, progress.total_bytes),
            (Some(completed), Some(total)) if total > 0 && completed >= total
        );
        let phase_changed = self.last_phase != Some(progress.phase);
        let attempt_changed = attempt.is_some() && self.last_attempt != attempt;
        let byte_threshold_crossed = matches!(
            (progress.completed_bytes, self.last_completed_bytes),
            (Some(completed), Some(previous)) if completed.saturating_sub(previous) >= PROGRESS_MINIMUM_BYTE_DELTA
        );
        let time_elapsed = self
            .last_emitted_at
            .is_none_or(|previous| now.duration_since(previous) >= PROGRESS_MINIMUM_INTERVAL);
        let should_emit = is_complete
            || phase_changed
            || attempt_changed
            || byte_threshold_crossed
            || time_elapsed;
        if should_emit {
            self.last_phase = Some(progress.phase);
            self.last_attempt = attempt;
            self.last_completed_bytes = progress.completed_bytes;
            self.last_emitted_at = Some(now);
        }
        should_emit
    }
}

fn current_job_snapshot(job_store: &JobStore, job_id: &str) -> Result<JobSnapshot, InstallerError> {
    let snapshot = job_store.get()?.ok_or_else(|| {
        InstallerError::new(InstallerErrorCode::JobNotFound)
            .with_diagnostic_message("the desktop installation job is not current")
    })?;
    if snapshot.job_id == job_id {
        Ok(snapshot)
    } else {
        Err(InstallerError::new(InstallerErrorCode::JobNotFound)
            .with_diagnostic_message("the desktop installation job is not current"))
    }
}

fn progress_stage_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::InternalError)
        .with_diagnostic_message("installer progress did not match the active job stage")
}

fn unsupported_status_error(reason: UnsupportedReason) -> InstallerError {
    let code = match reason {
        UnsupportedReason::Platform => InstallerErrorCode::PlatformUnsupported,
        UnsupportedReason::Architecture => InstallerErrorCode::ArchitectureUnsupported,
        UnsupportedReason::OsVersion => InstallerErrorCode::OsVersionUnsupported,
    };
    InstallerError::new(code).with_diagnostic_message("the current host cannot launch Codex")
}

fn recover_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        path::Path,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use bytes::Bytes;
    use futures::future::BoxFuture;
    use tokio::sync::Notify;

    use super::*;
    use crate::codex_desktop::{
        cancellation::Cancellation,
        download::{TransportError, TransportFuture, TransportResponse},
        platform::{PlatformInstallPlan, VerifiedPackage, WINDOWS_CODEX_STABLE_IDENTITY},
        types::{LaunchTarget, PlatformVersion, TrustedDownloadEndpoint},
        verify::{DiskSpaceProbeError, VolumeKey},
    };

    #[derive(Default)]
    struct FixedClock;

    impl InstallerClock for FixedClock {
        fn now_rfc3339(&self) -> String {
            "2026-07-29T00:00:00Z".to_owned()
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        snapshots: Mutex<Vec<JobSnapshot>>,
    }

    impl RecordingSink {
        fn snapshots(&self) -> Vec<JobSnapshot> {
            recover_lock(&self.snapshots).clone()
        }
    }

    impl JobEventSink for RecordingSink {
        fn emit_snapshot(&self, snapshot: JobSnapshot) {
            recover_lock(&self.snapshots).push(snapshot);
        }
    }

    #[derive(Clone)]
    struct FixtureSource {
        checked: ReleaseDescriptor,
        forced: Arc<Mutex<ReleaseDescriptor>>,
        forced_queue: Arc<Mutex<VecDeque<ReleaseDescriptor>>>,
        calls: Arc<Mutex<Vec<CacheMode>>>,
        force_gate: Option<Arc<Notify>>,
        panic_on_forced_refresh: Arc<AtomicBool>,
    }

    impl FixtureSource {
        fn new(checked: ReleaseDescriptor) -> Self {
            Self {
                forced: Arc::new(Mutex::new(checked.clone())),
                forced_queue: Arc::new(Mutex::new(VecDeque::new())),
                checked,
                calls: Arc::new(Mutex::new(Vec::new())),
                force_gate: None,
                panic_on_forced_refresh: Arc::new(AtomicBool::new(false)),
            }
        }

        fn with_force_gate(checked: ReleaseDescriptor, force_gate: Arc<Notify>) -> Self {
            Self {
                force_gate: Some(force_gate),
                ..Self::new(checked)
            }
        }

        fn set_forced_release(&self, release: ReleaseDescriptor) {
            *recover_lock(&self.forced) = release;
        }

        fn queue_forced_releases(&self, releases: impl IntoIterator<Item = ReleaseDescriptor>) {
            recover_lock(&self.forced_queue).extend(releases);
        }

        fn release_force_gate(&self) {
            if let Some(gate) = self.force_gate.as_ref() {
                gate.notify_one();
            }
        }

        fn set_panic_on_forced_refresh(&self, enabled: bool) {
            self.panic_on_forced_refresh
                .store(enabled, Ordering::SeqCst);
        }
    }

    impl ReleaseSource for FixtureSource {
        fn resolve_latest<'a>(
            &'a self,
            _platform: DesktopPlatform,
            _architecture: CpuArchitecture,
            cache_mode: CacheMode,
            cancellation: &'a dyn Cancellation,
        ) -> BoxFuture<'a, Result<ReleaseDescriptor, InstallerError>> {
            recover_lock(&self.calls).push(cache_mode);
            let checked = self.checked.clone();
            let forced = self.forced.clone();
            let forced_queue = self.forced_queue.clone();
            let force_gate = self.force_gate.clone();
            let panic_on_forced_refresh = self.panic_on_forced_refresh.clone();
            Box::pin(async move {
                if cache_mode == CacheMode::ForceRefresh {
                    if let Some(gate) = force_gate {
                        gate.notified().await;
                    }
                    if cancellation.is_cancelled() {
                        return Err(cancellation_error());
                    }
                    if panic_on_forced_refresh.load(Ordering::SeqCst) {
                        panic!("fixture source forced refresh panic");
                    }
                    return Ok(recover_lock(&forced_queue)
                        .pop_front()
                        .unwrap_or_else(|| recover_lock(&forced).clone()));
                }
                Ok(checked)
            })
        }
    }

    struct FixtureTransport {
        artifact: Mutex<Option<Vec<u8>>>,
    }

    impl FixtureTransport {
        fn new(artifact: Vec<u8>) -> Self {
            Self {
                artifact: Mutex::new(Some(artifact)),
            }
        }
    }

    impl HttpTransport for FixtureTransport {
        fn get(&self, _url: url::Url) -> TransportFuture<'_> {
            let artifact = recover_lock(&self.artifact).take();
            Box::pin(async move {
                let artifact = artifact.ok_or_else(|| {
                    TransportError::non_retryable("fixture artifact was requested more than once")
                })?;
                Ok(TransportResponse {
                    status: 200,
                    location: None,
                    content_length: Some(artifact.len() as u64),
                    retry_after: None,
                    body: Box::pin(futures::stream::iter(vec![Ok::<Bytes, TransportError>(
                        Bytes::from(artifact),
                    )])),
                })
            })
        }
    }

    #[derive(Default)]
    struct FixtureDiskProbe {
        paths: Mutex<Vec<PathBuf>>,
    }

    impl FixtureDiskProbe {
        fn paths(&self) -> Vec<PathBuf> {
            recover_lock(&self.paths).clone()
        }
    }

    impl DiskSpaceProbe for FixtureDiskProbe {
        fn volume_key(&self, path: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
            recover_lock(&self.paths).push(path.to_path_buf());
            VolumeKey::new("fixture-volume")
        }

        fn available_bytes(&self, _volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
            Ok(16 * 1024 * 1024)
        }
    }

    #[derive(Clone)]
    struct FixturePlatform {
        release: ReleaseDescriptor,
        initial_local_status: Arc<Mutex<LocalInstallStatus>>,
        preflight_calls: Arc<AtomicUsize>,
        install_calls: Arc<AtomicUsize>,
        launch_calls: Arc<AtomicUsize>,
        panic_on_preflight: Arc<AtomicBool>,
    }

    impl FixturePlatform {
        fn new(release: ReleaseDescriptor) -> Self {
            Self {
                initial_local_status: Arc::new(Mutex::new(LocalInstallStatus::NotInstalled {
                    platform: release.platform,
                    architecture: release.architecture,
                })),
                release,
                preflight_calls: Arc::new(AtomicUsize::new(0)),
                install_calls: Arc::new(AtomicUsize::new(0)),
                launch_calls: Arc::new(AtomicUsize::new(0)),
                panic_on_preflight: Arc::new(AtomicBool::new(false)),
            }
        }

        fn installed_application(&self) -> InstalledApplication {
            Self::application_for(&self.release)
        }

        fn application_for(release: &ReleaseDescriptor) -> InstalledApplication {
            InstalledApplication {
                stable_identity: WINDOWS_CODEX_STABLE_IDENTITY.to_owned(),
                display_name: Some("Codex".to_owned()),
                display_version: Some(release.display_version.clone()),
                platform_version: release.platform_version.clone(),
                architecture: release.architecture,
                location: Some("C:\\redacted".to_owned()),
                launch_target: LaunchTarget::WindowsAumid("fixture.app".to_owned()),
            }
        }

        fn set_initial_local_status(&self, status: LocalInstallStatus) {
            *recover_lock(&self.initial_local_status) = status;
        }

        fn set_panic_on_preflight(&self, enabled: bool) {
            self.panic_on_preflight.store(enabled, Ordering::SeqCst);
        }
    }

    impl CodexDesktopPlatform for FixturePlatform {
        fn platform(&self) -> Option<DesktopPlatform> {
            Some(DesktopPlatform::Windows)
        }

        fn architecture(&self) -> CpuArchitecture {
            CpuArchitecture::X86_64
        }

        fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
            let status = if self.install_calls.load(Ordering::SeqCst) > 0 {
                LocalInstallStatus::Installed {
                    application: self.installed_application(),
                }
            } else {
                recover_lock(&self.initial_local_status).clone()
            };
            Box::pin(async move { Ok(status) })
        }

        fn preflight<'a>(
            &'a self,
            _release: &'a ReleaseDescriptor,
            _temp_root: &'a Path,
        ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>> {
            self.preflight_calls.fetch_add(1, Ordering::SeqCst);
            let panic_on_preflight = self.panic_on_preflight.clone();
            Box::pin(async move {
                if panic_on_preflight.load(Ordering::SeqCst) {
                    panic!("fixture platform preflight panic");
                }
                Ok(PlatformInstallPlan::new(vec![PathBuf::from(
                    "fixture-install-target",
                )]))
            })
        }

        fn verify_package<'a>(
            &'a self,
            release: &'a ReleaseDescriptor,
            _artifact: &'a crate::codex_desktop::download::DownloadedArtifact,
        ) -> BoxFuture<'a, Result<VerifiedPackage, InstallerError>> {
            Box::pin(async move { Ok(VerifiedPackage::for_test(release)) })
        }

        fn install_current_user<'a>(
            &'a self,
            _package: &'a VerifiedPackage,
            progress: PlatformProgressSink,
        ) -> BoxFuture<'a, Result<(), InstallerError>> {
            self.install_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                progress.report_progress(JobProgress::new(
                    ProgressPhase::Installation,
                    Some(0),
                    Some(1),
                ));
                progress.report_progress(JobProgress::new(
                    ProgressPhase::Installation,
                    Some(1),
                    Some(1),
                ));
                Ok(())
            })
        }

        fn launch<'a>(
            &'a self,
            _installed: &'a InstalledApplication,
        ) -> BoxFuture<'a, Result<(), InstallerError>> {
            self.launch_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }
    }

    struct ServiceHarness {
        service: CodexDesktopService,
        source: Arc<FixtureSource>,
        platform: Arc<FixturePlatform>,
        disk_probe: Arc<FixtureDiskProbe>,
        temporary_parent: tempfile::TempDir,
        _log_directory: tempfile::TempDir,
    }

    fn release_for(artifact: &[u8], version: &str) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            version,
            PlatformVersion::parse_windows_msix(version).unwrap(),
            "OpenAI.Codex_fixture.msix",
            crate::codex_desktop::verify::sha256_hex(artifact),
            artifact.len() as u64,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .unwrap()
    }

    fn harness(
        release: ReleaseDescriptor,
        artifact: Vec<u8>,
        force_gate: Option<Arc<Notify>>,
    ) -> ServiceHarness {
        let temporary_parent = tempfile::tempdir().unwrap();
        let log_directory = tempfile::tempdir().unwrap();
        let source = Arc::new(match force_gate {
            Some(gate) => FixtureSource::with_force_gate(release.clone(), gate),
            None => FixtureSource::new(release.clone()),
        });
        let platform = Arc::new(FixturePlatform::new(release));
        let disk_probe = Arc::new(FixtureDiskProbe::default());
        let dependencies = CodexDesktopServiceDependencies::new(
            source.clone(),
            platform.clone(),
            Arc::new(FixtureTransport::new(artifact)),
            disk_probe.clone(),
            temporary_parent.path().join("installer-temp"),
            log_directory.path().to_path_buf(),
        );
        let service = CodexDesktopService::with_clock(dependencies, Arc::new(FixedClock));
        service.attach_log_directory_opener(Arc::new(|_: &Path| Ok(())));

        ServiceHarness {
            service,
            source,
            platform,
            disk_probe,
            temporary_parent,
            _log_directory: log_directory,
        }
    }

    async fn wait_for_terminal(service: &CodexDesktopService, job_id: &str) -> JobSnapshot {
        for _ in 0..100 {
            let snapshot = service
                .get_job()
                .unwrap()
                .expect("the started job remains queryable");
            if snapshot.job_id == job_id && snapshot.stage.is_terminal() {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("job {job_id} did not reach a terminal stage")
    }

    #[tokio::test]
    async fn happy_path_revalidates_downloads_verifies_installs_and_cleans_up() {
        let artifact = b"fixture installer package".to_vec();
        let release = release_for(&artifact, "1.2.3.4");
        let harness = harness(release, artifact, None);
        let events = Arc::new(RecordingSink::default());
        harness.service.attach_job_event_sink(events.clone());

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap();
        assert_eq!(started.stage, JobStage::Checking);

        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;
        assert_eq!(terminal.stage, JobStage::Succeeded);
        assert_eq!(terminal.result.as_ref().unwrap().warnings, Vec::new());
        assert_eq!(harness.platform.preflight_calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.platform.install_calls.load(Ordering::SeqCst), 1);

        let disk_paths = harness.disk_probe.paths();
        assert_eq!(disk_paths.len(), 2);
        assert!(disk_paths
            .iter()
            .any(|path| path.ends_with(Path::new(&started.job_id))));
        assert!(disk_paths
            .iter()
            .any(|path| path == Path::new("fixture-install-target")));

        let temporary_root = harness.temporary_parent.path().join("installer-temp");
        assert_eq!(std::fs::read_dir(temporary_root).unwrap().count(), 0);

        let stages = events
            .snapshots()
            .into_iter()
            .map(|snapshot| snapshot.stage)
            .collect::<Vec<_>>();
        for expected in [
            JobStage::Checking,
            JobStage::Preflight,
            JobStage::Downloading,
            JobStage::VerifyingDownload,
            JobStage::Installing,
            JobStage::VerifyingInstallation,
            JobStage::Succeeded,
        ] {
            assert!(stages.contains(&expected), "missing stage {expected:?}");
        }
    }

    #[tokio::test]
    async fn direct_install_request_launches_an_equal_local_version_without_downloading() {
        let artifact = b"fixture installer package".to_vec();
        let release = release_for(&artifact, "1.2.3.4");
        let harness = harness(release.clone(), artifact, None);
        harness
            .platform
            .set_initial_local_status(LocalInstallStatus::Installed {
                application: FixturePlatform::application_for(&release),
            });

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap();
        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;

        assert_eq!(terminal.stage, JobStage::Succeeded);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.platform.preflight_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.install_calls.load(Ordering::SeqCst), 0);
        assert!(harness.disk_probe.paths().is_empty());
        assert!(!harness
            .temporary_parent
            .path()
            .join("installer-temp")
            .exists());
    }

    #[tokio::test]
    async fn direct_install_request_launches_a_newer_local_version_without_downgrading() {
        let artifact = b"fixture installer package".to_vec();
        let release = release_for(&artifact, "1.2.3.4");
        let local_newer = release_for(&artifact, "1.2.3.5");
        let harness = harness(release, artifact, None);
        harness
            .platform
            .set_initial_local_status(LocalInstallStatus::Installed {
                application: FixturePlatform::application_for(&local_newer),
            });

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap();
        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;

        assert_eq!(terminal.stage, JobStage::Succeeded);
        assert_eq!(
            terminal
                .result
                .as_ref()
                .map(|result| &result.installed.platform_version),
            Some(&local_newer.platform_version)
        );
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.platform.preflight_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.install_calls.load(Ordering::SeqCst), 0);
        assert!(harness.disk_probe.paths().is_empty());
    }

    #[tokio::test]
    async fn metadata_change_after_check_fails_before_preflight_or_install() {
        let artifact = b"fixture installer package".to_vec();
        let original = release_for(&artifact, "1.2.3.4");
        let changed = release_for(&artifact, "1.2.3.5");
        let harness = harness(original, artifact, None);
        harness.source.set_forced_release(changed);

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap();
        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;

        assert_eq!(terminal.stage, JobStage::Failed);
        assert_eq!(
            terminal.error.as_ref().map(|error| error.code),
            Some(InstallerErrorCode::MetadataChanged)
        );
        assert_eq!(harness.platform.preflight_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.install_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn checksum_mismatch_refreshes_metadata_and_reports_changed_release() {
        let expected_artifact = b"expected".to_vec();
        let served_artifact = b"tampered".to_vec();
        let original = release_for(&expected_artifact, "1.2.3.4");
        let changed = release_for(&served_artifact, "1.2.3.5");
        let harness = harness(original.clone(), served_artifact, None);
        harness.source.queue_forced_releases([original, changed]);

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap();
        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;

        assert_eq!(terminal.stage, JobStage::Failed);
        assert_eq!(
            terminal.error.as_ref().map(|error| error.code),
            Some(InstallerErrorCode::MetadataChanged)
        );
        assert_eq!(
            recover_lock(&harness.source.calls).as_slice(),
            [
                CacheMode::UseCache,
                CacheMode::ForceRefresh,
                CacheMode::ForceRefresh
            ]
        );
        assert_eq!(harness.platform.preflight_calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.platform.install_calls.load(Ordering::SeqCst), 0);
        let temporary_root = harness.temporary_parent.path().join("installer-temp");
        assert_eq!(std::fs::read_dir(temporary_root).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn checksum_mismatch_with_unchanged_metadata_remains_terminal() {
        let expected_artifact = b"expected".to_vec();
        let served_artifact = b"tampered".to_vec();
        let release = release_for(&expected_artifact, "1.2.3.4");
        let harness = harness(release.clone(), served_artifact, None);
        harness
            .source
            .queue_forced_releases([release.clone(), release]);

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap();
        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;

        assert_eq!(terminal.stage, JobStage::Failed);
        assert_eq!(
            terminal.error.as_ref().map(|error| error.code),
            Some(InstallerErrorCode::ChecksumMismatch)
        );
        assert_eq!(
            recover_lock(&harness.source.calls).as_slice(),
            [
                CacheMode::UseCache,
                CacheMode::ForceRefresh,
                CacheMode::ForceRefresh
            ]
        );
        assert_eq!(harness.platform.preflight_calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.platform.install_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn cancellation_while_revalidating_never_reaches_platform_install() {
        let artifact = b"fixture installer package".to_vec();
        let release = release_for(&artifact, "1.2.3.4");
        let force_gate = Arc::new(Notify::new());
        let harness = harness(release, artifact, Some(force_gate));

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id.clone(),
            })
            .unwrap();
        let cancellation_requested = harness.service.cancel_install(&started.job_id).unwrap();
        assert_eq!(cancellation_requested.stage, JobStage::Checking);
        assert!(!cancellation_requested.cancellable);
        let blocked_start = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap_err();
        assert_eq!(blocked_start.code(), InstallerErrorCode::JobAlreadyRunning);
        harness.source.release_force_gate();

        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;
        assert_eq!(terminal.stage, JobStage::Cancelled);
        assert_eq!(harness.platform.preflight_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.install_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn source_panic_settles_failed_internal_error_and_releases_restart_claim() {
        let artifact = b"fixture installer package".to_vec();
        let release = release_for(&artifact, "1.2.3.4");
        let harness = harness(release, artifact, None);
        let events = Arc::new(RecordingSink::default());
        harness.service.attach_job_event_sink(events.clone());
        harness.source.set_panic_on_forced_refresh(true);

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap();
        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;

        assert_eq!(terminal.stage, JobStage::Failed);
        assert_eq!(
            terminal.error.as_ref().map(|error| error.code),
            Some(InstallerErrorCode::InternalError)
        );
        let published = events
            .snapshots()
            .into_iter()
            .last()
            .expect("the failed snapshot is published");
        assert_eq!(published.job_id, started.job_id);
        assert_eq!(published.stage, JobStage::Failed);
        assert_eq!(
            published.error.as_ref().map(|error| error.code),
            Some(InstallerErrorCode::InternalError)
        );
        harness
            .service
            .claim_restart()
            .expect("a failed worker no longer blocks restart claim");
    }

    #[tokio::test]
    async fn platform_panic_cleans_temp_and_releases_job_slot_for_next_start() {
        let artifact = b"fixture installer package".to_vec();
        let release = release_for(&artifact, "1.2.3.4");
        let harness = harness(release, artifact, None);
        harness.platform.set_panic_on_preflight(true);

        let checked = harness.service.check_latest(false).await.unwrap();
        let started = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id.clone(),
            })
            .unwrap();
        let terminal = wait_for_terminal(&harness.service, &started.job_id).await;

        assert_eq!(terminal.stage, JobStage::Failed);
        assert_eq!(
            terminal.error.as_ref().map(|error| error.code),
            Some(InstallerErrorCode::InternalError)
        );
        let temporary_root = harness.temporary_parent.path().join("installer-temp");
        assert_eq!(std::fs::read_dir(&temporary_root).unwrap().count(), 0);

        harness.platform.set_panic_on_preflight(false);
        let replacement = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .expect("a failed worker no longer blocks a new install");
        assert_ne!(replacement.job_id, started.job_id);
        let replacement_terminal = wait_for_terminal(&harness.service, &replacement.job_id).await;
        assert_eq!(replacement_terminal.stage, JobStage::Succeeded);
    }

    #[tokio::test]
    async fn restart_claim_blocks_a_subsequent_start_install() {
        let artifact = b"fixture installer package".to_vec();
        let release = release_for(&artifact, "1.2.3.4");
        let harness = harness(release, artifact, None);

        let checked = harness.service.check_latest(false).await.unwrap();
        harness.service.claim_restart().unwrap();

        let error = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: checked.release_id,
            })
            .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::JobAlreadyRunning);
        assert!(harness.service.get_job().unwrap().is_none());
        assert_eq!(harness.platform.preflight_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.install_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn only_the_checked_release_id_can_claim_a_job_slot_and_launch_stays_local() {
        let artifact = b"fixture installer package".to_vec();
        let release = release_for(&artifact, "1.2.3.4");
        let harness = harness(release.clone(), artifact, None);

        let error = harness
            .service
            .start_install(StartInstallRequest {
                expected_release_id: release.release_id().to_owned(),
            })
            .expect_err("a release must first be checked in this process");
        assert_eq!(error.code(), InstallerErrorCode::MetadataChanged);

        harness
            .platform
            .set_initial_local_status(LocalInstallStatus::Installed {
                application: FixturePlatform::application_for(&release),
            });
        harness.service.launch().await.unwrap();
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 1);
        assert!(recover_lock(&harness.source.calls).is_empty());
    }
}
