---
status: final
version: v1
verified_at: 2026-07-28
reference_policy: primary sources preferred; dynamic facts must be rechecked by agent
---

# 参考资料与证据

## 1. 使用原则

- 产品定义、Windows API、Apple签名规则优先使用官方一手资料；
- agentsmirror是本 V1 选定的第三方运行时镜像，其 README/Release 是镜像契约来源，不代表 OpenAI/Microsoft背书；
- 身份常量可能随官方包变化，合并前必须从当前签名包 fixture 再确认；
- 文档日期后的动态变化不能靠记忆，Agent应重新查询。

## 2. OpenAI / ChatGPT 官方资料

### ChatGPT desktop app

URL: https://developers.openai.com/codex/app

支持：当前安装指引是下载 Windows/macOS ChatGPT桌面应用，并可在应用中选择 Codex。

### Moving to the new ChatGPT desktop app

URL: https://help.openai.com/en/articles/20001276-moving-to-the-new-chatgpt-desktop-app

支持：原 Codex app更新后成为包含 Chat、Work、Codex的新 ChatGPT应用；旧 ChatGPT可能以 Classic共存。

### ChatGPT release notes

URL: https://help.openai.com/en/articles/6825453-chatgpt-release-notes

支持：新桌面应用整合 Chat、Work、Codex并提供 Windows/macOS版本。动态内容需按日期核验。

## 3. agentsmirror 镜像资料

### codex-app-mirror repository

URL: https://github.com/Wangnov/codex-app-mirror

支持：

- 镜像官方原始 MSIX/DMG，不构建、不修改、不重打包；
- Windows x64/ARM64、macOS ARM64/Intel端点；
- `/latest/manifest`、`/latest/checksums`；
- 中国大陆分流到 S3，其他区域 R2；
- 各架构 `latest` 可独立推进；
- SHA256与release manifest；
- 不绕过本机Windows策略；
- MIT许可且与OpenAI/Microsoft无隶属背书。

### Releases

URL: https://github.com/Wangnov/codex-app-mirror/releases

支持：当前平台资产命名、内部版本与 Windows四段包版本区分、发布状态。不得在代码中固定某个当日版本。

### 运行时端点

```text
https://codexapp.agentsmirror.com/latest/win-x64
https://codexapp.agentsmirror.com/latest/win-arm64
https://codexapp.agentsmirror.com/latest/mac-arm64
https://codexapp.agentsmirror.com/latest/checksums
https://codexapp.agentsmirror.com/latest/manifest
```

这些是 V1唯一运行时源契约。开发资料与CI不受此限制。

## 4. Microsoft 官方资料

### AddPackageByUriAsync

URL: https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.addpackagebyuriasync

支持：用指定 URI和选项为当前用户添加包；本地 `file://` 可用。

### ProvisionPackageForAllUsersAsync

URL: https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.provisionpackageforallusersasync

支持：调用者需管理员权限；包必须 staged且位于系统卷；clean reprovision语义。V1因此把all-users设为隐藏实验。

### Package identity overview

URL: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/package-identity-overview

支持：packaged app使用AUMID等身份，不能靠exe名称识别。

### AppListEntry.AppUserModelId

URL: https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.core.applistentry.appusermodelid

支持：AUMID基于Package Family Name和Package Relative Application ID。

### MSIX signing

URL: https://learn.microsoft.com/en-us/windows/msix/package/sign-app-package-using-signtool

支持：MSIX签名用于完整性和签名主体验证。实现可采用等价系统验证API，不要求调用命令行signtool。

## 5. Apple 官方资料

### CFBundleIdentifier

URL: https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleidentifier

支持：Bundle ID在系统中唯一识别app，路径/显示名不是身份根。

### TN2206: macOS Code Signing In Depth

URL: https://developer.apple.com/library/archive/technotes/tn2206/_index.html

支持：

- `codesign --verify --deep --strict`；
- `spctl`使用与Gatekeeper相同的安全评估子系统；
- 修改已签名Bundle会破坏签名；
- 顶层app进行spctl评估。

### Bundle configuration

URL: https://developer.apple.com/documentation/bundleresources/bundle-configuration

支持：Info.plist身份、版本与Bundle结构字段。

## 6. Tauri 与现有技术栈

### Tauri command/state/events

URL: https://v2.tauri.app/develop/calling-rust/

支持：前端invoke Rust commands、managed state与事件通信。实现应优先遵循仓库既有模式。

### Axum/Proxy资料

本功能不修改代理数据平面。无需为安装器引入Axum或新HTTP服务。

## 7. 本地源码基线

### CC Switch 3.18.0 上传快照

本地路径（设计时）：`/mnt/data/cc-switch-main/cc-switch-main`

重点：

- `src-tauri/src/store.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/services/mod.rs`
- `src-tauri/src/commands/misc.rs`
- `src-tauri/src/proxy/http_client.rs`
- `src-tauri/tauri.conf.json`
- `src/App.tsx`
- `src/components/settings/AboutSection.tsx`
- `src/i18n/index.ts`
- `.github/workflows/ci.yml`

实际开发仓库为私有 `NongHua123/cc-switch`，Agent应记录当前 SHA。本文未声称能读取其未上传的最新内容。

### VibeKey 上传快照

本地路径（设计时）：`/mnt/data/vibekey_new-dev-yongjie/vibekey_new-dev-yongjie`

参考：

- `desktop/src-tauri/src/config.rs` 中固定 `agentsmirror/win-x64`；
- `desktop/src-tauri/src/codex/*` 下载、任务、MSIX、inventory、deployment；
- 后端 operation授权展示旧耦合。

使用限制：只参考行为与测试，不复制架构，不引入backend/hardware/auth/断点续传。

## 8. 身份常量核验清单

在合并前，从当前下载且签名验证通过的包提取并写入测试 fixture：

### Windows

```text
Identity Name
Identity Publisher
Package Family Name / PublisherId
ProcessorArchitecture
Identity Version
TargetDeviceFamily MinVersion
Application Id
signature subject/chain result
file SHA-256
```

当前候选：`OpenAI.Codex`、PFN后缀 `2p2nqsd0c76g0`。候选不是放宽策略的理由。

### macOS

```text
CFBundleIdentifier
CFBundleShortVersionString
CFBundleVersion
CFBundleExecutable
LSMinimumSystemVersion
TeamIdentifier
Authority chain
Mach-O architectures
codesign result
spctl result
DMG/app SHA-256
```

当前候选：Bundle `com.openai.codex`、Team `2DC432GLL2`。

若常量变化：停止合并、给出包 hash和官方/镜像证据、更新ADR与fixtures；禁止自动接受“任意OpenAI字符串”。

## 9. 资料可信等级

| 等级 | 来源 | 用途 |
|---|---|---|
| A | OpenAI/Microsoft/Apple官方 | 产品与平台API规则 |
| B | agentsmirror仓库/Release | 选定镜像的端点、分流、元数据契约 |
| C | 上传源码 | 实际改造结构与兼容边界 |
| D | 参考项目/issue | 实现线索，必须由包fixture或官方API验证 |

## 10. 重新核验触发条件

- OpenAI再次改名/改变桌面应用分发；
- mirror endpoint/schema变化；
- MSIX Identity/Publisher/PFN变化；
- mac Bundle ID/Team变化；
- ARM64包长期不可用；
- PackageManager API最低Windows变化；
- macOS最低版本变化；
- 私有仓库升级 Tauri/reqwest/React Query；
- V1进入公开发行。
