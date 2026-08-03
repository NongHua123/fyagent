//! Stable macOS bundle inspection and verification.
//!
//! Bundle names are intentionally not an identity signal. A current Stable
//! package may be named `ChatGPT.app`, while an older valid installation may
//! still be named `Codex.app`; only the exact bundle ID and Team ID are trusted.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use serde::Deserialize;

use super::{
    command, error, is_not_found, stable_bundle_id, CommandRunner, MacosFileKind, MacosFilesystem,
    MacosHost, MacosVersion,
};
use crate::codex_desktop::{
    error::{InstallerError, InstallerErrorCode},
    platform::{RuntimeInspection, TrustedRuntimeInstance},
    types::{
        CpuArchitecture, DesktopPlatform, InstalledApplication, InstalledApplicationSummary,
        LaunchTarget, LocalInstallStatus, PlatformVersion, ReleaseDescriptor,
        TrustedDownloadEndpoint,
    },
};

/// Exact Stable Team allowlist. The downloaded-artifact fixture keeps this
/// value auditable, while production verification still obtains it from
/// `codesign` on the mounted bundle rather than trusting release metadata.
const OPENAI_TEAM_IDENTIFIER: &str = "2DC432GLL2";
const PLUTIL_OUTPUT_FORMAT: &str = "json";

/// This script is a fixed program constant. It receives no paths or metadata
/// as interpolation input, and its JSON output is parsed before it influences
/// the install decision.
const RUNNING_APPLICATIONS_JXA: &str = r#"
ObjC.import('AppKit');
const workspace = $.NSWorkspace.sharedWorkspace;
const applications = workspace.runningApplications;
const result = [];
for (let index = 0; index < applications.count; index += 1) {
  const application = applications.objectAtIndex(index);
  const url = application.bundleURL;
  result.push({
    bundleIdentifier: application.bundleIdentifier ? ObjC.unwrap(application.bundleIdentifier) : null,
    bundlePath: url ? ObjC.unwrap(url.path) : null,
    processIdentifier: application.processIdentifier,
    launchTimestampMs: application.launchDate
      ? Math.floor(Number(application.launchDate.timeIntervalSince1970) * 1000)
      : null,
    isFinishedLaunching: ObjC.unwrap(application.isFinishedLaunching),
  });
}
JSON.stringify(result);
"#;

/// Fixed JXA program used only after Rust has re-enumerated an exact trusted
/// running application. The mode, PID, and expected bundle path are internal
/// direct `osascript` arguments (never shell input or IPC fields). The script
/// repeats the Stable bundle-ID/path checks immediately before asking AppKit
/// to terminate the NSRunningApplication.
const TERMINATE_RUNNING_APPLICATION_JXA: &str = r#"
ObjC.import('AppKit');
const args = ObjC.unwrap($.NSProcessInfo.processInfo.arguments);
const [mode, pidText, expectedBundlePath, expectedLaunchTimestampText] = args.slice(-4);
const pid = Number(pidText);
const expectedLaunchTimestamp = Number(expectedLaunchTimestampText);
if (!Number.isSafeInteger(pid) || pid <= 0) {
  $.exit(2);
}
if (!Number.isSafeInteger(expectedLaunchTimestamp) || expectedLaunchTimestamp <= 0) {
  $.exit(7);
}
if (mode !== 'normal' && mode !== 'force') {
  $.exit(3);
}
const application = $.NSRunningApplication.runningApplicationWithProcessIdentifier(pid);
if (!application || !application.bundleIdentifier || !application.bundleURL) {
  $.exit(4);
}
const bundleIdentifier = ObjC.unwrap(application.bundleIdentifier);
const bundlePath = ObjC.unwrap(application.bundleURL.path);
const launchTimestamp = application.launchDate
  ? Math.floor(Number(application.launchDate.timeIntervalSince1970) * 1000)
  : 0;
if (bundleIdentifier !== 'com.openai.codex'
  || bundlePath !== expectedBundlePath
  || launchTimestamp !== expectedLaunchTimestamp) {
  $.exit(5);
}
const accepted = mode === 'force' ? application.forceTerminate() : application.terminate();
if (!accepted) {
  $.exit(6);
}
"#;

#[derive(Debug, Clone)]
pub(crate) struct BundleInfo {
    bundle_path: PathBuf,
    bundle_name: OsString,
    bundle_identifier: String,
    platform_version: PlatformVersion,
    display_version: Option<String>,
    display_name: Option<String>,
    minimum_os_version: Option<MacosVersion>,
    executable_path: PathBuf,
}

impl BundleInfo {
    pub(crate) fn bundle_path(&self) -> &Path {
        &self.bundle_path
    }

    pub(crate) fn bundle_name(&self) -> &OsStr {
        &self.bundle_name
    }

    pub(crate) fn bundle_identifier(&self) -> &str {
        &self.bundle_identifier
    }

    pub(crate) fn platform_version(&self) -> &PlatformVersion {
        &self.platform_version
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInfoPlist {
    #[serde(rename = "CFBundleIdentifier")]
    bundle_identifier: Option<String>,
    #[serde(rename = "CFBundleVersion")]
    bundle_version: Option<String>,
    #[serde(rename = "CFBundleShortVersionString")]
    short_version: Option<String>,
    #[serde(rename = "CFBundleExecutable")]
    executable: Option<String>,
    #[serde(rename = "LSMinimumSystemVersion")]
    minimum_system_version: Option<String>,
    #[serde(rename = "CFBundleDisplayName")]
    display_name: Option<String>,
    #[serde(rename = "CFBundleName")]
    bundle_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBundleIdentityPlist {
    #[serde(rename = "CFBundleIdentifier")]
    bundle_identifier: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunningApplication {
    bundle_identifier: Option<String>,
    bundle_path: Option<String>,
    process_identifier: Option<i32>,
    launch_timestamp_ms: Option<u64>,
    is_finished_launching: Option<bool>,
}

#[derive(Debug, Clone)]
struct TrustedRunningApplication {
    instance: TrustedRuntimeInstance,
    is_finished_launching: bool,
}

#[derive(Debug, Clone)]
enum TrustedRunningApplications {
    NotRunning,
    Candidates(Vec<TrustedRunningApplication>),
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BundleCandidatePolicy {
    StrictPackage,
    SkipEscapedLocalSymlink,
}

/// Scan direct bundle children under the two standard roots. Any Stable
/// candidate is fully verified before becoming a managed installation; Classic
/// and Beta bundles are never promoted based on their filename or Team alone.
pub(crate) fn inspect_local(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
) -> Result<LocalInstallStatus, InstallerError> {
    let stable_bundles = scan_stable_bundles(runner, filesystem, host)?;
    match stable_bundles.as_slice() {
        [] => Ok(LocalInstallStatus::NotInstalled {
            platform: DesktopPlatform::Macos,
            architecture: CpuArchitecture::Aarch64,
        }),
        [bundle] => Ok(LocalInstallStatus::Installed {
            application: installed_application(bundle),
        }),
        bundles => {
            let error = error(
                InstallerErrorCode::MacMultipleInstallations,
                "multiple Stable macOS bundles were found in standard Applications directories",
            )
            .to_dto();
            Ok(LocalInstallStatus::Ambiguous {
                candidates: bundles
                    .iter()
                    .map(|bundle| InstalledApplicationSummary::from(&installed_application(bundle)))
                    .collect(),
                error,
            })
        }
    }
}

pub(crate) fn scan_stable_bundles(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
) -> Result<Vec<BundleInfo>, InstallerError> {
    let mut stable_bundles = Vec::new();
    for root in [host.applications_dir(), host.user_applications_dir()] {
        let root = match canonical_existing_directory(filesystem, root) {
            Ok(root) => root,
            Err(error) if error.code() == InstallerErrorCode::MacAppNotFound => continue,
            Err(error) => return Err(error),
        };

        for entry in filesystem.read_dir(&root).map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "a standard Applications directory could not be enumerated",
            )
        })? {
            // Local discovery must not follow an escaped alias just to learn
            // its identity. Downloaded package discovery uses the strict
            // wrapper below and continues to reject the same path shape.
            let Some(bundle_path) = canonical_top_level_bundle_with_policy(
                filesystem,
                &root,
                &entry,
                BundleCandidatePolicy::SkipEscapedLocalSymlink,
            )?
            else {
                continue;
            };
            let Some(bundle_identifier) =
                probe_bundle_identifier(runner, filesystem, &bundle_path)?
            else {
                continue;
            };
            if bundle_identifier != stable_bundle_id() {
                continue;
            }
            let bundle = read_bundle_info(runner, filesystem, &bundle_path)?;
            if bundle.bundle_identifier() != stable_bundle_id() {
                continue;
            }
            validate_stable_bundle(runner, filesystem, host, &bundle, None)?;
            stable_bundles.push(bundle);
        }
    }
    Ok(stable_bundles)
}

/// Resolves one direct `.app` child and rejects a link that escapes the trusted
/// standard root. A symlink inside a root remains acceptable only when its
/// canonical target is still a direct child of that same root.
pub(crate) fn canonical_top_level_bundle(
    filesystem: &dyn MacosFilesystem,
    canonical_parent: &Path,
    candidate: &Path,
) -> Result<Option<PathBuf>, InstallerError> {
    canonical_top_level_bundle_with_policy(
        filesystem,
        canonical_parent,
        candidate,
        BundleCandidatePolicy::StrictPackage,
    )
}

fn canonical_top_level_bundle_with_policy(
    filesystem: &dyn MacosFilesystem,
    canonical_parent: &Path,
    candidate: &Path,
    policy: BundleCandidatePolicy,
) -> Result<Option<PathBuf>, InstallerError> {
    if !has_app_extension(candidate) {
        return Ok(None);
    }
    let candidate_kind = match filesystem.file_kind(candidate) {
        Ok(kind @ (MacosFileKind::Directory | MacosFileKind::Symlink)) => kind,
        Ok(_) => return Ok(None),
        Err(error) if is_not_found(error) => return Ok(None),
        Err(_) => {
            return Err(error(
                InstallerErrorCode::PackageParseFailed,
                "an application bundle candidate could not be inspected",
            ))
        }
    };

    let canonical = filesystem.canonicalize(candidate).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "an application bundle candidate could not be canonicalized",
        )
    })?;
    if candidate_kind == MacosFileKind::Symlink
        && policy == BundleCandidatePolicy::SkipEscapedLocalSymlink
        && !canonical.starts_with(canonical_parent)
    {
        return Ok(None);
    }
    if canonical.parent() != Some(canonical_parent)
        || !canonical.starts_with(canonical_parent)
        || filesystem.file_kind(&canonical) != Ok(MacosFileKind::Directory)
    {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "an application bundle candidate escaped its trusted directory",
        ));
    }
    Ok(Some(canonical))
}

pub(crate) fn read_bundle_info(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    canonical_bundle_path: &Path,
) -> Result<BundleInfo, InstallerError> {
    if filesystem.file_kind(canonical_bundle_path) != Ok(MacosFileKind::Directory) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application bundle is not a directory",
        ));
    }
    let bundle_name = canonical_bundle_path
        .file_name()
        .filter(|name| has_app_extension(Path::new(name)))
        .map(OsStr::to_os_string)
        .ok_or_else(|| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "application bundle name is invalid",
            )
        })?;
    let raw = read_raw_info_plist(runner, filesystem, canonical_bundle_path)?;

    let bundle_identifier = required_plist_string(raw.bundle_identifier, "bundle identifier")?;
    let bundle_version = required_plist_string(raw.bundle_version, "bundle version")?;
    let short_version = optional_plist_string(raw.short_version, "short version")?;
    let executable = required_plist_string(raw.executable, "bundle executable")?;
    validate_executable_name(&executable)?;
    let display_name = optional_plist_string(raw.display_name.or(raw.bundle_name), "display name")?;
    let minimum_os_version = match raw.minimum_system_version {
        Some(version) => {
            let version = required_plist_string(Some(version), "minimum system version")?;
            Some(MacosVersion::parse(&version).map_err(|_| {
                error(
                    InstallerErrorCode::PackageParseFailed,
                    "application minimum macOS version is invalid",
                )
            })?)
        }
        None => None,
    };
    let platform_version = PlatformVersion::parse_mac_bundle(bundle_version).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "application bundle version is invalid",
        )
    })?;
    let executable_path = canonical_bundle_path
        .join("Contents")
        .join("MacOS")
        .join(executable);
    if filesystem.file_kind(&executable_path) != Ok(MacosFileKind::File) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application executable is missing or is not a regular file",
        ));
    }
    let canonical_executable = filesystem.canonicalize(&executable_path).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "application executable could not be canonicalized",
        )
    })?;
    if !canonical_executable.starts_with(canonical_bundle_path) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application executable escaped its bundle",
        ));
    }

    Ok(BundleInfo {
        bundle_path: canonical_bundle_path.to_path_buf(),
        bundle_name,
        bundle_identifier,
        platform_version,
        display_version: short_version,
        display_name,
        minimum_os_version,
        executable_path: canonical_executable,
    })
}

/// Tries to identify a bundle before running the Stable-only verifier. This is
/// deliberately an exclusion-only probe: a positive result is re-read by
/// `read_bundle_info` below, so it never authorizes a bundle by itself.
fn probe_bundle_identifier(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    canonical_bundle_path: &Path,
) -> Result<Option<String>, InstallerError> {
    let info_plist = canonical_bundle_path.join("Contents").join("Info.plist");
    match filesystem.file_kind(&info_plist) {
        Ok(MacosFileKind::File) => {}
        Ok(_) => return Ok(None),
        Err(filesystem_error) if is_not_found(filesystem_error) => return Ok(None),
        Err(_) => {
            return Err(error(
                InstallerErrorCode::PackageParseFailed,
                "application Info.plist could not be inspected",
            ))
        }
    }

    let output = runner
        .run(&command(
            "plutil",
            vec![
                OsString::from("-convert"),
                OsString::from(PLUTIL_OUTPUT_FORMAT),
                OsString::from("-o"),
                OsString::from("-"),
                OsString::from("--"),
                info_plist.into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "application Info.plist could not be parsed",
            )
        })?;
    if !output.is_success() {
        return Ok(None);
    }
    let raw = match serde_json::from_slice::<RawBundleIdentityPlist>(output.stdout()) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    Ok(required_plist_string(raw.bundle_identifier, "bundle identifier").ok())
}

fn read_raw_info_plist(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    canonical_bundle_path: &Path,
) -> Result<RawInfoPlist, InstallerError> {
    let info_plist = canonical_bundle_path.join("Contents").join("Info.plist");
    if filesystem.file_kind(&info_plist) != Ok(MacosFileKind::File) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist is missing or unreadable",
        ));
    }

    let output = runner
        .run(&command(
            "plutil",
            vec![
                OsString::from("-convert"),
                OsString::from(PLUTIL_OUTPUT_FORMAT),
                OsString::from("-o"),
                OsString::from("-"),
                OsString::from("--"),
                info_plist.into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "application Info.plist could not be parsed",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist could not be parsed",
        ));
    }
    serde_json::from_slice::<RawInfoPlist>(output.stdout()).map_err(|_| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist did not produce valid JSON metadata",
        )
    })
}

/// Validates all immutable Stable identity properties. `expected_release` is
/// supplied for the downloaded DMG and omitted when re-checking an existing
/// local Stable app, whose version may legitimately be newer than latest.
pub(crate) fn validate_stable_bundle(
    runner: &dyn CommandRunner,
    _filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    bundle: &BundleInfo,
    expected_release: Option<&ReleaseDescriptor>,
) -> Result<(), InstallerError> {
    if bundle.bundle_identifier() != stable_bundle_id() {
        return Err(error(
            InstallerErrorCode::MacBundleIdMismatch,
            "application bundle identifier is not the Stable Codex identifier",
        ));
    }
    if let Some(minimum_os_version) = bundle.minimum_os_version {
        if host.os_version() < minimum_os_version {
            return Err(error(
                InstallerErrorCode::OsVersionUnsupported,
                "current macOS version is below the application bundle requirement",
            ));
        }
    }
    if let Some(release) = expected_release {
        validate_release_shape(release)?;
        if bundle.platform_version != release.platform_version {
            return Err(error(
                InstallerErrorCode::PackageParseFailed,
                "application bundle version does not match the verified release",
            ));
        }
        if let Some(minimum_os_version) = release.minimum_os_version.as_deref() {
            let minimum_os_version = MacosVersion::parse(minimum_os_version).map_err(|_| {
                error(
                    InstallerErrorCode::ReleaseMetadataInvalid,
                    "release minimum macOS version is invalid",
                )
            })?;
            if host.os_version() < minimum_os_version {
                return Err(error(
                    InstallerErrorCode::OsVersionUnsupported,
                    "current macOS version is below the release requirement",
                ));
            }
        }
    }

    let architecture_output = runner
        .run(&command(
            "lipo",
            vec![
                OsString::from("-archs"),
                bundle.executable_path.clone().into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "application architecture could not be inspected",
            )
        })?;
    if !architecture_output.is_success()
        || !output_contains_architecture(&architecture_output, "arm64")
    {
        return Err(error(
            InstallerErrorCode::PackageArchitectureMismatch,
            "application executable does not contain arm64",
        ));
    }

    let signature = runner
        .run(&command(
            "codesign",
            vec![
                OsString::from("--verify"),
                OsString::from("--deep"),
                OsString::from("--strict"),
                OsString::from("--verbose=2"),
                bundle.bundle_path.clone().into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::PackageSignatureInvalid,
                "application code signature could not be verified",
            )
        })?;
    if !signature.is_success() {
        return Err(error(
            InstallerErrorCode::PackageSignatureInvalid,
            "application code signature verification failed",
        ));
    }

    let display = runner
        .run(&command(
            "codesign",
            vec![
                OsString::from("--display"),
                OsString::from("--verbose=4"),
                bundle.bundle_path.clone().into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::PackageSignatureInvalid,
                "application signing identity could not be inspected",
            )
        })?;
    if !display.is_success() {
        return Err(error(
            InstallerErrorCode::PackageSignatureInvalid,
            "application signing identity inspection failed",
        ));
    }
    let team_identifier = parse_team_identifier(&display).ok_or_else(|| {
        error(
            InstallerErrorCode::PackageSignatureInvalid,
            "application signing identity did not include a TeamIdentifier",
        )
    })?;
    if team_identifier != OPENAI_TEAM_IDENTIFIER {
        return Err(error(
            InstallerErrorCode::MacTeamIdMismatch,
            "application signing TeamIdentifier is not the Stable Codex team",
        ));
    }

    let gatekeeper = runner
        .run(&command(
            "spctl",
            vec![
                OsString::from("--assess"),
                OsString::from("--type"),
                OsString::from("execute"),
                OsString::from("--verbose=4"),
                bundle.bundle_path.clone().into_os_string(),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::MacGatekeeperRejected,
                "macOS security assessment could not be completed",
            )
        })?;
    if !gatekeeper.is_success() {
        return Err(error(
            InstallerErrorCode::MacGatekeeperRejected,
            "macOS security assessment rejected the application bundle",
        ));
    }
    Ok(())
}

pub(crate) fn ensure_not_running(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    bundle_path: &Path,
) -> Result<(), InstallerError> {
    let bundle_path = filesystem.canonicalize(bundle_path).map_err(|_| {
        error(
            InstallerErrorCode::MacAppRunning,
            "running Stable application state could not be determined",
        )
    })?;
    let output = runner
        .run(&command(
            "osascript",
            vec![
                OsString::from("-l"),
                OsString::from("JavaScript"),
                OsString::from("-e"),
                OsString::from(RUNNING_APPLICATIONS_JXA),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application state could not be determined",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "running Stable application state could not be determined",
        ));
    }
    let applications =
        serde_json::from_slice::<Vec<RunningApplication>>(output.stdout()).map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application state could not be determined",
            )
        })?;
    for application in applications {
        if application.bundle_identifier.as_deref() != Some(stable_bundle_id()) {
            continue;
        }
        let path = application.bundle_path.ok_or_else(|| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application path could not be determined",
            )
        })?;
        let running_path = filesystem.canonicalize(Path::new(&path)).map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application path could not be determined",
            )
        })?;
        if running_path == bundle_path {
            return Err(error(
                InstallerErrorCode::MacAppRunning,
                "the Stable application is running",
            ));
        }
    }
    Ok(())
}

/// Inspect only running applications that match both the Stable bundle ID and
/// the canonical path of an already verified installation. A matching bundle
/// ID at a different path is intentionally ambiguous; it is never selected by
/// display name or process name.
pub(crate) fn inspect_runtime(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
) -> Result<RuntimeInspection, InstallerError> {
    match inspect_trusted_running_applications(runner, filesystem, installed)? {
        TrustedRunningApplications::NotRunning => Ok(RuntimeInspection::NotRunning),
        TrustedRunningApplications::Ambiguous => Ok(RuntimeInspection::Ambiguous),
        TrustedRunningApplications::Candidates(applications) => match applications.as_slice() {
            [application] if application.is_finished_launching => {
                Ok(RuntimeInspection::Running(vec![application
                    .instance
                    .clone()]))
            }
            [_] => {
                // NSWorkspace can enumerate the new process before AppKit has
                // finished launching it. It is not restart success evidence;
                // the service keeps polling this non-ready state for its full
                // bounded verification window.
                Ok(RuntimeInspection::NotRunning)
            }
            _ => Ok(RuntimeInspection::Ambiguous),
        },
    }
}

pub(crate) fn request_graceful_shutdown(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
) -> Result<(), InstallerError> {
    terminate_runtime(runner, filesystem, installed, instances, "normal")
}

pub(crate) fn force_shutdown(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
) -> Result<(), InstallerError> {
    terminate_runtime(runner, filesystem, installed, instances, "force")
}

/// Re-enumerate only to compare against the opaque instance evidence captured
/// by the restart operation. A new matching app, PID reuse, or another Stable
/// app is an identity failure rather than evidence that the original exited.
pub(crate) fn is_runtime_instance_running(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
) -> Result<bool, InstallerError> {
    // Liveness is stricter than readiness: an app that was already bound to a
    // restart operation remains alive even if a subsequent AppKit snapshot is
    // not ready. Treating it as exited could launch a second instance before
    // the original identity has actually gone away.
    match inspect_trusted_running_applications(runner, filesystem, installed)? {
        TrustedRunningApplications::NotRunning => Ok(false),
        TrustedRunningApplications::Candidates(current)
            if current
                .iter()
                .map(|application| &application.instance)
                .eq(instances.iter()) =>
        {
            Ok(true)
        }
        TrustedRunningApplications::Candidates(_) | TrustedRunningApplications::Ambiguous => {
            Err(error(
                InstallerErrorCode::MacAppRunning,
                "the trusted runtime changed before restart verification",
            ))
        }
    }
}

fn inspect_trusted_running_applications(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
) -> Result<TrustedRunningApplications, InstallerError> {
    let expected_bundle_path = trusted_installed_bundle_path(filesystem, installed)?;
    let applications = running_applications(runner)?;
    let mut candidates = Vec::new();

    for application in applications {
        if application.bundle_identifier.as_deref() != Some(stable_bundle_id()) {
            continue;
        }
        let raw_bundle_path = application.bundle_path.ok_or_else(|| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application path could not be determined",
            )
        })?;
        let process_id = application
            .process_identifier
            .filter(|pid| *pid > 0)
            .ok_or_else(|| {
                error(
                    InstallerErrorCode::MacAppRunning,
                    "running Stable application process identity could not be determined",
                )
            })?;
        let launch_timestamp_ms = application
            .launch_timestamp_ms
            .filter(|timestamp| *timestamp > 0)
            .ok_or_else(|| {
                error(
                    InstallerErrorCode::MacAppRunning,
                    "running Stable application launch identity could not be determined",
                )
            })?;
        let reported_bundle_path = PathBuf::from(&raw_bundle_path);
        let bundle_path = filesystem
            .canonicalize(Path::new(&raw_bundle_path))
            .map_err(|_| {
                error(
                    InstallerErrorCode::MacAppRunning,
                    "running Stable application path could not be determined",
                )
            })?;
        if bundle_path != expected_bundle_path {
            return Ok(TrustedRunningApplications::Ambiguous);
        }
        candidates.push(TrustedRunningApplication {
            instance: TrustedRuntimeInstance::Macos {
                process_id,
                bundle_path,
                reported_bundle_path,
                launch_timestamp_ms,
            },
            // A missing/invalid readiness value is never upgraded to a ready
            // runtime. The JXA program above always emits this field, so this
            // branch remains fail-closed if its output changes unexpectedly.
            is_finished_launching: application.is_finished_launching == Some(true),
        });
    }

    if candidates.is_empty() {
        Ok(TrustedRunningApplications::NotRunning)
    } else {
        Ok(TrustedRunningApplications::Candidates(candidates))
    }
}

fn trusted_installed_bundle_path(
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
) -> Result<PathBuf, InstallerError> {
    if installed.stable_identity != stable_bundle_id() {
        return Err(error(
            InstallerErrorCode::MacBundleIdMismatch,
            "runtime inspection was requested for a non-Stable bundle identity",
        ));
    }
    let LaunchTarget::MacBundlePath(path) = &installed.launch_target else {
        return Err(error(
            InstallerErrorCode::MacBundleIdMismatch,
            "runtime inspection was requested for a non-macOS launch target",
        ));
    };
    filesystem.canonicalize(path).map_err(|_| {
        error(
            InstallerErrorCode::MacAppRunning,
            "verified Stable bundle path is no longer available",
        )
    })
}

fn running_applications(
    runner: &dyn CommandRunner,
) -> Result<Vec<RunningApplication>, InstallerError> {
    let output = runner
        .run(&command(
            "osascript",
            vec![
                OsString::from("-l"),
                OsString::from("JavaScript"),
                OsString::from("-e"),
                OsString::from(RUNNING_APPLICATIONS_JXA),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "running Stable application state could not be determined",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "running Stable application state could not be determined",
        ));
    }
    serde_json::from_slice::<Vec<RunningApplication>>(output.stdout()).map_err(|_| {
        error(
            InstallerErrorCode::MacAppRunning,
            "running Stable application state could not be determined",
        )
    })
}

fn terminate_runtime(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
    mode: &'static str,
) -> Result<(), InstallerError> {
    let RuntimeInspection::Running(current) = inspect_runtime(runner, filesystem, installed)?
    else {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "the trusted runtime changed before termination",
        ));
    };
    if current != instances {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "the trusted runtime changed before termination",
        ));
    }
    let [TrustedRuntimeInstance::Macos {
        process_id,
        reported_bundle_path,
        launch_timestamp_ms,
        ..
    }] = instances
    else {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "the requested runtime is not one verified macOS application",
        ));
    };
    let output = runner
        .run(&command(
            "osascript",
            vec![
                OsString::from("-l"),
                OsString::from("JavaScript"),
                OsString::from("-e"),
                OsString::from(TERMINATE_RUNNING_APPLICATION_JXA),
                OsString::from("--"),
                OsString::from(mode),
                OsString::from(process_id.to_string()),
                reported_bundle_path.clone().into_os_string(),
                OsString::from(launch_timestamp_ms.to_string()),
            ],
        ))
        .map_err(|_| {
            error(
                InstallerErrorCode::MacAppRunning,
                "the trusted Stable application could not be asked to terminate",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::MacAppRunning,
            "the trusted Stable application rejected the termination request",
        ));
    }
    Ok(())
}

pub(crate) fn launch_verified(
    runner: &dyn CommandRunner,
    filesystem: &dyn MacosFilesystem,
    host: &MacosHost,
    installed: &InstalledApplication,
) -> Result<(), InstallerError> {
    if installed.stable_identity != stable_bundle_id() {
        return Err(error(
            InstallerErrorCode::LaunchFailed,
            "launch was requested for a non-Stable bundle identity",
        ));
    }
    let requested_path = match &installed.launch_target {
        LaunchTarget::MacBundlePath(path) => filesystem.canonicalize(path).map_err(|_| {
            error(
                InstallerErrorCode::LaunchFailed,
                "the verified Stable application path is no longer available",
            )
        })?,
        LaunchTarget::WindowsAumid(_) => {
            return Err(error(
                InstallerErrorCode::LaunchFailed,
                "launch target does not belong to macOS",
            ))
        }
    };
    let local = inspect_local(runner, filesystem, host)?;
    let current = match local {
        LocalInstallStatus::Installed { application } => application,
        LocalInstallStatus::Ambiguous { .. } => {
            return Err(error(
                InstallerErrorCode::MacMultipleInstallations,
                "multiple Stable bundles prevent a safe automatic launch",
            ))
        }
        _ => {
            return Err(error(
                InstallerErrorCode::LaunchFailed,
                "the verified Stable application is no longer installed",
            ))
        }
    };
    let current_path = match current.launch_target {
        LaunchTarget::MacBundlePath(path) => path,
        LaunchTarget::WindowsAumid(_) => {
            unreachable!("macOS local scan only produces bundle paths")
        }
    };
    if current_path != requested_path {
        return Err(error(
            InstallerErrorCode::LaunchFailed,
            "the Stable application changed after launch was requested",
        ));
    }
    let output = runner
        .run(&command("open", vec![requested_path.into_os_string()]))
        .map_err(|_| {
            error(
                InstallerErrorCode::LaunchFailed,
                "application launch could not be started",
            )
        })?;
    if !output.is_success() {
        return Err(error(
            InstallerErrorCode::LaunchFailed,
            "application launch was rejected by macOS",
        ));
    }
    Ok(())
}

fn installed_application(bundle: &BundleInfo) -> InstalledApplication {
    InstalledApplication {
        stable_identity: stable_bundle_id().to_owned(),
        display_name: bundle.display_name.clone(),
        display_version: bundle.display_version.clone(),
        platform_version: bundle.platform_version.clone(),
        architecture: CpuArchitecture::Aarch64,
        location: Some(bundle.bundle_path.to_string_lossy().into_owned()),
        launch_target: LaunchTarget::MacBundlePath(bundle.bundle_path.clone()),
    }
}

fn canonical_existing_directory(
    filesystem: &dyn MacosFilesystem,
    path: &Path,
) -> Result<PathBuf, InstallerError> {
    match filesystem.file_kind(path) {
        Ok(MacosFileKind::Directory) => filesystem.canonicalize(path).map_err(|_| {
            error(
                InstallerErrorCode::PackageParseFailed,
                "a standard Applications directory could not be canonicalized",
            )
        }),
        Err(filesystem_error) if is_not_found(filesystem_error) => Err(error(
            InstallerErrorCode::MacAppNotFound,
            "a standard Applications directory is absent",
        )),
        _ => Err(error(
            InstallerErrorCode::PackageParseFailed,
            "a standard Applications path is not a directory",
        )),
    }
}

fn has_app_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

fn required_plist_string(
    value: Option<String>,
    field: &'static str,
) -> Result<String, InstallerError> {
    let value = value.ok_or_else(|| {
        error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist is missing a required field",
        )
        .with_context("field", field)
    })?;
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application Info.plist has an invalid field",
        )
        .with_context("field", field));
    }
    Ok(value)
}

fn optional_plist_string(
    value: Option<String>,
    field: &'static str,
) -> Result<Option<String>, InstallerError> {
    value
        .map(|value| required_plist_string(Some(value), field))
        .transpose()
}

fn validate_executable_name(value: &str) -> Result<(), InstallerError> {
    if value.is_empty()
        || value.trim() != value
        || value == "."
        || value == ".."
        || value.contains(['/', '\\', '\0'])
        || value.contains("..")
        || value.chars().any(char::is_control)
    {
        return Err(error(
            InstallerErrorCode::PackageParseFailed,
            "application executable name is unsafe",
        ));
    }
    Ok(())
}

fn validate_release_shape(release: &ReleaseDescriptor) -> Result<(), InstallerError> {
    if release.platform != DesktopPlatform::Macos
        || release.architecture != CpuArchitecture::Aarch64
        || release.download_endpoint != TrustedDownloadEndpoint::MacArm64
    {
        return Err(error(
            InstallerErrorCode::ArchitectureUnsupported,
            "the release descriptor is not an Apple-Silicon macOS package",
        ));
    }
    Ok(())
}

fn output_contains_architecture(output: &super::CommandOutput, architecture: &str) -> bool {
    std::str::from_utf8(output.stdout())
        .ok()
        .into_iter()
        .flat_map(str::split_whitespace)
        .any(|value| value == architecture)
}

fn parse_team_identifier(output: &super::CommandOutput) -> Option<&str> {
    std::str::from_utf8(output.stdout())
        .ok()
        .into_iter()
        .flat_map(str::lines)
        .chain(
            std::str::from_utf8(output.stderr())
                .ok()
                .into_iter()
                .flat_map(str::lines),
        )
        .find_map(|line| line.trim().strip_prefix("TeamIdentifier="))
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use super::*;
    use crate::codex_desktop::{
        error::InstallerErrorCode,
        platform::macos::{
            test_support::{FakeFilesystem, FakeRunner},
            MacosHost,
        },
    };

    const SYSTEM_APPLICATIONS: &str = "/Applications";
    const USER_APPLICATIONS: &str = "/Users/test/Applications";

    fn host() -> MacosHost {
        MacosHost::new(
            CpuArchitecture::Aarch64,
            "14.4",
            SYSTEM_APPLICATIONS.into(),
            USER_APPLICATIONS.into(),
        )
        .unwrap()
    }

    fn plist(bundle_identifier: &str, bundle_version: &str, minimum_os: Option<&str>) -> Vec<u8> {
        let minimum_os = minimum_os
            .map(|value| format!(",\"LSMinimumSystemVersion\":\"{value}\""))
            .unwrap_or_default();
        format!(
            "{{\"CFBundleIdentifier\":\"{bundle_identifier}\",\"CFBundleVersion\":\"{bundle_version}\",\"CFBundleShortVersionString\":\"1.0\",\"CFBundleExecutable\":\"Codex\"{minimum_os}}}"
        )
        .into_bytes()
    }

    fn add_bundle(filesystem: &FakeFilesystem, bundle_path: &Path) {
        filesystem.add_dir(bundle_path);
        filesystem.add_file(bundle_path.join("Contents/Info.plist"));
        filesystem.add_file(bundle_path.join("Contents/MacOS/Codex"));
    }

    fn queue_bundle_read(runner: &FakeRunner, plist: Vec<u8>) {
        runner.queue_success("plutil", plist);
    }

    fn queue_bundle_validation(runner: &FakeRunner) {
        runner.queue_success("lipo", b"arm64 x86_64".to_vec());
        runner.queue_success("codesign", Vec::<u8>::new());
        runner.queue_success("codesign", b"TeamIdentifier=2DC432GLL2\n".to_vec());
        runner.queue_success("spctl", Vec::<u8>::new());
    }

    fn queue_stable_bundle_scan(runner: &FakeRunner, bundle_version: &str) {
        let bundle_plist = plist(stable_bundle_id(), bundle_version, None);
        queue_bundle_read(runner, bundle_plist.clone());
        queue_bundle_read(runner, bundle_plist);
        queue_bundle_validation(runner);
    }

    fn read_stable_bundle(
        runner: &FakeRunner,
        filesystem: &FakeFilesystem,
        path: &Path,
    ) -> BundleInfo {
        queue_bundle_read(runner, plist(stable_bundle_id(), "5848", Some("14.0")));
        read_bundle_info(runner, filesystem, path).unwrap()
    }

    #[test]
    fn stable_bundle_uses_identity_not_its_display_directory_name() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &bundle_path);
        let runner = FakeRunner::new();

        let bundle = read_stable_bundle(&runner, &filesystem, &bundle_path);
        queue_bundle_validation(&runner);
        validate_stable_bundle(&runner, &filesystem, &host(), &bundle, None).unwrap();
        runner.assert_drained();

        let invocations = runner.invocations();
        assert_eq!(invocations[0].program(), "plutil");
        assert_eq!(invocations[0].arguments()[0], "-convert");
        assert_eq!(invocations[0].arguments()[1], "json");
        assert_eq!(invocations[1].program(), "lipo");
        assert_eq!(invocations[1].arguments()[0], "-archs");
        assert!(invocations.iter().all(|invocation| {
            invocation.program() != "xattr"
                && !invocation
                    .arguments()
                    .iter()
                    .any(|argument| argument == "--force")
        }));
    }

    #[test]
    fn classic_and_beta_bundles_are_not_promoted_by_directory_name_or_team() {
        let filesystem = FakeFilesystem::new();
        filesystem.add_dir(SYSTEM_APPLICATIONS);
        filesystem.add_dir(USER_APPLICATIONS);
        let classic = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        let beta = Path::new(USER_APPLICATIONS).join("Codex Beta.app");
        add_bundle(&filesystem, &classic);
        add_bundle(&filesystem, &beta);
        let runner = FakeRunner::new();
        queue_bundle_read(&runner, plist("com.openai.chat", "5848", None));
        queue_bundle_read(&runner, plist("com.openai.codex.beta", "5848", None));

        assert!(matches!(
            inspect_local(&runner, &filesystem, &host()).unwrap(),
            LocalInstallStatus::NotInstalled {
                platform: DesktopPlatform::Macos,
                architecture: CpuArchitecture::Aarch64,
            }
        ));
        assert!(runner
            .invocations()
            .iter()
            .all(|invocation| invocation.program() == "plutil"));
        runner.assert_drained();
    }

    #[test]
    fn malformed_unrelated_bundle_does_not_block_a_valid_stable_bundle() {
        let filesystem = FakeFilesystem::new();
        let malformed = Path::new(SYSTEM_APPLICATIONS).join("Archive.app");
        let stable = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &malformed);
        add_bundle(&filesystem, &stable);
        let runner = FakeRunner::new();
        runner.queue_success(
            "plutil",
            br#"{"CFBundleIdentifier":"com.example.unrelated"}"#.to_vec(),
        );
        queue_stable_bundle_scan(&runner, "5848");

        let LocalInstallStatus::Installed { application } =
            inspect_local(&runner, &filesystem, &host()).unwrap()
        else {
            panic!("the valid Stable bundle must remain discoverable");
        };
        assert_eq!(application.location.as_deref(), stable.to_str());
        assert_eq!(
            runner
                .invocations()
                .iter()
                .filter(|invocation| invocation.program() == "lipo")
                .count(),
            1
        );
        runner.assert_drained();
    }

    #[test]
    fn escaped_unrelated_bundle_symlink_does_not_block_local_discovery() {
        let filesystem = FakeFilesystem::new();
        let escaped = Path::new(SYSTEM_APPLICATIONS).join("Archive.app");
        let escaped_target = Path::new("/Volumes/Archive/Archive.app");
        let stable = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        filesystem.add_dir(escaped_target);
        filesystem.add_symlink(&escaped, escaped_target);
        add_bundle(&filesystem, &stable);
        let runner = FakeRunner::new();
        queue_stable_bundle_scan(&runner, "5848");

        let LocalInstallStatus::Installed { application } =
            inspect_local(&runner, &filesystem, &host()).unwrap()
        else {
            panic!("the valid Stable bundle must remain discoverable");
        };
        assert_eq!(application.location.as_deref(), stable.to_str());
        assert_eq!(
            runner
                .invocations()
                .iter()
                .filter(|invocation| invocation.program() == "plutil")
                .count(),
            2
        );
        runner.assert_drained();
    }

    #[test]
    fn strict_package_candidate_check_rejects_an_escaped_bundle_symlink() {
        let filesystem = FakeFilesystem::new();
        let mount_point = Path::new("/Volumes/Codex Installer");
        let escaped = mount_point.join("Codex.app");
        let escaped_target = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        filesystem.add_dir(mount_point);
        filesystem.add_dir(&escaped_target);
        filesystem.add_symlink(&escaped, &escaped_target);

        assert_eq!(
            canonical_top_level_bundle(&filesystem, mount_point, &escaped)
                .unwrap_err()
                .code(),
            InstallerErrorCode::PackageParseFailed
        );
    }

    #[test]
    fn malformed_unrelated_bundle_is_not_installed() {
        let filesystem = FakeFilesystem::new();
        let malformed = Path::new(SYSTEM_APPLICATIONS).join("Archive.app");
        add_bundle(&filesystem, &malformed);
        let runner = FakeRunner::new();
        runner.queue_success("plutil", b"not-json".to_vec());

        assert!(matches!(
            inspect_local(&runner, &filesystem, &host()).unwrap(),
            LocalInstallStatus::NotInstalled {
                platform: DesktopPlatform::Macos,
                architecture: CpuArchitecture::Aarch64,
            }
        ));
        runner.assert_drained();
    }

    #[test]
    fn identified_stable_bundle_with_malformed_strict_metadata_fails_closed() {
        let filesystem = FakeFilesystem::new();
        let stable = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &stable);
        let runner = FakeRunner::new();
        let stable_identity_only = format!(
            "{{\"CFBundleIdentifier\":\"{}\",\"CFBundleVersion\":5848}}",
            stable_bundle_id()
        )
        .into_bytes();
        runner.queue_success("plutil", stable_identity_only.clone());
        runner.queue_success("plutil", stable_identity_only);

        assert_eq!(
            inspect_local(&runner, &filesystem, &host())
                .unwrap_err()
                .code(),
            InstallerErrorCode::PackageParseFailed
        );
        runner.assert_drained();
    }

    #[test]
    fn multiple_stable_bundles_are_ambiguous_and_never_auto_selected() {
        let filesystem = FakeFilesystem::new();
        let system_bundle = Path::new(SYSTEM_APPLICATIONS).join("Codex.app");
        let user_bundle = Path::new(USER_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &system_bundle);
        add_bundle(&filesystem, &user_bundle);
        let runner = FakeRunner::new();

        queue_stable_bundle_scan(&runner, "5848");
        queue_stable_bundle_scan(&runner, "5849");

        let status = inspect_local(&runner, &filesystem, &host()).unwrap();
        let LocalInstallStatus::Ambiguous { candidates, error } = status else {
            panic!("multiple Stable bundles must be ambiguous");
        };
        assert_eq!(candidates.len(), 2);
        assert_eq!(error.code, InstallerErrorCode::MacMultipleInstallations);
        runner.assert_drained();
    }

    #[test]
    fn verification_rejects_wrong_team_architecture_gatekeeper_and_minimum_os() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &bundle_path);

        let wrong_team_runner = FakeRunner::new();
        let bundle = read_stable_bundle(&wrong_team_runner, &filesystem, &bundle_path);
        wrong_team_runner.queue_success("lipo", b"arm64".to_vec());
        wrong_team_runner.queue_success("codesign", Vec::<u8>::new());
        wrong_team_runner.queue_success("codesign", b"TeamIdentifier=OTHERTEAM\n".to_vec());
        assert_eq!(
            validate_stable_bundle(&wrong_team_runner, &filesystem, &host(), &bundle, None)
                .unwrap_err()
                .code(),
            InstallerErrorCode::MacTeamIdMismatch
        );

        let wrong_arch_runner = FakeRunner::new();
        let bundle = read_stable_bundle(&wrong_arch_runner, &filesystem, &bundle_path);
        wrong_arch_runner.queue_success("lipo", b"x86_64".to_vec());
        assert_eq!(
            validate_stable_bundle(&wrong_arch_runner, &filesystem, &host(), &bundle, None)
                .unwrap_err()
                .code(),
            InstallerErrorCode::PackageArchitectureMismatch
        );

        let gatekeeper_runner = FakeRunner::new();
        let bundle = read_stable_bundle(&gatekeeper_runner, &filesystem, &bundle_path);
        gatekeeper_runner.queue_success("lipo", b"arm64".to_vec());
        gatekeeper_runner.queue_success("codesign", Vec::<u8>::new());
        gatekeeper_runner.queue_success("codesign", b"TeamIdentifier=2DC432GLL2\n".to_vec());
        gatekeeper_runner.queue_failure("spctl", Some(3), b"rejected".to_vec());
        assert_eq!(
            validate_stable_bundle(&gatekeeper_runner, &filesystem, &host(), &bundle, None)
                .unwrap_err()
                .code(),
            InstallerErrorCode::MacGatekeeperRejected
        );

        let min_os_runner = FakeRunner::new();
        queue_bundle_read(
            &min_os_runner,
            plist(stable_bundle_id(), "5848", Some("15.0")),
        );
        let bundle = read_bundle_info(&min_os_runner, &filesystem, &bundle_path).unwrap();
        assert_eq!(
            validate_stable_bundle(&min_os_runner, &filesystem, &host(), &bundle, None)
                .unwrap_err()
                .code(),
            InstallerErrorCode::OsVersionUnsupported
        );
        min_os_runner.assert_drained();
    }

    #[test]
    fn malformed_plist_is_a_package_parse_failure() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(&filesystem, &bundle_path);
        let runner = FakeRunner::new();
        runner.queue_success("plutil", b"not-json".to_vec());

        assert_eq!(
            read_bundle_info(&runner, &filesystem, &bundle_path)
                .unwrap_err()
                .code(),
            InstallerErrorCode::PackageParseFailed
        );
        runner.assert_drained();
    }

    #[test]
    fn running_state_matches_exact_bundle_id_and_canonical_path() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        filesystem.add_dir(&bundle_path);
        let runner = FakeRunner::new();
        runner.queue_success(
            "osascript",
            format!(
                "[{{\"bundleIdentifier\":\"{}\",\"bundlePath\":\"{}\"}}]",
                stable_bundle_id(),
                bundle_path.display()
            )
            .into_bytes(),
        );

        assert_eq!(
            ensure_not_running(&runner, &filesystem, &bundle_path)
                .unwrap_err()
                .code(),
            InstallerErrorCode::MacAppRunning
        );
        let invocation = runner.invocations().pop().unwrap();
        assert_eq!(invocation.program(), "osascript");
        assert_eq!(invocation.arguments()[0], "-l");
        assert_eq!(invocation.arguments()[1], "JavaScript");
        assert_eq!(invocation.arguments()[2], "-e");
        assert_ne!(invocation.arguments()[3], bundle_path.as_os_str());
        runner.assert_drained();
    }

    #[test]
    fn runtime_requires_a_matching_bundle_to_finish_launching_before_it_is_ready() {
        let filesystem = FakeFilesystem::new();
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        filesystem.add_dir(&bundle_path);
        let installed = InstalledApplication {
            stable_identity: stable_bundle_id().to_owned(),
            display_name: Some("Codex".to_owned()),
            display_version: Some("1.0".to_owned()),
            platform_version: PlatformVersion::parse_mac_bundle("5848").unwrap(),
            architecture: CpuArchitecture::Aarch64,
            location: None,
            launch_target: LaunchTarget::MacBundlePath(bundle_path.clone()),
        };
        let runner = FakeRunner::new();

        for is_finished_launching in [false, true] {
            runner.queue_success(
                "osascript",
                serde_json::json!([{
                    "bundleIdentifier": stable_bundle_id(),
                    "bundlePath": bundle_path.to_string_lossy(),
                    "processIdentifier": 4242,
                    "launchTimestampMs": 1000,
                    "isFinishedLaunching": is_finished_launching,
                }])
                .to_string()
                .into_bytes(),
            );
        }

        assert!(matches!(
            inspect_runtime(&runner, &filesystem, &installed).unwrap(),
            RuntimeInspection::NotRunning,
        ));
        let ready = inspect_runtime(&runner, &filesystem, &installed).unwrap();
        assert!(matches!(
            ready,
            RuntimeInspection::Running(ref instances) if instances.len() == 1,
        ));

        let invocations = runner.invocations();
        assert!(invocations.iter().all(|invocation| {
            invocation.program() == "osascript"
                && invocation.arguments()[3]
                    .to_string_lossy()
                    .contains("isFinishedLaunching")
        }));
        runner.assert_drained();
    }

    #[test]
    fn launch_rechecks_the_verified_path_and_never_uses_open_by_name() {
        let filesystem = Arc::new(FakeFilesystem::new());
        let bundle_path = Path::new(SYSTEM_APPLICATIONS).join("ChatGPT.app");
        add_bundle(filesystem.as_ref(), &bundle_path);
        filesystem.add_dir(USER_APPLICATIONS);
        let runner = FakeRunner::new();
        queue_stable_bundle_scan(&runner, "5848");
        runner.queue_success("open", Vec::<u8>::new());
        let installed = InstalledApplication {
            stable_identity: stable_bundle_id().to_owned(),
            display_name: Some("Codex".to_owned()),
            display_version: Some("1.0".to_owned()),
            platform_version: PlatformVersion::parse_mac_bundle("5848").unwrap(),
            architecture: CpuArchitecture::Aarch64,
            location: None,
            launch_target: LaunchTarget::MacBundlePath(bundle_path.clone()),
        };

        launch_verified(&runner, filesystem.as_ref(), &host(), &installed).unwrap();
        let invocation = runner.invocations().pop().unwrap();
        assert_eq!(invocation.program(), "open");
        assert_eq!(invocation.arguments(), [bundle_path.as_os_str()]);
        runner.assert_drained();
    }

    #[test]
    fn recorded_macos_identity_fixture_matches_the_exact_stable_allowlists() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/codex_desktop/Codex-mac-arm64-26.721.41059.identity.json"
        )))
        .expect("recorded macOS identity fixture must remain valid JSON");

        assert_eq!(
            fixture["source"]["artifactSha256"].as_str(),
            Some("ae864e2def7db56d0bb77a876a5cbe4e4c2f554ccc654cec921b946892583c0a")
        );
        assert_eq!(
            fixture["bundle"]["bundleIdentifier"].as_str(),
            Some(stable_bundle_id())
        );
        assert_eq!(
            fixture["bundle"]["bundleShortVersion"].as_str(),
            Some("26.721.41059")
        );
        assert_eq!(fixture["bundle"]["bundleVersion"].as_str(), Some("5848"));
        assert_eq!(
            fixture["bundle"]["launcherMachO"]["architecture"].as_str(),
            Some("arm64")
        );
        assert_eq!(
            fixture["signature"]["expectedTeamIdentifier"].as_str(),
            Some(OPENAI_TEAM_IDENTIFIER)
        );
        assert_eq!(
            fixture["signature"]["nativeVerification"]["status"].as_str(),
            Some("pending_macos_hil"),
            "a Windows-host fixture must not be mistaken for native codesign/spctl evidence"
        );
    }
}
