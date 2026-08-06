//! Identity-bound Windows Codex Desktop runtime control.
//!
//! This module never selects a process by executable name. It starts from the
//! exact PackageManager-verified AUMID held by `InstalledApplication`, derives
//! its package family, and accepts every matching top-level-window process.
//! Multiple windows for one process are folded into one `(PID, creation time)`
//! record; PIDs and creation times remain private runtime evidence and are
//! rechecked before any close or termination.

use std::{collections::HashSet, mem::size_of};

use windows::{
    core::{BOOL, PWSTR},
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_PARAMETER,
            ERROR_NO_MORE_FILES, FILETIME, HANDLE, HWND, LPARAM,
        },
        Storage::Packaging::Appx::GetPackageFamilyName,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                GetProcessTimes, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
                PROCESS_TERMINATE,
            },
        },
        UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId},
    },
};

use crate::codex_desktop::{
    error::{InstallerError, InstallerErrorCode},
    platform::{RuntimeInspection, TrustedRuntimeInstance, WINDOWS_CODEX_STABLE_IDENTITY},
    types::{InstalledApplication, LaunchTarget},
};

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(handle: HANDLE) -> Result<Self, InstallerError> {
        if handle.is_invalid() {
            return Err(runtime_identity_error());
        }
        Ok(Self(handle))
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // The handle was returned by OpenProcess/CreateToolhelp32Snapshot;
        // closing it is best effort and has no externally visible failure path.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Inspect every exact package-family process that owns a top-level window.
/// The window list is only a UI-process grouping aid: one process may own many
/// windows, but it produces exactly one private runtime record. Background
/// helpers without a top-level window are ignored and never become a
/// process-name-style close target.
pub(super) fn inspect(
    installed: &InstalledApplication,
) -> Result<RuntimeInspection, InstallerError> {
    let package_family_name = trusted_package_family_name(installed)?;
    let candidate_process_ids = matching_process_ids(&package_family_name)?;
    let top_level_windows = collect_top_level_windows(&candidate_process_ids)?;

    let process_ids = top_level_windows
        .into_iter()
        .map(|window| window.process_id)
        .collect::<HashSet<_>>();
    if process_ids.is_empty() {
        return Ok(RuntimeInspection::NotRunning);
    }

    let mut instances = process_ids
        .into_iter()
        .map(|process_id| {
            let process = open_verified_process(
                process_id,
                &package_family_name,
                PROCESS_QUERY_LIMITED_INFORMATION,
            )?;
            let creation_time = process_creation_time(process.raw())?;
            Ok(TrustedRuntimeInstance::Windows {
                package_family_name: package_family_name.clone(),
                process_id,
                creation_time,
            })
        })
        .collect::<Result<Vec<_>, InstallerError>>()?;
    instances.sort_by_key(|instance| instance.restart_identity_key());
    Ok(RuntimeInspection::Running(instances))
}

/// Force-stop every exact bound runtime after the one explicit user
/// confirmation. Each concrete process is package- and creation-time-verified
/// immediately before `TerminateProcess`; an exited process is harmless, but
/// a recycled PID or changed package identity fails closed and stops launch.
pub(super) fn force_shutdown(
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
) -> Result<(), InstallerError> {
    for target in bound_windows_runtimes(installed, instances)? {
        let process = match unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
                false,
                target.process_id,
            )
        } {
            Ok(handle) => OwnedHandle::new(handle)?,
            Err(_) if unsafe { GetLastError() } == ERROR_INVALID_PARAMETER => continue,
            Err(_) => return Err(runtime_identity_error()),
        };
        if package_family_for_process(process.raw()).as_deref()
            != Some(target.package_family_name.as_str())
            || process_creation_time(process.raw())? != target.creation_time
        {
            return Err(runtime_identity_error());
        }
        unsafe { TerminateProcess(process.raw(), 1) }.map_err(|_| force_shutdown_error())?;
    }
    Ok(())
}

/// Check every original exact process only. It deliberately does not call
/// general discovery: a later process is never a replacement shutdown target,
/// while a PID reuse with mismatched evidence fails closed.
pub(super) fn is_instance_running(
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
) -> Result<bool, InstallerError> {
    for target in bound_windows_runtimes(installed, instances)? {
        let process = match unsafe {
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, target.process_id)
        } {
            Ok(handle) => OwnedHandle::new(handle)?,
            Err(_) if unsafe { GetLastError() } == ERROR_INVALID_PARAMETER => continue,
            Err(_) => return Err(runtime_identity_error()),
        };
        if package_family_for_process(process.raw()).as_deref()
            != Some(target.package_family_name.as_str())
            || process_creation_time(process.raw())? != target.creation_time
        {
            return Err(runtime_identity_error());
        }
        return Ok(true);
    }
    Ok(false)
}

#[derive(Clone)]
struct WindowsRuntimeTarget {
    package_family_name: String,
    process_id: u32,
    creation_time: u64,
}

#[derive(Clone, Copy)]
struct TopLevelWindow {
    process_id: u32,
}

fn trusted_package_family_name(installed: &InstalledApplication) -> Result<String, InstallerError> {
    if installed.stable_identity != WINDOWS_CODEX_STABLE_IDENTITY {
        return Err(runtime_identity_error());
    }
    let LaunchTarget::WindowsAumid(aumid) = &installed.launch_target else {
        return Err(runtime_identity_error());
    };
    let Some((package_family_name, application_id)) = aumid.split_once('!') else {
        return Err(runtime_identity_error());
    };
    if package_family_name.is_empty()
        || application_id.is_empty()
        || package_family_name.contains('!')
        || package_family_name
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || application_id.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(runtime_identity_error());
    }
    Ok(package_family_name.to_owned())
}

fn bound_windows_runtimes(
    installed: &InstalledApplication,
    instances: &[TrustedRuntimeInstance],
) -> Result<Vec<WindowsRuntimeTarget>, InstallerError> {
    let expected_family = trusted_package_family_name(installed)?;
    if instances.is_empty() {
        return Err(runtime_identity_error());
    }
    let mut seen = HashSet::new();
    instances
        .iter()
        .map(|instance| match instance {
            TrustedRuntimeInstance::Windows {
                package_family_name,
                process_id,
                creation_time,
            } if package_family_name == &expected_family
                && *process_id != 0
                && *creation_time != 0 =>
            {
                Ok(WindowsRuntimeTarget {
                    package_family_name: expected_family.clone(),
                    process_id: *process_id,
                    creation_time: *creation_time,
                })
            }
            _ => Err(runtime_identity_error()),
        })
        .filter(|target| match target {
            Ok(target) => seen.insert((target.process_id, target.creation_time)),
            Err(_) => true,
        })
        .collect()
}

fn matching_process_ids(package_family_name: &str) -> Result<Vec<u32>, InstallerError> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
        .map_err(|_| runtime_identity_error())?;
    let snapshot = OwnedHandle::new(snapshot)?;
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    if unsafe { Process32FirstW(snapshot.raw(), &mut entry) }.is_err() {
        return if unsafe { GetLastError() } == ERROR_NO_MORE_FILES {
            Ok(Vec::new())
        } else {
            Err(runtime_identity_error())
        };
    }

    let mut matches = Vec::new();
    loop {
        let process_id = entry.th32ProcessID;
        if process_id != 0 {
            // A process we cannot inspect is never treated as trusted. This
            // may withhold restart control, but it can never broaden a close
            // set to a name or a path guess.
            if let Ok(handle) =
                unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
            {
                let handle = OwnedHandle::new(handle)?;
                if package_family_for_process(handle.raw()).as_deref() == Some(package_family_name)
                {
                    matches.push(process_id);
                }
            }
        }

        entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if unsafe { Process32NextW(snapshot.raw(), &mut entry) }.is_err() {
            return if unsafe { GetLastError() } == ERROR_NO_MORE_FILES {
                matches.sort_unstable();
                matches.dedup();
                Ok(matches)
            } else {
                Err(runtime_identity_error())
            };
        }
    }
}

fn collect_top_level_windows(
    candidate_process_ids: &[u32],
) -> Result<Vec<TopLevelWindow>, InstallerError> {
    if candidate_process_ids.is_empty() {
        return Ok(Vec::new());
    }
    let candidate_process_ids = candidate_process_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut windows = WindowCollection {
        candidate_process_ids,
        windows: Vec::new(),
    };
    let lparam = LPARAM((&mut windows as *mut WindowCollection) as isize);
    unsafe { EnumWindows(Some(collect_window_callback), lparam) }
        .map_err(|_| runtime_identity_error())?;
    Ok(windows.windows)
}

struct WindowCollection {
    candidate_process_ids: HashSet<u32>,
    windows: Vec<TopLevelWindow>,
}

unsafe extern "system" fn collect_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let collection = unsafe { &mut *(lparam.0 as *mut WindowCollection) };
    let mut process_id = 0_u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if collection.candidate_process_ids.contains(&process_id) {
        collection.windows.push(TopLevelWindow { process_id });
    }
    BOOL(1)
}

fn open_verified_process(
    process_id: u32,
    expected_package_family_name: &str,
    requested_access: windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS,
) -> Result<OwnedHandle, InstallerError> {
    let handle = unsafe { OpenProcess(requested_access, false, process_id) }
        .map_err(|_| runtime_identity_error())?;
    let handle = OwnedHandle::new(handle)?;
    if package_family_for_process(handle.raw()).as_deref() != Some(expected_package_family_name) {
        return Err(runtime_identity_error());
    }
    Ok(handle)
}

fn package_family_for_process(handle: HANDLE) -> Option<String> {
    let mut required_len = 0_u32;
    let first = unsafe { GetPackageFamilyName(handle, &mut required_len, None) };
    if first != ERROR_INSUFFICIENT_BUFFER || required_len == 0 {
        return None;
    }
    let mut buffer = vec![0_u16; required_len as usize];
    let second = unsafe {
        GetPackageFamilyName(handle, &mut required_len, Some(PWSTR(buffer.as_mut_ptr())))
    };
    if second.0 != 0 {
        return None;
    }
    let length = buffer.iter().position(|value| *value == 0)?;
    let value = String::from_utf16(&buffer[..length]).ok()?;
    (!value.is_empty() && !value.bytes().any(|byte| byte.is_ascii_control())).then_some(value)
}

fn process_creation_time(handle: HANDLE) -> Result<u64, InstallerError> {
    let mut creation_time = FILETIME::default();
    let mut exit_time = FILETIME::default();
    let mut kernel_time = FILETIME::default();
    let mut user_time = FILETIME::default();
    unsafe {
        GetProcessTimes(
            handle,
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        )
    }
    .map_err(|_| runtime_identity_error())?;
    let value =
        (u64::from(creation_time.dwHighDateTime) << 32) | u64::from(creation_time.dwLowDateTime);
    (value != 0)
        .then_some(value)
        .ok_or_else(runtime_identity_error)
}

fn runtime_identity_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
        .with_diagnostic_message("the Windows runtime could not be bound to the verified package")
}

fn force_shutdown_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::LaunchFailed)
        .with_diagnostic_message("the verified Windows runtime could not be force-stopped")
}
