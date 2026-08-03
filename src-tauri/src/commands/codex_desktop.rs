//! Thin IPC shell for the Codex desktop installer.
//!
//! All state, trust decisions, background work, and structured errors stay in
//! `services::codex_desktop`; these commands only obtain the one application
//! service instance and map its stable error object into the IPC DTO.

use tauri::State;

use crate::{
    codex_desktop::{
        error::{InstallerError, InstallerErrorDto},
        types::{JobSnapshot, LocalInstallStatus, RemoteReleaseStatus, StartInstallRequest},
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
