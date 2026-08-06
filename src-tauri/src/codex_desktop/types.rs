//! Domain DTOs and value objects for the Codex desktop installer.
//!
//! This module owns the backend contract. In particular, a release descriptor
//! never contains a manifest-provided URL and the only public install input is
//! the release identifier observed by the UI.

use std::{cmp::Ordering, path::PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::error::{InstallerError, InstallerErrorCode, InstallerErrorDto};

const RELEASE_ID_SCHEMA: &str = "fyagent-codex-release-v1";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DesktopPlatform {
    Windows,
    Macos,
}

impl DesktopPlatform {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CpuArchitecture {
    X86_64,
    Aarch64,
    X86_64UnsupportedMac,
    Unsupported,
}

impl CpuArchitecture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
            Self::X86_64UnsupportedMac => "x86_64_unsupported_mac",
            Self::Unsupported => "unsupported",
        }
    }
}

/// The only URLs that the installer can request. Manifest data may describe
/// upstream URLs and delta artifacts, but neither can be represented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrustedDownloadEndpoint {
    Manifest,
    Checksums,
    WinX64,
    WinArm64,
    MacArm64,
}

impl TrustedDownloadEndpoint {
    pub(crate) const fn url(self) -> &'static str {
        match self {
            Self::Manifest => "https://codexapp.agentsmirror.com/latest/manifest",
            Self::Checksums => "https://codexapp.agentsmirror.com/latest/checksums",
            Self::WinX64 => "https://codexapp.agentsmirror.com/latest/win-x64",
            Self::WinArm64 => "https://codexapp.agentsmirror.com/latest/win-arm64",
            Self::MacArm64 => "https://codexapp.agentsmirror.com/latest/mac-arm64",
        }
    }

    pub(crate) const fn kind(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Checksums => "checksums",
            Self::WinX64 => "win-x64",
            Self::WinArm64 => "win-arm64",
            Self::MacArm64 => "mac-arm64",
        }
    }

    pub(crate) const fn is_artifact(self) -> bool {
        matches!(self, Self::WinX64 | Self::WinArm64 | Self::MacArm64)
    }

    pub(crate) const fn matches_release(
        self,
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
    ) -> bool {
        matches!(
            (self, platform, architecture),
            (
                Self::WinX64,
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64
            ) | (
                Self::WinArm64,
                DesktopPlatform::Windows,
                CpuArchitecture::Aarch64
            ) | (
                Self::MacArm64,
                DesktopPlatform::Macos,
                CpuArchitecture::Aarch64
            )
        )
    }

    pub(crate) const fn expected_extension(self) -> Option<&'static str> {
        match self {
            Self::WinX64 | Self::WinArm64 => Some("msix"),
            Self::MacArm64 => Some("dmg"),
            Self::Manifest | Self::Checksums => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlatformVersion {
    WindowsMsix {
        major: u16,
        minor: u16,
        build: u16,
        revision: u16,
    },
    MacBundle {
        #[serde(rename = "bundleVersion")]
        bundle_version: String,
    },
}

impl PlatformVersion {
    pub fn parse_windows_msix(value: &str) -> Result<Self, InstallerError> {
        let parts = value.split('.').collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(invalid_metadata(
                "Windows MSIX version must have four components",
            ));
        }

        let values = parts
            .into_iter()
            .map(|part| {
                if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(invalid_metadata(
                        "Windows MSIX version contains a non-numeric component",
                    ));
                }
                part.parse::<u16>()
                    .map_err(|_| invalid_metadata("Windows MSIX version component is out of range"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self::WindowsMsix {
            major: values[0],
            minor: values[1],
            build: values[2],
            revision: values[3],
        })
    }

    pub fn parse_mac_bundle(value: impl Into<String>) -> Result<Self, InstallerError> {
        let components = parse_mac_bundle_components(&value.into()).map_err(|_| {
            invalid_metadata("macOS CFBundleVersion must contain dot-separated unsigned integers")
        })?;
        let bundle_version = format!("{}.{}.{}", components[0], components[1], components[2]);
        Ok(Self::MacBundle { bundle_version })
    }

    pub fn canonical(&self) -> String {
        match self {
            Self::WindowsMsix {
                major,
                minor,
                build,
                revision,
            } => format!("windows_msix:{major}.{minor}.{build}.{revision}"),
            Self::MacBundle { bundle_version } => format!("mac_bundle:{bundle_version}"),
        }
    }

    pub fn compare(&self, other: &Self) -> Result<Ordering, InstallerError> {
        match (self, other) {
            (
                Self::WindowsMsix {
                    major: left_major,
                    minor: left_minor,
                    build: left_build,
                    revision: left_revision,
                },
                Self::WindowsMsix {
                    major: right_major,
                    minor: right_minor,
                    build: right_build,
                    revision: right_revision,
                },
            ) => Ok(
                (*left_major, *left_minor, *left_build, *left_revision).cmp(&(
                    *right_major,
                    *right_minor,
                    *right_build,
                    *right_revision,
                )),
            ),
            (
                Self::MacBundle {
                    bundle_version: left,
                },
                Self::MacBundle {
                    bundle_version: right,
                },
            ) => {
                let left = parse_mac_bundle_components(left).map_err(|_| {
                    InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                        .with_diagnostic_message("installed macOS bundle version is not comparable")
                })?;
                let right = parse_mac_bundle_components(right).map_err(|_| {
                    InstallerError::new(InstallerErrorCode::ReleaseMetadataInvalid)
                        .with_diagnostic_message("target macOS bundle version is not comparable")
                })?;
                Ok(left.cmp(&right))
            }
            _ => Err(
                InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
                    .with_diagnostic_message("cannot compare versions from different platforms"),
            ),
        }
    }

    pub fn is_at_least(&self, target: &Self) -> Result<bool, InstallerError> {
        Ok(self.compare(target)? != Ordering::Less)
    }
}

fn parse_mac_bundle_components(value: &str) -> Result<[u64; 3], ()> {
    if value.is_empty() || value.len() > 128 {
        return Err(());
    }

    // CFBundleVersion compares major/minor/patch numerically: absent values
    // are zero, leading zeroes are not significant, and later numeric fields
    // do not change the system version. Keeping this canonical form also
    // prevents semantically equivalent manifests from changing release_id.
    let mut normalized = [0_u64; 3];
    for (index, part) in value.split('.').enumerate() {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(());
        }
        let component = part.parse::<u64>().map_err(|_| ())?;
        if index < normalized.len() {
            normalized[index] = component;
        }
    }

    Ok(normalized)
}

#[derive(Debug, Clone)]
pub struct ReleaseDescriptor {
    pub(crate) release_id: String,
    pub(crate) platform: DesktopPlatform,
    pub(crate) architecture: CpuArchitecture,
    pub(crate) display_version: String,
    pub(crate) platform_version: PlatformVersion,
    pub(crate) expected_sha256: String,
    pub(crate) expected_size: u64,
    pub(crate) download_endpoint: TrustedDownloadEndpoint,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) minimum_os_version: Option<String>,
}

impl ReleaseDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        display_version: impl Into<String>,
        platform_version: PlatformVersion,
        artifact_file_name: impl Into<String>,
        expected_sha256: impl AsRef<str>,
        expected_size: u64,
        download_endpoint: TrustedDownloadEndpoint,
        minimum_os_version: Option<String>,
    ) -> Result<Self, InstallerError> {
        let display_version = display_version.into();
        let artifact_file_name = artifact_file_name.into();
        let expected_sha256 = normalize_sha256(expected_sha256.as_ref())?;

        if display_version.trim().is_empty() || display_version.len() > 128 {
            return Err(invalid_metadata("release display version is invalid"));
        }
        if expected_size == 0 {
            return Err(invalid_metadata("release artifact size must be positive"));
        }
        if !download_endpoint.is_artifact()
            || !download_endpoint.matches_release(platform, architecture)
        {
            return Err(invalid_metadata(
                "release endpoint does not match platform and architecture",
            ));
        }
        if !platform_version_matches(platform, &platform_version) {
            return Err(invalid_metadata(
                "release platform version kind does not match platform",
            ));
        }
        validate_artifact_file_name(&artifact_file_name)?;
        validate_release_filename(&artifact_file_name, download_endpoint)?;

        let minimum_os_version = match minimum_os_version {
            Some(value) if value.trim().is_empty() || value.len() > 128 => {
                return Err(invalid_metadata("minimum OS version is invalid"));
            }
            Some(value) => Some(value),
            None => None,
        };

        let release_id = compute_release_id(
            platform,
            architecture,
            &artifact_file_name,
            &platform_version,
            &expected_sha256,
            expected_size,
        );

        Ok(Self {
            release_id,
            platform,
            architecture,
            display_version,
            platform_version,
            expected_sha256,
            expected_size,
            download_endpoint,
            minimum_os_version,
        })
    }

    pub fn release_id(&self) -> &str {
        &self.release_id
    }

    pub fn remote_status(&self, checked_at: impl Into<String>) -> RemoteReleaseStatus {
        RemoteReleaseStatus {
            release_id: self.release_id.clone(),
            display_version: self.display_version.clone(),
            platform_version: self.platform_version.clone(),
            expected_size: self.expected_size,
            checked_at: checked_at.into(),
        }
    }
}

fn platform_version_matches(platform: DesktopPlatform, version: &PlatformVersion) -> bool {
    matches!(
        (platform, version),
        (
            DesktopPlatform::Windows,
            PlatformVersion::WindowsMsix { .. }
        ) | (DesktopPlatform::Macos, PlatformVersion::MacBundle { .. })
    )
}

fn validate_release_filename(
    file_name: &str,
    endpoint: TrustedDownloadEndpoint,
) -> Result<(), InstallerError> {
    let expected_extension = endpoint
        .expected_extension()
        .ok_or_else(|| invalid_metadata("release endpoint is not an artifact endpoint"))?;
    let extension_matches = file_name
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case(expected_extension));
    if !extension_matches {
        return Err(invalid_metadata(
            "release artifact extension does not match endpoint",
        ));
    }
    Ok(())
}

/// Normalize the one digest representation shared by source, downloader and
/// verification code. Uppercase hex is accepted as an input compatibility
/// detail but never retained in a descriptor or release ID.
pub(crate) fn normalize_sha256(value: &str) -> Result<String, InstallerError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_metadata(
            "SHA-256 must be exactly 64 hexadecimal characters",
        ));
    }
    Ok(value.to_ascii_lowercase())
}

/// Reject names before any caller can use them as a filesystem component.
pub(crate) fn validate_artifact_file_name(value: &str) -> Result<(), InstallerError> {
    if value.is_empty()
        || value.len() > 255
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
        || value.contains("..")
    {
        return Err(invalid_metadata("artifact filename is unsafe"));
    }
    Ok(())
}

fn compute_release_id(
    platform: DesktopPlatform,
    architecture: CpuArchitecture,
    artifact_file_name: &str,
    platform_version: &PlatformVersion,
    expected_sha256: &str,
    expected_size: u64,
) -> String {
    let canonical_payload = format!(
        "schema={RELEASE_ID_SCHEMA}\nsource=agentsmirror\nplatform={}\narchitecture={}\nartifact_file_name={artifact_file_name}\nplatform_version={}\nexpected_sha256={expected_sha256}\nexpected_size={expected_size}\n",
        platform.as_str(),
        architecture.as_str(),
        platform_version.canonical(),
    );
    let digest = Sha256::digest(canonical_payload.as_bytes());
    format!("v1:{digest:x}")
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteReleaseStatus {
    pub release_id: String,
    pub display_version: String,
    pub platform_version: PlatformVersion,
    pub expected_size: u64,
    pub checked_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartInstallRequest {
    pub expected_release_id: String,
}

impl StartInstallRequest {
    pub fn validate(&self) -> Result<(), InstallerError> {
        let release_id = self.expected_release_id.as_str();
        if release_id.len() != 67
            || !release_id.starts_with("v1:")
            || !release_id[3..]
                .bytes()
                .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
        {
            return Err(InstallerError::new(InstallerErrorCode::MetadataChanged)
                .with_diagnostic_message("expected release ID is invalid"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApplication {
    pub stable_identity: String,
    pub display_name: Option<String>,
    pub display_version: Option<String>,
    pub platform_version: PlatformVersion,
    pub architecture: CpuArchitecture,
    #[serde(skip_serializing)]
    pub(crate) location: Option<String>,
    #[serde(skip_serializing)]
    pub(crate) launch_target: LaunchTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LaunchTarget {
    #[cfg_attr(
        not(any(target_os = "windows", test)),
        expect(
            dead_code,
            reason = "the Windows adapter constructs this target only on Windows"
        )
    )]
    WindowsAumid(String),
    #[cfg_attr(
        not(any(target_os = "macos", test)),
        expect(
            dead_code,
            reason = "the macOS adapter constructs this target only on macOS"
        )
    )]
    MacBundlePath(PathBuf),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApplicationSummary {
    pub stable_identity: String,
    pub display_version: Option<String>,
    pub platform_version: PlatformVersion,
    pub architecture: CpuArchitecture,
}

impl From<&InstalledApplication> for InstalledApplicationSummary {
    fn from(value: &InstalledApplication) -> Self {
        Self {
            stable_identity: value.stable_identity.clone(),
            display_version: value.display_version.clone(),
            platform_version: value.platform_version.clone(),
            architecture: value.architecture,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedReason {
    Platform,
    Architecture,
    OsVersion,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum LocalInstallStatus {
    NotInstalled {
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
    },
    Installed {
        application: InstalledApplication,
    },
    Unsupported {
        reason: UnsupportedReason,
    },
    Ambiguous {
        candidates: Vec<InstalledApplicationSummary>,
        error: InstallerErrorDto,
    },
}

/// Privacy-safe runtime state used only by the Codex restart coordinator.
/// The backend never serializes PIDs, executable paths, bundle paths, AUMIDs,
/// or package family names: those stay inside the trusted platform boundary.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CodexDesktopRuntimeStatus {
    NotInstalled,
    NotRunning,
    Running,
    Ambiguous {
        reason: CodexDesktopRuntimeAmbiguity,
    },
    Unsupported {
        reason: UnsupportedReason,
    },
    /// The host exposed a display-level installation record but did not
    /// provide the exact PFN / Bundle ID evidence required for lifecycle
    /// control.  This is intentionally distinct from an unsupported host:
    /// callers may still ask the user to restart manually, but must not select
    /// a process or launch target from a name, title, or path fallback.
    UntrustedTarget,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodexDesktopRuntimeAmbiguity {
    Installations,
    Instances,
    IdentityVerification,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodexDesktopRestartPromptReason {
    /// The standard single-installation/single-instance destructive action.
    /// It is separate from the three ambiguity reasons so the renderer never
    /// has to claim that multiple applications were found when that is false.
    UniqueRuntime,
    MultipleInstances,
    MultipleInstallations,
    IdentityBindingAmbiguous,
}

/// A lifecycle capability is unavailable without exposing local process or
/// installation details. The renderer can only offer a manual restart path.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodexDesktopManualRestartReason {
    UntrustedTarget,
    Unsupported,
}

/// Result of a capability-scoped Codex Desktop restart. Opaque capabilities
/// stay server-side and never encode a PID, path, package family, bundle ID,
/// version, candidate count, or failure phase. The only destructive branch is
/// entered after `ConfirmationRequired`; it always force-closes exact trusted
/// instances and never performs a graceful-close / second-confirmation flow.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CodexDesktopRestartOutcome {
    Restarted,
    ConfirmationRequired {
        token: String,
        reason: CodexDesktopRestartPromptReason,
    },
    NotRunning,
    ManualRestartRequired {
        reason: CodexDesktopManualRestartReason,
    },
    Incomplete {
        #[serde(rename = "retryToken")]
        retry_token: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallerWarningCode {
    TempCleanupFailed,
    MacDmgDetachWarning,
    LogWriteFailed,
    EventEmitFailed,
    RemoteCheckFailedLocalAvailable,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub installed: InstalledApplicationSummary,
    pub warnings: Vec<InstallerWarningCode>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStage {
    Checking,
    Preflight,
    Downloading,
    VerifyingDownload,
    Installing,
    VerifyingInstallation,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobStage {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    pub const fn is_cancellable(self) -> bool {
        matches!(
            self,
            Self::Checking | Self::Preflight | Self::Downloading | Self::VerifyingDownload
        )
    }

    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Checking,
                Self::Preflight | Self::Failed | Self::Cancelled
            ) | (
                Self::Preflight,
                Self::Downloading | Self::Failed | Self::Cancelled
            ) | (
                Self::Downloading,
                Self::Downloading | Self::VerifyingDownload | Self::Failed | Self::Cancelled
            ) | (
                Self::VerifyingDownload,
                Self::Installing | Self::Failed | Self::Cancelled
            ) | (Self::Installing, Self::VerifyingInstallation | Self::Failed)
                | (Self::VerifyingInstallation, Self::Succeeded | Self::Failed)
        )
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPhase {
    Download,
    Verification,
    Installation,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub phase: ProgressPhase,
    pub completed_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub percent: Option<f32>,
}

impl JobProgress {
    pub fn new(
        phase: ProgressPhase,
        completed_bytes: Option<u64>,
        total_bytes: Option<u64>,
    ) -> Self {
        let percent = match (completed_bytes, total_bytes) {
            (Some(completed), Some(total)) if total > 0 => {
                Some(((completed as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as f32)
            }
            _ => None,
        };
        Self {
            phase,
            completed_bytes,
            total_bytes,
            percent,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub job_id: String,
    pub sequence: u64,
    pub stage: JobStage,
    pub release: RemoteReleaseStatus,
    pub started_at: String,
    pub updated_at: String,
    pub progress: Option<JobProgress>,
    pub cancellable: bool,
    pub result: Option<InstallResult>,
    pub error: Option<InstallerErrorDto>,
}

impl JobSnapshot {
    pub(crate) fn checking(
        job_id: impl Into<String>,
        release: RemoteReleaseStatus,
        timestamp: impl Into<String>,
    ) -> Self {
        let timestamp = timestamp.into();
        Self {
            job_id: job_id.into(),
            sequence: 0,
            stage: JobStage::Checking,
            release,
            started_at: timestamp.clone(),
            updated_at: timestamp,
            progress: None,
            cancellable: true,
            result: None,
            error: None,
        }
    }
}

fn invalid_metadata(message: &str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::ReleaseMetadataInvalid).with_diagnostic_message(message)
}

#[cfg(test)]
mod tests {
    use super::super::error::SuggestedAction;
    use super::*;

    const DTO_CONTRACT_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/codexDesktopDtoContract.v1.json"
    ));

    fn fixture_release() -> ReleaseDescriptor {
        ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "OpenAI.Codex_1.2.3.4_x64__fixture.Msix",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            1_048_576,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .unwrap()
    }

    fn dto_contract_windows_application() -> InstalledApplication {
        InstalledApplication {
            stable_identity: "OpenAI.Codex".to_owned(),
            display_name: Some("ChatGPT".to_owned()),
            display_version: Some("26.721.4979".to_owned()),
            platform_version: PlatformVersion::WindowsMsix {
                major: 26,
                minor: 721,
                build: 4979,
                revision: 0,
            },
            architecture: CpuArchitecture::X86_64,
            location: Some(r"C:\Users\fixture\AppData\Local\Packages".to_owned()),
            launch_target: LaunchTarget::WindowsAumid("OpenAI.Codex_fixture!App".to_owned()),
        }
    }

    fn dto_contract_macos_summary() -> InstalledApplicationSummary {
        InstalledApplicationSummary {
            stable_identity: "com.openai.codex".to_owned(),
            display_version: Some("26.721.41059".to_owned()),
            platform_version: PlatformVersion::MacBundle {
                bundle_version: "5848.0.0".to_owned(),
            },
            architecture: CpuArchitecture::Aarch64,
        }
    }

    fn dto_contract_error() -> InstallerErrorDto {
        InstallerError::new(InstallerErrorCode::DownloadFailed)
            .with_stage(JobStage::Downloading)
            .with_endpoint_kind("mac-arm64")
            .with_attempt(2, 3)
            .with_http_status(503)
            .with_platform_error_code("0x80004005")
            .with_diagnostic_message("download interrupted")
            .with_context("source", "agentsmirror")
            .to_dto()
    }

    fn dto_contract_fixture_value() -> serde_json::Value {
        let release_id = format!("v1:{}", "a".repeat(64));
        let windows_application = dto_contract_windows_application();
        let windows_summary = InstalledApplicationSummary::from(&windows_application);
        let macos_summary = dto_contract_macos_summary();
        let error = dto_contract_error();
        let install_result = InstallResult {
            installed: macos_summary.clone(),
            warnings: vec![
                InstallerWarningCode::TempCleanupFailed,
                InstallerWarningCode::MacDmgDetachWarning,
                InstallerWarningCode::LogWriteFailed,
                InstallerWarningCode::EventEmitFailed,
                InstallerWarningCode::RemoteCheckFailedLocalAvailable,
            ],
        };
        let remote_release = RemoteReleaseStatus {
            release_id: release_id.clone(),
            display_version: "26.721.4979".to_owned(),
            platform_version: PlatformVersion::WindowsMsix {
                major: 26,
                minor: 721,
                build: 4979,
                revision: 0,
            },
            expected_size: 1_048_576,
            checked_at: "2026-07-29T00:00:00Z".to_owned(),
        };
        let local_statuses = vec![
            LocalInstallStatus::NotInstalled {
                platform: DesktopPlatform::Windows,
                architecture: CpuArchitecture::X86_64,
            },
            LocalInstallStatus::Installed {
                application: windows_application,
            },
            LocalInstallStatus::Unsupported {
                reason: UnsupportedReason::Platform,
            },
            LocalInstallStatus::Unsupported {
                reason: UnsupportedReason::Architecture,
            },
            LocalInstallStatus::Unsupported {
                reason: UnsupportedReason::OsVersion,
            },
            LocalInstallStatus::Ambiguous {
                candidates: vec![windows_summary, macos_summary.clone()],
                error: error.clone(),
            },
        ];
        let job_snapshot = JobSnapshot {
            job_id: "contract-job-001".to_owned(),
            sequence: 7,
            stage: JobStage::Failed,
            release: remote_release,
            started_at: "2026-07-29T00:00:00Z".to_owned(),
            updated_at: "2026-07-29T00:00:03Z".to_owned(),
            progress: Some(JobProgress {
                phase: ProgressPhase::Download,
                completed_bytes: Some(524_288),
                total_bytes: Some(1_048_576),
                percent: Some(50.0),
            }),
            cancellable: false,
            result: None,
            error: Some(error),
        };

        serde_json::json!({
            "contractVersion": 1,
            "desktopPlatforms": [DesktopPlatform::Windows, DesktopPlatform::Macos],
            "cpuArchitectures": [
                CpuArchitecture::X86_64,
                CpuArchitecture::Aarch64,
                CpuArchitecture::X86_64UnsupportedMac,
                CpuArchitecture::Unsupported,
            ],
            "platformVersions": [
                PlatformVersion::WindowsMsix {
                    major: 26,
                    minor: 721,
                    build: 4979,
                    revision: 0,
                },
                PlatformVersion::MacBundle {
                    bundle_version: "5848.0.0".to_owned(),
                },
            ],
            "unsupportedReasons": [
                UnsupportedReason::Platform,
                UnsupportedReason::Architecture,
                UnsupportedReason::OsVersion,
            ],
            "localInstallStatuses": local_statuses,
            "installerWarningCodes": [
                InstallerWarningCode::TempCleanupFailed,
                InstallerWarningCode::MacDmgDetachWarning,
                InstallerWarningCode::LogWriteFailed,
                InstallerWarningCode::EventEmitFailed,
                InstallerWarningCode::RemoteCheckFailedLocalAvailable,
            ],
            "jobStages": [
                JobStage::Checking,
                JobStage::Preflight,
                JobStage::Downloading,
                JobStage::VerifyingDownload,
                JobStage::Installing,
                JobStage::VerifyingInstallation,
                JobStage::Succeeded,
                JobStage::Failed,
                JobStage::Cancelled,
            ],
            "progressPhases": [
                ProgressPhase::Download,
                ProgressPhase::Verification,
                ProgressPhase::Installation,
            ],
            "installerErrorCodes": [
                InstallerErrorCode::PlatformUnsupported,
                InstallerErrorCode::OsVersionUnsupported,
                InstallerErrorCode::ArchitectureUnsupported,
                InstallerErrorCode::SourceUnavailable,
                InstallerErrorCode::ReleaseMetadataInvalid,
                InstallerErrorCode::ReleaseNotAvailable,
                InstallerErrorCode::MetadataChanged,
                InstallerErrorCode::RedirectRejected,
                InstallerErrorCode::DownloadFailed,
                InstallerErrorCode::DownloadTimeout,
                InstallerErrorCode::DownloadCancelled,
                InstallerErrorCode::InsufficientDiskSpace,
                InstallerErrorCode::ChecksumMissing,
                InstallerErrorCode::ChecksumMismatch,
                InstallerErrorCode::PackageParseFailed,
                InstallerErrorCode::PackageIdentityMismatch,
                InstallerErrorCode::PackageArchitectureMismatch,
                InstallerErrorCode::PackageSignatureInvalid,
                InstallerErrorCode::WindowsUacCancelled,
                InstallerErrorCode::WindowsElevationFailed,
                InstallerErrorCode::WindowsPackageInUse,
                InstallerErrorCode::WindowsDeploymentBlocked,
                InstallerErrorCode::WindowsDependencyMissing,
                InstallerErrorCode::WindowsAllUsersUnsupported,
                InstallerErrorCode::WindowsDeploymentFailed,
                InstallerErrorCode::MacDmgMountFailed,
                InstallerErrorCode::MacAppNotFound,
                InstallerErrorCode::MacBundleIdMismatch,
                InstallerErrorCode::MacTeamIdMismatch,
                InstallerErrorCode::MacGatekeeperRejected,
                InstallerErrorCode::MacAppRunning,
                InstallerErrorCode::MacMultipleInstallations,
                InstallerErrorCode::MacTargetPathConflict,
                InstallerErrorCode::MacCopyFailed,
                InstallerErrorCode::MacDmgDetachFailed,
                InstallerErrorCode::InstallationVerifyFailed,
                InstallerErrorCode::LaunchFailed,
                InstallerErrorCode::JobAlreadyRunning,
                InstallerErrorCode::JobNotFound,
                InstallerErrorCode::InternalError,
            ],
            "suggestedActions": [
                SuggestedAction::Retry,
                SuggestedAction::Refresh,
                SuggestedAction::CloseTargetAppAndRetry,
                SuggestedAction::ContactAdministrator,
                SuggestedAction::FreeDiskSpace,
                SuggestedAction::ResolvePathConflict,
                SuggestedAction::OpenLogs,
                SuggestedAction::None,
            ],
            "startInstallRequest": StartInstallRequest {
                expected_release_id: release_id,
            },
            "installResult": install_result,
            "jobSnapshot": job_snapshot,
        })
    }

    #[test]
    fn parses_and_compares_platform_versions_deterministically() {
        let windows = PlatformVersion::parse_windows_msix("1.2.3.4").unwrap();
        assert!(windows
            .is_at_least(&PlatformVersion::parse_windows_msix("1.2.3.3").unwrap())
            .unwrap());
        let windows_current = PlatformVersion::parse_windows_msix("26.721.4979.0").unwrap();
        let windows_newer = PlatformVersion::parse_windows_msix("26.721.41059.0").unwrap();
        assert_eq!(
            windows_current.compare(&windows_newer).unwrap(),
            Ordering::Less
        );
        assert!(PlatformVersion::parse_windows_msix("1.2.3").is_err());
        assert!(PlatformVersion::parse_windows_msix("1.2.3.65536").is_err());

        let mac_1_2 = PlatformVersion::parse_mac_bundle("1.2").unwrap();
        let mac_1_2_0 = PlatformVersion::parse_mac_bundle("1.2.0").unwrap();
        let mac_1_10 = PlatformVersion::parse_mac_bundle("1.10").unwrap();
        assert_eq!(mac_1_2.compare(&mac_1_2_0).unwrap(), Ordering::Equal);
        assert_eq!(mac_1_10.compare(&mac_1_2).unwrap(), Ordering::Greater);
        assert_eq!(
            PlatformVersion::parse_mac_bundle("01.02.003.99").unwrap(),
            PlatformVersion::parse_mac_bundle("1.2.3").unwrap()
        );
        assert_eq!(
            PlatformVersion::parse_mac_bundle("1.2.3.99")
                .unwrap()
                .canonical(),
            "mac_bundle:1.2.3"
        );
        assert!(PlatformVersion::parse_mac_bundle("1..2").is_err());
    }

    #[test]
    fn release_id_uses_the_frozen_canonical_payload() {
        let release = fixture_release();
        assert_eq!(
            release.release_id(),
            "v1:589a541b82290acd430f8bd74562c2cb608fb293b53eef7397d2a358d67c6c65"
        );

        let changed = ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "OpenAI.Codex_1.2.3.4_x64__fixture.Msix",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            1_048_576,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .unwrap();
        assert_ne!(release.release_id(), changed.release_id());
    }

    #[test]
    fn descriptor_rejects_url_like_filename_and_mismatched_endpoint() {
        let error = ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "../evil.msix",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            1,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::ReleaseMetadataInvalid);

        assert!(ReleaseDescriptor::new(
            DesktopPlatform::Windows,
            CpuArchitecture::Aarch64,
            "1.2.3.4",
            PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            "OpenAI.Codex_1.2.3.4_x64__fixture.Msix",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            1,
            TrustedDownloadEndpoint::WinX64,
            None,
        )
        .is_err());
    }

    #[test]
    fn mac_descriptor_accepts_a_safe_metadata_derived_dmg_name() {
        let release = ReleaseDescriptor::new(
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            "26.721.41059",
            PlatformVersion::parse_mac_bundle("5848").unwrap(),
            "OpenAI-Codex-26.721.41059-arm64.dmg",
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            593_861_752,
            TrustedDownloadEndpoint::MacArm64,
            None,
        )
        .expect("a safe DMG name remains bound to the fixed macOS endpoint");

        assert_eq!(release.download_endpoint, TrustedDownloadEndpoint::MacArm64);
    }

    #[test]
    fn start_request_is_the_only_serialized_input_and_rejects_unknown_fields() {
        let request = StartInstallRequest {
            expected_release_id: fixture_release().release_id.clone(),
        };
        request.validate().unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({ "expectedReleaseId": request.expected_release_id })
        );
        assert!(
            serde_json::from_value::<StartInstallRequest>(serde_json::json!({
                "expectedReleaseId": request.expected_release_id,
                "url": "https://attacker.example.test/package"
            }))
            .is_err()
        );
        let lower_case_release_id = fixture_release().release_id().to_string();
        assert!(StartInstallRequest {
            expected_release_id: format!("v1:{}", lower_case_release_id[3..].to_ascii_uppercase()),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn rust_dtos_match_the_shared_renderer_contract_fixture() {
        let expected: serde_json::Value = serde_json::from_str(DTO_CONTRACT_FIXTURE)
            .expect("contract fixture must be valid JSON");

        assert_eq!(dto_contract_fixture_value(), expected);
    }

    #[test]
    fn installed_application_does_not_serialize_a_local_path() {
        let application = InstalledApplication {
            stable_identity: "OpenAI.Codex".to_string(),
            display_name: Some("Codex".to_string()),
            display_version: Some("1.2.3".to_string()),
            platform_version: PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
            architecture: CpuArchitecture::X86_64,
            location: Some(r"C:\Users\alice\AppData\Local\Codex".to_string()),
            launch_target: LaunchTarget::WindowsAumid("OpenAI.Codex_123!App".to_string()),
        };

        let serialized = serde_json::to_value(application).unwrap();
        assert!(serialized.get("location").is_none());
        assert!(!serialized.to_string().contains("alice"));
    }

    #[test]
    fn job_transitions_and_progress_are_bounded() {
        assert!(JobStage::Checking.can_transition_to(JobStage::Preflight));
        assert!(!JobStage::Checking.can_transition_to(JobStage::Installing));
        assert!(!JobStage::Installing.is_cancellable());
        assert_eq!(
            JobProgress::new(ProgressPhase::Download, Some(150), Some(100)).percent,
            Some(100.0)
        );
        assert_eq!(
            JobProgress::new(ProgressPhase::Download, Some(10), None).percent,
            None
        );
    }
}
