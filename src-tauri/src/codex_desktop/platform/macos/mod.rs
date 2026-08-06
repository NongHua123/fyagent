//! macOS Apple-Silicon installer adapter.
//!
//! This module deliberately uses only a small command and filesystem boundary.
//! The adapter can therefore be tested with fakes on every host without
//! mounting a disk image or touching a real Applications directory. Platform
//! root code chooses whether this adapter is available for the current target.

mod bundle;
mod dmg;

use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use futures::future::BoxFuture;

use super::{
    CodexDesktopPlatform, PlatformInstallPlan, PlatformProgressSink, RestartCandidateInspection,
    RuntimeInspection, TrustedRuntimeInstance, VerifiedPackage, MACOS_CODEX_STABLE_IDENTITY,
};
use crate::codex_desktop::{
    download::DownloadedArtifact,
    error::{InstallerError, InstallerErrorCode},
    types::{
        CpuArchitecture, DesktopPlatform, InstalledApplication, LocalInstallStatus,
        ReleaseDescriptor, UnsupportedReason,
    },
    verify::{DiskSpaceProbe, DiskSpaceProbeError, VolumeKey},
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_COMMAND_OUTPUT_BYTES: usize = 16 * 1024;
const MACOS_MINIMUM_MAJOR_VERSION: u16 = 14;

/// Fixed invocation record passed to the narrow system-command boundary.
/// Programs are static literals and arguments are independent OS strings; no
/// adapter path ever constructs a shell command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    program: &'static str,
    arguments: Vec<OsString>,
    timeout: Duration,
}

impl CommandInvocation {
    pub fn new(
        program: &'static str,
        arguments: impl IntoIterator<Item = OsString>,
        timeout: Duration,
    ) -> Self {
        Self {
            program,
            arguments: arguments.into_iter().collect(),
            timeout,
        }
    }

    pub fn program(&self) -> &'static str {
        self.program
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// Bounded command result. Callers interpret a nonzero status in the context
/// of the operation instead of exposing command stderr as a user-facing error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    status_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl CommandOutput {
    pub fn success(stdout: impl Into<Vec<u8>>) -> Self {
        Self {
            status_code: Some(0),
            stdout: bounded_output(stdout.into()),
            stderr: Vec::new(),
        }
    }

    pub fn failure(status_code: Option<i32>, stderr: impl Into<Vec<u8>>) -> Self {
        Self {
            status_code,
            stdout: Vec::new(),
            stderr: bounded_output(stderr.into()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.status_code == Some(0)
    }

    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRunnerErrorKind {
    Spawn,
    Timeout,
    Io,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandRunnerError {
    kind: CommandRunnerErrorKind,
}

impl CommandRunnerError {
    pub const fn kind(self) -> CommandRunnerErrorKind {
        self.kind
    }
}

/// All macOS system commands pass through this interface. Test runners return
/// queued, bounded outcomes and record arguments without invoking a shell.
pub trait CommandRunner: Send + Sync {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError>;
}

/// Production implementation with a bounded wall-clock wait. It is not
/// constructed or called by tests; the adapter invokes it only from a blocking
/// task so service orchestration remains asynchronous.
#[derive(Debug, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
        let mut child = Command::new(invocation.program)
            .args(&invocation.arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(command_runner_error_from_io)?;
        let deadline = Instant::now() + invocation.timeout;

        loop {
            match child.try_wait().map_err(command_runner_error_from_io)? {
                Some(_) => {
                    let output = child
                        .wait_with_output()
                        .map_err(command_runner_error_from_io)?;
                    return Ok(CommandOutput {
                        status_code: output.status.code(),
                        stdout: bounded_output(output.stdout),
                        stderr: bounded_output(output.stderr),
                    });
                }
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CommandRunnerError {
                        kind: CommandRunnerErrorKind::Timeout,
                    });
                }
                None => thread::sleep(Duration::from_millis(10)),
            }
        }
    }
}

fn bounded_output(mut value: Vec<u8>) -> Vec<u8> {
    value.truncate(MAX_COMMAND_OUTPUT_BYTES);
    value
}

fn command_runner_error_from_io(error: std::io::Error) -> CommandRunnerError {
    let kind = if error.kind() == std::io::ErrorKind::NotFound {
        CommandRunnerErrorKind::Spawn
    } else {
        CommandRunnerErrorKind::Io
    };
    CommandRunnerError { kind }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacosFileKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacosFilesystemErrorKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    Invalid,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacosFilesystemError {
    kind: MacosFilesystemErrorKind,
}

impl MacosFilesystemError {
    pub const fn kind(self) -> MacosFilesystemErrorKind {
        self.kind
    }
}

/// Filesystem operations required by the macOS adapter. The implementation
/// deliberately has no generic arbitrary-path deletion API: all removal is
/// guarded by the DMG transaction code before it reaches this boundary.
pub trait MacosFilesystem: Send + Sync {
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, MacosFilesystemError>;

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MacosFilesystemError>;

    fn file_kind(&self, path: &Path) -> Result<MacosFileKind, MacosFilesystemError>;

    fn create_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError>;

    fn rename(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError>;

    fn remove_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError>;
}

/// Production filesystem adapter. It contains no macOS framework imports and
/// is usable only through the constrained path checks in `bundle` and `dmg`.
#[derive(Debug, Default)]
pub struct StdMacosFilesystem;

impl MacosFilesystem for StdMacosFilesystem {
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, MacosFilesystemError> {
        fs::read_dir(path)
            .map_err(macos_filesystem_error_from_io)?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(macos_filesystem_error_from_io)
            })
            .collect()
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MacosFilesystemError> {
        fs::canonicalize(path).map_err(macos_filesystem_error_from_io)
    }

    fn file_kind(&self, path: &Path) -> Result<MacosFileKind, MacosFilesystemError> {
        let metadata = fs::symlink_metadata(path).map_err(macos_filesystem_error_from_io)?;
        let file_type = metadata.file_type();
        Ok(if file_type.is_symlink() {
            MacosFileKind::Symlink
        } else if file_type.is_dir() {
            MacosFileKind::Directory
        } else if file_type.is_file() {
            MacosFileKind::File
        } else {
            MacosFileKind::Other
        })
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError> {
        fs::create_dir_all(path).map_err(macos_filesystem_error_from_io)
    }

    fn rename(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError> {
        fs::rename(source, destination).map_err(macos_filesystem_error_from_io)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError> {
        fs::remove_dir_all(path).map_err(macos_filesystem_error_from_io)
    }
}

fn macos_filesystem_error_from_io(error: std::io::Error) -> MacosFilesystemError {
    let kind = match error.kind() {
        std::io::ErrorKind::NotFound => MacosFilesystemErrorKind::NotFound,
        std::io::ErrorKind::PermissionDenied => MacosFilesystemErrorKind::PermissionDenied,
        std::io::ErrorKind::AlreadyExists => MacosFilesystemErrorKind::AlreadyExists,
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => {
            MacosFilesystemErrorKind::Invalid
        }
        _ => MacosFilesystemErrorKind::Other,
    };
    MacosFilesystemError { kind }
}

/// macOS target-volume probe backed by the fixed `df -Pk` command. The volume
/// key is an opaque filesystem token used only for one service preflight; raw
/// mount paths and command text never leave this adapter.
pub struct MacosDiskSpaceProbe {
    runner: Arc<dyn CommandRunner>,
    snapshots: Mutex<HashMap<VolumeKey, u64>>,
}

impl MacosDiskSpaceProbe {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self {
            runner,
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn for_current_host() -> Self {
        Self::new(Arc::new(SystemCommandRunner))
    }
}

impl DiskSpaceProbe for MacosDiskSpaceProbe {
    fn volume_key(&self, path: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
        let output = self
            .runner
            .run(&command(
                "df",
                vec![OsString::from("-Pk"), path.to_path_buf().into_os_string()],
            ))
            .map_err(|_| DiskSpaceProbeError::Unavailable)?;
        if !output.is_success() {
            return Err(DiskSpaceProbeError::Unavailable);
        }
        let (filesystem, available_blocks) =
            parse_df_snapshot(output.stdout()).ok_or(DiskSpaceProbeError::Unavailable)?;
        let available_bytes = available_blocks
            .checked_mul(1024)
            .ok_or(DiskSpaceProbeError::Unavailable)?;
        let volume = VolumeKey::new(format!("macos-df:{filesystem}"))
            .map_err(|_| DiskSpaceProbeError::InvalidVolumeKey)?;
        self.snapshots
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(volume.clone(), available_bytes);
        Ok(volume)
    }

    fn available_bytes(&self, volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
        self.snapshots
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(volume)
            .copied()
            .ok_or(DiskSpaceProbeError::Unavailable)
    }
}

fn parse_df_snapshot(bytes: &[u8]) -> Option<(&str, u64)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let line = text.lines().skip(1).find(|line| !line.trim().is_empty())?;
    let fields = line.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 6 {
        return None;
    }
    Some((fields[0], fields[3].parse().ok()?))
}

/// Parsed macOS version with numeric comparison semantics. It intentionally
/// does not use a display-version or lexical comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MacosVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

impl MacosVersion {
    pub(crate) fn parse(value: &str) -> Result<Self, InstallerError> {
        let mut parts = value.split('.');
        let parse_part = |part: Option<&str>| -> Result<u16, InstallerError> {
            let part = part.ok_or_else(|| {
                InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                    .with_diagnostic_message("macOS version is not a numeric version")
            })?;
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(
                    InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                        .with_diagnostic_message("macOS version is not a numeric version"),
                );
            }
            part.parse::<u16>().map_err(|_| {
                InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                    .with_diagnostic_message("macOS version is outside the supported range")
            })
        };

        let major = parse_part(parts.next())?;
        let minor = match parts.next() {
            Some(value) => parse_part(Some(value))?,
            None => 0,
        };
        let patch = match parts.next() {
            Some(value) => parse_part(Some(value))?,
            None => 0,
        };
        if parts.next().is_some() {
            return Err(
                InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                    .with_diagnostic_message("macOS version has too many components"),
            );
        }

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

/// Host facts are injected so unit tests never need to inspect the actual
/// workstation. The Applications roots are trusted local configuration, not
/// renderer input or mirror metadata.
#[derive(Debug, Clone)]
pub struct MacosHost {
    architecture: CpuArchitecture,
    os_version: MacosVersion,
    applications_dir: PathBuf,
    user_applications_dir: PathBuf,
}

impl MacosHost {
    pub fn new(
        architecture: CpuArchitecture,
        os_version: &str,
        applications_dir: PathBuf,
        user_applications_dir: PathBuf,
    ) -> Result<Self, InstallerError> {
        Ok(Self {
            architecture,
            os_version: MacosVersion::parse(os_version)?,
            applications_dir,
            user_applications_dir,
        })
    }

    #[cfg(target_os = "macos")]
    pub fn for_current_host() -> Result<Self, InstallerError> {
        let output = SystemCommandRunner
            .run(&command("sw_vers", vec![OsString::from("-productVersion")]))
            .map_err(|_| {
                InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                    .with_diagnostic_message("macOS version could not be detected")
            })?;
        if !output.is_success() {
            return Err(InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("macOS version could not be detected"));
        }
        let version = std::str::from_utf8(output.stdout()).map_err(|_| {
            InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("macOS version could not be decoded")
        })?;
        let home = std::env::var_os("HOME").ok_or_else(|| {
            InstallerError::new(InstallerErrorCode::PlatformUnsupported)
                .with_diagnostic_message("macOS home directory could not be determined")
        })?;
        let architecture = match std::env::consts::ARCH {
            "aarch64" => CpuArchitecture::Aarch64,
            "x86_64" => CpuArchitecture::X86_64UnsupportedMac,
            _ => CpuArchitecture::Unsupported,
        };
        Self::new(
            architecture,
            version.trim(),
            PathBuf::from("/Applications"),
            PathBuf::from(home).join("Applications"),
        )
    }

    pub(crate) fn architecture(&self) -> CpuArchitecture {
        self.architecture
    }

    pub(crate) fn os_version(&self) -> MacosVersion {
        self.os_version
    }

    pub(crate) fn applications_dir(&self) -> &Path {
        &self.applications_dir
    }

    pub(crate) fn user_applications_dir(&self) -> &Path {
        &self.user_applications_dir
    }
}

/// Apple-Silicon platform adapter. Its public constructor accepts injectable
/// host facts, command execution, and filesystem capabilities; this keeps all
/// macOS tests entirely fake and gives platform root a side-effect-free object
/// to construct during application setup.
pub(crate) struct MacosPlatformAdapter {
    runner: Arc<dyn CommandRunner>,
    filesystem: Arc<dyn MacosFilesystem>,
    host: MacosHost,
}

impl MacosPlatformAdapter {
    pub(crate) fn new(
        runner: Arc<dyn CommandRunner>,
        filesystem: Arc<dyn MacosFilesystem>,
        host: MacosHost,
    ) -> Self {
        Self {
            runner,
            filesystem,
            host,
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn for_current_host() -> Result<Self, InstallerError> {
        Ok(Self::new(
            Arc::new(SystemCommandRunner),
            Arc::new(StdMacosFilesystem),
            MacosHost::for_current_host()?,
        ))
    }

    fn host_support_error(&self) -> Option<InstallerError> {
        match self.host.architecture() {
            CpuArchitecture::Aarch64
                if self.host.os_version().major >= MACOS_MINIMUM_MAJOR_VERSION =>
            {
                None
            }
            CpuArchitecture::Aarch64 => Some(
                InstallerError::new(InstallerErrorCode::OsVersionUnsupported)
                    .with_diagnostic_message("macOS 14 or later is required for the installer"),
            ),
            architecture => Some(
                InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                    .with_context("architecture", architecture.as_str())
                    .with_diagnostic_message("macOS V1 supports Apple Silicon only"),
            ),
        }
    }
}

impl CodexDesktopPlatform for MacosPlatformAdapter {
    fn platform(&self) -> Option<DesktopPlatform> {
        Some(DesktopPlatform::Macos)
    }

    fn architecture(&self) -> CpuArchitecture {
        self.host.architecture()
    }

    fn inspect_local(&self) -> BoxFuture<'_, Result<LocalInstallStatus, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        Box::pin(async move {
            if host.architecture() != CpuArchitecture::Aarch64 {
                return Ok(LocalInstallStatus::Unsupported {
                    reason: UnsupportedReason::Architecture,
                });
            }
            if host.os_version().major < MACOS_MINIMUM_MAJOR_VERSION {
                return Ok(LocalInstallStatus::Unsupported {
                    reason: UnsupportedReason::OsVersion,
                });
            }
            run_blocking(move || bundle::inspect_local(runner.as_ref(), filesystem.as_ref(), &host))
                .await
        })
    }

    fn inspect_restart_candidates(
        &self,
    ) -> BoxFuture<'_, Result<RestartCandidateInspection, InstallerError>> {
        // v1.0.2 deliberately does not reuse the legacy macOS bundle-path
        // inspection as lifecycle authority. The target Bundle ID has not
        // received the required independent production evidence, so any
        // restart request must fail closed before it can enumerate a process,
        // send a close message, or launch an app. This also prevents a path,
        // display-name, or title fallback from creeping into the adapter.
        Box::pin(async { Ok(RestartCandidateInspection::UntrustedTarget) })
    }

    fn preflight<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        temp_root: &'a Path,
    ) -> BoxFuture<'a, Result<PlatformInstallPlan, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        let release = release.clone();
        let temp_root = temp_root.to_path_buf();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                dmg::preflight(
                    runner.as_ref(),
                    filesystem.as_ref(),
                    &host,
                    &release,
                    &temp_root,
                )
            })
            .await
        })
    }

    fn verify_package<'a>(
        &'a self,
        release: &'a ReleaseDescriptor,
        artifact: &'a DownloadedArtifact,
    ) -> BoxFuture<'a, Result<VerifiedPackage, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        let release = release.clone();
        let artifact = artifact.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                dmg::verify_package(
                    runner.as_ref(),
                    filesystem.as_ref(),
                    &host,
                    &release,
                    artifact,
                )
            })
            .await
        })
    }

    fn install_current_user<'a>(
        &'a self,
        package: &'a VerifiedPackage,
        progress: PlatformProgressSink,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        let package = package.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                dmg::install_current_user(
                    runner.as_ref(),
                    filesystem.as_ref(),
                    &host,
                    &package,
                    progress,
                )
            })
            .await
        })
    }

    fn launch<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let host = self.host.clone();
        let installed = installed.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                bundle::launch_verified(runner.as_ref(), filesystem.as_ref(), &host, &installed)
            })
            .await
        })
    }

    fn inspect_runtime<'a>(
        &'a self,
        installed: &'a InstalledApplication,
    ) -> BoxFuture<'a, Result<RuntimeInspection, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let installed = installed.clone();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                bundle::inspect_runtime(runner.as_ref(), filesystem.as_ref(), &installed)
            })
            .await
        })
    }

    fn force_shutdown<'a>(
        &'a self,
        installed: &'a InstalledApplication,
        instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<(), InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let installed = installed.clone();
        let instances = instances.to_vec();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                bundle::force_shutdown(runner.as_ref(), filesystem.as_ref(), &installed, &instances)
            })
            .await
        })
    }

    fn is_runtime_instance_running<'a>(
        &'a self,
        installed: &'a InstalledApplication,
        instances: &'a [TrustedRuntimeInstance],
    ) -> BoxFuture<'a, Result<bool, InstallerError>> {
        let runner = self.runner.clone();
        let filesystem = self.filesystem.clone();
        let installed = installed.clone();
        let instances = instances.to_vec();
        let host_error = self.host_support_error();
        Box::pin(async move {
            if let Some(error) = host_error {
                return Err(error);
            }
            run_blocking(move || {
                bundle::is_runtime_instance_running(
                    runner.as_ref(),
                    filesystem.as_ref(),
                    &installed,
                    &instances,
                )
            })
            .await
        })
    }
}

async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, InstallerError> + Send + 'static,
) -> Result<T, InstallerError> {
    tokio::task::spawn_blocking(operation).await.map_err(|_| {
        InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("macOS platform worker stopped unexpectedly")
    })?
}

pub(crate) fn command(
    program: &'static str,
    arguments: impl IntoIterator<Item = OsString>,
) -> CommandInvocation {
    CommandInvocation::new(program, arguments, COMMAND_TIMEOUT)
}

pub(crate) fn stable_bundle_id() -> &'static str {
    MACOS_CODEX_STABLE_IDENTITY
}

pub(crate) fn error(code: InstallerErrorCode, message: &'static str) -> InstallerError {
    InstallerError::new(code).with_diagnostic_message(message)
}

pub(crate) fn is_not_found(error: MacosFilesystemError) -> bool {
    error.kind() == MacosFilesystemErrorKind::NotFound
}

pub(crate) fn is_permission_denied(error: MacosFilesystemError) -> bool {
    error.kind() == MacosFilesystemErrorKind::PermissionDenied
}

#[cfg(test)]
pub(super) mod test_support {
    //! In-memory command/filesystem boundaries shared by the macOS unit tests.
    //!
    //! They intentionally model only the narrow adapter traits. No test here
    //! reaches a host `hdiutil`, `ditto`, `open`, or Applications directory.

    use std::{
        collections::{BTreeMap, HashMap, VecDeque},
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use super::{
        CommandInvocation, CommandOutput, CommandRunner, CommandRunnerError,
        CommandRunnerErrorKind, MacosFileKind, MacosFilesystem, MacosFilesystemError,
        MacosFilesystemErrorKind,
    };

    #[derive(Clone)]
    struct FakeEntry {
        kind: MacosFileKind,
        canonical: PathBuf,
    }

    #[derive(Default)]
    struct FakeFilesystemState {
        entries: BTreeMap<PathBuf, FakeEntry>,
        create_dir_failures: HashMap<PathBuf, MacosFilesystemErrorKind>,
        rename_failures: HashMap<(PathBuf, PathBuf), MacosFilesystemErrorKind>,
    }

    /// An in-memory filesystem with component-aware directory moves. Tests
    /// can configure only the errors relevant to their transaction branch.
    #[derive(Clone, Default)]
    pub struct FakeFilesystem {
        state: Arc<Mutex<FakeFilesystemState>>,
    }

    impl FakeFilesystem {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn add_dir(&self, path: impl AsRef<Path>) {
            let mut state = self.lock();
            insert_directory_and_ancestors(&mut state.entries, path.as_ref());
        }

        pub fn add_file(&self, path: impl AsRef<Path>) {
            let path = path.as_ref().to_path_buf();
            let mut state = self.lock();
            if let Some(parent) = path.parent() {
                insert_directory_and_ancestors(&mut state.entries, parent);
            }
            state.entries.insert(
                path.clone(),
                FakeEntry {
                    kind: MacosFileKind::File,
                    canonical: path,
                },
            );
        }

        pub fn add_symlink(&self, path: impl AsRef<Path>, canonical_target: impl AsRef<Path>) {
            let path = path.as_ref().to_path_buf();
            let mut state = self.lock();
            if let Some(parent) = path.parent() {
                insert_directory_and_ancestors(&mut state.entries, parent);
            }
            state.entries.insert(
                path,
                FakeEntry {
                    kind: MacosFileKind::Symlink,
                    canonical: canonical_target.as_ref().to_path_buf(),
                },
            );
        }

        pub fn contains(&self, path: impl AsRef<Path>) -> bool {
            self.lock().entries.contains_key(path.as_ref())
        }

        pub fn fail_create_dir_under(
            &self,
            path: impl AsRef<Path>,
            kind: MacosFilesystemErrorKind,
        ) {
            self.lock()
                .create_dir_failures
                .insert(path.as_ref().to_path_buf(), kind);
        }

        pub fn fail_rename(
            &self,
            source: impl AsRef<Path>,
            destination: impl AsRef<Path>,
            kind: MacosFilesystemErrorKind,
        ) {
            self.lock().rename_failures.insert(
                (
                    source.as_ref().to_path_buf(),
                    destination.as_ref().to_path_buf(),
                ),
                kind,
            );
        }

        /// Model a successful `ditto` without involving the host filesystem.
        pub fn copy_tree(
            &self,
            source: impl AsRef<Path>,
            destination: impl AsRef<Path>,
        ) -> Result<(), MacosFilesystemError> {
            let source = source.as_ref();
            let destination = destination.as_ref();
            let mut state = self.lock();
            copy_tree(&mut state.entries, source, destination)
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, FakeFilesystemState> {
            self.state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }

    impl MacosFilesystem for FakeFilesystem {
        fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, MacosFilesystemError> {
            let state = self.lock();
            match state.entries.get(path) {
                Some(FakeEntry {
                    kind: MacosFileKind::Directory,
                    ..
                }) => {}
                Some(_) => return Err(filesystem_error(MacosFilesystemErrorKind::Invalid)),
                None => return Err(filesystem_error(MacosFilesystemErrorKind::NotFound)),
            }
            Ok(state
                .entries
                .keys()
                .filter(|candidate| candidate.parent() == Some(path))
                .cloned()
                .collect())
        }

        fn canonicalize(&self, path: &Path) -> Result<PathBuf, MacosFilesystemError> {
            self.lock()
                .entries
                .get(path)
                .map(|entry| entry.canonical.clone())
                .ok_or_else(|| filesystem_error(MacosFilesystemErrorKind::NotFound))
        }

        fn file_kind(&self, path: &Path) -> Result<MacosFileKind, MacosFilesystemError> {
            self.lock()
                .entries
                .get(path)
                .map(|entry| entry.kind)
                .ok_or_else(|| filesystem_error(MacosFilesystemErrorKind::NotFound))
        }

        fn create_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError> {
            let mut state = self.lock();
            if let Some(kind) = state
                .create_dir_failures
                .iter()
                .find_map(|(root, kind)| path.starts_with(root).then_some(*kind))
            {
                return Err(filesystem_error(kind));
            }
            insert_directory_and_ancestors(&mut state.entries, path);
            Ok(())
        }

        fn rename(&self, source: &Path, destination: &Path) -> Result<(), MacosFilesystemError> {
            let mut state = self.lock();
            if let Some(kind) = state
                .rename_failures
                .get(&(source.to_path_buf(), destination.to_path_buf()))
                .copied()
            {
                return Err(filesystem_error(kind));
            }
            move_tree(&mut state.entries, source, destination)
        }

        fn remove_dir_all(&self, path: &Path) -> Result<(), MacosFilesystemError> {
            let mut state = self.lock();
            if state.entries.get(path).map(|entry| entry.kind) != Some(MacosFileKind::Directory) {
                return Err(filesystem_error(MacosFilesystemErrorKind::NotFound));
            }
            let paths = state
                .entries
                .keys()
                .filter(|candidate| candidate.starts_with(path))
                .cloned()
                .collect::<Vec<_>>();
            for path in paths {
                state.entries.remove(&path);
            }
            Ok(())
        }
    }

    fn insert_directory_and_ancestors(entries: &mut BTreeMap<PathBuf, FakeEntry>, path: &Path) {
        let mut ancestors = path.ancestors().collect::<Vec<_>>();
        ancestors.reverse();
        for ancestor in ancestors {
            if ancestor.as_os_str().is_empty() {
                continue;
            }
            entries
                .entry(ancestor.to_path_buf())
                .or_insert_with(|| FakeEntry {
                    kind: MacosFileKind::Directory,
                    canonical: ancestor.to_path_buf(),
                });
        }
    }

    fn copy_tree(
        entries: &mut BTreeMap<PathBuf, FakeEntry>,
        source: &Path,
        destination: &Path,
    ) -> Result<(), MacosFilesystemError> {
        if entries.get(source).map(|entry| entry.kind) != Some(MacosFileKind::Directory) {
            return Err(filesystem_error(MacosFilesystemErrorKind::NotFound));
        }
        if entries.contains_key(destination) {
            return Err(filesystem_error(MacosFilesystemErrorKind::AlreadyExists));
        }
        let copied = entries
            .iter()
            .filter(|(path, _)| path.starts_with(source))
            .map(|(path, entry)| (path.clone(), entry.clone()))
            .collect::<Vec<_>>();
        for (path, mut entry) in copied {
            let suffix = path
                .strip_prefix(source)
                .expect("copied source path remains below source root");
            let destination_path = destination.join(suffix);
            if entry.canonical.starts_with(source) {
                let canonical_suffix = entry
                    .canonical
                    .strip_prefix(source)
                    .expect("canonical copied path remains below source root");
                entry.canonical = destination.join(canonical_suffix);
            }
            entries.insert(destination_path, entry);
        }
        Ok(())
    }

    fn move_tree(
        entries: &mut BTreeMap<PathBuf, FakeEntry>,
        source: &Path,
        destination: &Path,
    ) -> Result<(), MacosFilesystemError> {
        if entries.get(source).map(|entry| entry.kind) != Some(MacosFileKind::Directory) {
            return Err(filesystem_error(MacosFilesystemErrorKind::NotFound));
        }
        if entries.contains_key(destination) {
            return Err(filesystem_error(MacosFilesystemErrorKind::AlreadyExists));
        }
        let moved = entries
            .iter()
            .filter(|(path, _)| path.starts_with(source))
            .map(|(path, entry)| (path.clone(), entry.clone()))
            .collect::<Vec<_>>();
        for (path, _) in &moved {
            entries.remove(path);
        }
        for (path, mut entry) in moved {
            let suffix = path
                .strip_prefix(source)
                .expect("moved source path remains below source root");
            let destination_path = destination.join(suffix);
            if entry.canonical.starts_with(source) {
                let canonical_suffix = entry
                    .canonical
                    .strip_prefix(source)
                    .expect("canonical moved path remains below source root");
                entry.canonical = destination.join(canonical_suffix);
            }
            entries.insert(destination_path, entry);
        }
        Ok(())
    }

    fn filesystem_error(kind: MacosFilesystemErrorKind) -> MacosFilesystemError {
        MacosFilesystemError { kind }
    }

    #[derive(Clone)]
    enum QueuedCommandResult {
        Output(CommandOutput),
        Error(CommandRunnerError),
    }

    #[derive(Clone)]
    struct QueuedCommand {
        program: &'static str,
        result: QueuedCommandResult,
    }

    type CommandHook = Arc<dyn Fn(&CommandInvocation) + Send + Sync>;

    /// Queue-based fake runner. It asserts the exact command order while
    /// keeping command arguments available for security assertions.
    #[derive(Default)]
    pub struct FakeRunner {
        queued: Mutex<VecDeque<QueuedCommand>>,
        invocations: Mutex<Vec<CommandInvocation>>,
        hook: Mutex<Option<CommandHook>>,
    }

    impl FakeRunner {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn queue_success(&self, program: &'static str, stdout: impl Into<Vec<u8>>) {
            self.queue(
                program,
                QueuedCommandResult::Output(CommandOutput::success(stdout)),
            );
        }

        pub fn queue_failure(
            &self,
            program: &'static str,
            status_code: Option<i32>,
            stderr: impl Into<Vec<u8>>,
        ) {
            self.queue(
                program,
                QueuedCommandResult::Output(CommandOutput::failure(status_code, stderr)),
            );
        }

        pub fn queue_error(&self, program: &'static str, kind: CommandRunnerErrorKind) {
            self.queue(
                program,
                QueuedCommandResult::Error(CommandRunnerError { kind }),
            );
        }

        pub fn set_hook(&self, hook: CommandHook) {
            *self
                .hook
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(hook);
        }

        pub fn invocations(&self) -> Vec<CommandInvocation> {
            self.invocations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }

        pub fn assert_drained(&self) {
            let queued = self
                .queued
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            assert!(queued.is_empty(), "unused fake command responses remain");
        }

        fn queue(&self, program: &'static str, result: QueuedCommandResult) {
            self.queued
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push_back(QueuedCommand { program, result });
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
            let queued = self
                .queued
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .pop_front()
                .expect("unexpected macOS command invocation");
            assert_eq!(queued.program, invocation.program());
            self.invocations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(invocation.clone());
            if let Some(hook) = self
                .hook
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
            {
                hook(invocation);
            }
            match queued.result {
                QueuedCommandResult::Output(output) => Ok(output),
                QueuedCommandResult::Error(error) => Err(error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use futures::executor::block_on;

    use super::{
        test_support::{FakeFilesystem, FakeRunner},
        CommandOutput, CpuArchitecture, DiskSpaceProbe, LocalInstallStatus, MacosDiskSpaceProbe,
        MacosHost, MacosPlatformAdapter, MacosVersion, UnsupportedReason, MAX_COMMAND_OUTPUT_BYTES,
    };
    use crate::codex_desktop::platform::CodexDesktopPlatform;

    #[test]
    fn version_parsing_uses_numeric_components() {
        assert!(MacosVersion::parse("14").unwrap() < MacosVersion::parse("14.1").unwrap());
        assert!(MacosVersion::parse("14.1.9").unwrap() < MacosVersion::parse("14.2").unwrap());
        for invalid in ["", "14.", "14.beta", "14.0.1.2", "65536"] {
            assert!(
                MacosVersion::parse(invalid).is_err(),
                "{invalid} must be rejected"
            );
        }
    }

    #[test]
    fn command_output_is_bounded_before_test_diagnostics_can_observe_it() {
        let output = CommandOutput::success(vec![b'x'; MAX_COMMAND_OUTPUT_BYTES + 1024]);
        assert_eq!(output.stdout().len(), MAX_COMMAND_OUTPUT_BYTES);
    }

    #[test]
    fn disk_space_probe_uses_fixed_df_arguments_and_caches_the_snapshot() {
        let runner = Arc::new(FakeRunner::new());
        runner.queue_success(
            "df",
            b"Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk3s1 1000 1 999 1% /\n"
                .to_vec(),
        );
        let probe = MacosDiskSpaceProbe::new(runner.clone());

        let volume = probe.volume_key(Path::new("/Applications")).unwrap();
        assert_eq!(probe.available_bytes(&volume).unwrap(), 999 * 1024);
        runner.assert_drained();

        let invocation = runner.invocations().pop().unwrap();
        assert_eq!(invocation.program(), "df");
        assert_eq!(invocation.arguments()[0], "-Pk");
        assert_eq!(invocation.arguments()[1], "/Applications");
    }

    #[test]
    fn intel_host_is_explicitly_unsupported_without_touching_a_real_host() {
        let runner = Arc::new(FakeRunner::new());
        let filesystem = Arc::new(FakeFilesystem::new());
        let host = MacosHost::new(
            CpuArchitecture::X86_64UnsupportedMac,
            "14.0",
            "/Applications".into(),
            "/Users/test/Applications".into(),
        )
        .unwrap();
        let adapter = MacosPlatformAdapter::new(runner.clone(), filesystem, host);

        assert_eq!(
            block_on(adapter.inspect_local()).unwrap(),
            LocalInstallStatus::Unsupported {
                reason: UnsupportedReason::Architecture,
            }
        );
        runner.assert_drained();
    }
}
