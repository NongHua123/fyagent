use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::{
    Foundation::{ERROR_INVALID_DATA, ERROR_MORE_DATA, ERROR_SUCCESS},
    System::ApplicationInstallationAndServicing::{
        MsiCloseHandle, MsiCreateRecord, MsiGetPropertyW, MsiProcessMessage, MsiRecordSetStringW,
        MsiSetPropertyW, INSTALLMESSAGE_INFO, MSIHANDLE,
    },
};

use crate::{
    messages,
    policy::{
        DirectoryPolicyError, NormalizedDosPath, TargetState, ValidationContext, ValidationPhase,
    },
};

pub(crate) const INSTALLDIR: &str = "INSTALLDIR";
// The package clears this public AppSearch target before its first sequence;
// afterward it may only contain the HKLM lookup result for the session.
const PREVIOUS_INSTALLDIR: &str = "FYAGENT_PREVIOUS_INSTALLDIR";
const VALID: &str = "FYAGENT_INSTALLDIR_VALID";
const ERROR_CODE: &str = "FYAGENT_INSTALLDIR_ERROR_CODE";
const ERROR_MESSAGE: &str = "FYAGENT_INSTALLDIR_ERROR_MESSAGE";
const NORMALIZED: &str = "FYAGENT_INSTALLDIR_NORMALIZED";
const CHECK_ID: &str = "FYAGENT_INSTALLDIR_CHECK_ID";

static CHECK_SEQUENCE: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Copy, Debug)]
pub(crate) struct MsiError {
    pub(crate) code: u32,
}

pub(crate) struct MsiSession {
    handle: MSIHANDLE,
}

impl MsiSession {
    pub(crate) const fn new(handle: MSIHANDLE) -> Self {
        Self { handle }
    }

    pub(crate) fn next_check_id(&self) -> String {
        // MSI may invoke the UI and execute actions in one process. A compact
        // monotonic identifier is enough to correlate those log lines without
        // adding a random or user-derived value to the installer surface.
        format!(
            "{:04X}",
            CHECK_SEQUENCE.fetch_add(1, Ordering::Relaxed) & 0xFFFF
        )
    }

    pub(crate) fn clear_validation_properties(&self, check_id: &str) -> Result<(), MsiError> {
        for property in [VALID, ERROR_CODE, ERROR_MESSAGE, NORMALIZED] {
            self.set_property(property, "")?;
        }
        self.set_property(CHECK_ID, check_id)
    }

    pub(crate) fn get_property(&self, name: &str) -> Result<String, MsiError> {
        let name = wide_null(name);
        let mut reported_length = 0_u32;
        let first_status = unsafe {
            // The MSI API reports the number of UTF-16 code units required;
            // no MAX_PATH-sized buffer is assumed.
            MsiGetPropertyW(
                self.handle,
                name.as_ptr(),
                std::ptr::null_mut(),
                &mut reported_length,
            )
        };
        if first_status == ERROR_SUCCESS {
            return Ok(String::new());
        }
        if first_status != ERROR_MORE_DATA {
            return Err(MsiError { code: first_status });
        }

        let mut capacity = reported_length
            .checked_add(1)
            .filter(|length| *length <= 32_768)
            .ok_or(MsiError {
                code: ERROR_INVALID_DATA,
            })?;
        loop {
            let mut buffer = vec![0_u16; capacity as usize];
            let mut length = capacity;
            let status = unsafe {
                // `buffer` remains allocated and writable for the complete
                // call; MSI writes at most `length` UTF-16 code units plus its
                // terminating NUL.
                MsiGetPropertyW(self.handle, name.as_ptr(), buffer.as_mut_ptr(), &mut length)
            };
            if status == ERROR_SUCCESS {
                return String::from_utf16(&buffer[..length as usize]).map_err(|_| MsiError {
                    code: ERROR_INVALID_DATA,
                });
            }
            if status != ERROR_MORE_DATA {
                return Err(MsiError { code: status });
            }
            capacity = length
                .checked_add(1)
                .filter(|next| *next > buffer.len() as u32 && *next <= 32_768)
                .ok_or(MsiError {
                    code: ERROR_INVALID_DATA,
                })?;
        }
    }

    pub(crate) fn validation_context(&self) -> Result<ValidationContext, MsiError> {
        let installed = self.property_is_set("Installed")?;
        let upgrade_detected = self.property_is_set("WIX_UPGRADE_DETECTED")?
            || self.property_is_set("UPGRADINGPRODUCTCODE")?;
        let previous = self.get_property(PREVIOUS_INSTALLDIR)?;
        let maintenance = installed || upgrade_detected || !previous.is_empty();
        if !maintenance {
            return Ok(ValidationContext::first_install());
        }

        match NormalizedDosPath::parse(&previous) {
            Ok(path) => Ok(ValidationContext::existing_product(path)),
            Err(_) => Ok(ValidationContext::existing_product_required()),
        }
    }

    pub(crate) fn set_allowed(&self, check_id: &str, normalized: &str) -> Result<(), MsiError> {
        self.set_property(VALID, "1")?;
        self.set_property(ERROR_CODE, "")?;
        self.set_property(ERROR_MESSAGE, "")?;
        self.set_property(NORMALIZED, normalized)?;
        self.set_property(CHECK_ID, check_id)
    }

    pub(crate) fn set_rejected(
        &self,
        check_id: &str,
        error: &DirectoryPolicyError,
        normalized: &str,
    ) -> Result<(), MsiError> {
        self.set_property(VALID, "0")?;
        self.set_property(ERROR_CODE, error.code())?;
        self.set_property(ERROR_MESSAGE, messages::user_message(error))?;
        self.set_property(NORMALIZED, normalized)?;
        self.set_property(CHECK_ID, check_id)
    }

    pub(crate) fn log_allow(
        &self,
        phase: ValidationPhase,
        check_id: &str,
        normalized: &str,
        ancestor: &str,
        target_state: TargetState,
    ) -> Result<(), MsiError> {
        self.log_info(&format!(
            "FyAgentInstallDir check_id={check_id} phase={} result=allow code=FYDIR000 path=\"{normalized}\" ancestor=\"{ancestor}\" target={}",
            phase.as_str(),
            target_state.as_str(),
        ))
    }

    pub(crate) fn log_reject(
        &self,
        phase: ValidationPhase,
        check_id: &str,
        error: &DirectoryPolicyError,
        normalized: &str,
    ) -> Result<(), MsiError> {
        let win32 = error
            .win32()
            .map(|value| format!(" win32={value}"))
            .unwrap_or_default();
        self.log_info(&format!(
            "FyAgentInstallDir check_id={check_id} phase={} result=reject code={} path=\"{normalized}\" stage={}{}",
            phase.as_str(),
            error.code(),
            error.stage(),
            win32,
        ))
    }

    pub(crate) fn log_fatal(
        &self,
        phase: ValidationPhase,
        code: &str,
        stage: &str,
        win32: Option<u32>,
    ) -> Result<(), MsiError> {
        let win32 = win32
            .map(|value| format!(" win32={value}"))
            .unwrap_or_default();
        self.log_info(&format!(
            "FyAgentInstallDir phase={} result=fatal code={code} stage={stage}{win32}",
            phase.as_str(),
        ))
    }

    fn property_is_set(&self, name: &str) -> Result<bool, MsiError> {
        Ok(!self.get_property(name)?.is_empty())
    }

    fn set_property(&self, name: &str, value: &str) -> Result<(), MsiError> {
        let name = wide_null(name);
        let value = wide_null(value);
        let status = unsafe {
            // Both strings are NUL-terminated and stay alive until MSI has
            // consumed them. This action only changes session properties.
            MsiSetPropertyW(self.handle, name.as_ptr(), value.as_ptr())
        };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(MsiError { code: status })
        }
    }

    fn log_info(&self, message: &str) -> Result<(), MsiError> {
        let record = unsafe { MsiCreateRecord(1) };
        if record == 0 {
            return Err(MsiError {
                code: ERROR_INVALID_DATA,
            });
        }
        let result = (|| {
            let message = wide_null(message);
            let status = unsafe {
                // Field zero is the formatted message template. No attacker
                // controlled raw path or ACL data reaches this record.
                MsiRecordSetStringW(record, 0, message.as_ptr())
            };
            if status != ERROR_SUCCESS {
                return Err(MsiError { code: status });
            }
            unsafe {
                MsiProcessMessage(self.handle, INSTALLMESSAGE_INFO, record);
            }
            Ok(())
        })();
        unsafe {
            MsiCloseHandle(record);
        }
        result
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
