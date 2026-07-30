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

### Tauri Windows installer / WiX template

URL: https://v2.tauri.app/distribute/windows-installer/

支持：Windows MSI 的 WiX 配置与 custom template 入口。本 V1 用它保持 per-user MSI 的
HKCU KeyPath 约束，并在实际 `candle`/`light` 打包中验证；DLL resource 扫描的具体行为以
项目锁定 Tauri bundler 源码和本地生成的 `main.wxs` 为准。

### Axum/Proxy资料

本功能不修改代理数据平面。无需为安装器引入Axum或新HTTP服务。

## 7. 源码基线

### CC Switch 3.18.0 历史快照

设计阶段参考了 CC Switch `3.18.0` 的历史源码快照；固定本地副本路径不属于公开契约。

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

实际实现以当前 `NongHua123/cc-switch` 仓库 checkout 为准，实施者应记录当前 SHA。历史快照不证明后续提交的状态。

### VibeKey 历史参考

设计阶段还参考了 VibeKey 的历史行为与测试材料；固定本地副本路径不属于公开契约。

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

### 2026-07-29 Windows 开发取证记录

本记录只用于锁定 V1 的精确 allowlist 与回归 fixture；它不是运行时下载
来源，也不替代安装时由 Windows `PackageManager` 执行的签名和信任链验证。

- 只读本机系统查询的当前用户包：
  `OpenAI.Codex_26.721.4979.0_x64__2p2nqsd0c76g0`；
  `Identity Name=OpenAI.Codex`，
  `Publisher=CN=50BDFD77-8903-4850-9FFE-6E8522F64D5B`，
  `Version=26.721.4979.0`，`ProcessorArchitecture=x64`，
  `Application Id=App`，`TargetDeviceFamily MinVersion=10.0.19041.0`。
- 同一查询返回 `PublisherId=2p2nqsd0c76g0`、`SignatureKind=Store`、
  `Status=Ok`、`IsDevelopmentMode=False`。这记录的是系统已部署包的信任状态；
  本轮没有下载、部署或保存完整生产 MSIX，也没有将其替代为独立 SignTool 结论。
- 同日从 V1 固定 agentsmirror 的 `/latest/manifest` 与 `/latest/checksums` 读取的
  x64 文件为 `OpenAI.Codex_26.721.4979.0_x64__2p2nqsd0c76g0.Msix`，SHA-256 为
  `f0c1d75045952a11a581d34f28f595d1d110fb13f8f7e5c5201802ed2bbd7093`；其包全名、
  版本和 Publisher ID 与上述 Store 包一致。
- 匿名化 Identity fixture：
  `src-tauri/tests/fixtures/codex_desktop/OpenAI.Codex_26.721.4979.0_x64__2p2nqsd0c76g0.AppxManifest.xml`。
  它只保留身份、架构、最小系统版本和 Application ID，绝不含完整包内容或签名。
- 若 Identity、Publisher、Publisher ID 或签名/系统状态发生变化，必须停止合并，
  重新取得同等证据并更新本记录、fixture 与 exact allowlist；不得根据 PFN、文件名、
  镜像元数据或前缀比较自动接受变化。

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

### 2026-07-29 macOS ARM64 开发取证记录

本记录固定当前下载包的最小 identity/版本/哈希证据，用于回归 fixture 与
exact allowlist 审计；它不替代 macOS 上的 `codesign`、`spctl` 或 Authority chain
验收，也不使 AgentsMirror 元数据成为运行时信任锚。

- 同日读取 V1 固定 AgentsMirror `/latest/manifest`（schema 5，
  `generatedAt=2026-07-24T21:24:48Z`，
  `publishedAt=2026-07-24T21:33:02Z`）。arm64 条目把镜像名
  `Codex-mac-arm64.dmg` 映射到上游文件
  `ChatGPT-26.721.41059-arm64.dmg`，长度 `593861752`，SHA-256 为
  `ae864e2def7db56d0bb77a876a5cbe4e4c2f554ccc654cec921b946892583c0a`。
- 开发主机在不保存进仓库的临时目录下载该当前 DMG、计算 SHA-256，并与 manifest
  一致。使用归档只读列举/提取顶层
  `ChatGPT Installer/ChatGPT.app/Contents/Info.plist`，得到：
  `CFBundleIdentifier=com.openai.codex`，
  `CFBundleShortVersionString=26.721.41059`，`CFBundleVersion=5848`，
  `CFBundleExecutable=ChatGPT`，`CFBundleName=ChatGPT`，
  `LSMinimumSystemVersion=12.0`。同一包的 launcher Mach-O header 是
  `cffaedfe` / CPU type `0x0100000c`，即 arm64。
- 匿名化 identity/provenance fixture：
  `src-tauri/tests/fixtures/codex_desktop/Codex-mac-arm64-26.721.41059.identity.json`。
  它只保留上述身份、版本、架构、来源日期和哈希，不含 DMG、预签名 URL、query 或个人路径；
  Rust 测试把 Bundle ID 和 Team allowlist 与该记录绑定。
- 当日 manifest 还声明 `teamIdentifier=2DC432GLL2` 与
  `sparkleArchiveIdentityVerified=true`。这些是镜像元数据交叉信息，**不是**本机
  `codesign` 证明，不能参与运行时信任决定。
- 当前开发主机是 Windows，未运行 Apple `codesign --verify --deep --strict`、
  `codesign --display --verbose=4` 或 `spctl --assess --type execute --verbose=4`，
  因而尚未获得 Authority chain、native codesign result 或 Gatekeeper result。
  这些证据必须在 Apple Silicon macOS HIL 中对同类当前包重新取得；在获得前，
  macOS 平台签收和 V1 最终签收均为 pending，绝不得在 release 中宣称该项通过。

## 9. 资料可信等级

| 等级 | 来源                       | 用途                                   |
| ---- | -------------------------- | -------------------------------------- |
| A    | OpenAI/Microsoft/Apple官方 | 产品与平台API规则                      |
| B    | agentsmirror仓库/Release   | 选定镜像的端点、分流、元数据契约       |
| C    | 上传源码                   | 实际改造结构与兼容边界                 |
| D    | 参考项目/issue             | 实现线索，必须由包fixture或官方API验证 |

## 10. 重新核验触发条件

- OpenAI再次改名/改变桌面应用分发；
- mirror endpoint/schema变化；
- MSIX Identity/Publisher/PFN变化；
- mac Bundle ID/Team变化；
- ARM64包长期不可用；
- PackageManager API最低Windows变化；
- macOS最低版本变化；
- 后续仓库升级 Tauri/reqwest/React Query；
- V1进入公开发行。
