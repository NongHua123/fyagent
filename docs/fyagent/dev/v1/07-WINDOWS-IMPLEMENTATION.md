# Windows 实现规格

## 1. 范围

正式：

- Windows x64 当前用户安装；
- Windows ARM64 当前用户安装；
- 本地检测、更新、启动；
- 应用运行中不自动结束；
- 设备策略/依赖/签名错误明确映射。

实验：

- 所有用户 Stage + Provision；
- 隐藏 CLI/headless；
- UAC；
- 不在普通 UI 或 Tauri start IPC 中暴露；
- 人工结果不阻断 V1。

## 2. 模块

```text
platform/windows/
├── mod.rs          adapter 装配与 cfg
├── manifest.rs     MSIX ZIP/AppxManifest 解析
├── deployment.rs   检测、当前用户部署、launch、post-check
└── elevation.rs    实验 all-users 的参数、UAC、子进程
```

所有 WinRT/Win32 类型留在 Windows 模块，通用层只接收领域 DTO。

## 3. 依赖与 feature

使用 target-specific `windows` crate，最小 feature 示例（Agent 依据实际 API 调整）：

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "...", features = [
  "Foundation",
  "Management_Deployment",
  "ApplicationModel",
  "System",
  "Win32_Foundation",
  "Win32_UI_Shell",
  "Win32_System_Com"
] }
quick-xml = "..."
```

不要开启整个 Windows API feature 集。现有 `windows-sys` 可继续服务原代码；新 WinRT 使用 `windows`，不要求全仓迁移。

## 4. OS 与架构预检

- 运行 target 必须是 Windows；
- 架构只允许 x64/ARM64；
- 从 MSIX `TargetDeviceFamily MinVersion` 获取包要求；
- 检查当前 Windows build；
- 检查所需 `PackageManager` API 是否可用；
- 不满足时下载前 `OS_VERSION_UNSUPPORTED`；
- ARM64 只下载 ARM64 latest；不回退 x64，即使 Windows 能仿真 x64。

## 5. MSIX 安全解析

MSIX 是 ZIP 容器，但 parser 必须有边界：

- 文件总大小已经过 source size 限制；
- 只读取根 `AppxManifest.xml`；
- manifest entry 解压大小设上限，例如 4 MiB；
- 拒绝加密、重复 manifest 或路径变体；
- XML 禁止外部实体/DTD；
- 使用事件式 `quick-xml`，不进行网络解析；
- 只提取允许字段。

建议输出：

```rust
pub struct WindowsPackageManifest {
    pub identity_name: String,
    pub publisher: String,
    pub version: WindowsMsixVersion,
    pub processor_architecture: String,
    pub package_family_name_suffix: Option<String>,
    pub applications: Vec<WindowsApplicationEntry>,
    pub min_windows_version: Option<WindowsVersion>,
    pub dependencies: Vec<PackageDependency>,
}

pub struct WindowsApplicationEntry {
    pub id: String,
    pub executable: Option<String>,
    pub entry_point: Option<String>,
}
```

## 6. 身份门禁

必须：

```text
Identity Name == OpenAI.Codex
Architecture == x64 或 arm64（与当前分支精确一致）
Version == ReleaseDescriptor.WindowsMsix
恰有一个可启动 Stable Application ID，或按 fixture 中明确规则选择
```

必须拒绝：

```text
OpenAI.CodexBeta
Identity 前缀匹配但非精确相等
未知 OpenAI 包
架构 neutral/x86（除非未来 ADR 明确支持）
manifest executable 名作为唯一身份
```

### Publisher allowlist

不要凭记忆硬写 Publisher DN。实现 Agent 应从当前真实官方包/镜像包 fixture 中提取 Publisher，并通过以下步骤固定：

1. 记录 package filename、SHA-256、manifest Identity/Publisher；
2. 使用 Windows PackageManager/系统属性验证该包确为官方签名；
3. 将**精确 Publisher 字符串**作为 V1 allowlist 常量和匿名 manifest fixture；
4. 在 `18-REFERENCES.md` 的开发记录补充取证日期；
5. Publisher 变化时 fail closed，需人工更新 allowlist，不用前缀或 Team 猜测放宽。

Package family name 由 name + publisher ID计算/系统查询，不能用文件名猜。

## 7. 本地安装检测

通过 WinRT `PackageManager` 查询当前用户包，精确筛选 `Id.Name == "OpenAI.Codex"`。

返回：

- `Package.Id.Version`；
- `Package.Id.FamilyName`；
- 当前 manifest 的 Application ID；
- display name（仅 UI）；
- 是否 registered/staged；
- 不泄露 WindowsApps 物理路径。

如果同一 current user 出现多个违反预期的 Stable 结果，返回 Ambiguous，禁止更新。

不要：

- 遍历磁盘找 `ChatGPT.exe`；
- 用进程名决定是否安装；
- 把 Beta 当 Stable；
- 根据 Store title 判断。

## 8. 当前用户安装

主路径：

```text
file://<verified-local-msix>
    ↓
PackageManager.AddPackageByUriAsync
    ↓
DeploymentProgress → JobSnapshot
    ↓
DeploymentResult
    ↓
重新查询 OpenAI.Codex
```

要求：

- 在生成 `file:///` URI 并交给 PackageManager 前，重新打开 job temp 中固定的 MSIX，
  校验它仍是受控目录内的 regular/non-reparse 文件，且 size/SHA-256 与同一锁定
  descriptor 精确一致；不得把早期通过校验的裸路径直接延后消费；
- 使用本地已验证文件，不让 PackageManager 再下载远程 URI；
- 默认不启用 `ForceApplicationShutdown`；
- 不启用绕过策略/开发模式选项；
- 不使用 PowerShell、winget、App Installer UI 作为主路径；
- 依赖包由 OS 正常解析；缺失时明确失败，不从未知站点自动下载；
- `ExtendedErrorCode/HRESULT` 保存诊断；
- 部署过程中的签名/信任失败映射 `PACKAGE_SIGNATURE_INVALID`；
- deployment progress 转换为安装阶段进度，但不和下载字节百分比混为同一尺度。

## 9. 应用运行中更新

不得：

- `taskkill`；
- ForceApplicationShutdown；
- 根据 `Codex.exe` 或 `ChatGPT.exe` 杀进程；
- 自动保存/关闭用户工作。

策略：

1. 可在部署前基于 package identity/AUMID 查询运行状态；
2. 若可靠检测为运行，提前 `WINDOWS_PACKAGE_IN_USE`；
3. 若无法可靠检测，可正常调用 PackageManager；
4. OS 返回占用错误时映射同一稳定码；
5. UI 提示用户保存工作、关闭 ChatGPT/Codex 后重试。

## 10. 安装后验证

部署成功返回不等于业务成功。必须重新查询：

```text
Id.Name == OpenAI.Codex
Package.Id.Version >= target version
Package.Id.Version >= pre-install version
FamilyName 非空
manifest Application ID 可解析
```

首次安装目标版本应等于目标；如果官方内部部署导致更高版本，也可接受 `>= target`，但日志记录实际版本。任何身份缺失或版本回退为 `INSTALLATION_VERIFY_FAILED`。

## 11. 启动

从已安装 package 的 manifest 读取 Application ID：

```text
AUMID = PackageFamilyName + "!" + ApplicationId
```

使用 Windows Shell/ApplicationActivationManager 等系统激活方式。不要：

- 直接运行 WindowsApps 中的 `ChatGPT.exe`；
- 假设 Application ID 永远是 `App` 而不读 manifest；
- 使用镜像文件名构造启动路径；
- 管理或启动 `OpenAI.CodexBeta`。

启动前重新验证 exact Stable identity。

## 12. 设备策略和依赖错误

明确映射：

| 场景                      | 错误码                       |
| ------------------------- | ---------------------------- |
| 组策略/侧载/组织策略阻断  | `WINDOWS_DEPLOYMENT_BLOCKED` |
| 缺少 framework/dependency | `WINDOWS_DEPENDENCY_MISSING` |
| 文件占用                  | `WINDOWS_PACKAGE_IN_USE`     |
| 签名/证书不受信           | `PACKAGE_SIGNATURE_INVALID`  |
| 通用部署 HRESULT          | `WINDOWS_DEPLOYMENT_FAILED`  |

不修改注册表、不开启开发者模式、不绕过策略、不调用第三方 Store 抓包工具。

## 13. 隐藏实验性所有用户安装

### 13.1 用户路径隔离

普通 UI/IPC 不出现 scope。实验入口建议：

```text
fyagent --experimental-install-codex-all-users
fyagent --elevated-provision-codex <job-file> <nonce>
```

第一个入口可以是内部人工测试入口；第二个只能由第一个生成并通过 UAC 启动。

### 13.2 提权原则

主 FyAgent 保持 `asInvoker`。通过 `ShellExecuteW` 的 `runas` verb 启动同一可执行文件的受限子命令。不要：

- 要求整个 GUI 永久管理员运行；
- 安装长期 Windows Service；
- 使用保存的管理员密码；
- 让 elevated child 接受任意命令或 URL。

### 13.3 Job 文件

普通进程：

- 下载并初步验证官方 MSIX；
- 将文件复制/确保位于受控 temp root；
- 创建仅包含必要字段的 job JSON；
- 创建高熵 nonce；
- 限制 ACL（在可行范围）；
- 启动 UAC child。

Job 字段：

```text
schemaVersion
jobId
nonceHash / nonce binding
canonicalPackagePath
expectedSha256
expectedSize
expectedIdentity
expectedPublisher
expectedVersion
expectedArchitecture
minimumOsVersion
createdAt / expiresAt
```

不得包含远程 token 或任意 shell command。

### 13.4 Elevated child 重新验证

必须重新执行：

- 参数数量和 schema；
- job 过期；
- canonical path 位于 FyAgent temp root；
- elevated child独立刷新固定官方 metadata，并逐项比对 job 的 SHA-256、大小、版本、最低 OS 和架构；
- 源文件以 no-follow handle 打开，handle 最终路径必须仍等于 capability path 且为固定本地磁盘；
- 复制到 ProgramData 的受保护 staging 后，再对复制件校验 SHA-256；
- MSIX manifest；
- Identity、Publisher、版本、架构；
- 当前 OS API；
- 系统卷空间；
- 包位于/被 stage 到系统卷。

不能信任 parent 的“verified=true”。

### 13.5 Stage + Provision

实验流程：

```text
StagePackageByUriAsync
       ↓
确认 staged package FamilyName
       ↓
ProvisionPackageForAllUsersAsync
       ↓
受限 headless exit code / 无敏感日志
```

注意：

- Provision 要管理员权限；
- elevated child 不向 parent-owned temp tree 写入结果文件；
- 包必须 staged 且位于系统卷；
- Windows 版本/API 可能限制；
- Store/machine license 或设备策略可能使官方 MSIX 不可预配；
- 不承诺所有已有用户立即产生相同 UI/登录状态；
- 失败不静默改为当前用户安装；
- 该实验不阻断 V1。

### 13.6 主程序启动分流

`src-tauri/src/main.rs` 在创建 Tauri runtime 之前解析受限 headless 参数：

```rust
fn main() {
    if let Some(exit_code) = maybe_run_codex_desktop_headless() {
        std::process::exit(exit_code);
    }
    fyagent_lib::run();
}
```

parser 只接受精确命令，未知参数走正常应用或明确失败。不要把任意文件执行能力暴露到 headless handler。

## 14. Windows 测试

### Manifest fixture

至少：

- Stable x64；
- Stable ARM64；
- Beta identity；
- wrong Publisher；
- wrong arch；
- malformed XML；
- DTD/entity；
- duplicate manifest；
- multiple applications；
- changed executable `ChatGPT.exe`；
- min OS too high。

### Deployment fake

覆盖：

- progress；
- success post-check；
- signature error；
- policy error；
- dependency error；
- package in use；
- local newer no deployment；
- no force shutdown flag；
- AUMID construction。

### Elevation tests

- job path escape；
- nonce mismatch；
- expired job；
- hash changed after parent verify；
- identity changed；
- UAC cancelled；
- 不向 user-temp 写入结果文件；
- no arbitrary command parsing。

## 15. Windows 完成定义

- Windows runner 编译、Clippy、测试通过；
- x64/ARM64 source mapping 独立；
- 当前用户路径无 UAC；
- UI 无 scope；
- 精确 Stable identity；
- 不杀应用、不绕过策略；
- 正确使用本地 MSIX + PackageManager；
- post-check 和 AUMID launch 有测试；
- all-users 仅隐藏入口且重新验证；
- 未执行真实安装自动化。

## 16. FyAgent 自身 MSI 打包边界

V1 只交付桌面宿主，不声明移动端构建支持。因此 `src-tauri/Cargo.toml` 只保留
`staticlib` 和 `rlib` 输出，不产生未被桌面宿主消费的 `cdylib`。

原因是 Tauri 的 WiX bundler 会扫描 release 目录中的 DLL 并作为 resource 写入 MSI；对于
`InstallScope="perUser"`，WiX ICE38 要求组件使用 HKCU registry value 而非文件作为
KeyPath。自定义 WiX template 已让主程序与未来显式 bundled binary 的 `File` 使用
`KeyPath="no"`，并以 `Software\\{{manufacturer}}\\{{product_name}}` 下的 HKCU value 作为
KeyPath；当前构建输入的产品与 bundle 身份均为 FyAgent。渲染后的精确 registry path、
UpgradeCode 和 AppUserModelID 由 Draft PR CI 生成的 WiX 证据确认。不能靠
关闭 ICE 验证或忽略链接错误来规避这一约束。

本地 Windows x64 已从全新的、忽略的 `src-tauri/target/v1-msi` 目录运行
`pnpm tauri build --bundles msi`，并完成 Tauri `candle`/`light` 链接。产物为
`FyAgent_3.18.0_x64_en-US.msi`，SHA-256 为
`49214C116A9DFE0D1E7FF1CE2A8EA1665FEF3F0961F1A613B6F842D046063948`；生成的
`main.wxs` 不含 `cc_switch_lib.dll` resource；这是 clean-break 重命名之前的历史构建证据，
仅证明当时的 ICE38 边界。重命名后的 `fyagent`/`fyagent_lib` 产物必须以 Draft PR CI
结果为准。该本地产物的 Authenticode 状态为
`NotSigned`，不构成发布签名或真实安装验收证据。
