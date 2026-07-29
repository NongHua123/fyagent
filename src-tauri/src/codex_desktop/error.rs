//! Stable, privacy-safe installer errors.
//!
//! The renderer must branch on `code`, never on an operating-system message.
//! Raw network, file-system and deployment text is retained only after this
//! module has removed credentials, query strings and user-specific paths.

use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use super::types::JobStage;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstallerErrorCode {
    PlatformUnsupported,
    OsVersionUnsupported,
    ArchitectureUnsupported,
    SourceUnavailable,
    ReleaseMetadataInvalid,
    ReleaseNotAvailable,
    MetadataChanged,
    RedirectRejected,
    DownloadFailed,
    DownloadTimeout,
    DownloadCancelled,
    InsufficientDiskSpace,
    ChecksumMissing,
    ChecksumMismatch,
    PackageParseFailed,
    PackageIdentityMismatch,
    PackageArchitectureMismatch,
    PackageSignatureInvalid,
    WindowsUacCancelled,
    WindowsElevationFailed,
    WindowsPackageInUse,
    WindowsDeploymentBlocked,
    WindowsDependencyMissing,
    WindowsAllUsersUnsupported,
    WindowsDeploymentFailed,
    MacDmgMountFailed,
    MacAppNotFound,
    MacBundleIdMismatch,
    MacTeamIdMismatch,
    MacGatekeeperRejected,
    MacAppRunning,
    MacMultipleInstallations,
    MacTargetPathConflict,
    MacCopyFailed,
    MacDmgDetachFailed,
    InstallationVerifyFailed,
    LaunchFailed,
    JobAlreadyRunning,
    JobNotFound,
    InternalError,
}

impl InstallerErrorCode {
    pub const fn message_key(self) -> &'static str {
        match self {
            Self::PlatformUnsupported => "codexDesktop.error.platformUnsupported",
            Self::OsVersionUnsupported => "codexDesktop.error.osVersionUnsupported",
            Self::ArchitectureUnsupported => "codexDesktop.error.architectureUnsupported",
            Self::SourceUnavailable => "codexDesktop.error.sourceUnavailable",
            Self::ReleaseMetadataInvalid => "codexDesktop.error.releaseMetadataInvalid",
            Self::ReleaseNotAvailable => "codexDesktop.error.releaseNotAvailable",
            Self::MetadataChanged => "codexDesktop.error.metadataChanged",
            Self::RedirectRejected => "codexDesktop.error.redirectRejected",
            Self::DownloadFailed => "codexDesktop.error.downloadFailed",
            Self::DownloadTimeout => "codexDesktop.error.downloadTimeout",
            Self::DownloadCancelled => "codexDesktop.error.downloadCancelled",
            Self::InsufficientDiskSpace => "codexDesktop.error.insufficientDiskSpace",
            Self::ChecksumMissing => "codexDesktop.error.checksumMissing",
            Self::ChecksumMismatch => "codexDesktop.error.checksumMismatch",
            Self::PackageParseFailed => "codexDesktop.error.packageParseFailed",
            Self::PackageIdentityMismatch => "codexDesktop.error.packageIdentityMismatch",
            Self::PackageArchitectureMismatch => "codexDesktop.error.packageArchitectureMismatch",
            Self::PackageSignatureInvalid => "codexDesktop.error.packageSignatureInvalid",
            Self::WindowsUacCancelled => "codexDesktop.error.windowsUacCancelled",
            Self::WindowsElevationFailed => "codexDesktop.error.windowsElevationFailed",
            Self::WindowsPackageInUse => "codexDesktop.error.windowsPackageInUse",
            Self::WindowsDeploymentBlocked => "codexDesktop.error.windowsDeploymentBlocked",
            Self::WindowsDependencyMissing => "codexDesktop.error.windowsDependencyMissing",
            Self::WindowsAllUsersUnsupported => "codexDesktop.error.windowsAllUsersUnsupported",
            Self::WindowsDeploymentFailed => "codexDesktop.error.windowsDeploymentFailed",
            Self::MacDmgMountFailed => "codexDesktop.error.macDmgMountFailed",
            Self::MacAppNotFound => "codexDesktop.error.macAppNotFound",
            Self::MacBundleIdMismatch => "codexDesktop.error.macBundleIdMismatch",
            Self::MacTeamIdMismatch => "codexDesktop.error.macTeamIdMismatch",
            Self::MacGatekeeperRejected => "codexDesktop.error.macGatekeeperRejected",
            Self::MacAppRunning => "codexDesktop.error.macAppRunning",
            Self::MacMultipleInstallations => "codexDesktop.error.macMultipleInstallations",
            Self::MacTargetPathConflict => "codexDesktop.error.macTargetPathConflict",
            Self::MacCopyFailed => "codexDesktop.error.macCopyFailed",
            Self::MacDmgDetachFailed => "codexDesktop.error.macDmgDetachFailed",
            Self::InstallationVerifyFailed => "codexDesktop.error.installationVerifyFailed",
            Self::LaunchFailed => "codexDesktop.error.launchFailed",
            Self::JobAlreadyRunning => "codexDesktop.error.jobAlreadyRunning",
            Self::JobNotFound => "codexDesktop.error.jobNotFound",
            Self::InternalError => "codexDesktop.error.internalError",
        }
    }

    const fn default_retryable(self) -> bool {
        matches!(
            self,
            Self::SourceUnavailable
                | Self::ReleaseNotAvailable
                | Self::MetadataChanged
                | Self::DownloadFailed
                | Self::DownloadTimeout
                | Self::DownloadCancelled
                | Self::InsufficientDiskSpace
                | Self::WindowsUacCancelled
                | Self::WindowsElevationFailed
                | Self::WindowsPackageInUse
                | Self::WindowsDeploymentFailed
                | Self::MacDmgMountFailed
                | Self::MacAppRunning
                | Self::MacCopyFailed
                | Self::MacDmgDetachFailed
                | Self::InstallationVerifyFailed
                | Self::LaunchFailed
        )
    }

    const fn suggested_action(self) -> SuggestedAction {
        match self {
            Self::SourceUnavailable
            | Self::DownloadFailed
            | Self::DownloadTimeout
            | Self::DownloadCancelled
            | Self::WindowsUacCancelled
            | Self::WindowsElevationFailed
            | Self::WindowsDeploymentFailed
            | Self::MacDmgMountFailed
            | Self::MacCopyFailed
            | Self::InstallationVerifyFailed
            | Self::LaunchFailed => SuggestedAction::Retry,
            Self::ReleaseNotAvailable | Self::MetadataChanged => SuggestedAction::Refresh,
            Self::WindowsPackageInUse | Self::MacAppRunning => {
                SuggestedAction::CloseTargetAppAndRetry
            }
            Self::WindowsDeploymentBlocked | Self::WindowsDependencyMissing => {
                SuggestedAction::ContactAdministrator
            }
            Self::InsufficientDiskSpace => SuggestedAction::FreeDiskSpace,
            Self::MacMultipleInstallations | Self::MacTargetPathConflict => {
                SuggestedAction::ResolvePathConflict
            }
            Self::PlatformUnsupported
            | Self::OsVersionUnsupported
            | Self::ArchitectureUnsupported
            | Self::WindowsAllUsersUnsupported
            | Self::JobAlreadyRunning
            | Self::JobNotFound => SuggestedAction::None,
            _ => SuggestedAction::OpenLogs,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedAction {
    Retry,
    Refresh,
    CloseTargetAppAndRetry,
    ContactAdministrator,
    FreeDiskSpace,
    ResolvePathConflict,
    OpenLogs,
    None,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallerDiagnosticDetails {
    pub endpoint_kind: Option<String>,
    pub attempt: Option<u8>,
    pub max_attempts: Option<u8>,
    pub http_status: Option<u16>,
    pub platform_error_code: Option<String>,
    pub redacted_message: Option<String>,
    pub context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallerErrorDto {
    pub code: InstallerErrorCode,
    pub stage: Option<JobStage>,
    pub message_key: String,
    pub retryable: bool,
    pub suggested_action: SuggestedAction,
    pub details: InstallerDiagnosticDetails,
}

#[derive(Debug, Clone, Error)]
#[error("Codex desktop installer error: {code:?}")]
pub struct InstallerError {
    code: InstallerErrorCode,
    stage: Option<JobStage>,
    retryable: bool,
    suggested_action: SuggestedAction,
    details: InstallerDiagnosticDetails,
}

impl InstallerError {
    pub fn new(code: InstallerErrorCode) -> Self {
        Self {
            stage: None,
            retryable: code.default_retryable(),
            suggested_action: code.suggested_action(),
            code,
            details: InstallerDiagnosticDetails::default(),
        }
    }

    pub fn with_stage(mut self, stage: JobStage) -> Self {
        self.stage = Some(stage);
        self
    }

    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn with_endpoint_kind(mut self, endpoint_kind: impl Into<String>) -> Self {
        self.details.endpoint_kind = Some(redact_diagnostic_text(&endpoint_kind.into()));
        self
    }

    pub fn with_attempt(mut self, attempt: u8, max_attempts: u8) -> Self {
        self.details.attempt = Some(attempt);
        self.details.max_attempts = Some(max_attempts);
        self
    }

    pub fn with_http_status(mut self, status: u16) -> Self {
        self.details.http_status = Some(status);
        self
    }

    pub fn with_platform_error_code(mut self, code: impl Into<String>) -> Self {
        self.details.platform_error_code = Some(redact_diagnostic_text(&code.into()));
        self
    }

    pub fn with_diagnostic_message(mut self, message: impl AsRef<str>) -> Self {
        self.details.redacted_message = Some(redact_diagnostic_text(message.as_ref()));
        self
    }

    /// Record only named, sanitized context. Unknown keys are intentionally
    /// dropped so an incidental raw header, URL or file path cannot enter IPC.
    pub fn with_context(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        let key = key.as_ref();
        if is_allowed_context_key(key) {
            self.details
                .context
                .insert(key.to_string(), redact_diagnostic_text(value.as_ref()));
        }
        self
    }

    pub const fn code(&self) -> InstallerErrorCode {
        self.code
    }

    pub fn to_dto(&self) -> InstallerErrorDto {
        InstallerErrorDto {
            code: self.code,
            stage: self.stage,
            message_key: self.code.message_key().to_string(),
            retryable: self.retryable,
            suggested_action: self.suggested_action,
            details: self.details.clone(),
        }
    }
}

impl From<InstallerError> for InstallerErrorDto {
    fn from(value: InstallerError) -> Self {
        value.to_dto()
    }
}

const ALLOWED_CONTEXT_KEYS: &[&str] = &[
    "architecture",
    "job_id",
    "local_display_version",
    "release_id_prefix",
    "source",
    "target_display_version",
    "target_platform_version",
];

fn is_allowed_context_key(key: &str) -> bool {
    ALLOWED_CONTEXT_KEYS.contains(&key)
}

static URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(?:https?|socks(?:5h?)?)://[^\s\]\[<>()\"']+"#)
        .expect("installer URL redaction regex is valid")
});
static WINDOWS_USER_PATH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)[a-z]:\\users\\[^\\/\s]+")
        .expect("installer Windows path redaction regex is valid")
});
static MACOS_USER_PATH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"/Users/[^/\s]+").expect("installer macOS path redaction regex is valid")
});
static LINUX_USER_PATH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"/home/[^/\s]+").expect("installer Linux path redaction regex is valid")
});
static SECRET_VALUE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b((?:proxy-)?authorization|cookie|(?:access[_-]?)?token|api[_-]?key|password)\s*[:=]\s*(?:(?:bearer|basic)\s+)?[^\s,;]+",
    )
        .expect("installer secret redaction regex is valid")
});
static SENSITIVE_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?im)\b(?:(?:proxy-)?authorization|cookie)\s*:\s*[^\r\n]*")
        .expect("installer sensitive header redaction regex is valid")
});

pub(crate) fn redact_diagnostic_text(value: &str) -> String {
    const MAX_DIAGNOSTIC_CHARS: usize = 2_048;

    let mut normalized = SENSITIVE_HEADER_RE
        .replace_all(value, "[redacted-header]")
        .into_owned()
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(MAX_DIAGNOSTIC_CHARS)
        .collect::<String>();

    normalized = URL_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            let raw = captures.get(0).map_or("", |capture| capture.as_str());
            Url::parse(raw)
                .ok()
                .and_then(|url| {
                    let host = url.host_str()?.to_string();
                    let port = url
                        .port()
                        .map(|value| format!(":{value}"))
                        .unwrap_or_default();
                    Some(format!("{}://{host}{port}{}", url.scheme(), url.path()))
                })
                .unwrap_or_else(|| "[redacted-url]".to_string())
        })
        .into_owned();
    normalized = WINDOWS_USER_PATH_RE
        .replace_all(&normalized, "%USERPROFILE%")
        .into_owned();
    normalized = MACOS_USER_PATH_RE
        .replace_all(&normalized, "~")
        .into_owned();
    normalized = LINUX_USER_PATH_RE
        .replace_all(&normalized, "~")
        .into_owned();
    SECRET_VALUE_RE
        .replace_all(&normalized, "$1=[redacted]")
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dto_uses_stable_code_and_default_action() {
        let dto = InstallerError::new(InstallerErrorCode::MetadataChanged).to_dto();

        assert_eq!(dto.code, InstallerErrorCode::MetadataChanged);
        assert_eq!(dto.message_key, "codexDesktop.error.metadataChanged");
        assert!(dto.retryable);
        assert_eq!(dto.suggested_action, SuggestedAction::Refresh);
    }

    #[test]
    fn diagnostics_redact_urls_secrets_and_user_paths() {
        let dto = InstallerError::new(InstallerErrorCode::DownloadFailed)
            .with_diagnostic_message(
                "GET https://user:password@cdn.example.test/file?token=secret#fragment \
                 C:\\Users\\alice\\AppData\\Local\\Temp auth=secret /Users/alice/Applications",
            )
            .with_context("job_id", "job-1")
            .with_context("unsafe_header", "Authorization: leaked")
            .to_dto();
        let text = dto.details.redacted_message.unwrap();

        assert!(text.contains("https://cdn.example.test/file"));
        assert!(!text.contains("token=secret"));
        assert!(!text.contains("password"));
        assert!(text.contains("%USERPROFILE%\\AppData"));
        assert!(text.contains("~/Applications"));
        assert_eq!(
            dto.details.context.get("job_id"),
            Some(&"job-1".to_string())
        );
        assert!(!dto.details.context.contains_key("unsafe_header"));
    }

    #[test]
    fn diagnostics_redact_proxy_credentials_and_complete_authorization_values() {
        let text = redact_diagnostic_text(
            "SOCKS5H://proxy-user:proxy-password@proxy.example.test:1080 \
             Proxy-Authorization: Basic cHJveHktc2VjcmV0\r\n\
             Authorization: Bearer application-secret\r\n\
             Cookie: session=another-secret; tracking=another-value \
             access_token=inline-secret",
        );

        assert!(text.contains("socks5h://proxy.example.test:1080"));
        assert!(!text.contains("proxy-user"));
        assert!(!text.contains("proxy-password"));
        assert!(!text.contains("cHJveHktc2VjcmV0"));
        assert!(!text.contains("application-secret"));
        assert!(!text.contains("another-secret"));
        assert!(!text.contains("inline-secret"));
    }
}
