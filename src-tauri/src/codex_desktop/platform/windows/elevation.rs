//! Windows-only pre-runtime entry point for experimental all-users provisioning.
//!
//! This module is intentionally not part of the normal `CodexDesktopPlatform`
//! trait or any Tauri command.  It accepts only the fixed flags parsed by the
//! generic protocol, uses `runas` on the current executable, and hands the
//! elevated child only a fixed control-file path plus a nonce.

use std::{
    ffi::OsString,
    fs::{self, File},
    io::{copy, Read, Write},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    sync::Arc,
};

use chrono::Utc;
use uuid::Uuid;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{LocalFree, GENERIC_READ, GENERIC_WRITE, HANDLE, HLOCAL},
        Security::{
            Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
            PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
        },
        Storage::FileSystem::{
            CreateDirectoryW, CreateFileW, FileAttributeTagInfo, GetDriveTypeW,
            GetFileInformationByHandleEx, GetFinalPathNameByHandleW, CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_SHARE_NONE, FILE_SHARE_READ, OPEN_EXISTING, VOLUME_NAME_DOS,
        },
        System::Com::CoTaskMemFree,
        System::WindowsProgramming::DRIVE_FIXED,
        UI::Shell::{
            FOLDERID_ProgramData, SHGetKnownFolderPath, ShellExecuteExW, KNOWN_FOLDER_FLAG,
            SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
        },
    },
};

use super::{
    current_official_publisher_evidence,
    deployment::{stage_and_provision_all_users, SystemWindowsDiskSpaceProbe},
    revalidate_all_users_package, WindowsHost,
};
use crate::codex_desktop::{
    all_users::{
        elevate_new_job, parse_headless_invocation, run_elevated_provisioning, write_new_job,
        AllUsersElevator, AllUsersJobControlReader, AllUsersPackageValidator, AllUsersProvisioner,
        AllUsersReleaseAnchor, AllUsersTrustPolicy, ElevatedProvisioningDependencies,
        HeadlessInvocation, ValidatedAllUsersJob, ELEVATED_ALL_USERS_FLAG, HEADLESS_EXIT_FAILURE,
        HEADLESS_EXIT_INVALID_ARGUMENTS, HEADLESS_EXIT_SUCCESS, HEADLESS_EXIT_UNSUPPORTED,
        MAX_JOB_FILE_BYTES,
    },
    cancellation::NeverCancelled,
    download::{download_release, DownloadProgressUpdate, HttpTransport},
    error::{InstallerError, InstallerErrorCode},
    runtime::{InstallerMetadataFetcher, InstallerTransportPurpose, RuntimeInstallerTransport},
    source::{AgentsMirrorSource, CacheMode, ReleaseSource},
    temp::JobTempDir,
    types::{CpuArchitecture, DesktopPlatform, ReleaseDescriptor},
    verify::{ensure_required_disk_space, verify_file},
};

/// Called by `main` before Tauri creates a runtime.  It deliberately returns
/// `None` for non-reserved arguments so existing normal startup behaviour is
/// not changed, but malformed use of either reserved flag cannot fall through
/// into the GUI.
pub(crate) fn maybe_run_from_process() -> Option<i32> {
    match parse_headless_invocation(std::env::args_os()) {
        HeadlessInvocation::NormalApplication => None,
        HeadlessInvocation::InvalidReservedArguments => Some(HEADLESS_EXIT_INVALID_ARGUMENTS),
        HeadlessInvocation::PrepareAllUsers => Some(run_parent()),
        HeadlessInvocation::ElevatedProvision { job_file, nonce } => {
            Some(run_elevated_child(&job_file, &nonce))
        }
    }
}

fn run_parent() -> i32 {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| {
            InstallerError::new(InstallerErrorCode::InternalError)
                .with_diagnostic_message("all-users headless runtime could not be created")
        });
    let result = runtime.and_then(|runtime| runtime.block_on(prepare_and_elevate()));
    headless_result_exit_code(result)
}

fn run_elevated_child(job_file: &Path, nonce: &str) -> i32 {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| {
            InstallerError::new(InstallerErrorCode::InternalError)
                .with_diagnostic_message("all-users headless runtime could not be created")
        });
    let result =
        runtime.and_then(|runtime| runtime.block_on(provision_elevated_child(job_file, nonce)));
    headless_result_exit_code(result)
}

async fn provision_elevated_child(job_file: &Path, nonce: &str) -> Result<(), InstallerError> {
    let host = WindowsHost::for_current_host()?;
    let publisher_evidence = current_official_publisher_evidence()?;
    let policy = AllUsersTrustPolicy::new(
        crate::codex_desktop::platform::WINDOWS_CODEX_STABLE_IDENTITY,
        publisher_evidence.publisher(),
        host.architecture(),
    )?;
    // The nonce constrains command-line shape but is not an authorization MAC
    // for the parent-owned job. The elevated child therefore resolves its own
    // fixed official descriptor and uses it as the deployment trust anchor.
    let release_anchor = ResolvedAllUsersReleaseAnchor {
        release: resolve_latest_windows_release(host.architecture()).await?,
    };
    let job_control_reader = WindowsAllUsersJobControlReader;
    let validator = WindowsAllUsersPackageValidator;
    let provisioner = WindowsAllUsersProvisioner;
    let disk_space_probe = SystemWindowsDiskSpaceProbe::new();
    let temporary_root = JobTempDir::system_root();
    validate_existing_local_temp_root(&temporary_root)?;
    run_elevated_provisioning(
        ElevatedProvisioningDependencies::new(
            &temporary_root,
            &policy,
            &release_anchor,
            &validator,
            &disk_space_probe,
            host.deployment_volume(),
            &provisioner,
        ),
        &job_control_reader,
        job_file,
        nonce,
        Utc::now(),
    )
}

async fn resolve_latest_windows_release(
    architecture: CpuArchitecture,
) -> Result<ReleaseDescriptor, InstallerError> {
    let metadata_transport: Arc<dyn HttpTransport> = Arc::new(RuntimeInstallerTransport::new(
        InstallerTransportPurpose::Metadata,
        installer_user_agent(),
    ));
    let source =
        AgentsMirrorSource::new(Arc::new(InstallerMetadataFetcher::new(metadata_transport)));
    let cancellation = NeverCancelled;
    source
        .resolve_latest(
            DesktopPlatform::Windows,
            architecture,
            CacheMode::ForceRefresh,
            &cancellation,
        )
        .await
}

struct ResolvedAllUsersReleaseAnchor {
    release: ReleaseDescriptor,
}

impl AllUsersReleaseAnchor for ResolvedAllUsersReleaseAnchor {
    fn resolve_release(&self) -> Result<ReleaseDescriptor, InstallerError> {
        Ok(self.release.clone())
    }
}

async fn prepare_and_elevate() -> Result<(), InstallerError> {
    let host = WindowsHost::for_current_host()?;
    let publisher_evidence = current_official_publisher_evidence()?;
    let policy = AllUsersTrustPolicy::new(
        crate::codex_desktop::platform::WINDOWS_CODEX_STABLE_IDENTITY,
        publisher_evidence.publisher(),
        host.architecture(),
    )?;
    let release = resolve_latest_windows_release(host.architecture()).await?;
    let cancellation = NeverCancelled;

    let temporary_root = JobTempDir::system_root();
    validate_local_temp_root_parent(&temporary_root)?;
    let directory = JobTempDir::create(&temporary_root, &Uuid::new_v4().hyphenated().to_string())?;
    validate_existing_local_temp_root(&temporary_root)?;
    let outcome = async {
        // Download remains subject to the same conservative three-volume
        // capacity policy as the normal installer, including the system volume
        // that PackageManager will use for staging/provisioning.
        let disk_space_probe = SystemWindowsDiskSpaceProbe::new();
        ensure_required_disk_space(
            &disk_space_probe,
            [directory.path(), host.deployment_volume()],
            release.expected_size,
        )?;
        let download_transport: Arc<dyn HttpTransport> = Arc::new(RuntimeInstallerTransport::new(
            InstallerTransportPurpose::Download,
            installer_user_agent(),
        ));
        let no_progress = |_update: DownloadProgressUpdate| {};
        let artifact = download_release(
            download_transport.as_ref(),
            &release,
            &directory,
            &cancellation,
            &no_progress,
        )
        .await?;
        // The downloader verifies the same fields, but a second explicit check
        // makes the parent-to-child package boundary unambiguous in this
        // independent experimental path.
        verify_file(
            artifact.path(),
            release.expected_size,
            &release.expected_sha256,
        )?;
        revalidate_all_users_package(&release, artifact.path())?;

        let nonce = Uuid::new_v4().simple().to_string();
        let job = crate::codex_desktop::all_users::AllUsersJob::from_verified_parent(
            &directory,
            &release,
            &policy,
            &nonce,
            Utc::now(),
        )?;
        write_new_job(&directory, &job)?;
        elevate_new_job(&directory, &nonce, &WindowsShellElevator)
    }
    .await;

    // After a successful `runas` handoff the child owns the fixed temporary
    // job and reports only a stable headless exit code; it never writes a
    // result into the parent-owned tree. This parent cannot observe child
    // completion, so stale cleanup owns eventual removal. Before handoff no
    // UAC child can rely on the directory, so remove only known
    // capability-owned files on failure.
    if outcome.is_err() {
        let _ = directory.cleanup();
    }
    outcome
}

/// Reads the parent-owned job control through the same native handle that
/// establishes its capability path. The generic protocol deliberately has no
/// fallback path-based read, so this is the only elevated JSON ingress.
struct WindowsAllUsersJobControlReader;

impl AllUsersJobControlReader for WindowsAllUsersJobControlReader {
    fn read_job_control(
        &self,
        expected_job_path: &Path,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, InstallerError> {
        // Keep the hard cap local to the elevated implementation as well as
        // the target-neutral protocol. A future caller cannot make a UAC
        // child allocate more than the V1 control-file limit.
        let maximum_bytes = maximum_bytes.min(MAX_JOB_FILE_BYTES);
        let mut file = open_untrusted_job_control_no_follow(expected_job_path)?;
        let metadata = file.metadata().map_err(|_| {
            protected_staging_error("all-users job control metadata could not be read")
        })?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum_bytes {
            return Err(protected_staging_error(
                "all-users job control file is outside its size limit",
            ));
        }

        // This consumes no more than 16 KiB from the already verified handle;
        // it must never reopen `expected_job_path` after the capability checks.
        let mut bounded = Read::by_ref(&mut file).take(maximum_bytes);
        let mut bytes = Vec::new();
        bounded
            .read_to_end(&mut bytes)
            .map_err(|_| protected_staging_error("all-users job control file could not be read"))?;
        if bytes.is_empty() || bytes.len() as u64 > maximum_bytes {
            return Err(protected_staging_error(
                "all-users job control file is outside its size limit",
            ));
        }
        Ok(bytes)
    }
}

struct WindowsAllUsersPackageValidator;

impl AllUsersPackageValidator for WindowsAllUsersPackageValidator {
    fn revalidate_package(
        &self,
        _job: &ValidatedAllUsersJob,
        release: &ReleaseDescriptor,
    ) -> Result<(), InstallerError> {
        // Do not parse the parent-owned package path in the elevated child.
        // The provisioner revalidates the protected copy below; this adapter
        // only rejects an anchor that cannot describe the current host.
        let host = WindowsHost::for_current_host()?;
        if release.platform != DesktopPlatform::Windows
            || release.architecture != host.architecture()
        {
            return Err(
                InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                    .with_diagnostic_message(
                        "all-users release anchor does not match the current Windows host",
                    ),
            );
        }
        Ok(())
    }
}

struct WindowsAllUsersProvisioner;

impl AllUsersProvisioner for WindowsAllUsersProvisioner {
    fn stage_and_provision(
        &self,
        job: &ValidatedAllUsersJob,
        release: &ReleaseDescriptor,
    ) -> Result<(), InstallerError> {
        // The job directory is owned by the unelevated parent.  Never hand
        // that mutable user-temp pathname directly to PackageManager after a
        // check: first copy it into a newly-created elevated-only directory,
        // then hash and parse that protected copy immediately before Stage.
        let staging = ProtectedStagingDirectory::create()?;
        let outcome = (|| {
            let protected_package = staging.copy_from_untrusted_source(job.package_path())?;
            verify_file(
                &protected_package,
                release.expected_size,
                &release.expected_sha256,
            )?;
            revalidate_all_users_package(release, &protected_package)?;
            stage_and_provision_all_users(
                &protected_package,
                job.expected_identity(),
                job.expected_publisher(),
                &release.platform_version,
                job.expected_architecture(),
            )
        })();
        let cleanup = staging.cleanup();
        match (outcome, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Err(cleanup_error)) | (Err(_), Err(cleanup_error)) => Err(cleanup_error),
        }
    }
}

/// The SDDL creates a protected DACL and assigns ownership to the elevated
/// Administrators group rather than the unelevated interactive user.  This is
/// a deliberate boundary: ordinary user tokens cannot change the DACL or
/// replace the file after the copy.  Administrators/SYSTEM remain trusted
/// principals because they can already control machine-wide provisioning.
const PROTECTED_STAGING_SDDL: &str = "O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";
const PROTECTED_STAGING_FILE_NAME: &str = "installer.msix";

struct ProtectedStagingDirectory {
    path: PathBuf,
}

impl ProtectedStagingDirectory {
    fn create() -> Result<Self, InstallerError> {
        let program_data = canonical_program_data_directory()?;
        let candidate = program_data.join(format!(
            "FyAgent-Codex-AllUsers-{}",
            Uuid::new_v4().hyphenated()
        ));
        let descriptor = ProtectedSecurityDescriptor::new()?;
        let attributes = descriptor.attributes();
        let wide_candidate = wide_path(&candidate)?;
        unsafe { CreateDirectoryW(PCWSTR(wide_candidate.as_ptr()), Some(&attributes)) }.map_err(
            |_| {
                protected_staging_error(
                    "protected all-users staging directory could not be created",
                )
            },
        )?;

        let metadata = fs::symlink_metadata(&candidate).map_err(|_| {
            protected_staging_error("protected all-users staging directory could not be inspected")
        })?;
        if !metadata.is_dir() || is_link_or_reparse_point(&candidate)? {
            return Err(protected_staging_error(
                "protected all-users staging directory is not a regular directory",
            ));
        }
        let canonical = fs::canonicalize(&candidate).map_err(|_| {
            protected_staging_error(
                "protected all-users staging directory could not be canonicalized",
            )
        })?;
        if canonical.parent() != Some(program_data.as_path()) {
            return Err(protected_staging_error(
                "protected all-users staging directory escaped ProgramData",
            ));
        }
        Ok(Self { path: canonical })
    }

    fn copy_from_untrusted_source(&self, source: &Path) -> Result<PathBuf, InstallerError> {
        let destination = self.path.join(PROTECTED_STAGING_FILE_NAME);
        // Open the untrusted parent-owned file without following a reparse
        // point and retain a read-only sharing lock while copying. A handle
        // that existed before this lock may still mutate its own bytes, but
        // the protected destination's mandatory post-copy hash makes such a
        // race fail closed; no untrusted source path is ever passed to Stage.
        let mut source_file = open_untrusted_source_no_follow(source)?;
        let mut destination_file = create_protected_file(&destination)?;
        copy(&mut source_file, &mut destination_file)
            .and_then(|_| destination_file.flush())
            .and_then(|_| destination_file.sync_all())
            .map_err(|_| {
                protected_staging_error("all-users package copy to protected staging failed")
            })?;
        drop(destination_file);

        let metadata = fs::symlink_metadata(&destination).map_err(|_| {
            protected_staging_error("protected all-users package could not be inspected")
        })?;
        if !metadata.is_file() || is_link_or_reparse_point(&destination)? {
            return Err(protected_staging_error(
                "protected all-users package is not a regular non-reparse file",
            ));
        }
        Ok(destination)
    }

    fn cleanup(&self) -> Result<(), InstallerError> {
        let package = self.path.join(PROTECTED_STAGING_FILE_NAME);
        match fs::symlink_metadata(&package) {
            Ok(metadata) => {
                if !metadata.is_file() || is_link_or_reparse_point(&package)? {
                    return Err(protected_staging_error(
                        "protected all-users cleanup refused an unexpected package entry",
                    ));
                }
                fs::remove_file(&package).map_err(|_| {
                    protected_staging_error("protected all-users package could not be removed")
                })?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(protected_staging_error(
                    "protected all-users package could not be inspected for cleanup",
                ))
            }
        }

        let metadata = fs::symlink_metadata(&self.path).map_err(|_| {
            protected_staging_error(
                "protected all-users directory could not be inspected for cleanup",
            )
        })?;
        if !metadata.is_dir() || is_link_or_reparse_point(&self.path)? {
            return Err(protected_staging_error(
                "protected all-users cleanup refused an unexpected directory",
            ));
        }
        fs::remove_dir(&self.path).map_err(|_| {
            protected_staging_error("protected all-users staging directory could not be removed")
        })
    }
}

struct ProtectedSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl ProtectedSecurityDescriptor {
    fn new() -> Result<Self, InstallerError> {
        let sddl = wide_null(PROTECTED_STAGING_SDDL);
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl.as_ptr()),
                1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|_| {
            protected_staging_error("protected all-users security descriptor is unavailable")
        })?;
        if descriptor.is_invalid() {
            return Err(protected_staging_error(
                "protected all-users security descriptor is invalid",
            ));
        }
        Ok(Self(descriptor))
    }

    fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0 .0,
            bInheritHandle: false.into(),
        }
    }
}

impl Drop for ProtectedSecurityDescriptor {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0 .0)));
            }
        }
    }
}

fn create_protected_file(path: &Path) -> Result<File, InstallerError> {
    let descriptor = ProtectedSecurityDescriptor::new()?;
    let attributes = descriptor.attributes();
    let wide_path = wide_path(path)?;
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            Some(&attributes),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|_| {
        protected_staging_error("protected all-users package file could not be created")
    })?;
    // The handle is uniquely owned by the returned File.  `CREATE_NEW` plus
    // the protected parent DACL prevent another process from pre-populating or
    // swapping the fixed destination name.
    Ok(unsafe { File::from_raw_handle(handle.0) })
}

fn open_untrusted_source_no_follow(path: &Path) -> Result<File, InstallerError> {
    open_untrusted_capability_file_no_follow(path)
}

fn open_untrusted_job_control_no_follow(path: &Path) -> Result<File, InstallerError> {
    open_untrusted_capability_file_no_follow(path)
}

/// Opens one parent-owned capability file exactly once. `OPEN_REPARSE_POINT`
/// covers the leaf, while the final-handle comparison below rejects redirected
/// parent components and remote paths before the returned handle is consumed.
fn open_untrusted_capability_file_no_follow(path: &Path) -> Result<File, InstallerError> {
    let wide_path = wide_path(path)?;
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|_| protected_staging_error("all-users capability file could not be opened safely"))?;
    let file = unsafe { File::from_raw_handle(handle.0) };

    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    let capability_handle = HANDLE(file.as_raw_handle());
    unsafe {
        GetFileInformationByHandleEx(
            capability_handle,
            FileAttributeTagInfo,
            &mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO as *mut _,
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    }
    .map_err(|_| {
        protected_staging_error("all-users capability file attributes could not be read")
    })?;
    if attributes.FileAttributes & 0x0400 != 0 {
        return Err(protected_staging_error(
            "all-users capability file changed into a reparse point before use",
        ));
    }
    // OPEN_REPARSE_POINT protects the leaf only. Resolve the opened handle
    // after Windows has traversed every parent component, then require it to
    // be exactly the capability path captured from JobTempDir. This rejects a
    // parent/root junction or UNC redirection that could otherwise be swapped
    // between pathname checks and this elevated open.
    let resolved_path = final_path_from_handle(capability_handle)?;
    if !is_local_windows_fixed_disk_path(&resolved_path) || !same_windows_path(&resolved_path, path)
    {
        return Err(protected_staging_error(
            "all-users capability file escaped its local capability path",
        ));
    }
    let metadata = file.metadata().map_err(|_| {
        protected_staging_error("all-users capability file metadata could not be read")
    })?;
    if !metadata.is_file() {
        return Err(protected_staging_error(
            "all-users capability file handle is not a regular file",
        ));
    }
    Ok(file)
}

fn final_path_from_handle(handle: HANDLE) -> Result<PathBuf, InstallerError> {
    let mut buffer = vec![0_u16; 512];
    loop {
        let length = unsafe { GetFinalPathNameByHandleW(handle, &mut buffer, VOLUME_NAME_DOS) };
        if length == 0 {
            return Err(protected_staging_error(
                "all-users capability file final path could not be determined",
            ));
        }
        if (length as usize) < buffer.len() {
            return Ok(PathBuf::from(OsString::from_wide(
                &buffer[..length as usize],
            )));
        }
        let required = (length as usize).saturating_add(1);
        if required > 32_768 {
            return Err(protected_staging_error(
                "all-users capability file final path is too long",
            ));
        }
        buffer.resize(required, 0);
    }
}

fn same_windows_path(left: &Path, right: &Path) -> bool {
    normalized_windows_path_units(left) == normalized_windows_path_units(right)
}

fn normalized_windows_path_units(path: &Path) -> Vec<u16> {
    let mut units = path.as_os_str().encode_wide().collect::<Vec<_>>();
    const VERBATIM_PREFIX: [u16; 4] = [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
    if units.starts_with(&VERBATIM_PREFIX) {
        units.drain(..VERBATIM_PREFIX.len());
    }
    for unit in &mut units {
        if (b'A' as u16..=b'Z' as u16).contains(unit) {
            *unit += (b'a' - b'A') as u16;
        }
    }
    units
}

fn validate_local_temp_root_parent(root: &Path) -> Result<(), InstallerError> {
    let parent = root.parent().ok_or_else(|| {
        protected_staging_error("all-users temporary root has no local parent directory")
    })?;
    validate_local_existing_directory(parent, "all-users temporary root parent")
}

fn validate_existing_local_temp_root(root: &Path) -> Result<(), InstallerError> {
    validate_local_existing_directory(root, "all-users temporary root")
}

fn validate_local_existing_directory(
    path: &Path,
    label: &'static str,
) -> Result<(), InstallerError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| protected_staging_error("all-users local directory could not be inspected"))?;
    if !metadata.is_dir() || is_link_or_reparse_point(path)? {
        return Err(protected_staging_error(
            "all-users temporary path must be a regular local directory",
        ));
    }
    let canonical = fs::canonicalize(path).map_err(|_| {
        protected_staging_error("all-users local directory could not be canonicalized")
    })?;
    if !is_local_windows_fixed_disk_path(&canonical) {
        return Err(
            InstallerError::new(InstallerErrorCode::WindowsElevationFailed)
                .with_diagnostic_message(label),
        );
    }
    Ok(())
}

fn is_local_windows_fixed_disk_path(path: &Path) -> bool {
    let Some(drive_root) = windows_drive_root(path) else {
        return false;
    };
    let wide_root = wide_null(&drive_root);
    unsafe { GetDriveTypeW(PCWSTR(wide_root.as_ptr())) == DRIVE_FIXED }
}

fn windows_drive_root(path: &Path) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }
    let drive = match path.components().next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
            _ => return None,
        },
        _ => return None,
    };
    Some(format!("{}:\\", char::from(drive)))
}

fn canonical_program_data_directory() -> Result<PathBuf, InstallerError> {
    // Environment variables are caller-controlled and cannot be used as an
    // elevated staging root. The Shell known-folder API supplies the machine
    // ProgramData location independent of TEMP/TMP/ProgramData overrides.
    let raw_path =
        unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramData, KNOWN_FOLDER_FLAG(0), None) }
            .map_err(|_| protected_staging_error("ProgramData known folder is unavailable"))?;
    let decoded = unsafe { raw_path.to_string() };
    unsafe {
        CoTaskMemFree(Some(raw_path.0.cast()));
    }
    let path = PathBuf::from(
        decoded.map_err(|_| protected_staging_error("ProgramData known folder is invalid"))?,
    );
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| protected_staging_error("ProgramData could not be inspected"))?;
    if !metadata.is_dir() || is_link_or_reparse_point(&path)? {
        return Err(protected_staging_error(
            "ProgramData is not a regular directory for protected staging",
        ));
    }
    let canonical = fs::canonicalize(&path)
        .map_err(|_| protected_staging_error("ProgramData could not be canonicalized"))?;
    if !is_local_windows_fixed_disk_path(&canonical) {
        return Err(protected_staging_error(
            "ProgramData is not a local disk path for protected staging",
        ));
    }
    Ok(canonical)
}

fn is_link_or_reparse_point(path: &Path) -> Result<bool, InstallerError> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| protected_staging_error("all-users staging path could not be inspected"))?;
    Ok(metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

fn wide_path(path: &Path) -> Result<Vec<u16>, InstallerError> {
    if path.as_os_str().is_empty() {
        return Err(protected_staging_error("all-users staging path is empty"));
    }
    Ok(path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect())
}

fn protected_staging_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsElevationFailed).with_diagnostic_message(message)
}

struct WindowsShellElevator;

impl AllUsersElevator for WindowsShellElevator {
    fn elevate(&self, job_file: &Path, nonce: &str) -> Result<(), InstallerError> {
        let executable = std::env::current_exe()
            .map_err(|_| elevation_error(InstallerErrorCode::WindowsElevationFailed, None))?;
        let executable = fs::canonicalize(&executable)
            .map_err(|_| elevation_error(InstallerErrorCode::WindowsElevationFailed, None))?;
        let metadata = fs::symlink_metadata(&executable)
            .map_err(|_| elevation_error(InstallerErrorCode::WindowsElevationFailed, None))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(elevation_error(
                InstallerErrorCode::WindowsElevationFailed,
                None,
            ));
        }

        let executable = path_as_utf8(&executable)?;
        let job_file = path_as_utf8(job_file)?;
        let parameters = format!(
            "{ELEVATED_ALL_USERS_FLAG} {} {}",
            quote_windows_argument(&job_file)?,
            quote_windows_argument(nonce)?
        );
        let verb = wide_null("runas");
        let executable = wide_null(&executable);
        let parameters = wide_null(&parameters);
        let mut shell_execute = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(executable.as_ptr()),
            lpParameters: PCWSTR(parameters.as_ptr()),
            // `nShow = 0` keeps the elevated provision child headless.  The
            // only UI it may produce is Windows' own UAC consent prompt.
            nShow: 0,
            ..Default::default()
        };
        unsafe { ShellExecuteExW(&mut shell_execute) }.map_err(|error| {
            let hresult = error.code().0;
            let code = if hresult as u32 == 0x8007_04C7 {
                InstallerErrorCode::WindowsUacCancelled
            } else {
                InstallerErrorCode::WindowsElevationFailed
            };
            elevation_error(code, Some(hresult))
        })
    }
}

fn installer_user_agent() -> String {
    format!(
        "FyAgent/{} codex-desktop-all-users-experimental",
        env!("CARGO_PKG_VERSION")
    )
}

fn path_as_utf8(path: &Path) -> Result<String, InstallerError> {
    path.to_str()
        .map(str::to_owned)
        .filter(|value| !value.is_empty() && !value.contains('\0'))
        .ok_or_else(|| elevation_error(InstallerErrorCode::WindowsElevationFailed, None))
}

fn quote_windows_argument(value: &str) -> Result<String, InstallerError> {
    if value.is_empty() || value.contains('\0') {
        return Err(elevation_error(
            InstallerErrorCode::WindowsElevationFailed,
            None,
        ));
    }
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    let mut backslashes = 0_usize;
    for character in value.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                quoted.extend(std::iter::repeat_n('\\', backslashes.saturating_mul(2) + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.extend(std::iter::repeat_n('\\', backslashes));
                quoted.push(character);
                backslashes = 0;
            }
        }
    }
    quoted.extend(std::iter::repeat_n('\\', backslashes.saturating_mul(2)));
    quoted.push('"');
    Ok(quoted)
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn elevation_error(code: InstallerErrorCode, hresult: Option<i32>) -> InstallerError {
    let mut error = InstallerError::new(code).with_diagnostic_message(
        "Windows could not start the constrained all-users provisioning child",
    );
    if let Some(hresult) = hresult {
        error = error.with_platform_error_code(format!("0x{:08X}", hresult as u32));
    }
    error
}

fn headless_result_exit_code(result: Result<(), InstallerError>) -> i32 {
    match result {
        Ok(()) => HEADLESS_EXIT_SUCCESS,
        Err(error) => {
            // Do not print raw filesystem, command line, PackageManager, or
            // HTTP details from a headless process.  The stable code is enough
            // for an internal experiment operator to correlate diagnostics.
            eprintln!(
                "FyAgent experimental all-users Codex provisioning failed: {:?}",
                error.code()
            );
            match error.code() {
                InstallerErrorCode::PlatformUnsupported
                | InstallerErrorCode::ArchitectureUnsupported
                | InstallerErrorCode::OsVersionUnsupported
                | InstallerErrorCode::WindowsAllUsersUnsupported => HEADLESS_EXIT_UNSUPPORTED,
                _ => HEADLESS_EXIT_FAILURE,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_sddl_has_a_protected_admin_owned_acl() {
        assert!(PROTECTED_STAGING_SDDL.starts_with("O:BA"));
        assert!(PROTECTED_STAGING_SDDL.contains("D:P"));
        assert!(PROTECTED_STAGING_SDDL.contains("(A;OICI;FA;;;SY)"));
        assert!(PROTECTED_STAGING_SDDL.contains("(A;OICI;FA;;;BA)"));
        assert!(!PROTECTED_STAGING_SDDL.contains(";;;BU"));
    }

    #[test]
    fn protected_staging_requires_a_native_fixed_drive_gate() {
        // This remains a static/mock regression on non-Windows CI. Windows
        // HIL must exercise a mapped drive and confirm GetDriveTypeW returns
        // DRIVE_REMOTE, causing the hidden all-users flow to fail closed.
        let source = include_str!("elevation.rs");
        assert!(source.contains("GetDriveTypeW(PCWSTR(wide_root.as_ptr())) == DRIVE_FIXED"));
        assert!(source.contains("Prefix::Disk(letter) | Prefix::VerbatimDisk(letter)"));
        assert!(source.contains("GetFinalPathNameByHandleW"));
        assert!(source.contains("same_windows_path(&resolved_path, path)"));
    }

    #[test]
    fn elevated_job_control_reader_uses_one_verified_handle_and_a_hard_bound() {
        // The Windows-specific behavior is statically covered on non-Windows
        // CI. HIL still owns exercising an actual reparse/mapped-drive input.
        let source = include_str!("elevation.rs");
        let reader = concat!("WindowsAllUsers", "JobControlReader");
        let no_follow_open = concat!(
            "open_untrusted_job_control_",
            "no_follow(expected_job_path)"
        );
        let hard_cap = concat!("maximum_bytes.min(", "MAX_JOB_FILE_BYTES)");
        let bounded_read = concat!("Read::by_ref(&mut file).", "take(maximum_bytes)");
        let path_reopen = concat!("fs::", "read(expected_job_path)");
        let obsolete_result_channel = concat!("writes its ", "bounded ", "result");
        let stable_exit_code = concat!("stable headless ", "exit code");

        assert!(source.contains(reader));
        assert!(source.contains(no_follow_open));
        assert!(source.contains(hard_cap));
        assert!(source.contains(bounded_read));
        assert!(!source.contains(path_reopen));
        assert!(!source.contains(obsolete_result_channel));
        assert!(source.contains(stable_exit_code));
    }

    #[test]
    fn stage_call_uses_the_protected_copy_not_the_parent_job_path() {
        let source = include_str!("elevation.rs");
        assert!(source.contains("let protected_package = staging.copy_from_untrusted_source"));
        assert!(source.contains("revalidate_all_users_package(release, &protected_package)"));
        assert!(
            source.contains("stage_and_provision_all_users(\n                &protected_package")
        );
        assert!(
            !source.contains("stage_and_provision_all_users(\n                job.package_path()")
        );
    }
}
