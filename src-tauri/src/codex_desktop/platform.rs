//! Object-safe boundary for platform-specific installation adapters.
//!
//! This module deliberately contains no Windows, macOS, or command-runner
//! implementation.  Platform adapters receive a release descriptor while
//! preparing/verifying and receive only a validated package when installing.
//! That prevents a raw downloaded path from reaching an installer entry point.

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::future::BoxFuture;

use super::{
    download::DownloadedArtifact,
    error::{InstallerError, InstallerErrorCode},
    types::{
        CpuArchitecture, DesktopPlatform, InstalledApplication, JobProgress, LocalInstallStatus,
        ReleaseDescriptor,
    },
};

#[cfg(any(not(any(target_os = "windows", target_os = "macos")), test))]
use super::types::UnsupportedReason;

// The command/filesystem boundary is target-neutral, so test builds include it
// on every host and can exercise the adapter with fakes. Runtime construction
// remains macOS-only in the platform factory.
#[cfg(any(target_os = "macos", test))]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

/// Exact stable package identity for the Windows Codex application.
///
/// This is a product allowlist, not metadata: the release source must never
/// be able to select a different local package identity.
pub(crate) const WINDOWS_CODEX_STABLE_IDENTITY: &str = "OpenAI.Codex";

/// Exact stable bundle identifier for the macOS Codex application.
///
/// See [`WINDOWS_CODEX_STABLE_IDENTITY`] for why this remains a local
/// allowlist rather than release metadata.
pub(crate) const MACOS_CODEX_STABLE_IDENTITY: &str = "com.openai.codex";

/// Reports one normalized platform progress update.  The caller owns mapping
/// the update to the current job snapshot and must not expose raw platform
/// messages through this boundary.
pub trait PlatformProgressReporter: Send + Sync {
    fn report_progress(&self, progress: JobProgress);
}

impl<F> PlatformProgressReporter for F
where
    F: Fn(JobProgress) + Send + Sync,
{
    fn report_progress(&self, progress: JobProgress) {
        self(progress);
    }
}

/// An owned, cloneable progress reporter suitable for platform async work.
pub type PlatformProgressSink = Arc<dyn PlatformProgressReporter>;

/// Opaque, platform-bound runtime evidence. It is deliberately crate-private
/// and non-serializable so IPC callers cannot select a process, bundle, path,
/// AUMID, or package family for a shutdown operation.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum TrustedRuntimeInstance {
    /// The one visible top-level window process for the verified Windows
    /// package family. Renderer/helper processes are deliberately excluded;
    /// more than one top-level window is reported as ambiguous instead of
    /// being collapsed into a process-name-style group.
    #[cfg_attr(
        not(any(target_os = "windows", test)),
        expect(
            dead_code,
            reason = "the Windows adapter constructs this evidence only on Windows"
        )
    )]
    Windows {
        package_family_name: String,
        process_id: u32,
        /// The process creation timestamp prevents a recycled numeric PID from
        /// being mistaken for the runtime that passed the initial identity
        /// check.
        creation_time: u64,
    },
    /// macOS represents an app instance by an NSRunningApplication PID plus
    /// the canonical bundle path verified against the installed bundle.
    #[cfg_attr(
        not(any(target_os = "macos", test)),
        expect(
            dead_code,
            reason = "the macOS adapter constructs this evidence only on macOS"
        )
    )]
    Macos {
        process_id: i32,
        bundle_path: PathBuf,
        reported_bundle_path: PathBuf,
        /// Like the Windows creation timestamp, this distinguishes a newly
        /// launched app that reused a PID from the instance approved for the
        /// current restart operation.
        launch_timestamp_ms: u64,
    },
}

impl fmt::Debug for TrustedRuntimeInstance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows { .. } => formatter
                .debug_struct("TrustedRuntimeInstance::Windows")
                .finish(),
            Self::Macos { .. } => formatter
                .debug_struct("TrustedRuntimeInstance::Macos")
                .finish(),
        }
    }
}

/// Runtime detection uses only [`TrustedRuntimeInstance`] evidence. A platform
/// may report ambiguity instead of trying to disambiguate with an executable
/// or display-name heuristic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeInspection {
    NotRunning,
    Running(Vec<TrustedRuntimeInstance>),
    Ambiguous,
}

/// Platform-specific preflight output consumed by the service's shared disk
/// check.  Paths are crate-private and intentionally omitted from `Debug` so
/// they cannot leak a user's home directory through diagnostics.
#[derive(Clone, Default)]
pub struct PlatformInstallPlan {
    additional_disk_paths: Vec<PathBuf>,
}

impl PlatformInstallPlan {
    /// Creates a plan containing target-volume paths in addition to the
    /// downloader's temporary directory.  Platform implementations should add
    /// only stable target roots required for free-space preflight.
    #[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
    pub(crate) fn new(additional_disk_paths: Vec<PathBuf>) -> Self {
        Self {
            additional_disk_paths,
        }
    }

    pub(crate) fn additional_disk_paths(&self) -> &[PathBuf] {
        &self.additional_disk_paths
    }
}

impl fmt::Debug for PlatformInstallPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlatformInstallPlan")
            .field(
                "additional_disk_path_count",
                &self.additional_disk_paths.len(),
            )
            .finish()
    }
}

/// Evidence that a platform adapter has completed its package validation.
///
/// The package path is deliberately private and this value is neither
/// serializable nor publicly constructible. Only platform validation code in
/// this module or one of its child adapters may create it through
/// `from_completed_validation`; installers then receive a reference without
/// accepting a caller-controlled path.
#[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
#[derive(Clone)]
pub struct VerifiedPackage {
    artifact_path: PathBuf,
    locked_release: ReleaseDescriptor,
    // Production evidence always contains the downloader capability. The
    // absent state exists only for trait-object unit fakes, which never reach
    // a native parser, mount, or deployment boundary.
    artifact: Option<DownloadedArtifact>,
}

#[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
impl VerifiedPackage {
    fn from_completed_validation(
        release: &ReleaseDescriptor,
        artifact: DownloadedArtifact,
    ) -> Result<Self, InstallerError> {
        // A platform parser may take time to complete. Repeat the descriptor
        // binding while converting its successful result into opaque evidence,
        // so a replacement just before this boundary cannot become installable.
        artifact.revalidate_against(release)?;
        let artifact_path = artifact.path().to_path_buf();

        Ok(Self {
            artifact_path,
            locked_release: release.clone(),
            artifact: Some(artifact),
        })
    }

    fn artifact_path(&self) -> &Path {
        &self.artifact_path
    }

    fn platform(&self) -> DesktopPlatform {
        self.locked_release.platform
    }

    fn architecture(&self) -> CpuArchitecture {
        self.locked_release.architecture
    }

    /// Repeats the controlled-artifact regular-file, exact-size, and SHA-256
    /// checks against the descriptor that was locked when platform validation
    /// succeeded. Installers call this immediately before they form a file URI
    /// or hand the artifact to a package consumer.
    pub(crate) fn revalidate_artifact(&self) -> Result<(), InstallerError> {
        match self.artifact.as_ref() {
            Some(artifact) => artifact.revalidate_against(&self.locked_release),
            #[cfg(test)]
            None => Ok(()),
            #[cfg(not(test))]
            None => Err(InstallerError::new(InstallerErrorCode::InternalError)
                .with_diagnostic_message(
                    "validated package is missing artifact integrity evidence",
                )),
        }
    }

    pub(crate) fn locked_release(&self) -> &ReleaseDescriptor {
        &self.locked_release
    }

    pub(crate) fn belongs_to(&self, release: &ReleaseDescriptor) -> bool {
        let locked_release = self.locked_release();
        locked_release.release_id == release.release_id
            && locked_release.platform == release.platform
            && locked_release.architecture == release.architecture
            && locked_release.platform_version == release.platform_version
            && locked_release.expected_size == release.expected_size
            && locked_release.expected_sha256 == release.expected_sha256
    }

    /// Creates opaque validation evidence for service unit tests. Production
    /// callers cannot construct a package: platform validation remains the
    /// only non-test path that can bind a downloaded artifact to a release.
    #[cfg(test)]
    pub(crate) fn for_test(release: &ReleaseDescriptor) -> Self {
        Self::for_test_at(release, PathBuf::from("verified-test-package"))
    }

    #[cfg(test)]
    pub(crate) fn for_test_at(release: &ReleaseDescriptor, artifact_path: PathBuf) -> Self {
        Self {
            artifact_path,
            locked_release: release.clone(),
            artifact: None,
        }
    }
}

impl fmt::Debug for VerifiedPackage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedPackage")
            .field("release_id", &self.locked_release.release_id)
            .field("platform", &self.locked_release.platform)
            .field("architecture", &self.locked_release.architecture)
            .field("artifact_path", &"<validated-package>")
            .finish()
    }
}

/// Confirms that a re-detected installation satisfies the release which was
/// actually installed. Identity is selected only from the trusted platform,
/// never from remote metadata or a caller-provided field.
pub(crate) fn installed_application_matches_release(
    application: &InstalledApplication,
    release: &ReleaseDescriptor,
) -> Result<bool, InstallerError> {
    let expected_identity = stable_identity_for(release.platform);
    if application.stable_identity != expected_identity
        || application.architecture != release.architecture
        || installed_platform(&application.platform_version) != release.platform
    {
        return Ok(false);
    }

    application
        .platform_version
        .is_at_least(&release.platform_version)
}

const fn stable_identity_for(platform: DesktopPlatform) -> &'static str {
    match platform {
        DesktopPlatform::Windows => WINDOWS_CODEX_STABLE_IDENTITY,
        DesktopPlatform::Macos => MACOS_CODEX_STABLE_IDENTITY,
    }
}

const fn installed_platform(version: &super::types::PlatformVersion) -> DesktopPlatform {
    match version {
        super::types::PlatformVersion::WindowsMsix { .. } => DesktopPlatform::Windows,
        super::types::PlatformVersion::MacBundle { .. } => DesktopPlatform::Macos,
    }
}

/// Platform interface used by the common service.
///
/// `BoxFuture` keeps this trait object-safe without adding `async-trait`.
/// `platform` is optional because Linux and other unsupported hosts cannot be
/// misrepresented by the V1 Windows/macOS enum.
pub(crate) trait CodexDesktopPlatform: Send + Sync {
    fn platform(&self) -> Option<DesktopPlatform>;

    fn architecture(&self) -> CpuArchitecture;

    fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>>;

    fn preflight<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        temp_root: &'a Path,
    ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>>;

    /// Performs platform-specific identity, architecture, and signature
    /// validation over a core-owned artifact whose hash and size have already
    /// been verified. No platform trait operation accepts a raw path.
    fn verify_package<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        artifact: &'a DownloadedArtifact,
    ) -> BoxFuture<'a, Result<VerifiedPackage, InstallerError>>;

    /// Installs only a package whose validation evidence is represented by
    /// `VerifiedPackage`; a raw path is intentionally absent from this API.
    fn install_current_user<'a>(
        &'a self,
        package: &'a VerifiedPackage,
        progress: PlatformProgressSink,
    ) -> BoxFuture<'a, Result<(), InstallerError>>;

    fn launch<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<(), InstallerError>>;

    /// Detect runtime instances that are bound to `installed`'s already
    /// verified identity. The default fails closed so a platform adapter cannot
    /// accidentally gain restart control merely by implementing installer
    /// discovery and launch.
    fn inspect_runtime<'a>(
        &'a self,
        _installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<RuntimeInspection, InstallerError>> {
        Box::pin(async {
            Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("trusted runtime inspection is unavailable"))
        })
    }

    /// Ask only the previously verified runtime instance(s) to close normally.
    /// The default makes forceful lifecycle behavior opt-in per platform.
    fn request_graceful_shutdown<'a>(
        &'a self,
        _installed: &'a InstalledApplication,
        _instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        Box::pin(async {
            Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("trusted graceful shutdown is unavailable"))
        })
    }

    /// Force only the previously verified runtime instance(s) after a separate
    /// user confirmation. Implementations must revalidate identity immediately
    /// before terminating anything.
    fn force_shutdown<'a>(
        &'a self,
        _installed: &'a InstalledApplication,
        _instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        Box::pin(async {
            Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("trusted force shutdown is unavailable"))
        })
    }

    /// Check whether the exact runtime evidence captured before a graceful or
    /// force request is still alive. Unlike general runtime discovery, this is
    /// allowed to observe a just-closed primary process while package helper
    /// processes finish their own shutdown; it must never select a replacement
    /// PID or a different instance.
    fn is_runtime_instance_running<'a>(
        &'a self,
        _installed: &'a InstalledApplication,
        _instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<bool, InstallerError>> {
        Box::pin(async {
            Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("trusted runtime liveness inspection is unavailable"))
        })
    }
}

/// A fail-closed adapter for hosts V1 cannot install on.
///
/// Supported-host production construction never needs this type. Unit tests
/// still compile it so the unsupported result remains covered on every host.
#[cfg(any(not(any(target_os = "windows", target_os = "macos")), test))]
#[derive(Debug, Clone)]
pub(crate) struct UnsupportedPlatformAdapter {
    platform: Option<DesktopPlatform>,
    architecture: CpuArchitecture,
    reason: UnsupportedReason,
    error_code: InstallerErrorCode,
}

#[cfg(any(not(any(target_os = "windows", target_os = "macos")), test))]
impl UnsupportedPlatformAdapter {
    pub(crate) fn platform_unsupported(architecture: CpuArchitecture) -> Self {
        Self {
            platform: None,
            architecture,
            reason: UnsupportedReason::Platform,
            error_code: InstallerErrorCode::PlatformUnsupported,
        }
    }

    pub(crate) fn architecture_unsupported(
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
    ) -> Self {
        Self {
            platform: Some(platform),
            architecture,
            reason: UnsupportedReason::Architecture,
            error_code: InstallerErrorCode::ArchitectureUnsupported,
        }
    }

    fn unsupported_error(&self) -> InstallerError {
        InstallerError::new(self.error_code)
            .with_context("architecture", self.architecture.as_str())
            .with_diagnostic_message("the current host is unsupported by the desktop installer")
    }
}

#[cfg(any(not(any(target_os = "windows", target_os = "macos")), test))]
impl CodexDesktopPlatform for UnsupportedPlatformAdapter {
    fn platform(&self) -> Option<DesktopPlatform> {
        self.platform
    }

    fn architecture(&self) -> CpuArchitecture {
        self.architecture
    }

    fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
        let reason = self.reason.clone();
        Box::pin(async move { Ok(LocalInstallStatus::Unsupported { reason }) })
    }

    fn preflight<'a>(
        &'a self,
        _release: &'a ReleaseDescriptor,
        _temp_root: &'a Path,
    ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }

    fn verify_package<'a>(
        &'a self,
        _release: &'a ReleaseDescriptor,
        _artifact: &'a DownloadedArtifact,
    ) -> BoxFuture<'a, Result<VerifiedPackage, InstallerError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }

    fn install_current_user<'a>(
        &'a self,
        _package: &'a VerifiedPackage,
        _progress: PlatformProgressSink,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }

    fn launch<'a>(
        &'a self,
        _installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        Box::pin(async move { Err(self.unsupported_error()) })
    }
}

/// A fail-closed adapter used only when the host-specific production adapter
/// cannot be constructed without weakening an installation trust boundary.
///
/// Keeping startup alive lets the renderer receive the structured reason and
/// open the existing log directory, while every local inspection, download
/// preflight, install, and launch operation still fails with that same stable
/// error. It must never be used as a successful fallback for a supported host.
#[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
#[derive(Debug, Clone)]
pub(crate) struct UnavailablePlatformAdapter {
    platform: DesktopPlatform,
    architecture: CpuArchitecture,
    error: InstallerError,
}

#[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
impl UnavailablePlatformAdapter {
    pub(crate) fn new(
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        error: InstallerError,
    ) -> Self {
        Self {
            platform,
            architecture,
            error,
        }
    }

    fn unavailable_error(&self) -> InstallerError {
        self.error.clone()
    }
}

impl CodexDesktopPlatform for UnavailablePlatformAdapter {
    fn platform(&self) -> Option<DesktopPlatform> {
        Some(self.platform)
    }

    fn architecture(&self) -> CpuArchitecture {
        self.architecture
    }

    fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
        let error = self.unavailable_error();
        Box::pin(async move { Err(error) })
    }

    fn preflight<'a>(
        &'a self,
        _release: &'a ReleaseDescriptor,
        _temp_root: &'a Path,
    ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>> {
        let error = self.unavailable_error();
        Box::pin(async move { Err(error) })
    }

    fn verify_package<'a>(
        &'a self,
        _release: &'a ReleaseDescriptor,
        _artifact: &'a DownloadedArtifact,
    ) -> BoxFuture<'a, Result<VerifiedPackage, InstallerError>> {
        let error = self.unavailable_error();
        Box::pin(async move { Err(error) })
    }

    fn install_current_user<'a>(
        &'a self,
        _package: &'a VerifiedPackage,
        _progress: PlatformProgressSink,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let error = self.unavailable_error();
        Box::pin(async move { Err(error) })
    }

    fn launch<'a>(
        &'a self,
        _installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let error = self.unavailable_error();
        Box::pin(async move { Err(error) })
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, sync::Arc};

    use super::*;
    use crate::codex_desktop::{
        download::DownloadedArtifact,
        error::InstallerErrorCode,
        temp::JobTempDir,
        types::{PlatformVersion, TrustedDownloadEndpoint},
        verify::{sha256_hex, ArtifactKind},
    };
    use uuid::Uuid;

    fn release() -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "OpenAI.Codex_1.2.3.4_x64__fixture.Msix",
            "a".repeat(64),
            1024,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .unwrap()
    }

    fn macos_release() -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            "26.721.41059",
            PlatformVersion::parse_mac_bundle("5848").unwrap(),
            "Codex-mac-arm64.dmg",
            "b".repeat(64),
            1024,
            TrustedDownloadEndpoint::MacArm64,
            None,
        )
        .unwrap()
    }

    fn release_for_artifact(bytes: &[u8]) -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "OpenAI.Codex_1.2.3.4_x64__fixture.Msix",
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

    fn installed_application(
        stable_identity: &str,
        platform_version: PlatformVersion,
        architecture: CpuArchitecture,
    ) -> InstalledApplication {
        let launch_target = match &platform_version {
            PlatformVersion::WindowsMsix { .. } => {
                super::super::types::LaunchTarget::WindowsAumid("fixture.app".to_owned())
            }
            PlatformVersion::MacBundle { .. } => super::super::types::LaunchTarget::MacBundlePath(
                PathBuf::from("/Applications/Codex.app"),
            ),
        };

        InstalledApplication {
            stable_identity: stable_identity.to_owned(),
            display_name: Some("Codex".to_owned()),
            display_version: None,
            platform_version,
            architecture,
            location: Some("/redacted".to_owned()),
            launch_target,
        }
    }

    #[test]
    fn verified_package_is_private_path_evidence_tied_to_one_release() {
        let trusted_bytes = b"trusted package bytes";
        let release = release_for_artifact(trusted_bytes);
        let (_root, artifact) = downloaded_artifact_for(&release, trusted_bytes);
        let artifact_path = artifact.path().to_path_buf();
        let package = VerifiedPackage::from_completed_validation(&release, artifact).unwrap();

        assert!(package.belongs_to(&release));
        assert_eq!(package.artifact_path(), artifact_path);
        let debug = format!("{package:?}");
        assert!(debug.contains("<validated-package>"));
        assert!(!debug.contains(artifact_path.to_string_lossy().as_ref()));

        fs::write(&artifact_path, b"mutated package bytes").unwrap();
        let error = package
            .revalidate_artifact()
            .expect_err("replaced evidence must not stay installable");
        assert_eq!(error.code(), InstallerErrorCode::ChecksumMismatch);
    }

    #[test]
    fn install_plan_debug_redacts_local_paths() {
        let plan = PlatformInstallPlan::new(vec![PathBuf::from("C:\\Users\\alice\\Apps")]);
        assert_eq!(plan.additional_disk_paths().len(), 1);
        let debug = format!("{plan:?}");
        assert!(debug.contains("additional_disk_path_count"));
        assert!(!debug.contains("alice"));
    }

    #[test]
    fn installed_application_matcher_uses_platform_specific_identity_and_version_floor() {
        let windows = release();
        let matching_windows = installed_application(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PlatformVersion::parse_windows_msix("1.2.3.5").unwrap(),
            CpuArchitecture::X86_64,
        );
        assert!(installed_application_matches_release(&matching_windows, &windows).unwrap());

        let wrong_windows_identity = installed_application(
            "OpenAI.CodexBeta",
            PlatformVersion::parse_windows_msix("1.2.3.5").unwrap(),
            CpuArchitecture::X86_64,
        );
        assert!(!installed_application_matches_release(&wrong_windows_identity, &windows).unwrap());

        let older_windows = installed_application(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PlatformVersion::parse_windows_msix("1.2.3.3").unwrap(),
            CpuArchitecture::X86_64,
        );
        assert!(!installed_application_matches_release(&older_windows, &windows).unwrap());

        let wrong_windows_architecture = installed_application(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PlatformVersion::parse_windows_msix("1.2.3.5").unwrap(),
            CpuArchitecture::Aarch64,
        );
        assert!(
            !installed_application_matches_release(&wrong_windows_architecture, &windows).unwrap()
        );

        let macos = macos_release();
        let matching_macos = installed_application(
            MACOS_CODEX_STABLE_IDENTITY,
            PlatformVersion::parse_mac_bundle("5848").unwrap(),
            CpuArchitecture::Aarch64,
        );
        assert!(installed_application_matches_release(&matching_macos, &macos).unwrap());

        let windows_identity_on_macos = installed_application(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PlatformVersion::parse_mac_bundle("5848").unwrap(),
            CpuArchitecture::Aarch64,
        );
        assert!(
            !installed_application_matches_release(&windows_identity_on_macos, &macos).unwrap()
        );
    }

    #[test]
    fn installed_application_matcher_rejects_platform_shape_and_reports_bad_installed_version() {
        let windows = release();
        let macos_shape = installed_application(
            WINDOWS_CODEX_STABLE_IDENTITY,
            PlatformVersion::parse_mac_bundle("5848").unwrap(),
            CpuArchitecture::X86_64,
        );
        assert!(!installed_application_matches_release(&macos_shape, &windows).unwrap());

        let macos = macos_release();
        let invalid_macos_version = installed_application(
            MACOS_CODEX_STABLE_IDENTITY,
            PlatformVersion::MacBundle {
                bundle_version: "not-a-version".to_owned(),
            },
            CpuArchitecture::Aarch64,
        );
        let error = installed_application_matches_release(&invalid_macos_version, &macos)
            .expect_err("an uncomparable installed version must not be accepted");
        assert_eq!(error.code(), InstallerErrorCode::InstallationVerifyFailed);
    }

    #[test]
    fn unsupported_adapter_returns_structured_platform_and_architecture_failures() {
        let linux = UnsupportedPlatformAdapter::platform_unsupported(CpuArchitecture::X86_64);
        assert_eq!(linux.platform(), None);
        assert_eq!(linux.architecture(), CpuArchitecture::X86_64);
        assert!(matches!(
            futures::executor::block_on(linux.inspect_local()).unwrap(),
            LocalInstallStatus::Unsupported {
                reason: UnsupportedReason::Platform
            }
        ));
        let error = futures::executor::block_on(linux.preflight(&release(), Path::new("temp")))
            .expect_err("unsupported hosts never preflight successfully");
        assert_eq!(error.code(), InstallerErrorCode::PlatformUnsupported);

        let intel_macos = UnsupportedPlatformAdapter::architecture_unsupported(
            DesktopPlatform::Macos,
            CpuArchitecture::X86_64UnsupportedMac,
        );
        assert_eq!(intel_macos.platform(), Some(DesktopPlatform::Macos));
        assert!(matches!(
            futures::executor::block_on(intel_macos.inspect_local()).unwrap(),
            LocalInstallStatus::Unsupported {
                reason: UnsupportedReason::Architecture
            }
        ));
        let error =
            futures::executor::block_on(intel_macos.preflight(&release(), Path::new("temp")))
                .expect_err("unsupported architectures never preflight successfully");
        assert_eq!(error.code(), InstallerErrorCode::ArchitectureUnsupported);
    }

    struct FakePlatform;

    impl CodexDesktopPlatform for FakePlatform {
        fn platform(&self) -> Option<DesktopPlatform> {
            Some(DesktopPlatform::Windows)
        }

        fn architecture(&self) -> CpuArchitecture {
            CpuArchitecture::X86_64
        }

        fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
            Box::pin(async {
                Ok(LocalInstallStatus::NotInstalled {
                    platform: DesktopPlatform::Windows,
                    architecture: CpuArchitecture::X86_64,
                })
            })
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
            _artifact: &'a DownloadedArtifact,
        ) -> BoxFuture<'a, Result<VerifiedPackage, InstallerError>> {
            Box::pin(async move { Ok(VerifiedPackage::for_test(release)) })
        }

        fn install_current_user<'a>(
            &'a self,
            package: &'a VerifiedPackage,
            progress: PlatformProgressSink,
        ) -> BoxFuture<'a, Result<(), InstallerError>> {
            Box::pin(async move {
                assert_eq!(package.platform(), DesktopPlatform::Windows);
                progress.report_progress(JobProgress::new(
                    super::super::types::ProgressPhase::Installation,
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
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn fake_adapter_confirms_the_trait_is_object_safe_and_package_only_at_install() {
        let adapter: Arc<dyn CodexDesktopPlatform> = Arc::new(FakePlatform);
        let release = release();
        let package = VerifiedPackage::for_test(&release);
        let reports = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink_reports = reports.clone();
        let sink: PlatformProgressSink = Arc::new(move |progress| {
            sink_reports.lock().unwrap().push(progress);
        });

        futures::executor::block_on(adapter.install_current_user(&package, sink)).unwrap();
        assert_eq!(reports.lock().unwrap().len(), 1);
    }
}
