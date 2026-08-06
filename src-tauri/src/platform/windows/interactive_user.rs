//! Windows Explorer-backed interactive-user launcher.
//!
//! Microsoft documents the Explorer `ShellExecute` route specifically for
//! starting an unelevated process from an elevated process. This adapter only
//! invokes that route and returns an unavailable error when the Explorer COM
//! objects cannot be acquired. It has no elevated fallback.

use std::path::Path;

use windows::{
    core::{Interface, BSTR},
    Win32::{
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_LOCAL_SERVER,
                COINIT_APARTMENTTHREADED,
            },
            Variant::VARIANT,
        },
        UI::Shell::{
            IShellDispatch2, IShellWindows, ShellWindows, SWC_EXPLORER, SWFO_NEEDDISPATCH,
        },
    },
};

use crate::platform::process_launch::{InteractiveUserLauncher, ProcessLaunchError};

pub(crate) struct ExplorerInteractiveUserLauncher;

impl InteractiveUserLauncher for ExplorerInteractiveUserLauncher {
    fn open_http_url(&self, url: &str) -> Result<(), ProcessLaunchError> {
        launch_from_explorer(url.to_owned())
    }

    fn open_directory(&self, directory: &Path) -> Result<(), ProcessLaunchError> {
        launch_from_explorer(directory.to_string_lossy().to_string())
    }

    fn open_terminal_script(&self, script: &Path) -> Result<(), ProcessLaunchError> {
        launch_from_explorer(script.to_string_lossy().to_string())
    }

    fn open_trusted_windows_app_aumid(&self, aumid: &str) -> Result<(), ProcessLaunchError> {
        // `shell:AppsFolder\<AUMID>` is Explorer's application namespace form.
        // The common launcher has already accepted only the strict AUMID
        // grammar; this adapter adds no executable, argument, or fallback.
        launch_from_explorer(format!(r"shell:AppsFolder\{aumid}"))
    }
}

/// Runs the COM automation call on a fresh STA thread. A fresh apartment keeps
/// the proxy independent from the Tauri runtime worker's COM mode and lets us
/// balance successful initialization with `CoUninitialize` on the same thread.
fn launch_from_explorer(target: String) -> Result<(), ProcessLaunchError> {
    std::thread::Builder::new()
        .name("fyagent-explorer-launch".to_owned())
        .spawn(move || launch_from_explorer_sta(&target))
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?
        .join()
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?
}

fn launch_from_explorer_sta(target: &str) -> Result<(), ProcessLaunchError> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if initialized.is_err() {
        return Err(ProcessLaunchError::InteractiveUserUnavailable);
    }

    let result = (|| {
        let shell_windows: IShellWindows =
            unsafe { CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER) }
                .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;

        let empty = VARIANT::default();
        let mut explorer_window = 0;
        let explorer_dispatch = unsafe {
            shell_windows.FindWindowSW(
                &empty,
                &empty,
                SWC_EXPLORER,
                &mut explorer_window,
                SWFO_NEEDDISPATCH,
            )
        }
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
        let shell_dispatch: IShellDispatch2 = explorer_dispatch
            .cast()
            .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;

        let target = BSTR::from(target);
        unsafe { shell_dispatch.ShellExecute(&target, &empty, &empty, &empty, &empty) }
            .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)
    })();

    unsafe { CoUninitialize() };
    result
}
