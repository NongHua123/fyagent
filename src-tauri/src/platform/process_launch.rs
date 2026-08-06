//! Fixed business actions for launching ordinary external applications.
//!
//! The renderer never receives a generic executable, argument, working
//! directory, or privilege selector. Callers can only ask this module to open
//! an HTTP(S) URL, a host-owned directory, a backend-generated terminal
//! script, or a verified Windows application as the interactive user.

use std::path::{Path, PathBuf};

use tauri::AppHandle;
use url::Url;

#[cfg(not(target_os = "windows"))]
use tauri_plugin_opener::OpenerExt;

/// Stable, target-free launch failures safe to return across the IPC boundary.
///
/// In particular, these codes intentionally omit URLs and paths because a URL
/// may contain credentials or other user data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessLaunchError {
    InvalidHttpUrl,
    InvalidDirectory,
    InvalidTerminalScript,
    InvalidWindowsAppAumid,
    InteractiveUserUnavailable,
    #[cfg(not(target_os = "windows"))]
    PlatformLaunchFailed,
    #[cfg(target_os = "windows")]
    WorkerFailed,
}

impl ProcessLaunchError {
    fn public_code(self) -> &'static str {
        match self {
            Self::InvalidHttpUrl => "external_launch_invalid_http_url",
            Self::InvalidDirectory => "external_launch_invalid_directory",
            Self::InvalidTerminalScript => "external_launch_invalid_terminal_script",
            Self::InvalidWindowsAppAumid => "external_launch_invalid_windows_app_aumid",
            Self::InteractiveUserUnavailable => "interactive_user_launcher_unavailable",
            #[cfg(not(target_os = "windows"))]
            Self::PlatformLaunchFailed => "external_launch_failed",
            #[cfg(target_os = "windows")]
            Self::WorkerFailed => "interactive_user_launcher_worker_failed",
        }
    }
}

/// The only privileged boundary available to this module. Implementations
/// receive already validated, fixed business actions; they cannot receive an
/// executable, arbitrary arguments, or a caller-selected privilege level.
pub(crate) trait InteractiveUserLauncher: Send + Sync {
    fn open_http_url(&self, url: &str) -> Result<(), ProcessLaunchError>;

    fn open_directory(&self, directory: &Path) -> Result<(), ProcessLaunchError>;

    /// Opens a host-generated `.bat` file without allowing a caller to choose
    /// an executable, an argument vector, or a command interpreter.
    fn open_terminal_script(&self, script: &Path) -> Result<(), ProcessLaunchError>;

    /// Opens a shape-validated AUMID that was already bound to a verified
    /// installed application by the Codex Desktop domain layer.
    fn open_trusted_windows_app_aumid(&self, aumid: &str) -> Result<(), ProcessLaunchError>;
}

/// Injectable business service used by the platform adapter and fake tests.
/// Validation stays here so every caller has the same HTTP(S) and directory
/// boundary before any platform launch operation is reached.
pub(crate) struct ProcessLaunchService<L> {
    launcher: L,
}

impl<L> ProcessLaunchService<L>
where
    L: InteractiveUserLauncher,
{
    pub(crate) fn new(launcher: L) -> Self {
        Self { launcher }
    }

    pub(crate) fn open_http_url_as_user(&self, raw_url: &str) -> Result<(), ProcessLaunchError> {
        let request = InteractiveUserLaunch::http_url(raw_url)?;
        self.dispatch(request)
    }

    pub(crate) fn open_directory_as_user(
        &self,
        directory: &Path,
    ) -> Result<(), ProcessLaunchError> {
        let request = InteractiveUserLaunch::directory(directory)?;
        self.dispatch(request)
    }

    fn open_terminal_script_as_user(&self, script: &Path) -> Result<(), ProcessLaunchError> {
        let request = InteractiveUserLaunch::terminal_script(script)?;
        self.dispatch(request)
    }

    fn open_trusted_windows_app_aumid_as_user(
        &self,
        aumid: &str,
    ) -> Result<(), ProcessLaunchError> {
        let request = InteractiveUserLaunch::trusted_windows_app_aumid(aumid)?;
        self.dispatch(request)
    }

    fn dispatch(&self, request: InteractiveUserLaunch) -> Result<(), ProcessLaunchError> {
        match request {
            InteractiveUserLaunch::HttpUrl(url) => self.launcher.open_http_url(&url),
            InteractiveUserLaunch::Directory(directory) => self.launcher.open_directory(&directory),
            InteractiveUserLaunch::TerminalScript(script) => {
                self.launcher.open_terminal_script(&script)
            }
            InteractiveUserLaunch::TrustedWindowsAppAumid(aumid) => {
                self.launcher.open_trusted_windows_app_aumid(&aumid)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InteractiveUserLaunch {
    HttpUrl(String),
    Directory(PathBuf),
    TerminalScript(PathBuf),
    TrustedWindowsAppAumid(String),
}

impl InteractiveUserLaunch {
    fn http_url(raw_url: &str) -> Result<Self, ProcessLaunchError> {
        Ok(Self::HttpUrl(normalize_http_url(raw_url)?))
    }

    fn directory(directory: &Path) -> Result<Self, ProcessLaunchError> {
        if directory.as_os_str().is_empty()
            || !directory.is_absolute()
            || directory.to_string_lossy().contains('\0')
        {
            return Err(ProcessLaunchError::InvalidDirectory);
        }

        Ok(Self::Directory(directory.to_path_buf()))
    }

    fn terminal_script(script: &Path) -> Result<Self, ProcessLaunchError> {
        let extension_is_batch = script
            .extension()
            .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("bat"));
        if script.as_os_str().is_empty()
            || !script.is_absolute()
            || script.to_string_lossy().contains('\0')
            || !extension_is_batch
        {
            return Err(ProcessLaunchError::InvalidTerminalScript);
        }

        Ok(Self::TerminalScript(script.to_path_buf()))
    }

    fn trusted_windows_app_aumid(aumid: &str) -> Result<Self, ProcessLaunchError> {
        if !is_valid_windows_app_aumid(aumid) {
            return Err(ProcessLaunchError::InvalidWindowsAppAumid);
        }

        Ok(Self::TrustedWindowsAppAumid(aumid.to_owned()))
    }
}

/// Opens an HTTP(S) URL through the interactive user's shell.
///
/// On Windows this takes the Explorer COM route and deliberately fails when
/// Explorer cannot supply the interactive shell. Other platforms retain the
/// existing Tauri opener behavior.
pub(crate) async fn open_http_url_as_user(app: AppHandle, raw_url: String) -> Result<(), String> {
    let request = InteractiveUserLaunch::http_url(&raw_url)
        .map_err(|error| error.public_code().to_owned())?;
    dispatch_with_platform_launcher(app, request)
        .await
        .map_err(|error| error.public_code().to_owned())
}

/// Opens a backend-derived directory through the interactive user's shell.
/// This is intentionally an internal function rather than a new renderer IPC
/// command accepting a filesystem path.
pub(crate) async fn open_directory_as_user(
    app: AppHandle,
    directory: PathBuf,
) -> Result<(), String> {
    let request = InteractiveUserLaunch::directory(&directory)
        .map_err(|error| error.public_code().to_owned())?;
    dispatch_with_platform_launcher(app, request)
        .await
        .map_err(|error| error.public_code().to_owned())
}

/// Opens a fixed, backend-generated terminal batch script through the
/// interactive user's shell. This is deliberately synchronous because the
/// existing terminal helpers already run on a blocking path. There is no
/// non-Windows or elevated-process fallback.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn launch_terminal_script_as_user(script: &Path) -> Result<(), String> {
    let request = InteractiveUserLaunch::terminal_script(script)
        .map_err(|error| error.public_code().to_owned())?;
    dispatch_sync_with_platform_launcher(request).map_err(|error| error.public_code().to_owned())
}

/// Opens a verified Windows application's AUMID through the interactive
/// user's Explorer shell. The caller is responsible for proving package
/// identity; this boundary only validates the AUMID shape before turning it
/// into an AppsFolder item. It is intentionally crate-private rather than an
/// IPC command accepting a renderer-selected app identity.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn launch_trusted_windows_app_aumid_as_user(aumid: &str) -> Result<(), String> {
    let request = InteractiveUserLaunch::trusted_windows_app_aumid(aumid)
        .map_err(|error| error.public_code().to_owned())?;
    dispatch_sync_with_platform_launcher(request).map_err(|error| error.public_code().to_owned())
}

#[cfg(target_os = "windows")]
async fn dispatch_with_platform_launcher(
    _app: AppHandle,
    request: InteractiveUserLaunch,
) -> Result<(), ProcessLaunchError> {
    tokio::task::spawn_blocking(move || {
        ProcessLaunchService::new(
            crate::platform::windows::interactive_user::ExplorerInteractiveUserLauncher,
        )
        .dispatch(request)
    })
    .await
    .map_err(|_| ProcessLaunchError::WorkerFailed)?
}

#[cfg(target_os = "windows")]
fn dispatch_sync_with_platform_launcher(
    request: InteractiveUserLaunch,
) -> Result<(), ProcessLaunchError> {
    ProcessLaunchService::new(
        crate::platform::windows::interactive_user::ExplorerInteractiveUserLauncher,
    )
    .dispatch(request)
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
fn dispatch_sync_with_platform_launcher(
    _request: InteractiveUserLaunch,
) -> Result<(), ProcessLaunchError> {
    Err(ProcessLaunchError::InteractiveUserUnavailable)
}

#[cfg(not(target_os = "windows"))]
async fn dispatch_with_platform_launcher(
    app: AppHandle,
    request: InteractiveUserLaunch,
) -> Result<(), ProcessLaunchError> {
    ProcessLaunchService::new(TauriOpenerInteractiveUserLauncher { app }).dispatch(request)
}

#[cfg(not(target_os = "windows"))]
struct TauriOpenerInteractiveUserLauncher {
    app: AppHandle,
}

#[cfg(not(target_os = "windows"))]
impl InteractiveUserLauncher for TauriOpenerInteractiveUserLauncher {
    fn open_http_url(&self, url: &str) -> Result<(), ProcessLaunchError> {
        self.app
            .opener()
            .open_url(url, None::<String>)
            .map_err(|_| ProcessLaunchError::PlatformLaunchFailed)
    }

    fn open_directory(&self, directory: &Path) -> Result<(), ProcessLaunchError> {
        self.app
            .opener()
            .open_path(directory.to_string_lossy().to_string(), None::<String>)
            .map_err(|_| ProcessLaunchError::PlatformLaunchFailed)
    }

    fn open_terminal_script(&self, _script: &Path) -> Result<(), ProcessLaunchError> {
        Err(ProcessLaunchError::InteractiveUserUnavailable)
    }

    fn open_trusted_windows_app_aumid(&self, _aumid: &str) -> Result<(), ProcessLaunchError> {
        Err(ProcessLaunchError::InteractiveUserUnavailable)
    }
}

fn normalize_http_url(raw_url: &str) -> Result<String, ProcessLaunchError> {
    let raw_url = raw_url.trim();
    if raw_url.is_empty() || raw_url.chars().any(char::is_control) {
        return Err(ProcessLaunchError::InvalidHttpUrl);
    }

    let candidate = match raw_url.split_once(':') {
        Some((scheme, remainder)) if is_uri_scheme(scheme) => {
            if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
                raw_url.to_owned()
            } else if is_bare_host_port(scheme, remainder) {
                format!("https://{raw_url}")
            } else {
                return Err(ProcessLaunchError::InvalidHttpUrl);
            }
        }
        _ => format!("https://{raw_url}"),
    };

    let parsed = Url::parse(&candidate).map_err(|_| ProcessLaunchError::InvalidHttpUrl)?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(ProcessLaunchError::InvalidHttpUrl);
    }

    Ok(parsed.into())
}

fn is_uri_scheme(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

/// Preserve the existing convenient `localhost:3000` / `host:443` shorthand
/// without allowing it to turn non-web URI schemes into launch targets.
fn is_bare_host_port(host: &str, remainder: &str) -> bool {
    if matches!(
        host.to_ascii_lowercase().as_str(),
        "about" | "data" | "file" | "ftp" | "javascript" | "mailto" | "ms-settings" | "tel"
    ) {
        return false;
    }

    let port = remainder.split(['/', '?', '#']).next().unwrap_or_default();
    !host.is_empty() && port.parse::<u16>().is_ok()
}

/// This validates syntax only. The Codex Desktop platform adapter separately
/// binds an AUMID to an exact verified Stable package before this fixed launch
/// action is available.
fn is_valid_windows_app_aumid(value: &str) -> bool {
    let Some((family_name, application_id)) = value.split_once('!') else {
        return false;
    };
    !family_name.is_empty()
        && !family_name.contains('!')
        && family_name.len() <= 512
        && !family_name.bytes().any(|byte| byte.is_ascii_control())
        && !application_id.is_empty()
        && application_id.len() <= 256
        && application_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        sync::{Arc, Mutex},
    };

    use super::{InteractiveUserLauncher, ProcessLaunchError, ProcessLaunchService};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum RecordedLaunch {
        HttpUrl(String),
        Directory(String),
        TerminalScript(String),
        WindowsAppAumid(String),
    }

    #[derive(Default, Clone)]
    struct FakeInteractiveUserLauncher {
        calls: Arc<Mutex<Vec<RecordedLaunch>>>,
        failure: Option<ProcessLaunchError>,
    }

    impl FakeInteractiveUserLauncher {
        fn failing_with(error: ProcessLaunchError) -> Self {
            Self {
                calls: Arc::default(),
                failure: Some(error),
            }
        }

        fn recorded_calls(&self) -> Vec<RecordedLaunch> {
            self.calls.lock().expect("fake lock").clone()
        }
    }

    impl InteractiveUserLauncher for FakeInteractiveUserLauncher {
        fn open_http_url(&self, url: &str) -> Result<(), ProcessLaunchError> {
            self.calls
                .lock()
                .expect("fake lock")
                .push(RecordedLaunch::HttpUrl(url.to_owned()));
            self.failure.map_or(Ok(()), Err)
        }

        fn open_directory(&self, directory: &Path) -> Result<(), ProcessLaunchError> {
            self.calls
                .lock()
                .expect("fake lock")
                .push(RecordedLaunch::Directory(directory.display().to_string()));
            self.failure.map_or(Ok(()), Err)
        }

        fn open_terminal_script(&self, script: &Path) -> Result<(), ProcessLaunchError> {
            self.calls
                .lock()
                .expect("fake lock")
                .push(RecordedLaunch::TerminalScript(script.display().to_string()));
            self.failure.map_or(Ok(()), Err)
        }

        fn open_trusted_windows_app_aumid(&self, aumid: &str) -> Result<(), ProcessLaunchError> {
            self.calls
                .lock()
                .expect("fake lock")
                .push(RecordedLaunch::WindowsAppAumid(aumid.to_owned()));
            self.failure.map_or(Ok(()), Err)
        }
    }

    #[test]
    fn http_shorthand_is_normalized_before_the_fake_launcher_receives_it() {
        let launcher = FakeInteractiveUserLauncher::default();
        let service = ProcessLaunchService::new(launcher.clone());

        service
            .open_http_url_as_user("example.test/releases")
            .expect("valid HTTP shortcut");

        assert_eq!(
            launcher.recorded_calls(),
            vec![RecordedLaunch::HttpUrl(
                "https://example.test/releases".to_owned()
            )]
        );
    }

    #[test]
    fn non_http_schemes_are_rejected_before_the_fake_launcher_runs() {
        let launcher = FakeInteractiveUserLauncher::default();
        let service = ProcessLaunchService::new(launcher.clone());

        for target in [
            "file:///tmp/config.toml",
            "ftp://example.test/file",
            "javascript:alert(1)",
            "mailto:operator@example.test",
            "data:text/plain,untrusted",
        ] {
            assert_eq!(
                service.open_http_url_as_user(target),
                Err(ProcessLaunchError::InvalidHttpUrl),
                "target {target:?} must not reach a launcher"
            );
        }

        assert!(launcher.recorded_calls().is_empty());
    }

    #[test]
    fn valid_host_port_shorthand_remains_a_https_target() {
        let launcher = FakeInteractiveUserLauncher::default();
        let service = ProcessLaunchService::new(launcher.clone());

        service
            .open_http_url_as_user("localhost:3000/dashboard")
            .expect("host and port shortcut");

        assert_eq!(
            launcher.recorded_calls(),
            vec![RecordedLaunch::HttpUrl(
                "https://localhost:3000/dashboard".to_owned()
            )]
        );
    }

    #[test]
    fn relative_or_nul_directories_are_rejected_before_the_fake_launcher_runs() {
        let launcher = FakeInteractiveUserLauncher::default();
        let service = ProcessLaunchService::new(launcher.clone());

        assert_eq!(
            service.open_directory_as_user(Path::new("relative-directory")),
            Err(ProcessLaunchError::InvalidDirectory)
        );
        assert_eq!(
            service.open_directory_as_user(Path::new("/tmp/contains\0nul")),
            Err(ProcessLaunchError::InvalidDirectory)
        );
        assert!(launcher.recorded_calls().is_empty());
    }

    #[test]
    fn interactive_user_failure_has_no_second_launcher_or_fallback() {
        let launcher = FakeInteractiveUserLauncher::failing_with(
            ProcessLaunchError::InteractiveUserUnavailable,
        );
        let service = ProcessLaunchService::new(launcher.clone());

        assert_eq!(
            service.open_http_url_as_user("https://example.test"),
            Err(ProcessLaunchError::InteractiveUserUnavailable)
        );
        assert_eq!(
            launcher.recorded_calls(),
            vec![RecordedLaunch::HttpUrl("https://example.test/".to_owned())]
        );
    }

    #[test]
    fn terminal_script_is_limited_to_an_absolute_backend_batch_file() {
        let launcher = FakeInteractiveUserLauncher::default();
        let service = ProcessLaunchService::new(launcher.clone());
        let temporary = tempfile::tempdir().expect("test directory");
        let batch = temporary.path().join("fyagent_action.bat");

        service
            .open_terminal_script_as_user(&batch)
            .expect("absolute batch path");
        assert_eq!(
            launcher.recorded_calls(),
            vec![RecordedLaunch::TerminalScript(batch.display().to_string())]
        );

        for invalid in [
            Path::new("relative.bat"),
            Path::new("/tmp/not-a-batch.cmd"),
            Path::new("/tmp/contains\0nul.bat"),
        ] {
            assert_eq!(
                service.open_terminal_script_as_user(invalid),
                Err(ProcessLaunchError::InvalidTerminalScript),
                "invalid terminal target {invalid:?} must not reach a launcher"
            );
        }
        assert_eq!(launcher.recorded_calls().len(), 1);
    }

    #[test]
    fn verified_windows_app_launch_rejects_non_aumid_input_before_the_fake_runs() {
        let launcher = FakeInteractiveUserLauncher::default();
        let service = ProcessLaunchService::new(launcher.clone());
        let aumid = "OpenAI.Codex_fixture!CodexApp";

        service
            .open_trusted_windows_app_aumid_as_user(aumid)
            .expect("shape-valid AUMID");
        assert_eq!(
            launcher.recorded_calls(),
            vec![RecordedLaunch::WindowsAppAumid(aumid.to_owned())]
        );

        for invalid in [
            "not-an-aumid",
            "OpenAI.Codex!App!extra",
            "OpenAI.Codex!App/../../unexpected",
            "OpenAI.Codex!\u{0000}App",
        ] {
            assert_eq!(
                service.open_trusted_windows_app_aumid_as_user(invalid),
                Err(ProcessLaunchError::InvalidWindowsAppAumid),
                "invalid AUMID {invalid:?} must not reach a launcher"
            );
        }
        assert_eq!(launcher.recorded_calls().len(), 1);
    }

    #[test]
    fn windows_adapter_source_is_explorer_only_and_has_no_shell_fallback() {
        let source = include_str!("windows/interactive_user.rs");

        assert!(source.contains("IShellWindows"));
        assert!(source.contains("FindWindowSW"));
        assert!(source.contains("ShellExecute"));
        assert!(!source.contains("Command::new"));
        assert!(!source.contains("ShellExecuteW"));
        assert!(!source.contains("\"cmd\""));
        assert!(!source.contains("PowerShell"));
        assert!(!source.contains("/C"));
    }

    #[test]
    fn windows_terminal_and_codex_launchers_stay_on_the_explorer_boundary() {
        let terminal_source = include_str!("../commands/misc.rs");
        let codex_source = include_str!("../codex_desktop/platform/windows/deployment.rs");

        assert_eq!(
            terminal_source
                .matches("launch_terminal_script_as_user")
                .count(),
            2,
            "the two Windows terminal call sites must use the fixed terminal action"
        );
        assert!(!terminal_source.contains("run_windows_start_command"));
        assert!(!terminal_source.contains("\"cmd\", \"/K\""));
        assert!(terminal_source.contains("if elevated_windows_cli_boundary_active()"));
        assert!(terminal_source.contains("ELEVATED_WINDOWS_CLI_BOUNDARY_MESSAGE"));
        assert!(terminal_source.contains("fn run_elevated_cli_lifecycle_whitelist"));
        assert!(!terminal_source.contains("pub fn run_elevated_cli_lifecycle_whitelist"));
        assert!(terminal_source.contains("fn write_persisted_temp_file"));
        assert!(terminal_source.contains("tempfile::Builder::new()"));
        assert!(!terminal_source.contains("temp_dir().join(format!(\"fyagent_"));
        assert!(codex_source.contains("launch_trusted_windows_app_aumid_as_user"));
        assert!(!codex_source.contains("ActivateApplication"));
        assert!(!codex_source.contains("ApplicationActivationManager"));
    }
}
