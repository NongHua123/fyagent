#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(windows), allow(dead_code))]

mod messages;
mod policy;

#[cfg(windows)]
mod msi;
#[cfg(windows)]
mod security;
#[cfg(windows)]
mod windows_path;

#[cfg(windows)]
use std::panic::{catch_unwind, AssertUnwindSafe};

#[cfg(windows)]
use msi::MsiSession;
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{ERROR_INSTALL_FAILURE, ERROR_SUCCESS},
    System::ApplicationInstallationAndServicing::MSIHANDLE,
};

pub use policy::{DirectoryPolicyError, ValidationPhase};

/// WiX Type 1 UI custom action. Expected policy denials are represented in
/// session properties and return success so the dialog can stay recoverable.
///
/// # Safety
///
/// WiX must call this entry point with a live `MSIHANDLE` for the current
/// installer session. The function does not retain or dereference the handle
/// after returning.
#[cfg(windows)]
#[no_mangle]
pub unsafe extern "system" fn ValidateFyAgentInstallDirUi(install: MSIHANDLE) -> u32 {
    invoke_action(install, ValidationPhase::Ui)
}

/// WiX Type 1 execute-sequence custom action. It shares the same policy core
/// as the UI entry; WiX's following Type 19 action turns VALID=0 into a clear
/// install failure before InstallValidate can copy files.
///
/// # Safety
///
/// WiX must call this entry point with a live `MSIHANDLE` for the current
/// installer session. The function does not retain or dereference the handle
/// after returning.
#[cfg(windows)]
#[no_mangle]
pub unsafe extern "system" fn ValidateFyAgentInstallDirExecute(install: MSIHANDLE) -> u32 {
    invoke_action(install, ValidationPhase::Execute)
}

#[cfg(windows)]
fn invoke_action(install: MSIHANDLE, phase: ValidationPhase) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| run_action(install, phase))) {
        Ok(status) => status,
        Err(_) => {
            // Logging itself formats an MSI record and could allocate. Keep a
            // second panic from escaping this FFI boundary while preserving
            // the required fatal MSI status.
            let _ = catch_unwind(AssertUnwindSafe(|| {
                let session = MsiSession::new(install);
                let _ = session.log_fatal(phase, "FYDIR099", "ffi", None);
            }));
            ERROR_INSTALL_FAILURE
        }
    }
}

#[cfg(windows)]
fn run_action(install: MSIHANDLE, phase: ValidationPhase) -> u32 {
    let session = MsiSession::new(install);
    let check_id = session.next_check_id();

    if session.clear_validation_properties(&check_id).is_err() {
        let _ = session.log_fatal(phase, "FYDIR099", "msi-property", None);
        return ERROR_INSTALL_FAILURE;
    }

    let requested = match session.get_property(msi::INSTALLDIR) {
        Ok(value) => value,
        Err(error) => {
            let _ = session.log_fatal(phase, "FYDIR099", "read-install-dir", Some(error.code));
            return ERROR_INSTALL_FAILURE;
        }
    };
    let context = match session.validation_context() {
        Ok(context) => context,
        Err(error) => {
            let _ = session.log_fatal(phase, "FYDIR099", "read-context", Some(error.code));
            return ERROR_INSTALL_FAILURE;
        }
    };

    let result = windows_path::validate_install_directory(&requested, &context);
    match &result.outcome {
        Ok(directory) => {
            if session
                .set_allowed(&check_id, directory.normalized_path.as_str())
                .is_err()
            {
                let _ = session.log_fatal(phase, "FYDIR099", "write-allow", None);
                return ERROR_INSTALL_FAILURE;
            }
            let _ = session.log_allow(
                phase,
                &check_id,
                directory.normalized_path.as_str(),
                directory.nearest_existing_ancestor.as_str(),
                directory.target_state,
            );
            ERROR_SUCCESS
        }
        Err(error) => {
            let normalized = result
                .normalized_path
                .as_ref()
                .map_or("", policy::NormalizedDosPath::as_str);
            if session.set_rejected(&check_id, error, normalized).is_err() {
                let _ = session.log_fatal(phase, "FYDIR099", "write-reject", None);
                return ERROR_INSTALL_FAILURE;
            }
            let _ = session.log_reject(phase, &check_id, error, normalized);
            ERROR_SUCCESS
        }
    }
}

// Keep workspace checks portable. These stubs never claim a policy result and
// therefore cannot accidentally make a non-Windows build usable as an MSI CA.
///
/// # Safety
///
/// This non-Windows test stub does not inspect its opaque handle value and
/// always returns the MSI installation-failure status.
#[cfg(not(windows))]
#[no_mangle]
pub unsafe extern "system" fn ValidateFyAgentInstallDirUi(_install: u32) -> u32 {
    1603
}

/// # Safety
///
/// This non-Windows test stub does not inspect its opaque handle value and
/// always returns the MSI installation-failure status.
#[cfg(not(windows))]
#[no_mangle]
pub unsafe extern "system" fn ValidateFyAgentInstallDirExecute(_install: u32) -> u32 {
    1603
}
