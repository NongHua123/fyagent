use crate::error::AppError;
#[cfg(not(target_os = "windows"))]
use auto_launch::{AutoLaunch, AutoLaunchBuilder};

/// 获取 macOS 上的 .app bundle 路径
/// 将 `/path/to/FyAgent.app/Contents/MacOS/FyAgent` 转换为 `/path/to/FyAgent.app`
#[cfg(target_os = "macos")]
fn get_macos_app_bundle_path(exe_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let path_str = exe_path.to_string_lossy();
    // 查找 .app/Contents/MacOS/ 模式
    if let Some(app_pos) = path_str.find(".app/Contents/MacOS/") {
        let app_bundle_end = app_pos + 4; // ".app" 的结束位置
        Some(std::path::PathBuf::from(&path_str[..app_bundle_end]))
    } else {
        None
    }
}

/// 初始化非 Windows 平台的 AutoLaunch 实例。
///
/// Windows 正式版始终以管理员权限运行，不能安全地注册为登录自启：
/// 该平台仅清理 FyAgent 自己的旧值，绝不触碰历史产品的 OS 注册。
#[cfg(not(target_os = "windows"))]
fn get_auto_launch() -> Result<AutoLaunch, AppError> {
    // macOS derives its login-item name from the bundle; Linux uses this
    // product name for its XDG autostart entry.
    let app_name = "FyAgent";
    let exe_path =
        std::env::current_exe().map_err(|e| AppError::Message(format!("无法获取应用路径: {e}")))?;

    // macOS 需要使用 .app bundle 路径，否则 AppleScript login item 会打开终端
    #[cfg(target_os = "macos")]
    let app_path = get_macos_app_bundle_path(&exe_path).unwrap_or(exe_path);

    #[cfg(not(target_os = "macos"))]
    let app_path = exe_path;

    // 使用 AutoLaunchBuilder 消除平台差异
    // macOS: 使用 AppleScript 方式（默认），需要 .app bundle 路径
    // Linux: 使用 XDG autostart
    let auto_launch = AutoLaunchBuilder::new()
        .set_app_name(app_name)
        .set_app_path(&app_path.to_string_lossy())
        .build()
        .map_err(|e| AppError::Message(format!("创建 AutoLaunch 失败: {e}")))?;

    Ok(auto_launch)
}

#[cfg(target_os = "windows")]
const WINDOWS_RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const WINDOWS_AUTO_LAUNCH_VALUE: &str = "FyAgent";

#[cfg(target_os = "windows")]
fn clear_windows_auto_launch_entry() -> Result<(), AppError> {
    use std::io::ErrorKind;

    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
        RegKey,
    };

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = match current_user.open_subkey_with_flags(WINDOWS_RUN_KEY, KEY_READ | KEY_WRITE) {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(AppError::Message(format!(
                "无法读取 Windows 启动项以清理 FyAgent 旧自启: {error}"
            )));
        }
    };

    match run_key.delete_value(WINDOWS_AUTO_LAUNCH_VALUE) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Message(format!(
            "无法清理 Windows FyAgent 旧自启项: {error}"
        ))),
    }
}

/// Enforce the platform startup policy before the application starts serving UI.
///
/// The Windows branch is intentionally idempotent so an upgrade can clean the
/// prior FyAgent Run value before the settings page has rendered.
#[cfg(target_os = "windows")]
pub fn enforce_platform_auto_launch_policy() -> Result<(), AppError> {
    clear_windows_auto_launch_entry()
}

/// 启用开机自启
pub fn enable_auto_launch() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        clear_windows_auto_launch_entry()?;
        return Err(AppError::Message(
            "Windows 版本已禁用开机自启；已清理 FyAgent 的旧启动项".to_owned(),
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let auto_launch = get_auto_launch()?;
        auto_launch
            .enable()
            .map_err(|e| AppError::Message(format!("启用开机自启失败: {e}")))?;
        log::info!("已启用开机自启");
        Ok(())
    }
}

/// 禁用开机自启
pub fn disable_auto_launch() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        clear_windows_auto_launch_entry()?;
        log::info!("Windows 已禁用并清理 FyAgent 开机自启");
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let auto_launch = get_auto_launch()?;
        auto_launch
            .disable()
            .map_err(|e| AppError::Message(format!("禁用开机自启失败: {e}")))?;
        log::info!("已禁用开机自启");
        Ok(())
    }
}

/// 检查是否已启用开机自启
pub fn is_auto_launch_enabled() -> Result<bool, AppError> {
    #[cfg(target_os = "windows")]
    {
        clear_windows_auto_launch_entry()?;
        return Ok(false);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let auto_launch = get_auto_launch()?;
        auto_launch
            .is_enabled()
            .map_err(|e| AppError::Message(format!("检查开机自启状态失败: {e}")))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_valid() {
        let exe_path = std::path::Path::new("/Applications/FyAgent.app/Contents/MacOS/FyAgent");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(
            result,
            Some(std::path::PathBuf::from("/Applications/FyAgent.app"))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_with_spaces() {
        let exe_path =
            std::path::Path::new("/Users/test/My Apps/FyAgent.app/Contents/MacOS/FyAgent");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(
            result,
            Some(std::path::PathBuf::from("/Users/test/My Apps/FyAgent.app"))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_not_in_bundle() {
        let exe_path = std::path::Path::new("/usr/local/bin/fyagent");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(result, None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_macos_app_bundle_path_dev_build() {
        // 开发环境下的路径通常不在 .app bundle 内
        let exe_path = std::path::Path::new("/Users/dev/project/target/debug/fyagent");
        let result = get_macos_app_bundle_path(exe_path);
        assert_eq!(result, None);
    }
}
