//! Thin IPC shell for the Codex desktop installer.
//!
//! All state, trust decisions, background work, and structured errors stay in
//! `services::codex_desktop`; these commands only obtain the one application
//! service instance and map its stable error object into the IPC DTO.

use tauri::State;

use crate::{
    codex_desktop::{
        error::{InstallerError, InstallerErrorDto},
        types::{
            CodexDesktopRestartOutcome, CodexDesktopRuntimeStatus, JobSnapshot, LocalInstallStatus,
            RemoteReleaseStatus, StartInstallRequest,
        },
    },
    store::AppState,
};

#[tauri::command]
pub async fn codex_desktop_get_local_status(
    state: State<'_, AppState>,
) -> Result<LocalInstallStatus, InstallerErrorDto> {
    state
        .codex_desktop_service
        .get_local_status()
        .await
        .map_err(to_ipc_error)
}

/// Return a privacy-safe runtime state. The DTO deliberately excludes local
/// paths, PIDs, package identities, and launch commands.
#[tauri::command]
pub async fn get_codex_desktop_runtime_status(
    state: State<'_, AppState>,
) -> Result<CodexDesktopRuntimeStatus, InstallerErrorDto> {
    state
        .codex_desktop_service
        .get_runtime_status()
        .await
        .map_err(to_ipc_error)
}

/// Request a normal, identity-bound Codex Desktop restart. A graceful quit
/// timeout returns only an opaque confirmation token; it never forces a
/// process from this first command.
#[tauri::command]
pub async fn request_codex_desktop_restart(
    state: State<'_, AppState>,
) -> Result<CodexDesktopRestartOutcome, InstallerErrorDto> {
    // Operational failures remain encoded in the restart outcome so the
    // renderer can distinguish unavailable/confirmation/phase failures. The
    // Result envelope satisfies Tauri's async command contract without
    // reclassifying those expected state-machine results as transport errors.
    Ok(state.codex_desktop_service.request_restart().await)
}

/// Complete the force branch only after the renderer has shown a second
/// confirmation. `token` is opaque and is bound server-side to the originally
/// verified runtime instance, rather than being a PID/path/command from IPC.
#[tauri::command]
pub async fn continue_codex_desktop_restart_with_force(
    token: String,
    state: State<'_, AppState>,
) -> Result<CodexDesktopRestartOutcome, InstallerErrorDto> {
    Ok(state
        .codex_desktop_service
        .continue_restart_with_force(&token)
        .await)
}

/// Discard a pending force-confirmation continuation when the user chooses to
/// restart manually. `token` remains opaque; this command neither returns nor
/// accepts a PID, path, process name, or launch instruction.
#[tauri::command]
pub async fn cancel_codex_desktop_restart_with_force(
    token: String,
    state: State<'_, AppState>,
) -> Result<(), InstallerErrorDto> {
    state
        .codex_desktop_service
        .cancel_restart_with_force(&token);
    Ok(())
}

#[tauri::command]
pub async fn codex_desktop_check_latest(
    force: Option<bool>,
    state: State<'_, AppState>,
) -> Result<RemoteReleaseStatus, InstallerErrorDto> {
    state
        .codex_desktop_service
        .check_latest(force.unwrap_or(false))
        .await
        .map_err(to_ipc_error)
}

#[tauri::command]
pub async fn codex_desktop_get_job(
    state: State<'_, AppState>,
) -> Result<Option<JobSnapshot>, InstallerErrorDto> {
    state.codex_desktop_service.get_job().map_err(to_ipc_error)
}

#[tauri::command]
pub async fn codex_desktop_start_install(
    request: StartInstallRequest,
    state: State<'_, AppState>,
) -> Result<JobSnapshot, InstallerErrorDto> {
    state
        .codex_desktop_service
        .start_install(request)
        .map_err(to_ipc_error)
}

#[tauri::command]
pub async fn codex_desktop_cancel_install(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<JobSnapshot, InstallerErrorDto> {
    state
        .codex_desktop_service
        .cancel_install(&job_id)
        .map_err(to_ipc_error)
}

#[tauri::command]
pub async fn codex_desktop_launch(state: State<'_, AppState>) -> Result<(), InstallerErrorDto> {
    state
        .codex_desktop_service
        .launch()
        .await
        .map_err(to_ipc_error)
}

#[tauri::command]
pub async fn codex_desktop_open_log_directory(
    state: State<'_, AppState>,
) -> Result<(), InstallerErrorDto> {
    state
        .codex_desktop_service
        .open_log_directory()
        .map_err(to_ipc_error)
}

fn to_ipc_error(error: InstallerError) -> InstallerErrorDto {
    error.to_dto()
}
