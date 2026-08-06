//! Orchestration for the Codex desktop installer.
//!
//! This layer deliberately owns no Tauri state and accepts no installer paths,
//! URLs, scopes, identities, or checksum values from IPC. It coordinates the
//! already-constrained core adapters and exposes only the fixed V1 operations
//! that the Tauri command shell delegates to.

mod restart_plan;

use std::{
    collections::HashMap,
    panic::AssertUnwindSafe,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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
        PlatformProgressSink, RestartCandidateInspection, RuntimeInspection,
        TrustedInstallationCandidate,
    },
    source::{CacheMode, ReleaseSource},
    temp::JobTempDir,
    types::{
        CodexDesktopManualRestartReason, CodexDesktopRestartOutcome, CodexDesktopRuntimeAmbiguity,
        CodexDesktopRuntimeStatus, CpuArchitecture, DesktopPlatform, InstallResult,
        InstalledApplication, InstallerWarningCode, JobProgress, JobSnapshot, JobStage,
        LocalInstallStatus, ProgressPhase, ReleaseDescriptor, RemoteReleaseStatus,
        StartInstallRequest, UnsupportedReason,
    },
    verify::{ensure_required_disk_space, DiskSpaceProbe},
};

#[cfg(test)]
use crate::codex_desktop::platform::TrustedRuntimeInstance;

use self::restart_plan::{RestartInstallationRuntime, RestartPlan};

/// Fixed event name used by the Tauri integration adapter.
pub const JOB_UPDATED_EVENT: &str = "codex-desktop-installer://job-updated";

const PROGRESS_MINIMUM_INTERVAL: Duration = Duration::from_millis(100);
const PROGRESS_MINIMUM_BYTE_DELTA: u64 = 1024 * 1024;
const RESTART_CLOSE_TIMEOUT: Duration = Duration::from_secs(8);
const RESTART_LAUNCH_VERIFY_TIMEOUT: Duration = Duration::from_secs(15);
const RESTART_POLL_INTERVAL: Duration = Duration::from_millis(200);
const RESTART_CAPABILITY_TTL: Duration = Duration::from_secs(120);

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
pub(crate) struct CodexDesktopServiceDependencies {
    source: Arc<dyn ReleaseSource>,
    platform: Arc<dyn CodexDesktopPlatform>,
    transport: Arc<dyn HttpTransport>,
    disk_space_probe: Arc<dyn DiskSpaceProbe>,
    temp_root: PathBuf,
    log_directory: PathBuf,
}

impl CodexDesktopServiceDependencies {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
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

#[derive(Debug, Clone, Copy)]
struct RestartTiming {
    close_timeout: Duration,
    launch_verify_timeout: Duration,
    poll_interval: Duration,
    capability_ttl: Duration,
}

impl Default for RestartTiming {
    fn default() -> Self {
        Self {
            close_timeout: RESTART_CLOSE_TIMEOUT,
            launch_verify_timeout: RESTART_LAUNCH_VERIFY_TIMEOUT,
            poll_interval: RESTART_POLL_INTERVAL,
            capability_ttl: RESTART_CAPABILITY_TTL,
        }
    }
}

#[derive(Debug, Clone)]
struct PendingRestart {
    action: PendingRestartAction,
    app_identity: String,
    selected_installation: String,
    plan_revision: String,
    expires_at_millis: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingRestartAction {
    Confirm,
    Retry,
}

#[derive(Default)]
struct RestartState {
    in_progress: bool,
    pending: HashMap<String, PendingRestart>,
}

/// A tiny injectable clock used only for opaque restart capability expiry.
/// It intentionally does not expose wall-clock values through IPC; fake tests
/// can advance it without sleeping or touching any desktop application.
trait RestartCapabilityClock: Send + Sync {
    fn now_millis(&self) -> u64;
}

#[derive(Debug, Default)]
struct SystemRestartCapabilityClock;

impl RestartCapabilityClock for SystemRestartCapabilityClock {
    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

/// Concatenating two independent UUID v4 values leaves 244 random bits after
/// their version/variant bits. The value remains an opaque printable token,
/// but now exceeds the shared 128-bit capability-entropy floor.
fn new_opaque_restart_capability_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
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
    restart_timing: RestartTiming,
    restart_capability_clock: Arc<dyn RestartCapabilityClock>,
    restart_state: Arc<Mutex<RestartState>>,
}

enum InstallFlowOutcome {
    Installed(InstalledApplication),
    LaunchedExisting(InstalledApplication),
}

enum RestartPlanInspection {
    NotInstalled,
    UntrustedTarget,
    Unsupported(UnsupportedReason),
    Plan(RestartPlan),
}

fn manual_untrusted_restart() -> CodexDesktopRestartOutcome {
    // Invalid/expired/reused capabilities intentionally collapse into the
    // same safe manual path as an untrusted target. This avoids turning token
    // probing into an oracle for local process or installation state.
    CodexDesktopRestartOutcome::ManualRestartRequired {
        reason: CodexDesktopManualRestartReason::UntrustedTarget,
    }
}

impl CodexDesktopService {
    pub(crate) fn new(dependencies: CodexDesktopServiceDependencies) -> Self {
        Self::with_clock(dependencies, Arc::new(SystemInstallerClock))
    }

    #[cfg(test)]
    pub(crate) fn with_clock(
        dependencies: CodexDesktopServiceDependencies,
        clock: Arc<dyn InstallerClock>,
    ) -> Self {
        Self::build(dependencies, clock)
    }

    #[cfg(test)]
    fn with_restart_timing(mut self, restart_timing: RestartTiming) -> Self {
        self.restart_timing = restart_timing;
        self
    }

    #[cfg(test)]
    fn with_restart_capability_clock(
        mut self,
        restart_capability_clock: Arc<dyn RestartCapabilityClock>,
    ) -> Self {
        self.restart_capability_clock = restart_capability_clock;
        self
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
            restart_timing: RestartTiming::default(),
            restart_capability_clock: Arc::new(SystemRestartCapabilityClock),
            restart_state: Arc::new(Mutex::new(RestartState::default())),
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

    /// Inspect the privacy-safe runtime state without selecting, closing, or
    /// launching anything. Multiple exact candidates remain an actionable
    /// restart-plan state rather than an automatic no-op; untrusted discovery
    /// remains fail-closed.
    pub async fn get_runtime_status(&self) -> Result<CodexDesktopRuntimeStatus, InstallerError> {
        Ok(match self.inspect_restart_plan().await? {
            RestartPlanInspection::NotInstalled => CodexDesktopRuntimeStatus::NotInstalled,
            RestartPlanInspection::UntrustedTarget => CodexDesktopRuntimeStatus::UntrustedTarget,
            RestartPlanInspection::Unsupported(reason) => {
                CodexDesktopRuntimeStatus::Unsupported { reason }
            }
            RestartPlanInspection::Plan(plan) if plan.is_not_running() => {
                CodexDesktopRuntimeStatus::NotRunning
            }
            RestartPlanInspection::Plan(plan) if plan.has_identity_binding_ambiguity() => {
                CodexDesktopRuntimeStatus::Ambiguous {
                    reason: CodexDesktopRuntimeAmbiguity::IdentityVerification,
                }
            }
            RestartPlanInspection::Plan(plan) if plan.installations.len() > 1 => {
                CodexDesktopRuntimeStatus::Ambiguous {
                    reason: CodexDesktopRuntimeAmbiguity::Installations,
                }
            }
            RestartPlanInspection::Plan(plan) if plan.runtime_instances.len() > 1 => {
                CodexDesktopRuntimeStatus::Ambiguous {
                    reason: CodexDesktopRuntimeAmbiguity::Instances,
                }
            }
            RestartPlanInspection::Plan(_) => CodexDesktopRuntimeStatus::Running,
        })
    }

    /// Prepare the one explicit destructive confirmation. This call never
    /// requests a graceful shutdown, force-closes a process, or launches an
    /// application. All close/launch work happens only when the opaque token
    /// is consumed by `continue_restart_with_force`.
    pub async fn request_restart(&self) -> CodexDesktopRestartOutcome {
        if !self.claim_restart_operation() {
            return manual_untrusted_restart();
        }

        let outcome = match self.inspect_restart_plan().await {
            Err(error) => {
                log::warn!(
                    "Codex restart plan could not be prepared: code={:?}",
                    error.code()
                );
                CodexDesktopRestartOutcome::ManualRestartRequired {
                    reason: CodexDesktopManualRestartReason::Unsupported,
                }
            }
            Ok(RestartPlanInspection::UntrustedTarget) => manual_untrusted_restart(),
            Ok(RestartPlanInspection::Unsupported(_)) => {
                CodexDesktopRestartOutcome::ManualRestartRequired {
                    reason: CodexDesktopManualRestartReason::Unsupported,
                }
            }
            Ok(RestartPlanInspection::Plan(plan)) if plan.is_not_running() => {
                CodexDesktopRestartOutcome::NotRunning
            }
            Ok(RestartPlanInspection::Plan(plan)) => {
                let token = self.issue_restart_capability(&plan, PendingRestartAction::Confirm);
                CodexDesktopRestartOutcome::ConfirmationRequired {
                    token,
                    reason: plan.prompt_reason(),
                }
            }
            Ok(RestartPlanInspection::NotInstalled) => CodexDesktopRestartOutcome::NotRunning,
        };

        if !matches!(
            outcome,
            CodexDesktopRestartOutcome::ConfirmationRequired { .. }
        ) {
            self.complete_restart_operation();
        }
        outcome
    }

    /// Consume an opaque confirmation or retry capability and directly carry
    /// out the force-close-and-relaunch algorithm. The capability is single
    /// use; the current exact candidates and runtime evidence are always
    /// re-enumerated so a process that appeared after confirmation is included
    /// and no stale PID becomes a lifecycle target.
    pub async fn continue_restart_with_force(&self, token: &str) -> CodexDesktopRestartOutcome {
        let Some(_pending) = self.take_restart_capability(token) else {
            return manual_untrusted_restart();
        };

        let outcome = match self.inspect_restart_plan().await {
            Err(error) => {
                log::warn!(
                    "Codex restart execution plan could not be rebuilt: code={:?}",
                    error.code()
                );
                CodexDesktopRestartOutcome::ManualRestartRequired {
                    reason: CodexDesktopManualRestartReason::Unsupported,
                }
            }
            Ok(RestartPlanInspection::UntrustedTarget) => manual_untrusted_restart(),
            Ok(RestartPlanInspection::Unsupported(_)) => {
                CodexDesktopRestartOutcome::ManualRestartRequired {
                    reason: CodexDesktopManualRestartReason::Unsupported,
                }
            }
            Ok(RestartPlanInspection::NotInstalled) => manual_untrusted_restart(),
            Ok(RestartPlanInspection::Plan(plan)) if plan.has_identity_binding_ambiguity() => {
                self.incomplete_restart(&plan)
            }
            Ok(RestartPlanInspection::Plan(plan)) => {
                // This must happen before the first force-close. If a selected
                // installation vanished or lost its exact identity, no live
                // instance is touched and the renderer receives only the
                // generic retry/manual recovery UI.
                if self
                    .revalidate_selected_installation(plan.selected())
                    .await
                    .is_err()
                {
                    self.incomplete_restart(&plan)
                } else {
                    let close_targets = plan.close_targets();
                    let close_result = self.force_shutdown_targets(&close_targets).await;
                    if close_result.is_ok()
                        && self
                            .wait_for_bound_instances_exit(&close_targets)
                            .await
                            .unwrap_or(false)
                    {
                        match self.launch_and_verify_restart(plan.selected()).await {
                            Ok(()) => CodexDesktopRestartOutcome::Restarted,
                            Err(_) => self.incomplete_restart(&plan),
                        }
                    } else {
                        // Any failed force call, liveness read, or timeout is
                        // terminal for this attempt. Never launch after a
                        // partial close set.
                        self.incomplete_restart(&plan)
                    }
                }
            }
        };
        self.complete_restart_operation();
        outcome
    }

    /// Discard a force-confirmation continuation after the user elects to
    /// restart manually. Invalid, expired, or already-consumed tokens are an
    /// intentional no-op so this capability cannot reveal process state.
    pub fn cancel_restart_with_force(&self, token: &str) {
        self.discard_restart_capability(token);
    }

    async fn inspect_restart_plan(&self) -> Result<RestartPlanInspection, InstallerError> {
        let candidates = match self.platform.inspect_restart_candidates().await? {
            RestartCandidateInspection::NotInstalled => {
                return Ok(RestartPlanInspection::NotInstalled)
            }
            RestartCandidateInspection::UntrustedTarget => {
                return Ok(RestartPlanInspection::UntrustedTarget)
            }
            RestartCandidateInspection::Unsupported(reason) => {
                return Ok(RestartPlanInspection::Unsupported(reason))
            }
            RestartCandidateInspection::Trusted(candidates) => candidates,
        };

        if candidates.is_empty()
            || candidates.iter().any(|candidate| {
                candidate.application.stable_identity != candidates[0].application.stable_identity
            })
        {
            return Ok(RestartPlanInspection::UntrustedTarget);
        }

        let mut installations = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let (instances, identity_binding_ambiguous) =
                match self.platform.inspect_runtime(&candidate.application).await {
                    Ok(RuntimeInspection::NotRunning) => (Vec::new(), false),
                    Ok(RuntimeInspection::Running(instances)) => (instances, false),
                    Ok(RuntimeInspection::Ambiguous) => (Vec::new(), true),
                    Err(error)
                        if matches!(error.code(), InstallerErrorCode::PlatformUnsupported) =>
                    {
                        return Ok(RestartPlanInspection::Unsupported(
                            UnsupportedReason::Platform,
                        ));
                    }
                    Err(error) => return Err(error),
                };
            installations.push(RestartInstallationRuntime {
                candidate,
                instances,
                identity_binding_ambiguous,
            });
        }

        RestartPlan::new(installations)
            .map(RestartPlanInspection::Plan)
            .ok_or_else(|| {
                InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                    .with_diagnostic_message("restart plan did not contain a trusted candidate")
            })
    }

    /// Re-enumerate trusted candidates and require the selected stable key to
    /// remain exact before force-closing any runtime. This check deliberately
    /// does not substitute a newly preferred candidate; execution itself has
    /// already rebuilt the plan with the fixed comparator.
    async fn revalidate_selected_installation(
        &self,
        expected: &TrustedInstallationCandidate,
    ) -> Result<(), InstallerError> {
        match self.platform.inspect_restart_candidates().await? {
            RestartCandidateInspection::Trusted(candidates)
                if candidates.iter().any(|candidate| {
                    candidate.stable_key == expected.stable_key
                        && candidate.application == expected.application
                }) =>
            {
                Ok(())
            }
            _ => Err(
                InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                    .with_diagnostic_message(
                        "the selected Codex installation changed before force restart",
                    ),
            ),
        }
    }

    async fn force_shutdown_targets(
        &self,
        targets: &[RestartInstallationRuntime],
    ) -> Result<(), InstallerError> {
        for target in targets {
            self.platform
                .force_shutdown(&target.candidate.application, &target.instances)
                .await?;
        }
        Ok(())
    }

    async fn wait_for_bound_instances_exit(
        &self,
        targets: &[RestartInstallationRuntime],
    ) -> Result<bool, InstallerError> {
        let deadline = Instant::now() + self.restart_timing.close_timeout;
        loop {
            let mut any_running = false;
            for target in targets {
                if self
                    .platform
                    .is_runtime_instance_running(&target.candidate.application, &target.instances)
                    .await?
                {
                    any_running = true;
                }
            }
            if !any_running {
                return Ok(true);
            }

            if Instant::now() >= deadline {
                return Ok(false);
            }
            tokio::time::sleep(self.restart_timing.poll_interval).await;
        }
    }

    async fn launch_and_verify_restart(
        &self,
        expected: &TrustedInstallationCandidate,
    ) -> Result<(), InstallerError> {
        self.revalidate_selected_installation(expected).await?;
        self.platform.launch(&expected.application).await?;

        let deadline = Instant::now() + self.restart_timing.launch_verify_timeout;
        loop {
            self.revalidate_selected_installation(expected).await?;
            match self.platform.inspect_runtime(&expected.application).await {
                Ok(RuntimeInspection::Running(instances)) if !instances.is_empty() => return Ok(()),
                Ok(RuntimeInspection::Running(_)) => {}
                Ok(RuntimeInspection::Ambiguous) => {
                    return Err(
                        InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                            .with_diagnostic_message(
                                "the trusted runtime could not be bound after launch",
                            ),
                    );
                }
                Ok(RuntimeInspection::NotRunning) => {}
                Err(error) => return Err(error),
            }
            if Instant::now() >= deadline {
                return Err(InstallerError::new(InstallerErrorCode::LaunchFailed)
                    .with_diagnostic_message("the trusted runtime did not appear after launch"));
            }
            tokio::time::sleep(self.restart_timing.poll_interval).await;
        }
    }

    fn claim_restart_operation(&self) -> bool {
        let now = self.restart_capability_clock.now_millis();
        let mut state = recover_lock(&self.restart_state);
        state
            .pending
            .retain(|_, pending| pending.expires_at_millis > now);
        if state.in_progress || !state.pending.is_empty() {
            return false;
        }
        state.in_progress = true;
        true
    }

    fn complete_restart_operation(&self) {
        let mut state = recover_lock(&self.restart_state);
        state.in_progress = false;
    }

    fn issue_restart_capability(&self, plan: &RestartPlan, action: PendingRestartAction) -> String {
        let token = new_opaque_restart_capability_token();
        let now = self.restart_capability_clock.now_millis();
        let capability_ttl_millis = self
            .restart_timing
            .capability_ttl
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let mut state = recover_lock(&self.restart_state);
        state.pending.insert(
            token.clone(),
            PendingRestart {
                action,
                app_identity: plan.app_identity.clone(),
                selected_installation: plan.selected_installation.clone(),
                plan_revision: plan.plan_revision.clone(),
                expires_at_millis: now.saturating_add(capability_ttl_millis),
            },
        );
        state.in_progress = false;
        token
    }

    fn take_restart_capability(&self, token: &str) -> Option<PendingRestart> {
        let now = self.restart_capability_clock.now_millis();
        let mut state = recover_lock(&self.restart_state);
        state
            .pending
            .retain(|_, pending| pending.expires_at_millis > now);
        let pending = state.pending.remove(token)?;
        // The token is opaque, but its server-side capability record still
        // binds it to a known action, exact app identity, selected stable key,
        // and a fixed plan revision. We intentionally do not compare those
        // transient selections to the execution plan: the algorithm must
        // re-enumerate and apply the current deterministic comparator.
        if !matches!(
            pending.action,
            PendingRestartAction::Confirm | PendingRestartAction::Retry
        ) || pending.app_identity.is_empty()
            || pending.selected_installation.is_empty()
            || pending.plan_revision.len() != 64
        {
            return None;
        }
        state.in_progress = true;
        Some(pending)
    }

    fn discard_restart_capability(&self, token: &str) {
        let now = self.restart_capability_clock.now_millis();
        let mut state = recover_lock(&self.restart_state);
        state
            .pending
            .retain(|_, pending| pending.expires_at_millis > now);
        // Do not mutate `in_progress`: a continuation removes its token before
        // it starts and owns that flag until it completes. A cancellation
        // racing with it must never unlock a live destructive operation.
        state.pending.remove(token);
    }

    fn incomplete_restart(&self, plan: &RestartPlan) -> CodexDesktopRestartOutcome {
        let retry_token = self.issue_restart_capability(plan, PendingRestartAction::Retry);
        CodexDesktopRestartOutcome::Incomplete { retry_token }
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
        platform::{
            PlatformInstallPlan, RestartCandidateInspection, TrustedInstallationCandidate,
            VerifiedPackage, WINDOWS_CODEX_STABLE_IDENTITY,
        },
        types::{LaunchTarget, PlatformVersion, TrustedDownloadEndpoint},
        verify::{DiskSpaceProbeError, VolumeKey},
    };

    #[test]
    fn restart_capabilities_exceed_the_required_random_entropy_floor() {
        let first = new_opaque_restart_capability_token();
        let second = new_opaque_restart_capability_token();

        assert_eq!(first.len(), 64);
        assert_eq!(second.len(), 64);
        assert_ne!(first, second);
    }

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

    #[derive(Clone)]
    struct RestartFixturePlatform {
        fallback_candidate_inspection: Arc<Mutex<RestartCandidateInspection>>,
        candidate_inspections: Arc<Mutex<VecDeque<RestartCandidateInspection>>>,
        runtime_inspections: Arc<Mutex<VecDeque<RuntimeInspection>>>,
        fallback_runtime: Arc<Mutex<RuntimeInspection>>,
        liveness: Arc<Mutex<VecDeque<bool>>>,
        fallback_liveness: Arc<AtomicBool>,
        force_results: Arc<Mutex<VecDeque<bool>>>,
        force_calls: Arc<AtomicUsize>,
        launch_calls: Arc<AtomicUsize>,
        force_targets: Arc<Mutex<Vec<Vec<TrustedRuntimeInstance>>>>,
        liveness_targets: Arc<Mutex<Vec<Vec<TrustedRuntimeInstance>>>>,
        launch_targets: Arc<Mutex<Vec<InstalledApplication>>>,
    }

    impl RestartFixturePlatform {
        fn new(installed: InstalledApplication) -> Self {
            Self {
                fallback_candidate_inspection: Arc::new(Mutex::new(
                    RestartCandidateInspection::Trusted(vec![restart_candidate(installed)]),
                )),
                candidate_inspections: Arc::new(Mutex::new(VecDeque::new())),
                runtime_inspections: Arc::new(Mutex::new(VecDeque::new())),
                fallback_runtime: Arc::new(Mutex::new(RuntimeInspection::NotRunning)),
                liveness: Arc::new(Mutex::new(VecDeque::new())),
                fallback_liveness: Arc::new(AtomicBool::new(false)),
                force_results: Arc::new(Mutex::new(VecDeque::new())),
                force_calls: Arc::new(AtomicUsize::new(0)),
                launch_calls: Arc::new(AtomicUsize::new(0)),
                force_targets: Arc::new(Mutex::new(Vec::new())),
                liveness_targets: Arc::new(Mutex::new(Vec::new())),
                launch_targets: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn queue_candidates(&self, values: impl IntoIterator<Item = RestartCandidateInspection>) {
            recover_lock(&self.candidate_inspections).extend(values);
        }

        fn queue_runtime(&self, values: impl IntoIterator<Item = RuntimeInspection>) {
            recover_lock(&self.runtime_inspections).extend(values);
        }

        fn queue_liveness(&self, values: impl IntoIterator<Item = bool>) {
            recover_lock(&self.liveness).extend(values);
        }

        fn queue_force_results(&self, values: impl IntoIterator<Item = bool>) {
            recover_lock(&self.force_results).extend(values);
        }

        fn next_candidates(&self) -> RestartCandidateInspection {
            recover_lock(&self.candidate_inspections)
                .pop_front()
                .unwrap_or_else(|| recover_lock(&self.fallback_candidate_inspection).clone())
        }

        fn next_runtime(&self) -> RuntimeInspection {
            recover_lock(&self.runtime_inspections)
                .pop_front()
                .unwrap_or_else(|| recover_lock(&self.fallback_runtime).clone())
        }

        fn next_liveness(&self) -> bool {
            recover_lock(&self.liveness)
                .pop_front()
                .unwrap_or_else(|| self.fallback_liveness.load(Ordering::SeqCst))
        }

        fn next_force_result(&self) -> bool {
            recover_lock(&self.force_results)
                .pop_front()
                .unwrap_or(true)
        }
    }

    impl CodexDesktopPlatform for RestartFixturePlatform {
        fn platform(&self) -> Option<DesktopPlatform> {
            Some(DesktopPlatform::Windows)
        }

        fn architecture(&self) -> CpuArchitecture {
            CpuArchitecture::X86_64
        }

        fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
            let inspection = self.next_candidates();
            Box::pin(async move {
                Ok(match inspection {
                    RestartCandidateInspection::Trusted(mut candidates)
                        if candidates.len() == 1 =>
                    {
                        LocalInstallStatus::Installed {
                            application: candidates.remove(0).application,
                        }
                    }
                    RestartCandidateInspection::NotInstalled => LocalInstallStatus::NotInstalled {
                        platform: DesktopPlatform::Windows,
                        architecture: CpuArchitecture::X86_64,
                    },
                    RestartCandidateInspection::Unsupported(reason) => {
                        LocalInstallStatus::Unsupported { reason }
                    }
                    RestartCandidateInspection::Trusted(candidates) => {
                        LocalInstallStatus::Ambiguous {
                            candidates: candidates
                                .iter()
                                .map(|candidate| {
                                    crate::codex_desktop::types::InstalledApplicationSummary::from(
                                        &candidate.application,
                                    )
                                })
                                .collect(),
                            error: InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                                .to_dto(),
                        }
                    }
                    RestartCandidateInspection::UntrustedTarget => LocalInstallStatus::Ambiguous {
                        candidates: Vec::new(),
                        error: InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                            .to_dto(),
                    },
                })
            })
        }

        fn inspect_restart_candidates(
            &self,
        ) -> BoxFuture<'_, Result<RestartCandidateInspection, InstallerError>> {
            let inspection = self.next_candidates();
            Box::pin(async move { Ok(inspection) })
        }

        fn preflight<'a>(
            &'a self,
            _release: &'a ReleaseDescriptor,
            _temp_root: &'a Path,
        ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>> {
            Box::pin(async { Ok(PlatformInstallPlan::default()) })
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
            _progress: PlatformProgressSink,
        ) -> BoxFuture<'a, Result<(), InstallerError>> {
            Box::pin(async { Ok(()) })
        }

        fn launch<'a>(
            &'a self,
            installed: &'a InstalledApplication,
        ) -> BoxFuture<'a, Result<(), InstallerError>> {
            self.launch_calls.fetch_add(1, Ordering::SeqCst);
            recover_lock(&self.launch_targets).push(installed.clone());
            Box::pin(async { Ok(()) })
        }

        fn inspect_runtime<'a>(
            &'a self,
            _installed: &'a InstalledApplication,
        ) -> BoxFuture<'a, Result<RuntimeInspection, InstallerError>> {
            let inspection = self.next_runtime();
            Box::pin(async move { Ok(inspection) })
        }

        fn force_shutdown<'a>(
            &'a self,
            _installed: &'a InstalledApplication,
            instances: &'a [TrustedRuntimeInstance],
        ) -> BoxFuture<'a, Result<(), InstallerError>> {
            self.force_calls.fetch_add(1, Ordering::SeqCst);
            recover_lock(&self.force_targets).push(instances.to_vec());
            let succeeds = self.next_force_result();
            Box::pin(async move {
                succeeds.then_some(()).ok_or_else(|| {
                    InstallerError::new(InstallerErrorCode::LaunchFailed)
                        .with_diagnostic_message("fixture force failure")
                })
            })
        }

        fn is_runtime_instance_running<'a>(
            &'a self,
            _installed: &'a InstalledApplication,
            instances: &'a [TrustedRuntimeInstance],
        ) -> BoxFuture<'a, Result<bool, InstallerError>> {
            recover_lock(&self.liveness_targets).push(instances.to_vec());
            let running = self.next_liveness();
            Box::pin(async move { Ok(running) })
        }
    }

    fn restart_application(launch_id: &str) -> InstalledApplication {
        InstalledApplication {
            stable_identity: WINDOWS_CODEX_STABLE_IDENTITY.to_owned(),
            display_name: Some("Codex".to_owned()),
            display_version: Some("1.2.3.4".to_owned()),
            platform_version: PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            architecture: CpuArchitecture::X86_64,
            location: Some(format!("C:\\redacted\\{launch_id}")),
            launch_target: LaunchTarget::WindowsAumid(format!("fixture.{launch_id}!App")),
        }
    }

    fn restart_candidate(installed: InstalledApplication) -> TrustedInstallationCandidate {
        let LaunchTarget::WindowsAumid(aumid) = &installed.launch_target else {
            unreachable!("restart fixtures always use a Windows AUMID");
        };
        TrustedInstallationCandidate {
            stable_key: format!("windows-pfn:{aumid}"),
            application: installed,
            scope: crate::codex_desktop::platform::RestartInstallationScope::CurrentUser,
        }
    }

    fn restart_instance(process_id: u32) -> TrustedRuntimeInstance {
        TrustedRuntimeInstance::Windows {
            package_family_name: "fixture_family".to_owned(),
            process_id,
            creation_time: process_id as u64,
        }
    }

    fn immediate_restart_timing() -> RestartTiming {
        RestartTiming {
            close_timeout: Duration::ZERO,
            launch_verify_timeout: Duration::ZERO,
            poll_interval: Duration::ZERO,
            capability_ttl: Duration::from_secs(60),
        }
    }

    #[derive(Default)]
    struct FixtureRestartCapabilityClock {
        now_millis: AtomicUsize,
    }

    impl FixtureRestartCapabilityClock {
        fn advance_millis(&self, milliseconds: usize) {
            self.now_millis.fetch_add(milliseconds, Ordering::SeqCst);
        }
    }

    impl RestartCapabilityClock for FixtureRestartCapabilityClock {
        fn now_millis(&self) -> u64 {
            self.now_millis.load(Ordering::SeqCst) as u64
        }
    }

    struct RestartHarness {
        service: CodexDesktopService,
        platform: Arc<RestartFixturePlatform>,
        capability_clock: Arc<FixtureRestartCapabilityClock>,
        _temporary_parent: tempfile::TempDir,
        _log_directory: tempfile::TempDir,
    }

    fn restart_harness(installed: InstalledApplication) -> RestartHarness {
        restart_harness_with_timing(installed, immediate_restart_timing())
    }

    fn restart_harness_with_timing(
        installed: InstalledApplication,
        restart_timing: RestartTiming,
    ) -> RestartHarness {
        let release = release_for(b"restart fixture", "1.2.3.4");
        let temporary_parent = tempfile::tempdir().unwrap();
        let log_directory = tempfile::tempdir().unwrap();
        let platform = Arc::new(RestartFixturePlatform::new(installed));
        let capability_clock = Arc::new(FixtureRestartCapabilityClock::default());
        let dependencies = CodexDesktopServiceDependencies::new(
            Arc::new(FixtureSource::new(release)),
            platform.clone(),
            Arc::new(FixtureTransport::new(Vec::new())),
            Arc::new(FixtureDiskProbe::default()),
            temporary_parent.path().join("restart-temp"),
            log_directory.path().to_path_buf(),
        );
        let service = CodexDesktopService::with_clock(dependencies, Arc::new(FixedClock))
            .with_restart_timing(restart_timing)
            .with_restart_capability_clock(capability_clock.clone());

        RestartHarness {
            service,
            platform,
            capability_clock,
            _temporary_parent: temporary_parent,
            _log_directory: log_directory,
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

    #[test]
    fn restart_deadlines_match_the_documented_close_and_verify_bounds() {
        let timing = RestartTiming::default();
        assert_eq!(timing.close_timeout, Duration::from_secs(8));
        assert_eq!(timing.launch_verify_timeout, Duration::from_secs(15));
        assert_eq!(timing.poll_interval, Duration::from_millis(200));
        assert_eq!(timing.capability_ttl, Duration::from_secs(120));
    }

    #[tokio::test]
    async fn confirmation_is_inert_then_reenumerates_and_force_closes_all_exact_instances() {
        let installed = restart_application("primary");
        let original_instance = restart_instance(4242);
        let instance_started_after_confirmation = restart_instance(4243);
        let harness = restart_harness(installed.clone());
        harness.platform.queue_runtime([
            RuntimeInspection::Running(vec![original_instance.clone()]),
            // The execution-time scan observes a new exact instance. It must
            // enter the same close set without asking the user again.
            RuntimeInspection::Running(vec![
                original_instance.clone(),
                instance_started_after_confirmation.clone(),
            ]),
            RuntimeInspection::Running(vec![restart_instance(9000)]),
        ]);
        harness.platform.queue_liveness([false]);

        let first = harness.service.request_restart().await;
        let CodexDesktopRestartOutcome::ConfirmationRequired { token, .. } = first else {
            panic!("a running exact target must require one opaque confirmation");
        };

        // Preparing confirmation is observational only: no force close or
        // launch is allowed before the user confirms.
        assert_eq!(harness.platform.force_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 0);

        let outcome = harness.service.continue_restart_with_force(&token).await;
        assert_eq!(outcome, CodexDesktopRestartOutcome::Restarted);
        assert_eq!(harness.platform.force_calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            recover_lock(&harness.platform.force_targets).as_slice(),
            [vec![
                original_instance.clone(),
                instance_started_after_confirmation,
            ]]
        );
        assert_eq!(
            recover_lock(&harness.platform.liveness_targets).as_slice(),
            [vec![original_instance, restart_instance(4243)]]
        );
        assert_eq!(
            recover_lock(&harness.platform.launch_targets).as_slice(),
            [installed],
            "restart must launch the selected exact installation exactly once"
        );
    }

    #[tokio::test]
    async fn identity_binding_ambiguity_prompts_then_fails_incomplete_without_process_actions() {
        let harness = restart_harness(restart_application("primary"));
        harness.platform.queue_runtime([
            RuntimeInspection::Ambiguous,
            RuntimeInspection::Ambiguous,
            RuntimeInspection::Ambiguous,
        ]);

        assert_eq!(
            harness.service.get_runtime_status().await.unwrap(),
            CodexDesktopRuntimeStatus::Ambiguous {
                reason: CodexDesktopRuntimeAmbiguity::IdentityVerification
            }
        );
        let first = harness.service.request_restart().await;
        let CodexDesktopRestartOutcome::ConfirmationRequired { token, reason } = first else {
            panic!("an identity ambiguity must enter the one-confirmation UI state");
        };
        assert_eq!(
            reason,
            crate::codex_desktop::types::CodexDesktopRestartPromptReason::IdentityBindingAmbiguous
        );
        let outcome = harness.service.continue_restart_with_force(&token).await;
        assert!(matches!(
            outcome,
            CodexDesktopRestartOutcome::Incomplete { .. }
        ));
        assert_eq!(harness.platform.force_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn selected_installation_loss_before_close_returns_incomplete_without_force_or_launch() {
        let installed = restart_application("primary");
        let original_instance = restart_instance(4242);
        let harness = restart_harness(installed.clone());
        harness.platform.queue_runtime([
            RuntimeInspection::Running(vec![original_instance.clone()]),
            RuntimeInspection::Running(vec![original_instance]),
        ]);

        let first = harness.service.request_restart().await;
        let CodexDesktopRestartOutcome::ConfirmationRequired { token, .. } = first else {
            panic!("a trusted running fixture must enter confirmation");
        };

        // Execution may rebuild its plan, but a second exact candidate read
        // must still validate the selected app before any close call.
        harness.platform.queue_candidates([
            RestartCandidateInspection::Trusted(vec![restart_candidate(installed)]),
            RestartCandidateInspection::NotInstalled,
        ]);
        let second = harness.service.continue_restart_with_force(&token).await;
        assert!(matches!(
            second,
            CodexDesktopRestartOutcome::Incomplete { .. }
        ));
        assert_eq!(harness.platform.force_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn force_failure_returns_incomplete_and_never_launches() {
        let original_instance = restart_instance(4242);
        let harness = restart_harness(restart_application("primary"));
        harness.platform.queue_runtime([
            RuntimeInspection::Running(vec![original_instance.clone()]),
            RuntimeInspection::Running(vec![original_instance]),
        ]);
        harness.platform.queue_force_results([false]);

        let first = harness.service.request_restart().await;
        let CodexDesktopRestartOutcome::ConfirmationRequired { token, .. } = first else {
            panic!("first request must create a confirmation capability");
        };
        let outcome = harness.service.continue_restart_with_force(&token).await;

        assert!(matches!(
            outcome,
            CodexDesktopRestartOutcome::Incomplete { .. }
        ));
        assert_eq!(harness.platform.force_calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn launch_verification_waits_for_a_trusted_runtime_to_become_ready() {
        let original_instance = restart_instance(4242);
        let harness = restart_harness_with_timing(
            restart_application("primary"),
            RestartTiming {
                close_timeout: Duration::ZERO,
                launch_verify_timeout: Duration::from_secs(1),
                poll_interval: Duration::ZERO,
                capability_ttl: Duration::from_secs(60),
            },
        );
        harness.platform.queue_runtime([
            RuntimeInspection::Running(vec![original_instance.clone()]),
            RuntimeInspection::Running(vec![original_instance]),
            // The exact runtime is not immediately visible after launch. The
            // service must keep waiting without exposing phase details.
            RuntimeInspection::NotRunning,
            RuntimeInspection::Running(vec![restart_instance(9000)]),
        ]);
        harness.platform.queue_liveness([false]);

        let first = harness.service.request_restart().await;
        let CodexDesktopRestartOutcome::ConfirmationRequired { token, .. } = first else {
            panic!("the running fixture must produce a confirmation capability");
        };
        let outcome = harness.service.continue_restart_with_force(&token).await;

        assert_eq!(outcome, CodexDesktopRestartOutcome::Restarted);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn launch_verification_failure_is_incomplete_without_technical_error_details() {
        let original_instance = restart_instance(4242);
        let harness = restart_harness(restart_application("primary"));
        harness.platform.queue_runtime([
            RuntimeInspection::Running(vec![original_instance.clone()]),
            RuntimeInspection::Running(vec![original_instance]),
            RuntimeInspection::NotRunning,
        ]);
        harness.platform.queue_liveness([false]);

        let first = harness.service.request_restart().await;
        let CodexDesktopRestartOutcome::ConfirmationRequired { token, .. } = first else {
            panic!("the running fixture must produce a confirmation capability");
        };
        let outcome = harness.service.continue_restart_with_force(&token).await;

        assert!(matches!(
            outcome,
            CodexDesktopRestartOutcome::Incomplete { .. }
        ));
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn declining_restart_discards_the_capability_and_allows_a_new_confirmation() {
        let original_instance = restart_instance(4242);
        let harness = restart_harness(restart_application("primary"));
        harness.platform.queue_runtime([
            RuntimeInspection::Running(vec![original_instance.clone()]),
            RuntimeInspection::Running(vec![original_instance]),
        ]);

        let first = harness.service.request_restart().await;
        let CodexDesktopRestartOutcome::ConfirmationRequired { token, .. } = first else {
            panic!("the first request must issue an opaque confirmation capability");
        };

        harness.service.cancel_restart_with_force(&token);
        let retry = harness.service.request_restart().await;

        assert!(matches!(
            retry,
            CodexDesktopRestartOutcome::ConfirmationRequired { .. }
        ));
        assert_eq!(harness.platform.force_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn expired_or_reused_capabilities_fail_closed_without_process_actions() {
        let original_instance = restart_instance(4242);
        let harness = restart_harness_with_timing(
            restart_application("primary"),
            RestartTiming {
                capability_ttl: Duration::from_millis(1),
                ..immediate_restart_timing()
            },
        );
        harness
            .platform
            .queue_runtime([RuntimeInspection::Running(vec![original_instance])]);

        let first = harness.service.request_restart().await;
        let CodexDesktopRestartOutcome::ConfirmationRequired { token, .. } = first else {
            panic!("the running fixture must issue a capability");
        };
        harness.capability_clock.advance_millis(2);

        assert_eq!(
            harness.service.continue_restart_with_force(&token).await,
            manual_untrusted_restart()
        );
        assert_eq!(harness.platform.force_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 0);

        // A consumed/unknown opaque token produces the same fail-closed DTO;
        // callers cannot use it to infer local runtime state.
        assert_eq!(
            harness
                .service
                .continue_restart_with_force("tampered-token")
                .await,
            manual_untrusted_restart()
        );
    }

    #[tokio::test]
    async fn untrusted_target_never_reaches_runtime_force_or_launch_boundaries() {
        let harness = restart_harness(restart_application("primary"));
        harness
            .platform
            .queue_candidates([RestartCandidateInspection::UntrustedTarget]);

        assert_eq!(
            harness.service.request_restart().await,
            CodexDesktopRestartOutcome::ManualRestartRequired {
                reason: CodexDesktopManualRestartReason::UntrustedTarget,
            }
        );
        assert_eq!(harness.platform.force_calls.load(Ordering::SeqCst), 0);
        assert_eq!(harness.platform.launch_calls.load(Ordering::SeqCst), 0);
        assert!(recover_lock(&harness.platform.force_targets).is_empty());
        assert!(recover_lock(&harness.platform.launch_targets).is_empty());
    }
}
