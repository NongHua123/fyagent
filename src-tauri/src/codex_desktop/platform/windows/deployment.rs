//! Narrow Windows PackageManager, AUMID, and disk-space boundaries.
//!
//! The adapter above this module owns all allowlist decisions.  This layer
//! accepts only a local `file://` URI, reports normalized progress, and never
//! exposes WinRT, HRESULT text, or a raw filesystem path to the common domain.

use std::{path::Path, sync::Arc};

use url::Url;

use crate::codex_desktop::{
    error::{InstallerError, InstallerErrorCode},
    types::{CpuArchitecture, PlatformVersion},
};

/// Callback used by the PackageManager facade. Values are normalized into the
/// inclusive `[0, 100]` range before leaving this module.
pub(crate) type WindowsDeploymentProgressSink = Arc<dyn Fn(u32) + Send + Sync>;

/// Current-user package facts obtained from PackageManager, not from a path,
/// process name, or executable scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowsPackageRecord {
    pub(crate) identity_name: String,
    pub(crate) publisher: String,
    pub(crate) family_name: String,
    pub(crate) version: PlatformVersion,
    pub(crate) architecture: CpuArchitecture,
    pub(crate) display_name: Option<String>,
    pub(crate) application_ids: Vec<String>,
}

impl WindowsPackageRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identity_name: impl Into<String>,
        publisher: impl Into<String>,
        family_name: impl Into<String>,
        version: PlatformVersion,
        architecture: CpuArchitecture,
        display_name: Option<String>,
        application_ids: Vec<String>,
    ) -> Self {
        Self {
            identity_name: identity_name.into(),
            publisher: publisher.into(),
            family_name: family_name.into(),
            version,
            architecture,
            display_name,
            application_ids,
        }
    }
}

/// The only system boundary the common Windows adapter needs. Implementations
/// must query the current user only and must not use PowerShell, winget, or a
/// shell command as a deployment fallback.
pub(crate) trait WindowsPackageManager: Send + Sync {
    fn current_user_packages(&self) -> Result<Vec<WindowsPackageRecord>, WindowsNativeError>;

    fn deploy_current_user(
        &self,
        package_file_uri: &str,
        progress: WindowsDeploymentProgressSink,
    ) -> Result<(), WindowsNativeError>;

    /// Launches an already verified app identity. The system implementation
    /// delegates this to the interactive user's Explorer shell rather than
    /// activating the app from the elevated FyAgent process.
    fn launch_aumid(&self, aumid: &str) -> Result<(), WindowsNativeError>;
}

/// A sanitized native failure. Raw system text is intentionally not retained:
/// it can contain deployment paths, policy details, or user-specific data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WindowsNativeError {
    hresult: Option<i32>,
}

impl WindowsNativeError {
    pub(crate) const fn from_hresult(hresult: i32) -> Self {
        Self {
            hresult: Some(hresult),
        }
    }

    pub(crate) const fn unavailable() -> Self {
        Self { hresult: None }
    }

    pub(crate) const fn hresult(self) -> Option<i32> {
        self.hresult
    }
}

/// Converts a PackageManager failure into a stable installer error. Only
/// documented HRESULTs with a clear recovery action receive a specialized
/// code; all other deployment failures remain generic and retain the numeric
/// HRESULT for diagnostics.
pub(crate) fn deployment_error(error: WindowsNativeError) -> InstallerError {
    let code = error
        .hresult()
        .map(map_deployment_hresult)
        .unwrap_or(InstallerErrorCode::WindowsDeploymentFailed);
    let mut installer_error = InstallerError::new(code).with_diagnostic_message(match code {
        InstallerErrorCode::WindowsPackageInUse => {
            "Windows reported that the package is currently in use"
        }
        InstallerErrorCode::WindowsDeploymentBlocked => {
            "Windows policy or deployment settings blocked the package"
        }
        InstallerErrorCode::WindowsDependencyMissing => {
            "Windows reported an unsatisfied package dependency"
        }
        InstallerErrorCode::PackageSignatureInvalid => {
            "Windows rejected the package signature or certificate trust"
        }
        InstallerErrorCode::PackageParseFailed => "Windows rejected malformed MSIX package data",
        _ => "Windows PackageManager deployment failed",
    });
    if let Some(hresult) = error.hresult() {
        installer_error = installer_error.with_platform_error_code(format_hresult(hresult));
    }
    installer_error
}

/// `StagePackageByUriAsync` / `ProvisionPackageForAllUsersAsync` are optional
/// on older or restricted Windows deployments.  Keep that unsupported state
/// distinct from package policy/signature/dependency failures, which retain
/// the existing deployment HRESULT mapping.
#[cfg(target_os = "windows")]
fn all_users_api_error(error: WindowsNativeError) -> InstallerError {
    match error.hresult().map(|value| value as u32) {
        // E_NOINTERFACE, ERROR_CALL_NOT_IMPLEMENTED, and ERROR_PROC_NOT_FOUND.
        Some(0x8000_4002 | 0x8007_0078 | 0x8007_007F) => {
            let mut installer_error =
                InstallerError::new(InstallerErrorCode::WindowsAllUsersUnsupported)
                    .with_diagnostic_message(
                        "Windows does not expose the required all-users PackageManager API",
                    );
            if let Some(hresult) = error.hresult() {
                installer_error = installer_error.with_platform_error_code(format_hresult(hresult));
            }
            installer_error
        }
        _ => deployment_error(error),
    }
}

#[cfg(target_os = "windows")]
fn all_users_stage_record_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsAllUsersUnsupported).with_diagnostic_message(
        "Windows did not expose one exact staged Stable package family for provisioning",
    )
}

/// Verified application launch failures are not package deployment results.
/// Preserve an HRESULT if present, but always expose the stable launch-specific
/// code.
pub(crate) fn launch_error(error: WindowsNativeError) -> InstallerError {
    let mut installer_error = InstallerError::new(InstallerErrorCode::LaunchFailed)
        .with_diagnostic_message("Windows could not launch the verified application identity");
    if let Some(hresult) = error.hresult() {
        installer_error = installer_error.with_platform_error_code(format_hresult(hresult));
    }
    installer_error
}

fn format_hresult(hresult: i32) -> String {
    format!("0x{:08X}", hresult as u32)
}

fn map_deployment_hresult(hresult: i32) -> InstallerErrorCode {
    match hresult as u32 {
        // ERROR_PACKAGES_IN_USE and its newer deployment equivalent.
        0x8007_3D02 | 0x8007_3D06 => InstallerErrorCode::WindowsPackageInUse,
        // Deployment blocked by machine/profile/volume policy, or by the
        // legacy sideloading policy failure.
        0x8007_3CFF | 0x8007_3D01 | 0x8007_3D19 | 0x8007_3D21 | 0x8007_3D22 | 0x8007_3D23
        | 0x8007_0005 => InstallerErrorCode::WindowsDeploymentBlocked,
        // ERROR_INSTALL_RESOLVE_DEPENDENCY_FAILED and
        // ERROR_INSTALL_PREREQUISITE_FAILED.
        0x8007_3CF3 | 0x8007_3CFD => InstallerErrorCode::WindowsDependencyMissing,
        // Trust failures reported by the deployment platform. The `CF0` case
        // also covers a package that cannot be opened because its signature
        // and manifest publisher cannot be validated.
        0x8007_3CF0 | 0x800B_0100 | 0x800B_0109 | 0x800B_010A | 0x800B_0004 => {
            InstallerErrorCode::PackageSignatureInvalid
        }
        // Malformed manifest/block-map/corrupt package data is not a retryable
        // deployment result and should not be presented as a signature issue.
        0x8008_0204..=0x8008_0207 => InstallerErrorCode::PackageParseFailed,
        _ => InstallerErrorCode::WindowsDeploymentFailed,
    }
}

/// Converts a validated, local artifact path into the only URI form accepted
/// by the normal install path. A relative path, network URL, query, or
/// fragment cannot cross this boundary.
pub(crate) fn local_file_uri(package_path: &Path) -> Result<String, InstallerError> {
    let metadata = std::fs::metadata(package_path).map_err(|_| {
        InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("validated MSIX package is no longer available")
    })?;
    if !metadata.is_file() {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("validated MSIX package path is not a regular file"));
    }

    let uri = Url::from_file_path(package_path).map_err(|_| {
        InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("validated MSIX package path cannot form a file URI")
    })?;
    if uri.scheme() != "file"
        || uri.host().is_some()
        || uri.query().is_some()
        || uri.fragment().is_some()
    {
        return Err(InstallerError::new(InstallerErrorCode::PackageParseFailed)
            .with_diagnostic_message("validated MSIX package URI is not a local file URI"));
    }
    Ok(uri.into())
}

/// Experimental all-users provisioning stays outside [`WindowsPackageManager`]
/// because the normal platform trait must remain current-user only.  The
/// elevated caller reaches this narrow helper only after its job protocol has
/// repeated hash, manifest, identity, architecture, OS, and disk checks.
#[cfg(target_os = "windows")]
pub(crate) fn stage_and_provision_all_users(
    package_path: &Path,
    expected_identity: &str,
    expected_publisher: &str,
    expected_version: &PlatformVersion,
    expected_architecture: CpuArchitecture,
) -> Result<(), InstallerError> {
    let package_file_uri = local_file_uri(package_path)?;
    native::stage_and_provision_all_users(
        &package_file_uri,
        expected_identity,
        expected_publisher,
        expected_version,
        expected_architecture,
    )
}

#[cfg(target_os = "windows")]
#[derive(Debug, Default)]
pub struct SystemWindowsPackageManager;

#[cfg(target_os = "windows")]
impl WindowsPackageManager for SystemWindowsPackageManager {
    fn current_user_packages(&self) -> Result<Vec<WindowsPackageRecord>, WindowsNativeError> {
        native::current_user_packages()
    }

    fn deploy_current_user(
        &self,
        package_file_uri: &str,
        progress: WindowsDeploymentProgressSink,
    ) -> Result<(), WindowsNativeError> {
        native::deploy_current_user(package_file_uri, progress)
    }

    fn launch_aumid(&self, aumid: &str) -> Result<(), WindowsNativeError> {
        native::launch_aumid(aumid)
    }
}

#[cfg(target_os = "windows")]
pub struct SystemWindowsDiskSpaceProbe {
    volumes: std::sync::Mutex<
        std::collections::HashMap<crate::codex_desktop::verify::VolumeKey, std::path::PathBuf>,
    >,
}

#[cfg(target_os = "windows")]
impl SystemWindowsDiskSpaceProbe {
    pub fn new() -> Self {
        Self {
            volumes: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[cfg(target_os = "windows")]
impl Default for SystemWindowsDiskSpaceProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
impl crate::codex_desktop::verify::DiskSpaceProbe for SystemWindowsDiskSpaceProbe {
    fn volume_key(
        &self,
        path: &Path,
    ) -> Result<
        crate::codex_desktop::verify::VolumeKey,
        crate::codex_desktop::verify::DiskSpaceProbeError,
    > {
        let volume_path = native::volume_root_for(path)
            .map_err(|_| crate::codex_desktop::verify::DiskSpaceProbeError::Unavailable)?;
        let key = crate::codex_desktop::verify::VolumeKey::new(volume_path.to_string_lossy())?;
        self.volumes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(key.clone(), volume_path);
        Ok(key)
    }

    fn available_bytes(
        &self,
        volume: &crate::codex_desktop::verify::VolumeKey,
    ) -> Result<u64, crate::codex_desktop::verify::DiskSpaceProbeError> {
        let path = self
            .volumes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(volume)
            .cloned()
            .ok_or(crate::codex_desktop::verify::DiskSpaceProbeError::Unavailable)?;
        native::available_bytes(&path)
            .map_err(|_| crate::codex_desktop::verify::DiskSpaceProbeError::Unavailable)
    }
}

#[cfg(target_os = "windows")]
mod native {
    use std::{
        ffi::OsString,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
    };

    use windows::{
        core::{HSTRING, PCWSTR},
        Foundation::Uri,
        Management::Deployment::{
            AddPackageOptions, DeploymentProgress, DeploymentResult, PackageManager,
            StagePackageOptions,
        },
        System::ProcessorArchitecture,
        Win32::{
            Storage::FileSystem::{GetDiskFreeSpaceExW, GetVolumePathNameW},
            System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
        },
    };
    use windows_future::AsyncOperationProgressHandler;

    use super::{
        all_users_api_error, all_users_stage_record_error, deployment_error,
        WindowsDeploymentProgressSink, WindowsNativeError, WindowsPackageRecord,
    };
    use crate::codex_desktop::{
        error::InstallerError,
        platform::WINDOWS_CODEX_STABLE_IDENTITY,
        types::{CpuArchitecture, PlatformVersion},
    };

    pub(super) fn current_user_packages() -> Result<Vec<WindowsPackageRecord>, WindowsNativeError> {
        let _apartment = WinRtApartment::initialize()?;
        let package_manager = PackageManager::new().map_err(WindowsNativeError::from_windows)?;
        let mut records = Vec::new();

        let packages = package_manager
            .FindPackages()
            .map_err(WindowsNativeError::from_windows)?;
        let iterator = packages.First().map_err(WindowsNativeError::from_windows)?;
        while iterator
            .HasCurrent()
            .map_err(WindowsNativeError::from_windows)?
        {
            let package = iterator
                .Current()
                .map_err(WindowsNativeError::from_windows)?;
            let package_id = package.Id().map_err(WindowsNativeError::from_windows)?;
            let identity_name = package_id
                .Name()
                .map_err(WindowsNativeError::from_windows)?
                .to_string();
            if identity_name != WINDOWS_CODEX_STABLE_IDENTITY {
                iterator
                    .MoveNext()
                    .map_err(WindowsNativeError::from_windows)?;
                continue;
            }

            let version = package_id
                .Version()
                .map_err(WindowsNativeError::from_windows)?;
            let architecture = map_architecture(
                package_id
                    .Architecture()
                    .map_err(WindowsNativeError::from_windows)?,
            );
            let family_name = package_id
                .FamilyName()
                .map_err(WindowsNativeError::from_windows)?
                .to_string();
            let app_entries = package
                .GetAppListEntriesAsync()
                .map_err(WindowsNativeError::from_windows)?
                .get()
                .map_err(WindowsNativeError::from_windows)?;
            let mut application_ids = Vec::new();
            let app_count = app_entries
                .Size()
                .map_err(WindowsNativeError::from_windows)?;
            for index in 0..app_count {
                let entry = app_entries
                    .GetAt(index)
                    .map_err(WindowsNativeError::from_windows)?;
                let aumid = entry
                    .AppUserModelId()
                    .map_err(WindowsNativeError::from_windows)?
                    .to_string();
                let Some((aumid_family, application_id)) = aumid.split_once('!') else {
                    return Err(WindowsNativeError::unavailable());
                };
                if aumid_family != family_name || application_id.is_empty() {
                    return Err(WindowsNativeError::unavailable());
                }
                application_ids.push(application_id.to_owned());
            }

            let display_name = package
                .DisplayName()
                .ok()
                .map(|value| value.to_string())
                .filter(|value| !value.trim().is_empty());
            records.push(WindowsPackageRecord::new(
                identity_name,
                package_id
                    .Publisher()
                    .map_err(WindowsNativeError::from_windows)?
                    .to_string(),
                family_name,
                PlatformVersion::WindowsMsix {
                    major: version.Major,
                    minor: version.Minor,
                    build: version.Build,
                    revision: version.Revision,
                },
                architecture,
                display_name,
                application_ids,
            ));
            iterator
                .MoveNext()
                .map_err(WindowsNativeError::from_windows)?;
        }
        Ok(records)
    }

    pub(super) fn deploy_current_user(
        package_file_uri: &str,
        progress: WindowsDeploymentProgressSink,
    ) -> Result<(), WindowsNativeError> {
        let _apartment = WinRtApartment::initialize()?;
        let package_manager = PackageManager::new().map_err(WindowsNativeError::from_windows)?;
        let uri = Uri::CreateUri(&HSTRING::from(package_file_uri))
            .map_err(WindowsNativeError::from_windows)?;
        // `AddPackageOptions::new()` leaves ForceAppShutdown and all developer
        // / unsigned options disabled. V1 asks the user to close the target
        // rather than terminating it or weakening deployment trust.
        let options = AddPackageOptions::new().map_err(WindowsNativeError::from_windows)?;
        progress(0);
        let operation = package_manager
            .AddPackageByUriAsync(&uri, &options)
            .map_err(WindowsNativeError::from_windows)?;
        let progress_callback = progress.clone();
        operation
            .SetProgress(&AsyncOperationProgressHandler::<
                DeploymentResult,
                DeploymentProgress,
            >::new(move |_, deployment_progress| {
                progress_callback(deployment_progress.percentage.min(100));
                Ok(())
            }))
            .map_err(WindowsNativeError::from_windows)?;
        let result = operation.get().map_err(WindowsNativeError::from_windows)?;
        let extended_error = result
            .ExtendedErrorCode()
            .map_err(WindowsNativeError::from_windows)?;
        if extended_error.0 != 0 {
            return Err(WindowsNativeError::from_hresult(extended_error.0));
        }
        if !result
            .IsRegistered()
            .map_err(WindowsNativeError::from_windows)?
        {
            return Err(WindowsNativeError::unavailable());
        }
        progress(100);
        Ok(())
    }

    pub(super) fn stage_and_provision_all_users(
        package_file_uri: &str,
        expected_identity: &str,
        expected_publisher: &str,
        expected_version: &PlatformVersion,
        expected_architecture: CpuArchitecture,
    ) -> Result<(), InstallerError> {
        let _apartment = WinRtApartment::initialize().map_err(all_users_api_error)?;
        let package_manager = PackageManager::new()
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?;
        let uri = Uri::CreateUri(&HSTRING::from(package_file_uri))
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?;
        // Defaults keep unsigned/developer options disabled.  Staging is used
        // specifically to let PackageManager validate the signed local MSIX on
        // its normal system volume before all-users provisioning is attempted.
        let options = StagePackageOptions::new()
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?;
        let stage_result = package_manager
            .StagePackageByUriAsync(&uri, &options)
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?
            .get()
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?;
        ensure_deployment_success(&stage_result)?;

        let family_name = staged_package_family_name(
            &package_manager,
            expected_identity,
            expected_publisher,
            expected_version,
            expected_architecture,
        )?;
        let provision_result = package_manager
            .ProvisionPackageForAllUsersAsync(&family_name)
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?
            .get()
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?;
        ensure_deployment_success(&provision_result)
    }

    fn ensure_deployment_success(result: &DeploymentResult) -> Result<(), InstallerError> {
        let extended_error = result
            .ExtendedErrorCode()
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?;
        if extended_error.0 != 0 {
            return Err(deployment_error(WindowsNativeError::from_hresult(
                extended_error.0,
            )));
        }
        Ok(())
    }

    fn staged_package_family_name(
        package_manager: &PackageManager,
        expected_identity: &str,
        expected_publisher: &str,
        expected_version: &PlatformVersion,
        expected_architecture: CpuArchitecture,
    ) -> Result<HSTRING, InstallerError> {
        let packages = package_manager
            .FindPackages()
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?;
        let iterator = packages
            .First()
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?;
        let mut family_name = None;
        while iterator
            .HasCurrent()
            .map_err(WindowsNativeError::from_windows)
            .map_err(all_users_api_error)?
        {
            let package = iterator
                .Current()
                .map_err(WindowsNativeError::from_windows)
                .map_err(all_users_api_error)?;
            let package_id = package
                .Id()
                .map_err(WindowsNativeError::from_windows)
                .map_err(all_users_api_error)?;
            let version = package_id
                .Version()
                .map_err(WindowsNativeError::from_windows)
                .map_err(all_users_api_error)?;
            let identity_name = package_id
                .Name()
                .map_err(WindowsNativeError::from_windows)
                .map_err(all_users_api_error)?
                .to_string();
            let publisher = package_id
                .Publisher()
                .map_err(WindowsNativeError::from_windows)
                .map_err(all_users_api_error)?
                .to_string();
            let matches_expected = identity_name == expected_identity
                && publisher == expected_publisher
                && PlatformVersion::WindowsMsix {
                    major: version.Major,
                    minor: version.Minor,
                    build: version.Build,
                    revision: version.Revision,
                } == *expected_version
                && map_architecture(
                    package_id
                        .Architecture()
                        .map_err(WindowsNativeError::from_windows)
                        .map_err(all_users_api_error)?,
                ) == expected_architecture;
            if matches_expected {
                let candidate = package_id
                    .FamilyName()
                    .map_err(WindowsNativeError::from_windows)
                    .map_err(all_users_api_error)?;
                if candidate.is_empty() || family_name.replace(candidate).is_some() {
                    return Err(all_users_stage_record_error());
                }
            }
            iterator
                .MoveNext()
                .map_err(WindowsNativeError::from_windows)
                .map_err(all_users_api_error)?;
        }
        family_name.ok_or_else(all_users_stage_record_error)
    }

    pub(super) fn launch_aumid(aumid: &str) -> Result<(), WindowsNativeError> {
        crate::platform::process_launch::launch_trusted_windows_app_aumid_as_user(aumid)
            .map_err(|_| WindowsNativeError::unavailable())
    }

    pub(super) fn volume_root_for(path: &Path) -> Result<PathBuf, WindowsNativeError> {
        let path = wide_path(path)?;
        let mut volume = vec![0_u16; 32_768];
        unsafe { GetVolumePathNameW(PCWSTR(path.as_ptr()), &mut volume) }
            .map_err(WindowsNativeError::from_windows)?;
        let length = volume
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(volume.len());
        if length == 0 || length == volume.len() {
            return Err(WindowsNativeError::unavailable());
        }
        Ok(PathBuf::from(OsString::from_wide(&volume[..length])))
    }

    pub(super) fn available_bytes(path: &Path) -> Result<u64, WindowsNativeError> {
        let path = wide_path(path)?;
        let mut available = 0_u64;
        unsafe { GetDiskFreeSpaceExW(PCWSTR(path.as_ptr()), Some(&mut available), None, None) }
            .map_err(WindowsNativeError::from_windows)?;
        Ok(available)
    }

    fn wide_path(path: &Path) -> Result<Vec<u16>, WindowsNativeError> {
        if path.as_os_str().is_empty() {
            return Err(WindowsNativeError::unavailable());
        }
        Ok(path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect())
    }

    fn map_architecture(architecture: ProcessorArchitecture) -> CpuArchitecture {
        match architecture {
            ProcessorArchitecture::X64 => CpuArchitecture::X86_64,
            ProcessorArchitecture::Arm64 => CpuArchitecture::Aarch64,
            _ => CpuArchitecture::Unsupported,
        }
    }

    struct WinRtApartment;

    impl WinRtApartment {
        fn initialize() -> Result<Self, WindowsNativeError> {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
                .map_err(WindowsNativeError::from_windows)?;
            Ok(Self)
        }
    }

    impl Drop for WinRtApartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }

    impl WindowsNativeError {
        fn from_windows(error: windows::core::Error) -> Self {
            Self::from_hresult(error.code().0)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn maps_documented_deployment_hresult_values_to_stable_errors() {
        let cases = [
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
                0x8008_0205_u32 as i32,
                InstallerErrorCode::PackageParseFailed,
            ),
            (
                0x8123_4567_u32 as i32,
                InstallerErrorCode::WindowsDeploymentFailed,
            ),
        ];
        for (hresult, expected) in cases {
            let error = deployment_error(WindowsNativeError::from_hresult(hresult));
            assert_eq!(error.code(), expected);
            assert_eq!(
                error.to_dto().details.platform_error_code,
                Some(format!("0x{:08X}", hresult as u32))
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn all_users_missing_package_manager_apis_remain_explicitly_unsupported() {
        for hresult in [0x8000_4002_u32 as i32, 0x8007_0078_u32 as i32] {
            let error = all_users_api_error(WindowsNativeError::from_hresult(hresult));
            assert_eq!(error.code(), InstallerErrorCode::WindowsAllUsersUnsupported);
            assert_eq!(
                error.to_dto().details.platform_error_code,
                Some(format!("0x{:08X}", hresult as u32))
            );
        }
    }

    #[test]
    fn file_uri_requires_an_existing_regular_absolute_file() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("installer.msix");
        std::fs::write(&file, b"fixture").unwrap();
        let uri = local_file_uri(&file).unwrap();
        assert!(uri.starts_with("file:///"));
        assert!(uri.ends_with("installer.msix"));

        let missing = local_file_uri(&directory.path().join("missing.msix")).unwrap_err();
        assert_eq!(missing.code(), InstallerErrorCode::PackageParseFailed);

        let relative = PathBuf::from("installer.msix");
        let relative_error = local_file_uri(&relative).unwrap_err();
        assert_eq!(
            relative_error.code(),
            InstallerErrorCode::PackageParseFailed
        );
    }
}
