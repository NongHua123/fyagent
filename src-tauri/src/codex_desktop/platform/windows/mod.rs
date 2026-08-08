//! Windows x64 and ARM64 current-user MSIX adapter.
//!
//! The normal adapter has no install scope, no arbitrary URL/path input, and
//! no elevation capability. It accepts only core-owned `VerifiedPackage`
//! evidence, deploys it by local `file://` URI through PackageManager, then
//! relies on the common service to re-query the registered package.

mod deployment;
#[cfg(target_os = "windows")]
pub(crate) mod elevation;
mod manifest;

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::future::BoxFuture;

use self::{
    deployment::{
        deployment_error, launch_error, local_file_uri, WindowsDeploymentProgressSink,
        WindowsPackageManager, WindowsPackageRecord,
    },
    manifest::{parse_msix_manifest, WindowsPackageManifest},
};

#[cfg(test)]
use self::deployment::WindowsNativeError;
#[cfg(target_os = "windows")]
mod runtime;
use super::{
    CodexDesktopPlatform, PlatformInstallPlan, PlatformProgressSink, RestartCandidateInspection,
    RestartInstallationScope, RuntimeInspection, TrustedInstallationCandidate,
    TrustedRuntimeInstance, VerifiedPackage, WINDOWS_CODEX_STABLE_IDENTITY,
};
use crate::codex_desktop::{
    download::DownloadedArtifact,
    error::{InstallerError, InstallerErrorCode},
    types::{
        CpuArchitecture, DesktopPlatform, InstalledApplication, InstalledApplicationSummary,
        JobProgress, LaunchTarget, LocalInstallStatus, PlatformVersion, ProgressPhase,
        ReleaseDescriptor, UnsupportedReason,
    },
};

#[cfg(target_os = "windows")]
#[cfg_attr(test, allow(unused_imports))]
pub use deployment::SystemWindowsDiskSpaceProbe;
#[cfg(target_os = "windows")]
pub use deployment::SystemWindowsPackageManager;

/// Exact Publisher allowlist from read-only local Windows evidence collected on
/// 2026-07-29. The current-user Microsoft Store package was
/// `OpenAI.Codex_26.721.4979.0_x64__2p2nqsd0c76g0` with
/// `Name=OpenAI.Codex`, version `26.721.4979.0`,
/// `PublisherId=2p2nqsd0c76g0`, `SignatureKind=Store`, `Status=Ok`, and
/// `IsDevelopmentMode=False`. The same-day AgentsMirror x64 package moniker,
/// version, and Package Family Name suffix matched.
///
/// This is deliberately an exact Publisher DN, not a PFN suffix, prefix, or
/// mirror field. A Publisher change must fail closed until a human reviews
/// equivalent signed-package and system-trust evidence before updating it.
const OFFICIAL_WINDOWS_CODEX_PUBLISHER: &str = "CN=50BDFD77-8903-4850-9FFE-6E8522F64D5B";

/// Opaque evidence that a Publisher string has passed the production evidence
/// gate. The production constructor remains confined to this module, so
/// release metadata cannot select a different trusted Publisher.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct VerifiedPublisherEvidence {
    publisher: String,
}

impl VerifiedPublisherEvidence {
    pub(crate) fn publisher(&self) -> &str {
        &self.publisher
    }

    #[cfg(test)]
    pub(crate) fn for_test(publisher: &str) -> Self {
        assert!(
            !publisher.is_empty(),
            "test Publisher evidence must be non-empty"
        );
        assert!(
            !publisher.bytes().any(|byte| byte.is_ascii_control()),
            "test Publisher evidence must not contain control characters"
        );
        Self {
            publisher: publisher.to_owned(),
        }
    }
}

impl fmt::Debug for VerifiedPublisherEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedPublisherEvidence(<redacted>)")
    }
}

/// Builds production evidence from the reviewed exact Publisher allowlist.
///
/// PackageManager still validates the MSIX signature and trust chain at
/// deployment. This gate keeps pre-deployment identity validation exact rather
/// than accepting a PFN suffix, a mirror field, or a prefix comparison.
pub(crate) fn current_official_publisher_evidence(
) -> Result<VerifiedPublisherEvidence, InstallerError> {
    Ok(VerifiedPublisherEvidence {
        publisher: OFFICIAL_WINDOWS_CODEX_PUBLISHER.to_owned(),
    })
}

/// Host facts are injected for fake-based tests. The deployment volume is a
/// trusted system root used only for shared free-space preflight; it is never a
/// user-selected install directory.
#[derive(Debug, Clone)]
pub struct WindowsHost {
    architecture: CpuArchitecture,
    os_version: PlatformVersion,
    deployment_volume: PathBuf,
}

impl WindowsHost {
    pub fn new(
        architecture: CpuArchitecture,
        os_version: &str,
        deployment_volume: PathBuf,
    ) -> Result<Self, InstallerError> {
        if deployment_volume.as_os_str().is_empty() {
            return Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("Windows deployment volume could not be determined"));
        }
        let os_version = PlatformVersion::parse_windows_msix(os_version).map_err(|_| {
            InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                .with_diagnostic_message("Windows version could not be parsed")
        })?;
        Ok(Self {
            architecture,
            os_version,
            deployment_volume,
        })
    }

    #[cfg(target_os = "windows")]
    pub fn for_current_host() -> Result<Self, InstallerError> {
        let version = windows_version::OsVersion::current();
        let revision = windows_version::revision();
        let version_text = format!(
            "{}.{}.{}.{}",
            version.major, version.minor, version.build, revision
        );
        Self::new(
            native_host::architecture(),
            &version_text,
            native_host::deployment_volume()?,
        )
    }

    pub(crate) fn architecture(&self) -> CpuArchitecture {
        self.architecture
    }

    pub(crate) fn os_version(&self) -> &PlatformVersion {
        &self.os_version
    }

    pub(crate) fn deployment_volume(&self) -> &Path {
        &self.deployment_volume
    }
}

/// Windows installer adapter with injectable PackageManager facts. The public
/// construction boundary is side-effect-free, so tests never query, deploy,
/// or activate a real system package.
pub(crate) struct WindowsPlatformAdapter {
    package_manager: Arc<dyn WindowsPackageManager>,
    host: WindowsHost,
    publisher_evidence: VerifiedPublisherEvidence,
}

impl WindowsPlatformAdapter {
    pub(crate) fn new(
        package_manager: Arc<dyn WindowsPackageManager>,
        host: WindowsHost,
        publisher_evidence: VerifiedPublisherEvidence,
    ) -> Self {
        Self {
            package_manager,
            host,
            publisher_evidence,
        }
    }

    /// Production factory. Callers must first pass the evidence gate above;
    /// this module deliberately cannot construct one from unverified metadata.
    #[cfg(target_os = "windows")]
    pub(crate) fn for_current_host(
        publisher_evidence: VerifiedPublisherEvidence,
    ) -> Result<Self, InstallerError> {
        Ok(Self::new(
            Arc::new(SystemWindowsPackageManager),
            WindowsHost::for_current_host()?,
            publisher_evidence,
        ))
    }

    fn host_support_error(&self) -> Option<InstallerError> {
        match self.host.architecture() {
            CpuArchitecture::X86_64 | CpuArchitecture::Aarch64 => None,
            architecture => Some(
                InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                    .with_context("architecture", architecture.as_str())
                    .with_diagnostic_message("Windows V1 supports x64 and ARM64 only"),
            ),
        }
    }
}

impl CodexDesktopPlatform for WindowsPlatformAdapter {
    fn platform(&self) -> Option<DesktopPlatform> {
        Some(DesktopPlatform::Windows)
    }

    fn architecture(&self) -> CpuArchitecture {
        self.host.architecture()
    }

    fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
        let package_manager = self.package_manager.clone();
        let host = self.host.clone();
        let publisher_evidence = self.publisher_evidence.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if host.architecture() != CpuArchitecture::X86_64
                && host.architecture() != CpuArchitecture::Aarch64
            {
                return Ok(LocalInstallStatus::Unsupported {
                    reason: UnsupportedReason::Architecture,
                });
            }
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                inspect_local(package_manager.as_ref(), &host, &publisher_evidence)
            })
            .await
        })
    }

    fn inspect_restart_candidates(
        &self,
    ) -> BoxFuture<'_, Result<RestartCandidateInspection, InstallerError>> {
        let package_manager = self.package_manager.clone();
        let host = self.host.clone();
        let publisher_evidence = self.publisher_evidence.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if host.architecture() != CpuArchitecture::X86_64
                && host.architecture() != CpuArchitecture::Aarch64
            {
                return Ok(RestartCandidateInspection::Unsupported(
                    UnsupportedReason::Architecture,
                ));
            }
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                inspect_restart_candidates(package_manager.as_ref(), &host, &publisher_evidence)
            })
            .await
        })
    }

    fn preflight<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        temp_root: &'a Path,
    ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>> {
        let host = self.host.clone();
        let release = release.clone();
        let temp_root = temp_root.to_path_buf();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || preflight(&host, &release, &temp_root)).await
        })
    }

    fn verify_package<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        artifact: &'a DownloadedArtifact,
    ) -> BoxFuture<'a, Result<VerifiedPackage, InstallerError>> {
        let host = self.host.clone();
        let publisher_evidence = self.publisher_evidence.clone();
        let release = release.clone();
        let artifact = artifact.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                artifact.revalidate_against(&release)?;
                validate_package(&host, &publisher_evidence, &release, artifact.path())?;
                VerifiedPackage::from_completed_validation(&release, artifact)
            })
            .await
        })
    }

    fn install_current_user<'a>(
        &'a self,
        package: &'a VerifiedPackage,
        progress: PlatformProgressSink,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let package_manager = self.package_manager.clone();
        let host = self.host.clone();
        let package = package.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                install_current_user(package_manager.as_ref(), &host, &package, progress)
            })
            .await
        })
    }

    fn launch<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let package_manager = self.package_manager.clone();
        let host = self.host.clone();
        let installed = installed.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || launch(package_manager.as_ref(), &host, &installed)).await
        })
    }

    fn inspect_runtime<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<RuntimeInspection, InstallerError>> {
        let installed = installed.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || runtime::inspect(&installed)).await
        })
    }

    fn force_shutdown<'a>(
        &'a self,
        installed: &'a InstalledApplication,
        instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let installed = installed.clone();
        let instances = instances.to_vec();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || runtime::force_shutdown(&installed, &instances)).await
        })
    }

    fn is_runtime_instance_running<'a>(
        &'a self,
        installed: &'a InstalledApplication,
        instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<bool, InstallerError>> {
        let installed = installed.clone();
        let instances = instances.to_vec();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || runtime::is_instance_running(&installed, &instances)).await
        })
    }
}

fn inspect_local(
    package_manager: &dyn WindowsPackageManager,
    host: &WindowsHost,
    publisher_evidence: &VerifiedPublisherEvidence,
) -> Result<LocalInstallStatus, InstallerError> {
    let records = package_manager
        .current_user_packages()
        .map_err(deployment_error)?;
    let stable_records = records
        .iter()
        .filter(|record| record.identity_name == WINDOWS_CODEX_STABLE_IDENTITY)
        .collect::<Vec<_>>();
    if stable_records.is_empty() {
        return Ok(LocalInstallStatus::NotInstalled {
            platform: DesktopPlatform::Windows,
            architecture: host.architecture(),
        });
    }

    let applications = stable_records
        .into_iter()
        .map(|record| installed_application_from_record(record, host, publisher_evidence))
        .collect::<Result<Vec<_>, _>>()?;
    match applications.as_slice() {
        [application] => Ok(LocalInstallStatus::Installed {
            application: application.clone(),
        }),
        _ => Ok(LocalInstallStatus::Ambiguous {
            candidates: applications
                .iter()
                .map(InstalledApplicationSummary::from)
                .collect(),
            error: InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                .with_diagnostic_message(
                    "multiple Stable Windows packages prevent a safe update or launch",
                )
                .to_dto(),
        }),
    }
}

/// Produces every current-user exact PFN-bound installation candidate for the
/// v1.0.2 restart planner. `family_name` is obtained from PackageManager and
/// is validated while forming the verified AUMID; display name, executable
/// name, window title, and package path never participate in candidate
/// discovery or ordering.
fn inspect_restart_candidates(
    package_manager: &dyn WindowsPackageManager,
    host: &WindowsHost,
    publisher_evidence: &VerifiedPublisherEvidence,
) -> Result<RestartCandidateInspection, InstallerError> {
    let records = package_manager
        .current_user_packages()
        .map_err(deployment_error)?;
    let stable_records = records
        .iter()
        .filter(|record| record.identity_name == WINDOWS_CODEX_STABLE_IDENTITY)
        .collect::<Vec<_>>();
    if stable_records.is_empty() {
        return Ok(RestartCandidateInspection::NotInstalled);
    }

    let candidates = stable_records
        .into_iter()
        .map(|record| {
            let application = installed_application_from_record(record, host, publisher_evidence)?;
            Ok(TrustedInstallationCandidate {
                // The Package Family Name is the exact Windows lifecycle
                // identity. It stays private to the planner/token record and
                // never crosses IPC or appears in ordinary diagnostics.
                stable_key: format!("windows-pfn:{}", record.family_name),
                application,
                scope: RestartInstallationScope::CurrentUser,
            })
        })
        .collect::<Result<Vec<_>, InstallerError>>()?;
    Ok(RestartCandidateInspection::Trusted(candidates))
}

fn installed_application_from_record(
    record: &WindowsPackageRecord,
    host: &WindowsHost,
    publisher_evidence: &VerifiedPublisherEvidence,
) -> Result<InstalledApplication, InstallerError> {
    if record.identity_name != WINDOWS_CODEX_STABLE_IDENTITY {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                .with_diagnostic_message("PackageManager record does not have the Stable identity"),
        );
    }
    if record.publisher != publisher_evidence.publisher() {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                .with_diagnostic_message(
                    "PackageManager Publisher does not match verified evidence",
                ),
        );
    }
    if record.architecture != host.architecture() {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageArchitectureMismatch)
                .with_context("architecture", record.architecture.as_str())
                .with_diagnostic_message(
                    "installed Stable package architecture does not match this host",
                ),
        );
    }
    if !matches!(&record.version, PlatformVersion::WindowsMsix { .. }) {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("PackageManager returned a non-Windows package version"));
    }
    let application_id = single_application_id(record)?;
    let aumid = verified_aumid(&record.family_name, application_id)?;
    Ok(InstalledApplication {
        stable_identity: WINDOWS_CODEX_STABLE_IDENTITY.to_owned(),
        display_name: record.display_name.clone(),
        display_version: Some(windows_version_text(&record.version)?),
        platform_version: record.version.clone(),
        architecture: record.architecture,
        location: None,
        launch_target: LaunchTarget::WindowsAumid(aumid),
    })
}

fn preflight(
    host: &WindowsHost,
    release: &ReleaseDescriptor,
    temp_root: &Path,
) -> Result<PlatformInstallPlan, InstallerError> {
    validate_release_for_host(host, release)?;
    if !temp_root.is_dir() {
        return Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("installer temporary root is not an available directory"));
    }
    Ok(PlatformInstallPlan::new(vec![host
        .deployment_volume()
        .to_path_buf()]))
}

fn validate_package(
    host: &WindowsHost,
    publisher_evidence: &VerifiedPublisherEvidence,
    release: &ReleaseDescriptor,
    artifact_path: &Path,
) -> Result<(), InstallerError> {
    validate_release_for_host(host, release)?;
    let manifest = parse_msix_manifest(artifact_path)?;
    validate_manifest_for_release(&manifest, host, publisher_evidence, release)?;
    // Structural ZIP/manifest checks and the exact Publisher evidence gate are
    // complete here. PackageManager performs Windows' actual MSIX signature
    // and chain validation during `AddPackageByUriAsync`; a deployment failure
    // can therefore never become a successful installation result.
    Ok(())
}

/// Repeats the Windows host and MSIX manifest trust gates for the experimental
/// elevated child.  It deliberately returns no `VerifiedPackage`: all-users
/// provisioning is not part of the normal current-user platform trait and
/// cannot be reached through ordinary IPC.
#[cfg(target_os = "windows")]
pub(crate) fn revalidate_all_users_package(
    release: &ReleaseDescriptor,
    artifact_path: &Path,
) -> Result<(), InstallerError> {
    let host = WindowsHost::for_current_host()?;
    let publisher_evidence = current_official_publisher_evidence()?;
    validate_package(&host, &publisher_evidence, release, artifact_path)
}

fn install_current_user(
    package_manager: &dyn WindowsPackageManager,
    host: &WindowsHost,
    package: &VerifiedPackage,
    progress: PlatformProgressSink,
) -> Result<(), InstallerError> {
    if package.platform() != DesktopPlatform::Windows
        || package.architecture() != host.architecture()
    {
        return Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message(
                "non-Windows validation evidence reached the Windows installer",
            ));
    }
    // Re-open the downloader-owned fixed artifact and bind its current bytes
    // to the descriptor retained by `VerifiedPackage` immediately before the
    // `file://` URI is handed to PackageManager.
    package.revalidate_artifact()?;
    let package_file_uri = local_file_uri(package.artifact_path())?;
    progress.report_progress(JobProgress::new(
        ProgressPhase::Installation,
        Some(0),
        Some(100),
    ));
    let progress_for_native = progress.clone();
    let native_reported_completion = Arc::new(AtomicBool::new(false));
    let native_reported_completion_for_sink = native_reported_completion.clone();
    let native_progress: WindowsDeploymentProgressSink = Arc::new(move |percentage| {
        let percentage = percentage.min(100) as u64;
        if percentage == 100 {
            native_reported_completion_for_sink.store(true, Ordering::Release);
        }
        progress_for_native.report_progress(JobProgress::new(
            ProgressPhase::Installation,
            Some(percentage),
            Some(100),
        ));
    });
    package_manager
        .deploy_current_user(&package_file_uri, native_progress)
        .map_err(deployment_error)?;
    if !native_reported_completion.load(Ordering::Acquire) {
        progress.report_progress(JobProgress::new(
            ProgressPhase::Installation,
            Some(100),
            Some(100),
        ));
    }
    Ok(())
}

fn launch(
    package_manager: &dyn WindowsPackageManager,
    host: &WindowsHost,
    installed: &InstalledApplication,
) -> Result<(), InstallerError> {
    if installed.stable_identity != WINDOWS_CODEX_STABLE_IDENTITY
        || installed.architecture != host.architecture()
        || !matches!(
            &installed.platform_version,
            PlatformVersion::WindowsMsix { .. }
        )
    {
        return Err(
            InstallerError::new(InstallerErrorCode::LaunchFailed).with_diagnostic_message(
                "launch request does not contain a verified Stable Windows app",
            ),
        );
    }
    let LaunchTarget::WindowsAumid(aumid) = &installed.launch_target else {
        return Err(InstallerError::new(InstallerErrorCode::LaunchFailed)
            .with_diagnostic_message("launch request does not contain a Windows AUMID"));
    };
    if !is_valid_aumid(aumid) {
        return Err(InstallerError::new(InstallerErrorCode::LaunchFailed)
            .with_diagnostic_message("launch request contains an invalid Windows AUMID"));
    }
    package_manager.launch_aumid(aumid).map_err(launch_error)
}

fn validate_release_for_host(
    host: &WindowsHost,
    release: &ReleaseDescriptor,
) -> Result<(), InstallerError> {
    if release.platform != DesktopPlatform::Windows
        || !matches!(
            &release.platform_version,
            PlatformVersion::WindowsMsix { .. }
        )
    {
        return Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
            .with_diagnostic_message("Windows adapter received a non-Windows release"));
    }
    if !matches!(
        release.architecture,
        CpuArchitecture::X86_64 | CpuArchitecture::Aarch64
    ) || release.architecture != host.architecture()
    {
        return Err(
            InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                .with_context("architecture", release.architecture.as_str())
                .with_diagnostic_message("Windows release architecture does not match this host"),
        );
    }
    if let Some(minimum_os_version) = release.minimum_os_version.as_deref() {
        let minimum_os_version =
            PlatformVersion::parse_windows_msix(minimum_os_version).map_err(|_| {
                InstallerError::new(InstallerErrorCode::ReleaseMetadataInvalid)
                    .with_diagnostic_message("Windows release minimum OS version is invalid")
            })?;
        ensure_host_meets_minimum_os(host, &minimum_os_version)?;
    }
    Ok(())
}

fn validate_manifest_for_release(
    manifest: &WindowsPackageManifest,
    host: &WindowsHost,
    publisher_evidence: &VerifiedPublisherEvidence,
    release: &ReleaseDescriptor,
) -> Result<(), InstallerError> {
    if manifest.identity_name() != WINDOWS_CODEX_STABLE_IDENTITY {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                .with_diagnostic_message(
                    "MSIX Identity Name is not the exact Stable allowlist value",
                ),
        );
    }
    if manifest.publisher() != publisher_evidence.publisher() {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                .with_diagnostic_message(
                    "MSIX Publisher does not match verified official evidence",
                ),
        );
    }
    if manifest.architecture() != release.architecture {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageArchitectureMismatch)
                .with_context("architecture", manifest.architecture().as_str())
                .with_diagnostic_message("MSIX architecture does not match the resolved release"),
        );
    }
    if manifest.version() != &release.platform_version {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                .with_diagnostic_message(
                    "MSIX Identity Version does not match the resolved release",
                ),
        );
    }
    if let Some(release_minimum) = release.minimum_os_version.as_deref() {
        let release_minimum =
            PlatformVersion::parse_windows_msix(release_minimum).map_err(|_| {
                InstallerError::new(InstallerErrorCode::ReleaseMetadataInvalid)
                    .with_diagnostic_message("Windows release minimum OS version is invalid")
            })?;
        if manifest.minimum_os_version() != &release_minimum {
            return Err(
                InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                    .with_diagnostic_message(
                        "MSIX TargetDeviceFamily MinVersion does not match release metadata",
                    ),
            );
        }
    }
    ensure_host_meets_minimum_os(host, manifest.minimum_os_version())
}

fn ensure_host_meets_minimum_os(
    host: &WindowsHost,
    minimum_os_version: &PlatformVersion,
) -> Result<(), InstallerError> {
    if !host.os_version().is_at_least(minimum_os_version)? {
        return Err(
            InstallerError::new(InstallerErrorCode::OsVersionUnsupported).with_diagnostic_message(
                "Windows version does not meet the MSIX minimum requirement",
            ),
        );
    }
    Ok(())
}

fn single_application_id(record: &WindowsPackageRecord) -> Result<&str, InstallerError> {
    let [application_id] = record.application_ids.as_slice() else {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message(
                "installed Stable package does not have exactly one app entry",
            ));
    };
    if !is_valid_application_id(application_id) {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("installed Stable package Application Id is invalid"));
    }
    Ok(application_id)
}

fn verified_aumid(family_name: &str, application_id: &str) -> Result<String, InstallerError> {
    if family_name.is_empty()
        || family_name.len() > 512
        || family_name.contains('!')
        || family_name.bytes().any(|byte| byte.is_ascii_control())
        || !is_valid_application_id(application_id)
    {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("installed Stable package cannot form a verified AUMID"));
    }
    Ok(format!("{family_name}!{application_id}"))
}

fn is_valid_application_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn is_valid_aumid(value: &str) -> bool {
    let Some((family_name, application_id)) = value.split_once('!') else {
        return false;
    };
    !family_name.is_empty()
        && !family_name.contains('!')
        && family_name.len() <= 512
        && !family_name.bytes().any(|byte| byte.is_ascii_control())
        && is_valid_application_id(application_id)
}

fn windows_version_text(version: &PlatformVersion) -> Result<String, InstallerError> {
    let PlatformVersion::WindowsMsix {
        major,
        minor,
        build,
        revision,
    } = version
    else {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("installed package version is not a Windows MSIX version"));
    };
    Ok(format!("{major}.{minor}.{build}.{revision}"))
}

async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, InstallerError> + Send + 'static,
) -> Result<T, InstallerError> {
    tokio::task::spawn_blocking(operation).await.map_err(|_| {
        InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("Windows platform worker stopped unexpectedly")
    })?
}

#[cfg(target_os = "windows")]
mod native_host {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

    use windows::Win32::System::SystemInformation::{
        GetNativeSystemInfo, GetWindowsDirectoryW, PROCESSOR_ARCHITECTURE_AMD64,
        PROCESSOR_ARCHITECTURE_ARM64, SYSTEM_INFO,
    };

    use crate::codex_desktop::{error::InstallerError, types::CpuArchitecture};

    pub(super) fn architecture() -> CpuArchitecture {
        let mut info = SYSTEM_INFO::default();
        unsafe { GetNativeSystemInfo(&mut info) };
        let native_architecture = unsafe { info.Anonymous.Anonymous.wProcessorArchitecture };
        match native_architecture {
            PROCESSOR_ARCHITECTURE_AMD64 => CpuArchitecture::X86_64,
            PROCESSOR_ARCHITECTURE_ARM64 => CpuArchitecture::Aarch64,
            _ => CpuArchitecture::Unsupported,
        }
    }

    pub(super) fn deployment_volume() -> Result<PathBuf, InstallerError> {
        let mut buffer = vec![0_u16; 32_768];
        let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if length == 0 || length >= buffer.len() {
            return Err(InstallerError::new(
                crate::codex_desktop::error::InstallerErrorCode::PlatformUnsupported,
            )
            .with_diagnostic_message("Windows deployment volume could not be determined"));
        }
        let windows_directory = PathBuf::from(OsString::from_wide(&buffer[..length]));
        windows_directory
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| {
                InstallerError::new(
                    crate::codex_desktop::error::InstallerErrorCode::PlatformUnsupported,
                )
                .with_diagnostic_message("Windows deployment volume could not be determined")
            })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::codex_desktop::{
        download::DownloadedArtifact,
        error::InstallerErrorCode,
        temp::JobTempDir,
        types::{PlatformVersion, TrustedDownloadEndpoint},
        verify::{sha256_hex, ArtifactKind},
    };
    use uuid::Uuid;
    use zip::{write::SimpleFileOptions, ZipWriter};

    const PUBLISHER: &str = "CN=fixture publisher";
    const FAMILY_NAME: &str = "OpenAI.Codex_fixture";

    struct FakePackageManager {
        records: Mutex<Vec<WindowsPackageRecord>>,
        deployment_result: Mutex<Result<(), WindowsNativeError>>,
        deployment_progress: Mutex<Vec<u32>>,
        deployed_uris: Mutex<Vec<String>>,
        launched_aumids: Mutex<Vec<String>>,
        launch_result: Mutex<Result<(), WindowsNativeError>>,
    }

    impl FakePackageManager {
        fn with_records(records: Vec<WindowsPackageRecord>) -> Self {
            Self {
                records: Mutex::new(records),
                deployment_result: Mutex::new(Ok(())),
                deployment_progress: Mutex::new(vec![35, 80]),
                deployed_uris: Mutex::new(Vec::new()),
                launched_aumids: Mutex::new(Vec::new()),
                launch_result: Mutex::new(Ok(())),
            }
        }

        fn set_deployment_result(&self, result: Result<(), WindowsNativeError>) {
            *self
                .deployment_result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = result;
        }

        fn set_launch_result(&self, result: Result<(), WindowsNativeError>) {
            *self
                .launch_result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = result;
        }
    }

    impl Default for FakePackageManager {
        fn default() -> Self {
            Self::with_records(Vec::new())
        }
    }

    impl WindowsPackageManager for FakePackageManager {
        fn current_user_packages(&self) -> Result<Vec<WindowsPackageRecord>, WindowsNativeError> {
            Ok(self
                .records
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone())
        }

        fn deploy_current_user(
            &self,
            package_file_uri: &str,
            progress: WindowsDeploymentProgressSink,
        ) -> Result<(), WindowsNativeError> {
            self.deployed_uris
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(package_file_uri.to_owned());
            for value in self
                .deployment_progress
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .copied()
            {
                progress(value);
            }
            *self
                .deployment_result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }

        fn launch_aumid(&self, aumid: &str) -> Result<(), WindowsNativeError> {
            self.launched_aumids
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(aumid.to_owned());
            *self
                .launch_result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }

    fn host(architecture: CpuArchitecture, version: &str) -> WindowsHost {
        WindowsHost::new(architecture, version, PathBuf::from("C:\\")).unwrap()
    }

    fn release(
        architecture: CpuArchitecture,
        minimum_os_version: Option<&str>,
    ) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            architecture,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "OpenAI.Codex_1.2.3.4_fixture.msix",
            "a".repeat(64),
            1024,
            match architecture {
                CpuArchitecture::X86_64 => TrustedDownloadEndpoint::WinX64,
                CpuArchitecture::Aarch64 => TrustedDownloadEndpoint::WinArm64,
                _ => panic!("fixture release architecture must be supported"),
            },
            minimum_os_version.map(str::to_owned),
        )
        .unwrap()
    }

    fn record(
        identity_name: &str,
        publisher: &str,
        architecture: CpuArchitecture,
        application_ids: Vec<&str>,
    ) -> WindowsPackageRecord {
        WindowsPackageRecord::new(
            identity_name,
            publisher,
            FAMILY_NAME,
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            architecture,
            Some("Codex".to_owned()),
            application_ids.into_iter().map(str::to_owned).collect(),
        )
    }

    fn adapter(manager: Arc<dyn WindowsPackageManager>) -> WindowsPlatformAdapter {
        WindowsPlatformAdapter::new(
            manager,
            host(CpuArchitecture::X86_64, "10.0.22631.0"),
            VerifiedPublisherEvidence::for_test(PUBLISHER),
        )
    }

    fn release_for_artifact(bytes: &[u8]) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "OpenAI.Codex_1.2.3.4_fixture.msix",
            sha256_hex(bytes),
            bytes.len() as u64,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .unwrap()
    }

    fn downloaded_artifact_for(
        release: &ReleaseDescriptor,
        bytes: &[u8],
    ) -> (tempfile::TempDir, DownloadedArtifact) {
        let root = tempfile::tempdir().unwrap();
        let directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        fs::write(directory.final_path(ArtifactKind::Msix), bytes).unwrap();
        let artifact = DownloadedArtifact::from_test_file(&directory, release).unwrap();
        (root, artifact)
    }

    fn verified_msix_artifact() -> (tempfile::TempDir, ReleaseDescriptor, DownloadedArtifact) {
        let root = tempfile::tempdir().unwrap();
        let directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let path = directory.final_path(ArtifactKind::Msix);
        let file = File::create(&path).unwrap();
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        archive.start_file("AppxManifest.xml", options).unwrap();
        archive
            .write_all(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/codex_desktop/OpenAI.Codex_26.721.4979.0_x64__2p2nqsd0c76g0.AppxManifest.xml"
            )))
            .unwrap();
        archive.start_file("AppxBlockMap.xml", options).unwrap();
        archive.write_all(b"fixture block map").unwrap();
        archive.start_file("AppxSignature.p7x", options).unwrap();
        archive.write_all(b"fixture signature").unwrap();
        archive.finish().unwrap();

        let bytes = fs::read(&path).unwrap();
        let release = ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "26.721.4979",
            PlatformVersion::parse_windows_msix("26.721.4979.0").unwrap(),
            "OpenAI.Codex_26.721.4979.0_fixture.msix",
            sha256_hex(&bytes),
            bytes.len() as u64,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .unwrap();
        let artifact = DownloadedArtifact::from_test_file(&directory, &release).unwrap();
        (root, release, artifact)
    }

    #[tokio::test]
    async fn current_user_inventory_uses_exact_identity_publisher_architecture_and_aumid() {
        let manager = Arc::new(FakePackageManager::with_records(vec![
            record(
                "OpenAI.CodexBeta",
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["Beta"],
            ),
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["CodexApp"],
            ),
        ]));
        let status = adapter(manager).inspect_local().await.unwrap();
        let LocalInstallStatus::Installed { application } = status else {
            panic!("exact Stable record should be installed")
        };
        assert_eq!(application.stable_identity, WINDOWS_CODEX_STABLE_IDENTITY);
        assert_eq!(application.display_version.as_deref(), Some("1.2.3.4"));
        assert_eq!(
            application.launch_target,
            LaunchTarget::WindowsAumid(format!("{FAMILY_NAME}!CodexApp"))
        );
        assert_eq!(application.location, None);
    }

    #[tokio::test]
    async fn inventory_fails_closed_for_wrong_publisher_architecture_and_multiple_apps() {
        for record in [
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                "CN=untrusted",
                CpuArchitecture::X86_64,
                vec!["CodexApp"],
            ),
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::Aarch64,
                vec!["CodexApp"],
            ),
            record(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
                vec!["One", "Two"],
            ),
        ] {
            let error = adapter(Arc::new(FakePackageManager::with_records(vec![record])))
                .inspect_local()
                .await
                .unwrap_err();
            assert!(matches!(
                error.code(),
                InstallerErrorCode::PackageIdentityMismatch
                    | InstallerErrorCode::PackageArchitectureMismatch
                    | InstallerErrorCode::PackageParseFailed
            ));
        }
    }

    #[tokio::test]
    async fn preflight_rejects_architecture_and_minimum_os_without_a_native_call() {
        let adapter = adapter(Arc::new(FakePackageManager::default()));
        let temporary = tempfile::tempdir().unwrap();
        let plan = adapter
            .preflight(&release(CpuArchitecture::X86_64, None), temporary.path())
            .await
            .unwrap();
        assert_eq!(plan.additional_disk_paths(), &[PathBuf::from("C:\\")]);

        let architecture_error = adapter
            .preflight(&release(CpuArchitecture::Aarch64, None), temporary.path())
            .await
            .unwrap_err();
        assert_eq!(
            architecture_error.code(),
            InstallerErrorCode::ArchitectureUnsupported
        );

        let minimum_os_error = adapter
            .preflight(
                &release(CpuArchitecture::X86_64, Some("10.0.65535.0")),
                temporary.path(),
            )
            .await
            .unwrap_err();
        assert_eq!(
            minimum_os_error.code(),
            InstallerErrorCode::OsVersionUnsupported
        );
    }

    #[test]
    fn manifest_release_gate_requires_exact_stable_identity_publisher_architecture_and_versions() {
        let host = host(CpuArchitecture::X86_64, "10.0.22631.0");
        let publisher_evidence = VerifiedPublisherEvidence::for_test(PUBLISHER);
        let descriptor = release(CpuArchitecture::X86_64, Some("10.0.19041.0"));
        let valid = manifest::manifest_for_test(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PUBLISHER,
            CpuArchitecture::X86_64,
            "1.2.3.4",
            "10.0.19041.0",
            "CodexApp",
        );
        validate_manifest_for_release(&valid, &host, &publisher_evidence, &descriptor).unwrap();

        for (manifest, expected) in [
            (
                manifest::manifest_for_test(
                    "OpenAI.CodexBeta",
                    PUBLISHER,
                    CpuArchitecture::X86_64,
                    "1.2.3.4",
                    "10.0.19041.0",
                    "CodexApp",
                ),
                InstallerErrorCode::PackageIdentityMismatch,
            ),
            (
                manifest::manifest_for_test(
                    WINDOWS_CODEX_STABLE_IDENTITY,
                    "CN=untrusted",
                    CpuArchitecture::X86_64,
                    "1.2.3.4",
                    "10.0.19041.0",
                    "CodexApp",
                ),
                InstallerErrorCode::PackageIdentityMismatch,
            ),
            (
                manifest::manifest_for_test(
                    WINDOWS_CODEX_STABLE_IDENTITY,
                    PUBLISHER,
                    CpuArchitecture::Aarch64,
                    "1.2.3.4",
                    "10.0.19041.0",
                    "CodexApp",
                ),
                InstallerErrorCode::PackageArchitectureMismatch,
            ),
            (
                manifest::manifest_for_test(
                    WINDOWS_CODEX_STABLE_IDENTITY,
                    PUBLISHER,
                    CpuArchitecture::X86_64,
                    "1.2.3.5",
                    "10.0.19041.0",
                    "CodexApp",
                ),
                InstallerErrorCode::PackageIdentityMismatch,
            ),
            (
                manifest::manifest_for_test(
                    WINDOWS_CODEX_STABLE_IDENTITY,
                    PUBLISHER,
                    CpuArchitecture::X86_64,
                    "1.2.3.4",
                    "10.0.19042.0",
                    "CodexApp",
                ),
                InstallerErrorCode::PackageIdentityMismatch,
            ),
        ] {
            let error =
                validate_manifest_for_release(&manifest, &host, &publisher_evidence, &descriptor)
                    .unwrap_err();
            assert_eq!(error.code(), expected);
        }

        let host_os_error = validate_manifest_for_release(
            &manifest::manifest_for_test(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
                "1.2.3.4",
                "10.0.65535.0",
                "CodexApp",
            ),
            &host,
            &publisher_evidence,
            &release(CpuArchitecture::X86_64, None),
        )
        .unwrap_err();
        assert_eq!(
            host_os_error.code(),
            InstallerErrorCode::OsVersionUnsupported
        );
    }

    #[tokio::test]
    async fn fake_current_user_deployment_reports_progress_uses_file_uri_and_maps_failures() {
        let manager = Arc::new(FakePackageManager::default());
        let adapter = adapter(manager.clone());
        let trusted_bytes = b"fixture";
        let release = release_for_artifact(trusted_bytes);
        let (_root, artifact) = downloaded_artifact_for(&release, trusted_bytes);
        let package = VerifiedPackage::from_completed_validation(&release, artifact).unwrap();
        let reported = Arc::new(Mutex::new(Vec::<u64>::new()));
        let reported_for_sink = reported.clone();
        let progress: PlatformProgressSink = Arc::new(move |progress: JobProgress| {
            reported_for_sink
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(progress.completed_bytes.unwrap());
        });
        adapter
            .install_current_user(&package, progress)
            .await
            .unwrap();
        assert_eq!(
            *reported
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![0, 35, 80, 100]
        );
        let deployed = manager
            .deployed_uris
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert_eq!(deployed.len(), 1);
        assert!(deployed[0].starts_with("file:///"));
        assert!(!deployed[0].starts_with("https://"));

        for (hresult, expected) in [
            (
                0x8007_3D02_u32 as i32,
                InstallerErrorCode::WindowsPackageInUse,
            ),
            (
                0x8007_3D01_u32 as i32,
                InstallerErrorCode::WindowsDeploymentBlocked,
            ),
            (
                0x8007_3CF3_u32 as i32,
                InstallerErrorCode::WindowsDependencyMissing,
            ),
            (
                0x800B_0100_u32 as i32,
                InstallerErrorCode::PackageSignatureInvalid,
            ),
            (
                0x8123_4567_u32 as i32,
                InstallerErrorCode::WindowsDeploymentFailed,
            ),
        ] {
            manager.set_deployment_result(Err(WindowsNativeError::from_hresult(hresult)));
            let error = adapter
                .install_current_user(&package, Arc::new(|_| {}))
                .await
                .unwrap_err();
            assert_eq!(error.code(), expected);
        }
    }

    #[tokio::test]
    async fn replacement_after_platform_verification_never_reaches_current_user_deployment() {
        let manager = Arc::new(FakePackageManager::default());
        let adapter = WindowsPlatformAdapter::new(
            manager.clone(),
            host(CpuArchitecture::X86_64, "10.0.22631.0"),
            VerifiedPublisherEvidence::for_test(OFFICIAL_WINDOWS_CODEX_PUBLISHER),
        );
        let (_root, release, artifact) = verified_msix_artifact();
        let package = adapter.verify_package(&release, &artifact).await.unwrap();
        let mut replacement = fs::read(package.artifact_path()).unwrap();
        replacement[0] ^= 0x01;
        fs::write(package.artifact_path(), replacement).unwrap();

        let error = adapter
            .install_current_user(&package, Arc::new(|_| {}))
            .await
            .expect_err("a post-verification replacement must not reach PackageManager");

        assert_eq!(error.code(), InstallerErrorCode::ChecksumMismatch);
        assert!(manager
            .deployed_uris
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty());
    }

    #[tokio::test]
    async fn launch_accepts_only_verified_aumid_and_preserves_a_stable_error() {
        let manager = Arc::new(FakePackageManager::default());
        let adapter = adapter(manager.clone());
        let installed = InstalledApplication {
            stable_identity: WINDOWS_CODEX_STABLE_IDENTITY.to_owned(),
            display_name: Some("Codex".to_owned()),
            display_version: Some("1.2.3.4".to_owned()),
            platform_version: PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            architecture: CpuArchitecture::X86_64,
            location: None,
            launch_target: LaunchTarget::WindowsAumid(format!("{FAMILY_NAME}!CodexApp")),
        };
        adapter.launch(&installed).await.unwrap();
        assert_eq!(
            *manager
                .launched_aumids
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![format!("{FAMILY_NAME}!CodexApp")]
        );

        manager.set_launch_result(Err(WindowsNativeError::from_hresult(
            0x8000_4005_u32 as i32,
        )));
        let error = adapter.launch(&installed).await.unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::LaunchFailed);

        let invalid = InstalledApplication {
            launch_target: LaunchTarget::WindowsAumid("not-an-aumid".to_owned()),
            ..installed
        };
        let error = adapter.launch(&invalid).await.unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::LaunchFailed);
    }

    #[test]
    fn production_publisher_evidence_returns_the_audited_exact_publisher() {
        let evidence = current_official_publisher_evidence()
            .expect("audited production Publisher evidence should be available");
        assert_eq!(
            evidence.publisher(),
            "CN=50BDFD77-8903-4850-9FFE-6E8522F64D5B"
        );
    }

    #[test]
    fn production_publisher_evidence_matches_the_reviewed_identity_fixture() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/codex_desktop/OpenAI.Codex_26.721.4979.0_x64__2p2nqsd0c76g0.AppxManifest.xml"
        ));
        let evidence = current_official_publisher_evidence()
            .expect("audited production Publisher evidence should be available");

        assert!(fixture.contains(r#"Name="OpenAI.Codex""#));
        assert!(fixture.contains(r#"Version="26.721.4979.0""#));
        assert!(fixture.contains(r#"ProcessorArchitecture="x64""#));
        assert!(fixture.contains(r#"MinVersion="10.0.19041.0""#));
        assert!(fixture.contains(r#"Id="App""#));
        assert!(fixture.contains(&format!(r#"Publisher="{}""#, evidence.publisher())));
    }
}
