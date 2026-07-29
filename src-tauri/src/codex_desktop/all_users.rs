//! Constrained, experimental Windows all-users provisioning protocol.
//!
//! This module intentionally contains no Tauri command, renderer DTO, URL, or
//! arbitrary process execution surface.  The Windows-only caller is reached
//! before Tauri creates a runtime.  Keeping the nonce-bound job protocol here
//! makes the security-critical parsing and revalidation behaviour testable on
//! every supported CI host.

use std::{
    ffi::{OsStr, OsString},
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    error::{InstallerError, InstallerErrorCode},
    platform::WINDOWS_CODEX_STABLE_IDENTITY,
    temp::JobTempDir,
    types::{
        normalize_sha256, CpuArchitecture, DesktopPlatform, PlatformVersion, ReleaseDescriptor,
    },
    verify::{ensure_required_disk_space, sha256_hex, verify_file, ArtifactKind, DiskSpaceProbe},
};

pub(crate) const EXPERIMENTAL_ALL_USERS_FLAG: &str = "--experimental-install-codex-all-users";
pub(crate) const ELEVATED_ALL_USERS_FLAG: &str = "--elevated-provision-codex";

pub(crate) const HEADLESS_EXIT_SUCCESS: i32 = 0;
pub(crate) const HEADLESS_EXIT_INVALID_ARGUMENTS: i32 = 64;
pub(crate) const HEADLESS_EXIT_UNSUPPORTED: i32 = 69;
pub(crate) const HEADLESS_EXIT_FAILURE: i32 = 70;

const JOB_SCHEMA_VERSION: u8 = 1;
pub(crate) const MAX_JOB_FILE_BYTES: u64 = 16 * 1024;
const MAX_JOB_LIFETIME: Duration = Duration::minutes(15);
const MAX_FUTURE_CREATED_AT_SKEW: Duration = Duration::minutes(1);

/// The startup parser distinguishes ordinary application arguments from the
/// two reserved experimental flags.  Any malformed use of a reserved flag is
/// rejected before a GUI can be started, while unrelated arguments retain the
/// host application's normal startup behaviour.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HeadlessInvocation {
    NormalApplication,
    PrepareAllUsers,
    ElevatedProvision { job_file: PathBuf, nonce: String },
    InvalidReservedArguments,
}

pub(crate) fn parse_headless_invocation<I>(arguments: I) -> HeadlessInvocation
where
    I: IntoIterator<Item = OsString>,
{
    let arguments = arguments.into_iter().skip(1).collect::<Vec<_>>();
    let experimental = OsStr::new(EXPERIMENTAL_ALL_USERS_FLAG);
    let elevated = OsStr::new(ELEVATED_ALL_USERS_FLAG);

    match arguments.as_slice() {
        [flag] if flag == experimental => HeadlessInvocation::PrepareAllUsers,
        [flag, job_file, nonce] if flag == elevated => {
            let nonce = nonce.to_string_lossy().into_owned();
            if job_file.is_empty() || nonce.is_empty() {
                HeadlessInvocation::InvalidReservedArguments
            } else {
                HeadlessInvocation::ElevatedProvision {
                    job_file: PathBuf::from(job_file),
                    nonce,
                }
            }
        }
        _ if arguments
            .iter()
            .any(|argument| argument == experimental || argument == elevated) =>
        {
            HeadlessInvocation::InvalidReservedArguments
        }
        _ => HeadlessInvocation::NormalApplication,
    }
}

/// Exact facts that may cross the parent-to-elevated-child boundary.  The
/// production factory constructs this only from the audited Publisher evidence
/// and the native Windows host; callers cannot supply it through IPC or CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AllUsersTrustPolicy {
    expected_identity: String,
    expected_publisher: String,
    expected_architecture: CpuArchitecture,
}

impl AllUsersTrustPolicy {
    pub(crate) fn new(
        expected_identity: impl Into<String>,
        expected_publisher: impl Into<String>,
        expected_architecture: CpuArchitecture,
    ) -> Result<Self, InstallerError> {
        let expected_identity = expected_identity.into();
        let expected_publisher = expected_publisher.into();
        if expected_identity != WINDOWS_CODEX_STABLE_IDENTITY {
            return Err(
                InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                    .with_diagnostic_message("all-users policy must use the exact Stable identity"),
            );
        }
        if !is_safe_policy_text(&expected_publisher, 512) {
            return Err(
                InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                    .with_diagnostic_message("all-users policy Publisher evidence is invalid"),
            );
        }
        if !matches!(
            expected_architecture,
            CpuArchitecture::X86_64 | CpuArchitecture::Aarch64
        ) {
            return Err(
                InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                    .with_context("architecture", expected_architecture.as_str())
                    .with_diagnostic_message("all-users provisioning supports x64 and ARM64 only"),
            );
        }
        Ok(Self {
            expected_identity,
            expected_publisher,
            expected_architecture,
        })
    }

    pub(crate) fn expected_identity(&self) -> &str {
        &self.expected_identity
    }

    pub(crate) fn expected_publisher(&self) -> &str {
        &self.expected_publisher
    }

    pub(crate) const fn expected_architecture(&self) -> CpuArchitecture {
        self.expected_architecture
    }
}

/// The only serialized parent-to-child payload.  It carries no remote URL,
/// token, command, or `verified` boolean: the elevated child repeats every
/// relevant filesystem, hash, manifest, identity, host, and deployment check.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AllUsersJob {
    schema_version: u8,
    job_id: String,
    nonce_hash: String,
    canonical_package_path: PathBuf,
    expected_sha256: String,
    expected_size: u64,
    expected_identity: String,
    expected_publisher: String,
    expected_version: String,
    expected_architecture: CpuArchitecture,
    minimum_os_version: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl AllUsersJob {
    pub(crate) fn from_verified_parent(
        directory: &JobTempDir,
        release: &ReleaseDescriptor,
        policy: &AllUsersTrustPolicy,
        nonce: &str,
        created_at: DateTime<Utc>,
    ) -> Result<Self, InstallerError> {
        validate_nonce(nonce)?;
        if release.platform != DesktopPlatform::Windows
            || release.architecture != policy.expected_architecture()
            || !matches!(
                &release.platform_version,
                PlatformVersion::WindowsMsix { .. }
            )
        {
            return Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message(
                "all-users parent received a release incompatible with the trusted Windows host",
            ));
        }

        let package_path = directory.final_path(ArtifactKind::Msix);
        directory.validate_existing_artifact(&package_path)?;
        verify_file(
            &package_path,
            release.expected_size,
            &release.expected_sha256,
        )?;

        let job_id = directory
            .path()
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                all_users_error("all-users temporary directory has no canonical job ID")
            })?
            .to_owned();
        if !is_canonical_job_id(&job_id) {
            return Err(all_users_error(
                "all-users temporary directory job ID is invalid",
            ));
        }

        let expected_version = windows_version_text(&release.platform_version)?;
        Ok(Self {
            schema_version: JOB_SCHEMA_VERSION,
            job_id,
            nonce_hash: sha256_hex(nonce.as_bytes()),
            canonical_package_path: package_path,
            expected_sha256: release.expected_sha256.clone(),
            expected_size: release.expected_size,
            expected_identity: policy.expected_identity().to_owned(),
            expected_publisher: policy.expected_publisher().to_owned(),
            expected_version,
            expected_architecture: policy.expected_architecture(),
            minimum_os_version: release.minimum_os_version.clone(),
            created_at,
            expires_at: created_at + MAX_JOB_LIFETIME,
        })
    }
}

/// A fully bound job cannot expose the caller-provided job-file or any raw
/// parent payload. Native code can ask only for the parent-owned package path
/// and local identity policy fields; release metadata is accepted separately
/// from a child-resolved trusted source.
pub(crate) struct ValidatedAllUsersJob {
    job: AllUsersJob,
}

impl ValidatedAllUsersJob {
    pub(crate) fn package_path(&self) -> &Path {
        &self.job.canonical_package_path
    }

    pub(crate) fn expected_identity(&self) -> &str {
        &self.job.expected_identity
    }

    pub(crate) fn expected_publisher(&self) -> &str {
        &self.job.expected_publisher
    }

    pub(crate) fn expected_architecture(&self) -> CpuArchitecture {
        self.job.expected_architecture
    }

    #[cfg(test)]
    pub(crate) fn expected_version(&self) -> &str {
        &self.job.expected_version
    }

    fn require_authoritative_release(
        &self,
        release: &ReleaseDescriptor,
    ) -> Result<(), InstallerError> {
        let expected_version = windows_version_text(&release.platform_version)?;
        if release.platform != DesktopPlatform::Windows
            || release.architecture != self.job.expected_architecture
            || release.expected_sha256 != self.job.expected_sha256
            || release.expected_size != self.job.expected_size
            || expected_version != self.job.expected_version
            || release.minimum_os_version != self.job.minimum_os_version
        {
            return Err(InstallerError::new(InstallerErrorCode::MetadataChanged)
                .with_diagnostic_message(
                    "all-users job does not match the child-resolved release metadata",
                ));
        }
        Ok(())
    }
}

/// The elevated child obtains a fresh release descriptor from its own fixed,
/// authenticated metadata route. This boundary intentionally has no job
/// parameter, so caller-controlled job fields cannot select the metadata that
/// authorizes a privileged package deployment.
pub(crate) trait AllUsersReleaseAnchor: Send + Sync {
    fn resolve_release(&self) -> Result<ReleaseDescriptor, InstallerError>;
}

/// The elevated native implementation owns the only read of the parent-owned
/// job JSON. It must open `expected_job_path` once with a no-follow handle,
/// prove that handle still resolves to the fixed local capability path, and
/// read at most `maximum_bytes` from that same handle. Keeping that operation
/// behind this boundary prevents the target-neutral protocol from checking a
/// user path and then reopening it by pathname.
pub(crate) trait AllUsersJobControlReader: Send + Sync {
    fn read_job_control(
        &self,
        expected_job_path: &Path,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, InstallerError>;
}

/// Native code provides this narrow revalidation boundary. It must inspect
/// the MSIX again against the current host and audited Publisher evidence;
/// receiving a [`ValidatedAllUsersJob`] is not permission to trust the parent.
pub(crate) trait AllUsersPackageValidator: Send + Sync {
    fn revalidate_package(
        &self,
        job: &ValidatedAllUsersJob,
        release: &ReleaseDescriptor,
    ) -> Result<(), InstallerError>;
}

/// Native code provides the one allowed system action: local MSIX stage plus
/// PackageManager all-users provision.  There is intentionally no generic
/// process-command parameter or current-user fallback.
pub(crate) trait AllUsersProvisioner: Send + Sync {
    fn stage_and_provision(
        &self,
        job: &ValidatedAllUsersJob,
        release: &ReleaseDescriptor,
    ) -> Result<(), InstallerError>;
}

/// The parent may ask only for the exact fixed child mode, job control file,
/// and high-entropy nonce.  Windows owns the `runas` implementation; this
/// target-neutral boundary makes a UAC-cancelled outcome testable without ever
/// attempting elevation in automated tests.
pub(crate) trait AllUsersElevator: Send + Sync {
    fn elevate(&self, job_file: &Path, nonce: &str) -> Result<(), InstallerError>;
}

/// Native dependencies for the elevated child.  Grouping them keeps the
/// protocol entry point small while making every side-effecting boundary
/// explicit and injectable in tests.
pub(crate) struct ElevatedProvisioningDependencies<'a> {
    root: &'a Path,
    policy: &'a AllUsersTrustPolicy,
    release_anchor: &'a dyn AllUsersReleaseAnchor,
    package_validator: &'a dyn AllUsersPackageValidator,
    disk_space_probe: &'a dyn DiskSpaceProbe,
    system_volume: &'a Path,
    provisioner: &'a dyn AllUsersProvisioner,
}

impl<'a> ElevatedProvisioningDependencies<'a> {
    pub(crate) fn new(
        root: &'a Path,
        policy: &'a AllUsersTrustPolicy,
        release_anchor: &'a dyn AllUsersReleaseAnchor,
        package_validator: &'a dyn AllUsersPackageValidator,
        disk_space_probe: &'a dyn DiskSpaceProbe,
        system_volume: &'a Path,
        provisioner: &'a dyn AllUsersProvisioner,
    ) -> Self {
        Self {
            root,
            policy,
            release_anchor,
            package_validator,
            disk_space_probe,
            system_volume,
            provisioner,
        }
    }
}

pub(crate) fn elevate_new_job(
    directory: &JobTempDir,
    nonce: &str,
    elevator: &dyn AllUsersElevator,
) -> Result<(), InstallerError> {
    validate_nonce(nonce)?;
    let job_file = directory.all_users_job_path();
    directory.validate_existing_all_users_control(&job_file)?;
    elevator.elevate(&job_file, nonce)
}

/// Executes the elevated child flow after exact argument parsing.  The caller
/// supplies the trusted system volume and Windows-specific adapters; all
/// untrusted file/nonce/job handling stays here and runs before either adapter.
pub(crate) fn run_elevated_provisioning(
    dependencies: ElevatedProvisioningDependencies<'_>,
    job_control_reader: &dyn AllUsersJobControlReader,
    job_file_argument: &Path,
    nonce: &str,
    now: DateTime<Utc>,
) -> Result<(), InstallerError> {
    let job = load_validated_job(
        dependencies.root,
        job_file_argument,
        nonce,
        dependencies.policy,
        job_control_reader,
        now,
    )?;
    let release = dependencies.release_anchor.resolve_release()?;
    job.require_authoritative_release(&release)?;
    // The elevated child never writes a result into the parent-owned temp
    // tree. Stable headless exit codes are the experimental result channel;
    // the deployment itself uses only protected staging and the system volume.
    ensure_required_disk_space(
        dependencies.disk_space_probe,
        [dependencies.system_volume],
        release.expected_size,
    )?;
    dependencies
        .package_validator
        .revalidate_package(&job, &release)?;
    dependencies.provisioner.stage_and_provision(&job, &release)
}

/// The parent writes once under a fresh, capability-owned directory.  `create_new`
/// prevents replacement of a job by a concurrent ordinary process.
pub(crate) fn write_new_job(
    directory: &JobTempDir,
    job: &AllUsersJob,
) -> Result<(), InstallerError> {
    if job.job_id
        != directory
            .path()
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
    {
        return Err(all_users_error(
            "all-users job ID does not match its directory",
        ));
    }
    let path = directory.all_users_job_path();
    directory.validate_all_users_control_path(&path)?;
    write_new_json_file(&path, job)
}

fn load_validated_job(
    root: &Path,
    job_file_argument: &Path,
    nonce: &str,
    policy: &AllUsersTrustPolicy,
    job_control_reader: &dyn AllUsersJobControlReader,
    now: DateTime<Utc>,
) -> Result<ValidatedAllUsersJob, InstallerError> {
    validate_nonce(nonce)?;
    let directory = directory_for_job_argument(root, job_file_argument)?;
    let expected_job_path = directory.all_users_job_path();
    // `job_file_argument` supplies only the fixed filename and canonical UUID
    // directory ID. The native reader below opens the reconstructed capability
    // path once; this generic layer must not inspect, canonicalize, or reopen
    // the parent-owned file by pathname after that boundary.
    let bytes = job_control_reader.read_job_control(&expected_job_path, MAX_JOB_FILE_BYTES)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_JOB_FILE_BYTES {
        return Err(all_users_error(
            "all-users job file is outside its size limit",
        ));
    }
    let job: AllUsersJob = serde_json::from_slice(&bytes)
        .map_err(|_| all_users_error("all-users job JSON does not match the expected schema"))?;

    validate_job(&directory, &job, nonce, policy, now)?;
    Ok(ValidatedAllUsersJob { job })
}

fn directory_for_job_argument(
    root: &Path,
    job_file_argument: &Path,
) -> Result<JobTempDir, InstallerError> {
    if job_file_argument.file_name() != Some(OsStr::new("all-users-job.json")) {
        return Err(all_users_error(
            "all-users child received a non-fixed job filename",
        ));
    }
    let job_id = job_file_argument
        .parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .ok_or_else(|| all_users_error("all-users job argument has no canonical directory ID"))?;
    if !is_canonical_job_id(job_id) {
        return Err(all_users_error(
            "all-users job argument directory ID is invalid",
        ));
    }
    JobTempDir::open_existing(root, job_id)
}

fn validate_job(
    directory: &JobTempDir,
    job: &AllUsersJob,
    nonce: &str,
    policy: &AllUsersTrustPolicy,
    now: DateTime<Utc>,
) -> Result<(), InstallerError> {
    if job.schema_version != JOB_SCHEMA_VERSION {
        return Err(all_users_error(
            "all-users job schema version is unsupported",
        ));
    }
    let directory_job_id = directory
        .path()
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| all_users_error("all-users temporary directory has no job ID"))?;
    if !is_canonical_job_id(&job.job_id) || job.job_id != directory_job_id {
        return Err(all_users_error(
            "all-users job ID does not match its directory",
        ));
    }
    if job.nonce_hash.len() != 64
        || !job
            .nonce_hash
            .bytes()
            .all(|value| value.is_ascii_hexdigit())
        || job.nonce_hash != sha256_hex(nonce.as_bytes())
    {
        return Err(
            InstallerError::new(InstallerErrorCode::WindowsElevationFailed)
                .with_diagnostic_message("all-users nonce binding did not match the job"),
        );
    }
    if job.created_at > now + MAX_FUTURE_CREATED_AT_SKEW
        || job.expires_at <= now
        || job.expires_at <= job.created_at
        || job.expires_at - job.created_at > MAX_JOB_LIFETIME
    {
        return Err(
            InstallerError::new(InstallerErrorCode::WindowsElevationFailed)
                .with_diagnostic_message("all-users job is expired or has an invalid lifetime"),
        );
    }
    if job.expected_identity != policy.expected_identity()
        || job.expected_publisher != policy.expected_publisher()
        || job.expected_architecture != policy.expected_architecture()
    {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                .with_diagnostic_message(
                    "all-users job identity does not match the trusted policy",
                ),
        );
    }
    if !is_safe_policy_text(&job.expected_version, 64)
        || !is_safe_policy_text(&job.expected_publisher, 512)
        || job.expected_size == 0
    {
        return Err(all_users_error(
            "all-users job contains invalid trusted package fields",
        ));
    }
    PlatformVersion::parse_windows_msix(&job.expected_version)?;
    if let Some(minimum_os_version) = job.minimum_os_version.as_deref() {
        if !is_safe_policy_text(minimum_os_version, 64) {
            return Err(all_users_error(
                "all-users job minimum OS version is invalid",
            ));
        }
        PlatformVersion::parse_windows_msix(minimum_os_version)?;
    }
    let normalized_sha256 = normalize_sha256(&job.expected_sha256)?;
    if normalized_sha256 != job.expected_sha256 {
        return Err(all_users_error("all-users job SHA-256 is not canonical"));
    }

    let expected_package_path = directory.final_path(ArtifactKind::Msix);
    if job.canonical_package_path != expected_package_path {
        return Err(all_users_error(
            "all-users job path does not match its fixed temporary-directory capability",
        ));
    }
    // The elevated child intentionally performs no filesystem access to this
    // parent-owned pathname here. The Windows provisioner opens it once with a
    // no-follow handle, verifies that handle's final path, copies it to
    // protected staging, and validates only that protected copy.
    Ok(())
}

fn write_new_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), InstallerError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| all_users_error("all-users control JSON could not be serialized"))?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_JOB_FILE_BYTES {
        return Err(all_users_error(
            "all-users control JSON is outside its size limit",
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| all_users_error("all-users job file could not be created"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| all_users_error("all-users job file could not be persisted"))
}

fn validate_nonce(nonce: &str) -> Result<(), InstallerError> {
    if nonce.len() != 32
        || !nonce.bytes().all(|value| {
            value.is_ascii_digit() || (value.is_ascii_lowercase() && value.is_ascii_hexdigit())
        })
    {
        return Err(
            InstallerError::new(InstallerErrorCode::WindowsElevationFailed)
                .with_diagnostic_message("all-users child nonce has an invalid format"),
        );
    }
    Ok(())
}

fn is_safe_policy_text(value: &str, maximum_length: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_length
        && !value.bytes().any(|value| value.is_ascii_control())
}

fn is_canonical_job_id(value: &str) -> bool {
    Uuid::parse_str(value)
        .map(|parsed| parsed.hyphenated().to_string() == value)
        .unwrap_or(false)
}

fn windows_version_text(version: &PlatformVersion) -> Result<String, InstallerError> {
    let PlatformVersion::WindowsMsix {
        major,
        minor,
        build,
        revision,
    } = version
    else {
        return Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
            .with_diagnostic_message("all-users job requires a Windows MSIX version"));
    };
    Ok(format!("{major}.{minor}.{build}.{revision}"))
}

fn all_users_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsElevationFailed).with_diagnostic_message(message)
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Mutex};

    use super::*;
    use crate::codex_desktop::{
        types::TrustedDownloadEndpoint,
        verify::{DiskSpaceProbeError, VolumeKey},
    };

    const PUBLISHER: &str = "CN=50BDFD77-8903-4850-9FFE-6E8522F64D5B";
    const NONCE: &str = "0123456789abcdef0123456789abcdef";

    #[derive(Default)]
    struct FakeDiskSpaceProbe;

    impl DiskSpaceProbe for FakeDiskSpaceProbe {
        fn volume_key(&self, _path: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
            VolumeKey::new("test-volume")
        }

        fn available_bytes(&self, _volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
            Ok(1024 * 1024)
        }
    }

    struct FakeReleaseAnchor {
        release: ReleaseDescriptor,
        calls: Mutex<u32>,
    }

    impl FakeReleaseAnchor {
        fn new(release: ReleaseDescriptor) -> Self {
            Self {
                release,
                calls: Mutex::new(0),
            }
        }
    }

    impl AllUsersReleaseAnchor for FakeReleaseAnchor {
        fn resolve_release(&self) -> Result<ReleaseDescriptor, InstallerError> {
            *self.calls.lock().unwrap() += 1;
            Ok(self.release.clone())
        }
    }

    /// This test-only reader exercises the target-neutral protocol on every
    /// host without emulating Windows path semantics. The production Windows
    /// reader is separately responsible for the no-follow, final-handle,
    /// fixed-drive checks before it reads any bytes.
    struct StaticJobControlReader {
        bytes: Vec<u8>,
        calls: Mutex<Vec<(PathBuf, u64)>>,
    }

    impl StaticJobControlReader {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                bytes,
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl AllUsersJobControlReader for StaticJobControlReader {
        fn read_job_control(
            &self,
            expected_job_path: &Path,
            maximum_bytes: u64,
        ) -> Result<Vec<u8>, InstallerError> {
            self.calls
                .lock()
                .unwrap()
                .push((expected_job_path.to_path_buf(), maximum_bytes));
            Ok(self.bytes.clone())
        }
    }

    #[derive(Default)]
    struct FakeValidator {
        outcome: Mutex<Option<InstallerErrorCode>>,
        calls: Mutex<u32>,
    }

    impl FakeValidator {
        fn failing(code: InstallerErrorCode) -> Self {
            Self {
                outcome: Mutex::new(Some(code)),
                calls: Mutex::new(0),
            }
        }
    }

    impl AllUsersPackageValidator for FakeValidator {
        fn revalidate_package(
            &self,
            job: &ValidatedAllUsersJob,
            release: &ReleaseDescriptor,
        ) -> Result<(), InstallerError> {
            *self.calls.lock().unwrap() += 1;
            assert_eq!(release.platform, DesktopPlatform::Windows);
            assert_eq!(release.architecture, job.expected_architecture());
            match *self.outcome.lock().unwrap() {
                Some(code) => Err(InstallerError::new(code)
                    .with_diagnostic_message("C:\\Users\\example\\secret package error")),
                None => Ok(()),
            }
        }
    }

    #[derive(Default)]
    struct FakeProvisioner {
        outcome: Mutex<Option<InstallerErrorCode>>,
        calls: Mutex<u32>,
    }

    impl FakeProvisioner {
        fn failing(code: InstallerErrorCode) -> Self {
            Self {
                outcome: Mutex::new(Some(code)),
                calls: Mutex::new(0),
            }
        }
    }

    impl AllUsersProvisioner for FakeProvisioner {
        fn stage_and_provision(
            &self,
            job: &ValidatedAllUsersJob,
            release: &ReleaseDescriptor,
        ) -> Result<(), InstallerError> {
            *self.calls.lock().unwrap() += 1;
            assert_eq!(job.expected_identity(), WINDOWS_CODEX_STABLE_IDENTITY);
            assert_eq!(job.expected_publisher(), PUBLISHER);
            assert_eq!(job.expected_architecture(), CpuArchitecture::X86_64);
            assert_eq!(job.expected_version(), "1.2.3.4");
            assert_eq!(release.expected_sha256, sha256_hex(b"test-msix"));
            assert_eq!(release.expected_size, 9);
            match *self.outcome.lock().unwrap() {
                Some(code) => Err(InstallerError::new(code)
                    .with_diagnostic_message("C:\\Users\\example\\secret deployment error")),
                None => Ok(()),
            }
        }
    }

    struct FakeElevator {
        outcome: Option<InstallerErrorCode>,
        calls: Mutex<Vec<(PathBuf, String)>>,
    }

    impl FakeElevator {
        fn cancelled() -> Self {
            Self {
                outcome: Some(InstallerErrorCode::WindowsUacCancelled),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl AllUsersElevator for FakeElevator {
        fn elevate(&self, job_file: &Path, nonce: &str) -> Result<(), InstallerError> {
            self.calls
                .lock()
                .unwrap()
                .push((job_file.to_path_buf(), nonce.to_owned()));
            match self.outcome {
                Some(code) => Err(InstallerError::new(code)),
                None => Ok(()),
            }
        }
    }

    struct Fixture {
        root: tempfile::TempDir,
        directory: JobTempDir,
        policy: AllUsersTrustPolicy,
        release: ReleaseDescriptor,
        job_path: PathBuf,
        now: DateTime<Utc>,
    }

    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let directory =
                JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
            let package_path = directory.final_path(ArtifactKind::Msix);
            fs::write(&package_path, b"test-msix").unwrap();
            let policy = AllUsersTrustPolicy::new(
                WINDOWS_CODEX_STABLE_IDENTITY,
                PUBLISHER,
                CpuArchitecture::X86_64,
            )
            .unwrap();
            let release = ReleaseDescriptor::new(
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64,
                "1.2.3.4",
                PlatformVersion::parse_windows_msix("1.2.3.4").unwrap(),
                "fixture.msix",
                sha256_hex(b"test-msix"),
                9,
                TrustedDownloadEndpoint::WinX64,
                Some("10.0.19041.0".to_owned()),
            )
            .unwrap();
            let now = Utc::now();
            let job = AllUsersJob::from_verified_parent(&directory, &release, &policy, NONCE, now)
                .unwrap();
            write_new_job(&directory, &job).unwrap();
            let job_path = directory.all_users_job_path();
            Self {
                root,
                directory,
                policy,
                release,
                job_path,
                now,
            }
        }

        fn rewrite_job(&self, update: impl FnOnce(&mut AllUsersJob)) {
            let bytes = fs::read(&self.job_path).unwrap();
            let mut job: AllUsersJob = serde_json::from_slice(&bytes).unwrap();
            update(&mut job);
            fs::remove_file(&self.job_path).unwrap();
            write_new_job(&self.directory, &job).unwrap();
        }

        fn job_control_reader(&self) -> StaticJobControlReader {
            StaticJobControlReader::new(fs::read(&self.job_path).unwrap())
        }

        fn run(
            &self,
            nonce: &str,
            validator: &dyn AllUsersPackageValidator,
            provisioner: &dyn AllUsersProvisioner,
        ) -> Result<(), InstallerError> {
            let anchor = FakeReleaseAnchor::new(self.release.clone());
            let job_control_reader = self.job_control_reader();
            self.run_with_anchor_and_reader(
                nonce,
                &job_control_reader,
                &anchor,
                validator,
                provisioner,
            )
        }

        fn run_with_anchor(
            &self,
            nonce: &str,
            anchor: &dyn AllUsersReleaseAnchor,
            validator: &dyn AllUsersPackageValidator,
            provisioner: &dyn AllUsersProvisioner,
        ) -> Result<(), InstallerError> {
            let job_control_reader = self.job_control_reader();
            self.run_with_anchor_and_reader(
                nonce,
                &job_control_reader,
                anchor,
                validator,
                provisioner,
            )
        }

        fn run_with_anchor_and_reader(
            &self,
            nonce: &str,
            job_control_reader: &dyn AllUsersJobControlReader,
            anchor: &dyn AllUsersReleaseAnchor,
            validator: &dyn AllUsersPackageValidator,
            provisioner: &dyn AllUsersProvisioner,
        ) -> Result<(), InstallerError> {
            run_elevated_provisioning(
                ElevatedProvisioningDependencies::new(
                    self.root.path(),
                    &self.policy,
                    anchor,
                    validator,
                    &FakeDiskSpaceProbe,
                    self.root.path(),
                    provisioner,
                ),
                job_control_reader,
                &self.job_path,
                nonce,
                self.now,
            )
        }
    }

    #[test]
    fn parser_accepts_only_exact_reserved_forms() {
        assert_eq!(
            parse_headless_invocation([OsString::from("fyagent")]),
            HeadlessInvocation::NormalApplication
        );
        assert_eq!(
            parse_headless_invocation([
                OsString::from("fyagent"),
                OsString::from(EXPERIMENTAL_ALL_USERS_FLAG),
            ]),
            HeadlessInvocation::PrepareAllUsers
        );
        assert!(matches!(
            parse_headless_invocation([
                OsString::from("fyagent"),
                OsString::from(ELEVATED_ALL_USERS_FLAG),
                OsString::from("C:\\temp\\job\\all-users-job.json"),
                OsString::from(NONCE),
            ]),
            HeadlessInvocation::ElevatedProvision { .. }
        ));
        for arguments in [
            vec![
                OsString::from("fyagent"),
                OsString::from(EXPERIMENTAL_ALL_USERS_FLAG),
                OsString::from("extra"),
            ],
            vec![
                OsString::from("fyagent"),
                OsString::from(ELEVATED_ALL_USERS_FLAG),
            ],
            vec![
                OsString::from("fyagent"),
                OsString::from("--other"),
                OsString::from(ELEVATED_ALL_USERS_FLAG),
            ],
        ] {
            assert_eq!(
                parse_headless_invocation(arguments),
                HeadlessInvocation::InvalidReservedArguments
            );
        }
    }

    #[test]
    fn nonce_mismatch_and_expired_jobs_do_not_reach_native_adapters() {
        let fixture = Fixture::new();
        let validator = FakeValidator::default();
        let provisioner = FakeProvisioner::default();
        let error = fixture
            .run("fedcba9876543210fedcba9876543210", &validator, &provisioner)
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::WindowsElevationFailed);
        assert_eq!(*validator.calls.lock().unwrap(), 0);
        assert_eq!(*provisioner.calls.lock().unwrap(), 0);

        fixture.rewrite_job(|job| job.expires_at = fixture.now - Duration::seconds(1));
        let error = fixture.run(NONCE, &validator, &provisioner).unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::WindowsElevationFailed);
        assert_eq!(*validator.calls.lock().unwrap(), 0);
        assert_eq!(*provisioner.calls.lock().unwrap(), 0);
    }

    #[test]
    fn path_escape_is_rejected_before_the_metadata_anchor_or_native_adapters() {
        let fixture = Fixture::new();
        let job_control_reader = StaticJobControlReader::new(Vec::new());
        let anchor = FakeReleaseAnchor::new(fixture.release.clone());
        let validator = FakeValidator::default();
        let provisioner = FakeProvisioner::default();
        let outside = fixture.root.path().join("all-users-job.json");
        fs::write(&outside, b"{}").unwrap();
        let error = run_elevated_provisioning(
            ElevatedProvisioningDependencies::new(
                fixture.root.path(),
                &fixture.policy,
                &anchor,
                &validator,
                &FakeDiskSpaceProbe,
                fixture.root.path(),
                &provisioner,
            ),
            &job_control_reader,
            &outside,
            NONCE,
            fixture.now,
        )
        .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::WindowsElevationFailed);
        assert_eq!(*anchor.calls.lock().unwrap(), 0);
        assert_eq!(*validator.calls.lock().unwrap(), 0);
        assert_eq!(*provisioner.calls.lock().unwrap(), 0);
        assert!(job_control_reader.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn elevated_loader_uses_the_injected_bounded_job_control_reader() {
        let fixture = Fixture::new();
        let job_control_reader = fixture.job_control_reader();
        let anchor = FakeReleaseAnchor::new(fixture.release.clone());
        let validator = FakeValidator::default();
        let provisioner = FakeProvisioner::default();

        fixture
            .run_with_anchor_and_reader(
                NONCE,
                &job_control_reader,
                &anchor,
                &validator,
                &provisioner,
            )
            .unwrap();

        assert_eq!(
            job_control_reader.calls.lock().unwrap().as_slice(),
            &[(fixture.job_path.clone(), MAX_JOB_FILE_BYTES)]
        );
        assert_eq!(*anchor.calls.lock().unwrap(), 1);
        assert_eq!(*validator.calls.lock().unwrap(), 1);
        assert_eq!(*provisioner.calls.lock().unwrap(), 1);
    }

    #[test]
    fn oversized_job_control_is_rejected_before_metadata_or_native_adapters() {
        let fixture = Fixture::new();
        let job_control_reader =
            StaticJobControlReader::new(vec![b'{'; (MAX_JOB_FILE_BYTES + 1) as usize]);
        let anchor = FakeReleaseAnchor::new(fixture.release.clone());
        let validator = FakeValidator::default();
        let provisioner = FakeProvisioner::default();

        let error = fixture
            .run_with_anchor_and_reader(
                NONCE,
                &job_control_reader,
                &anchor,
                &validator,
                &provisioner,
            )
            .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::WindowsElevationFailed);
        assert_eq!(
            job_control_reader.calls.lock().unwrap().as_slice(),
            &[(fixture.job_path.clone(), MAX_JOB_FILE_BYTES)]
        );
        assert_eq!(*anchor.calls.lock().unwrap(), 0);
        assert_eq!(*validator.calls.lock().unwrap(), 0);
        assert_eq!(*provisioner.calls.lock().unwrap(), 0);
    }

    #[test]
    fn metadata_anchor_rejects_a_nonce_valid_same_identity_package_swap_before_native_calls() {
        let fixture = Fixture::new();
        let alternate_package = b"alternate-fixture-msix";
        fs::write(
            fixture.directory.final_path(ArtifactKind::Msix),
            alternate_package,
        )
        .unwrap();
        fixture.rewrite_job(|job| {
            // Preserve the valid nonce binding and exact local policy fields,
            // while making the mutable job describe a different valid-looking
            // release. The child anchor, not this job, must authorize it.
            assert_eq!(job.nonce_hash, sha256_hex(NONCE.as_bytes()));
            assert_eq!(job.expected_identity, WINDOWS_CODEX_STABLE_IDENTITY);
            assert_eq!(job.expected_publisher, PUBLISHER);
            assert_eq!(job.expected_architecture, CpuArchitecture::X86_64);
            job.expected_sha256 = sha256_hex(alternate_package);
            job.expected_size = alternate_package.len() as u64;
            job.expected_version = "9.9.9.9".to_owned();
            job.minimum_os_version = Some("10.0.22621.0".to_owned());
        });
        let anchor = FakeReleaseAnchor::new(fixture.release.clone());
        let validator = FakeValidator::default();
        let provisioner = FakeProvisioner::default();

        let error = fixture
            .run_with_anchor(NONCE, &anchor, &validator, &provisioner)
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::MetadataChanged);
        assert_eq!(*anchor.calls.lock().unwrap(), 1);
        assert_eq!(*validator.calls.lock().unwrap(), 0);
        assert_eq!(*provisioner.calls.lock().unwrap(), 0);
    }

    #[test]
    fn identity_revalidation_failure_never_provisions_or_writes_parent_temp() {
        let fixture = Fixture::new();
        let validator = FakeValidator::failing(InstallerErrorCode::PackageIdentityMismatch);
        let provisioner = FakeProvisioner::default();
        let error = fixture.run(NONCE, &validator, &provisioner).unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::PackageIdentityMismatch);
        assert_eq!(*validator.calls.lock().unwrap(), 1);
        assert_eq!(*provisioner.calls.lock().unwrap(), 0);
    }

    #[test]
    fn stage_failure_returns_a_stable_error_and_success_reaches_provisioner() {
        let failed = Fixture::new();
        let validator = FakeValidator::default();
        let provisioner = FakeProvisioner::failing(InstallerErrorCode::WindowsDeploymentFailed);
        let error = failed.run(NONCE, &validator, &provisioner).unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::WindowsDeploymentFailed);
        assert_eq!(*provisioner.calls.lock().unwrap(), 1);

        let succeeded = Fixture::new();
        let validator = FakeValidator::default();
        let provisioner = FakeProvisioner::default();
        succeeded.run(NONCE, &validator, &provisioner).unwrap();
        assert_eq!(*validator.calls.lock().unwrap(), 1);
        assert_eq!(*provisioner.calls.lock().unwrap(), 1);
    }

    #[test]
    fn job_policy_rejects_tampered_identity_before_native_calls() {
        let fixture = Fixture::new();
        fixture.rewrite_job(|job| job.expected_identity = "OpenAI.CodexBeta".to_owned());
        let validator = FakeValidator::default();
        let provisioner = FakeProvisioner::default();
        let error = fixture.run(NONCE, &validator, &provisioner).unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::PackageIdentityMismatch);
        assert_eq!(*validator.calls.lock().unwrap(), 0);
        assert_eq!(*provisioner.calls.lock().unwrap(), 0);
    }

    #[test]
    fn parent_elevation_boundary_keeps_the_fixed_job_path_and_uac_cancellation() {
        let fixture = Fixture::new();
        let elevator = FakeElevator::cancelled();
        let error = elevate_new_job(&fixture.directory, NONCE, &elevator).unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::WindowsUacCancelled);
        assert_eq!(
            elevator.calls.lock().unwrap().as_slice(),
            &[(fixture.directory.all_users_job_path(), NONCE.to_owned())]
        );
    }

    #[test]
    fn ordinary_ipc_and_renderer_api_do_not_expose_the_experimental_scope() {
        let commands = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/commands/codex_desktop.rs"
        ));
        let renderer_api = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../src/lib/api/codex-desktop.ts"
        ));
        for forbidden in [
            EXPERIMENTAL_ALL_USERS_FLAG,
            ELEVATED_ALL_USERS_FLAG,
            "allUsers",
            "all_users",
            "installScope",
        ] {
            assert!(
                !commands.contains(forbidden),
                "ordinary Codex desktop IPC must not expose {forbidden}"
            );
            assert!(
                !renderer_api.contains(forbidden),
                "renderer API must not expose {forbidden}"
            );
        }
    }

    #[test]
    fn elevated_flow_has_no_parent_temp_result_write_surface() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/codex_desktop/all_users.rs"
        ));
        let result_path_method = concat!("all_users_", "result_path");
        let result_write = concat!("write_", "result(");
        assert!(!source.contains(result_path_method));
        assert!(!source.contains(result_write));
        assert!(source.contains("[dependencies.system_volume]"));
    }

    #[test]
    fn generic_elevated_loader_never_reopens_the_control_file_by_path() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/codex_desktop/all_users.rs"
        ));
        let direct_read = concat!("fs::", "read(&expected_job_path)");
        let direct_metadata = concat!("fs::", "metadata(&expected_job_path)");
        let direct_canonicalize = concat!("fs::", "canonicalize(&expected_job_path)");
        let argument_metadata = concat!("fs::", "symlink_metadata(job_file_argument)");
        let injected_reader = concat!("job_control_reader.", "read_job_control");

        assert!(!source.contains(direct_read));
        assert!(!source.contains(direct_metadata));
        assert!(!source.contains(direct_canonicalize));
        assert!(!source.contains(argument_metadata));
        assert!(source.contains(injected_reader));
    }
}
