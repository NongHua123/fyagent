//! Narrow Q0 runtime-privilege status for Settings/About.
//!
//! This command deliberately returns only renderer-safe booleans/categories.
//! Windows token details remain inside `windows_runtime` and are never
//! serialized across IPC.

use crate::windows_runtime::RuntimePrivilegeStatus;

#[tauri::command]
pub async fn get_runtime_privilege_status() -> RuntimePrivilegeStatus {
    crate::windows_runtime::runtime_privilege_status()
}
