# 稳定错误码与诊断规格

## 1. 原则

- UI 依赖稳定错误码，不解析系统文本；
- 一项错误包含用户文案、可重试性、阶段和脱敏诊断；
- 原始 HRESULT、exit code、stderr 进入诊断，不直接成为主文案；
- 严格校验错误不得提供绕过；
- 网络“重试”与用户点击“重试任务”区分。

## 2. 错误载荷

```rust
pub struct InstallerErrorPayload {
    pub code: InstallerErrorCode,
    pub stage: Option<JobStage>,
    pub message_key: String,
    pub retryable: bool,
    pub suggested_action: SuggestedAction,
    pub details: DiagnosticDetails,
}
```

```rust
pub enum SuggestedAction {
    Retry,
    Refresh,
    CloseTargetAppAndRetry,
    ContactAdministrator,
    FreeDiskSpace,
    ResolvePathConflict,
    OpenLogs,
    None,
}
```

## 3. 通用诊断字段

```text
fyagent_version
os
os_version
architecture
job_id
stage
error_code
local_display_version
target_display_version
target_platform_version
release_id_prefix
source = agents-mirror
endpoint_kind (manifest/checksums/win-x64/win-arm64/mac-arm64)
attempt / max_attempts
http_status
platform_error_code
platform_activity_id
redacted_message
timestamp_utc
```

### 禁止字段

- API Key；
- OAuth Token；
- Cookie；
- 完整预签名 URL/query；
- 完整 home 路径；
- Codex 对话或项目内容；
- 其他用户账号信息；
- 安装包二进制内容；
- 证书私钥。

路径脱敏示例：

```text
C:\Users\alice\AppData\Local\Temp\... -> %TEMP%\fyagent-codex-installer\...
/Users/alice/Applications/... -> ~/Applications/...
```

## 4. 错误表

### 4.1 平台

| Code                       | 触发条件                    | 中文主文案                         | English                                             | Retry | Action |
| -------------------------- | --------------------------- | ---------------------------------- | --------------------------------------------------- | ----- | ------ |
| `PLATFORM_UNSUPPORTED`     | Linux/未知平台              | 当前平台不支持一键安装 Codex。     | This platform is not supported.                     | 否    | None   |
| `OS_VERSION_UNSUPPORTED`   | OS低于包/产品要求           | 当前系统版本不满足最新版应用要求。 | Your OS version does not meet the app requirements. | 否    | None   |
| `ARCHITECTURE_UNSUPPORTED` | Intel Mac/未知架构/包不兼容 | 当前处理器架构不受 V1 支持。       | This CPU architecture is not supported in V1.       | 否    | None   |

### 4.2 源与元数据

| Code                       | 触发条件                                                                                                 | 中文主文案                             | English                                                         | Retry | Action   |
| -------------------------- | -------------------------------------------------------------------------------------------------------- | -------------------------------------- | --------------------------------------------------------------- | ----- | -------- |
| `SOURCE_UNAVAILABLE`       | manifest/checksum源经重试仍不可达                                                                        | 暂时无法连接中国大陆优化镜像。         | Unable to reach the China-friendly mirror.                      | 是    | Retry    |
| `RELEASE_METADATA_INVALID` | JSON/schema/字段/size/version无效                                                                        | 最新版本信息无效，已停止安装。         | The release metadata is invalid. Installation was stopped.      | 否    | OpenLogs |
| `RELEASE_NOT_AVAILABLE`    | 当前架构无可下载条目、catalog-only                                                                       | 当前平台的最新版安装包暂不可用。       | The latest package for this platform is not available yet.      | 是    | Refresh  |
| `METADATA_CHANGED`         | expected release与重新解析不一致，或锁定 metadata 后下载 hash 不匹配且刷新 metadata 的 release_id 已变化 | 最新版本已发生变化，请刷新后重新确认。 | The latest release changed. Refresh and try again.              | 是    | Refresh  |
| `REDIRECT_REJECTED`        | metadata 或下载 HTTP降级、超 hop、非法 URL                                                               | 更新源重定向不符合安全策略。           | The update-source redirect was rejected by the security policy. | 否    | OpenLogs |

### 4.3 下载

| Code                      | 触发条件                | 中文主文案         | English                           | Retry        | Action        |
| ------------------------- | ----------------------- | ------------------ | --------------------------------- | ------------ | ------------- |
| `DOWNLOAD_FAILED`         | 非超时传输失败/长度不符 | 安装包下载失败。   | The installer download failed.    | 是（按原因） | Retry         |
| `DOWNLOAD_TIMEOUT`        | 连接/读取超时           | 安装包下载超时。   | The installer download timed out. | 是           | Retry         |
| `DOWNLOAD_CANCELLED`      | 用户取消                | 下载已取消。       | The download was cancelled.       | 是           | Retry         |
| `INSUFFICIENT_DISK_SPACE` | 任一必要卷少于包大小3倍 | 可用磁盘空间不足。 | Not enough free disk space.       | 是           | FreeDiskSpace |

### 4.4 校验

| Code                            | 触发条件                         | 中文主文案                                 | English                                                          | Retry | Action   |
| ------------------------------- | -------------------------------- | ------------------------------------------ | ---------------------------------------------------------------- | ----- | -------- |
| `CHECKSUM_MISSING`              | 目标文件无精确 checksum          | 镜像未提供该安装包的校验和。               | No checksum is available for this package.                       | 否    | OpenLogs |
| `CHECKSUM_MISMATCH`             | 下载 hash与预期不同              | 安装包完整性校验失败，已阻止安装。         | Package integrity verification failed. Installation was blocked. | 否    | OpenLogs |
| `PACKAGE_PARSE_FAILED`          | MSIX/DMG/manifest/Bundle无法解析 | 无法识别下载的安装包。                     | The downloaded package could not be parsed.                      | 否    | OpenLogs |
| `PACKAGE_IDENTITY_MISMATCH`     | Stable身份/Publisher不匹配       | 安装包不是预期的 OpenAI 官方 Stable 应用。 | The package is not the expected official OpenAI Stable app.      | 否    | OpenLogs |
| `PACKAGE_ARCHITECTURE_MISMATCH` | 包架构与 endpoint/机器不符       | 安装包架构与当前设备不匹配。               | The package architecture does not match this device.             | 否    | OpenLogs |
| `PACKAGE_SIGNATURE_INVALID`     | Win trust/codesign失败           | 安装包签名验证失败，已阻止安装。           | Package signature verification failed. Installation was blocked. | 否    | OpenLogs |

### 4.5 Windows

| Code                            | 触发条件                       | 中文主文案                                       | English                                                              | Retry      | Action                 |
| ------------------------------- | ------------------------------ | ------------------------------------------------ | -------------------------------------------------------------------- | ---------- | ---------------------- |
| `WINDOWS_UAC_CANCELLED`         | 内部实验UAC被取消              | 已取消管理员授权。                               | Administrator approval was cancelled.                                | 是         | Retry                  |
| `WINDOWS_ELEVATION_FAILED`      | runas/受限 headless 子进程失败 | 无法启动管理员安装流程。                         | The elevated installation process could not be started.              | 是         | OpenLogs               |
| `WINDOWS_PACKAGE_IN_USE`        | 部署因目标应用运行被拒绝       | 请关闭 ChatGPT/Codex 桌面应用后重试。            | Close the ChatGPT/Codex desktop app and try again.                   | 是         | CloseTargetAppAndRetry |
| `WINDOWS_DEPLOYMENT_BLOCKED`    | 组织/侧载/AppX策略阻断         | Windows 策略阻止了应用安装，请联系设备管理员。   | Windows policy blocked the installation. Contact your administrator. | 否         | ContactAdministrator   |
| `WINDOWS_DEPENDENCY_MISSING`    | PackageManager报告缺少依赖     | Windows 缺少安装所需的系统依赖。                 | A required Windows package dependency is missing.                    | 否         | ContactAdministrator   |
| `WINDOWS_ALL_USERS_UNSUPPORTED` | 实验预配/license/平台不支持    | 当前设备不支持所有用户预配，请使用当前用户安装。 | All-user provisioning is not supported on this device.               | 否         | None                   |
| `WINDOWS_DEPLOYMENT_FAILED`     | 其他部署失败                   | Windows 安装失败。                               | Windows installation failed.                                         | 视原因为准 | Retry/OpenLogs         |

### 4.6 macOS

| Code                         | 触发条件                       | 中文主文案                             | English                                                          | Retry            | Action                 |
| ---------------------------- | ------------------------------ | -------------------------------------- | ---------------------------------------------------------------- | ---------------- | ---------------------- |
| `MAC_DMG_MOUNT_FAILED`       | hdiutil attach失败             | 无法挂载安装镜像。                     | The disk image could not be mounted.                             | 是               | Retry                  |
| `MAC_APP_NOT_FOUND`          | DMG中无唯一Stable app          | 安装镜像中未找到预期应用。             | The expected app was not found in the disk image.                | 否               | OpenLogs               |
| `MAC_BUNDLE_ID_MISMATCH`     | Bundle ID不是Stable            | 应用身份不匹配，已阻止安装。           | The app bundle identity does not match.                          | 否               | OpenLogs               |
| `MAC_TEAM_ID_MISMATCH`       | Team ID不匹配                  | 应用开发者签名身份不匹配。             | The app developer team identity does not match.                  | 否               | OpenLogs               |
| `MAC_GATEKEEPER_REJECTED`    | spctl拒绝                      | macOS 安全检查未接受该应用。           | macOS security assessment rejected the app.                      | 否               | OpenLogs               |
| `MAC_APP_RUNNING`            | Stable运行中更新               | 请关闭 ChatGPT/Codex 桌面应用后重试。  | Close the ChatGPT/Codex desktop app and try again.               | 是               | CloseTargetAppAndRetry |
| `MAC_MULTIPLE_INSTALLATIONS` | 两处以上Stable                 | 检测到多个目标应用安装，请先手动处理。 | Multiple installations were found. Resolve them before updating. | 否               | ResolvePathConflict    |
| `MAC_TARGET_PATH_CONFLICT`   | 两个候选路径均被其他Bundle占用 | 安装路径已被其他应用占用。             | The target app path is occupied by another app.                  | 否               | ResolvePathConflict    |
| `MAC_COPY_FAILED`            | ditto/rename/权限失败          | 无法将应用复制到目标目录。             | The app could not be copied to the target folder.                | 是（权限修复后） | OpenLogs               |
| `MAC_DMG_DETACH_FAILED`      | hdiutil detach失败             | 安装镜像未能正常推出。                 | The disk image could not be detached cleanly.                    | 是/警告          | OpenLogs               |

### 4.7 通用任务

| Code                         | 触发条件                | 中文主文案                   | English                                              | Retry  | Action         |
| ---------------------------- | ----------------------- | ---------------------------- | ---------------------------------------------------- | ------ | -------------- |
| `INSTALLATION_VERIFY_FAILED` | 平台API成功但重扫不匹配 | 安装结果验证失败。           | Installation completed but could not be verified.    | 是     | Retry/OpenLogs |
| `LAUNCH_FAILED`              | 系统激活失败            | 无法启动 Codex 桌面应用。    | The Codex desktop app could not be launched.         | 是     | Retry          |
| `JOB_ALREADY_RUNNING`        | 已有非terminal job      | 已有安装任务正在运行。       | Another installation task is already running.        | 否     | None           |
| `INTERNAL_ERROR`             | 不变量/未知错误         | FyAgent 安装器发生内部错误。 | The FyAgent installer encountered an internal error. | 视情况 | OpenLogs       |

## 5. 非致命 Warning

某些情况不应覆盖主要成功结果：

```text
TEMP_CLEANUP_FAILED
MAC_DMG_DETACH_WARNING
LOG_WRITE_FAILED
EVENT_EMIT_FAILED
REMOTE_CHECK_FAILED_LOCAL_AVAILABLE
```

建议 `JobSnapshot.result.warnings` 返回稳定 warning code。UI 可轻量展示或仅写日志。

## 6. 错误分类

### 6.1 自动网络重试

只对 `SOURCE_UNAVAILABLE`、`DOWNLOAD_TIMEOUT`、部分 `DOWNLOAD_FAILED` 的底层可重试原因在单次任务内自动重试。UI 不应看到每次失败为 terminal error；进度显示 attempt。

### 6.2 用户重试

即使 `retryable=true`，用户点击重试会创建新 Job ID并重新执行全部 preflight/metadata validation。

### 6.3 严格不可绕过

以下永远不提供忽略：

- checksum；
- package parse；
- identity；
- architecture；
- signature；
- Gatekeeper；
- OS version；
- redirect policy。

## 7. 平台错误映射方法

### Windows

建立纯函数：

```rust
fn map_deployment_error(
    hresult: i32,
    error_text: &str,
) -> InstallerErrorCode
```

优先使用 HRESULT/Win32 code；文本只作补充，不以本地化字符串为唯一判断。实际错误集合在 Windows真机人工验收时补 fixture，但不得把未知错误误映射为策略阻断。

### macOS

命令封装提供：

```text
program
exit_status
bounded stdout
bounded stderr
```

映射优先按执行阶段和 exit status。不要依赖完整英文 stderr 作为唯一语义；对 `hdiutil -plist` 使用结构化输出。

## 8. 复制错误详情格式

```text
FyAgent: 3.18.0-based
Time: 2026-...
OS: Windows 11 ... / macOS ...
Architecture: arm64
Job: <uuid>
Stage: installing
Error: WINDOWS_DEPLOYMENT_BLOCKED
Local version: ...
Target version: ...
Release: ab12cd34...
Source: agents-mirror / win-arm64
Attempt: 3/3
HTTP status: ...
Platform code: 0x...
Activity ID: ...
Message: <redacted bounded text>
```

- 预签名 URL只显示 host；
- 用户目录替换为变量；
- 原始文本长度上限，例如 2–4 KiB；
- 控制字符清理；
- 复制失败显示普通 Toast，不影响 Job。

## 9. 日志事件

建议统一前缀：

```text
[CodexDesktop]
[CodexDesktop][Source]
[CodexDesktop][Download]
[CodexDesktop][Windows]
[CodexDesktop][macOS]
```

每个 Job 日志包含 job_id短前缀。避免日志刷屏：进度只在每 10% 或一定字节间隔记录。

## 10. 隐私测试

自动化测试应构造：

- 带 query token的 URL；
- Windows用户路径；
- macOS home路径；
- Authorization header；
- 超长 stderr；
- 控制字符；

断言复制详情和日志格式化器不泄露敏感字段。
