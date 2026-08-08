# 09 — 实施变更地图

## 1. 目的

本文把已确认方案映射到当前上传源码中的具体文件，供实现者按依赖顺序修改。路径均相对仓库根目录。

建议实现时保持小步提交，但最终作为同一个 FyAgent `0.2.1` 补丁版本发布，不拆分用户可见版本。

## 2. 变更总览

| 领域 | 新增 | 修改 | 删除/替换 |
|---|---|---|---|
| 全局版本 | `scripts/version.mjs`、`tests/version.test.mjs` | `package.json`、`src-tauri/Cargo.toml`、`Cargo.lock`、CI/release 脚本 | `package.json.version`、`tauri.conf.json.version`、硬编码版本测试 |
| 原生校验 | `src-tauri/installer-actions/**` | Windows 构建脚本/workflow | 内联 VBScript/WMI |
| WiX UI | `src-tauri/wix/fyagent-install-dir-ui.wxs`（优先） | `per-machine-main.wxs`、`tauri.conf.json` | 旧 UI/Execute 调度 |
| 发布契约 | version-contract job、版本/产物校验脚本 | `.github/workflows/release.yml`、`ci.yml`、下载清单脚本 | 各平台自行从 tag/多配置读取版本 |
| 测试 | Rust、Node、MSI 静态/生命周期测试 | 现有 workflow 测试 | 只断言存在 VBScript 的测试 |

## 3. 第一阶段：建立全局版本单一真源

### 3.1 `src-tauri/Cargo.toml`

**当前状态**：

```toml
[package]
name = "fyagent"
version = "0.2.0"
```

**目标变更**：

```toml
[workspace]
members = [".", "installer-actions"]
resolver = "2"

[workspace.package]
version = "0.2.1"

[package]
name = "fyagent"
version.workspace = true
```

保留其他 package metadata、dependencies、target dependencies 和 profile。不得因为建立 workspace 顺带升级依赖或 resolver。

追踪：`VER-001`～`VER-005`、`NAT-002`。

### 3.2 `src-tauri/installer-actions/Cargo.toml`（新增）

职责：定义只在 Windows 构建的原生 Custom Action `cdylib`。

最低字段：

```toml
[package]
name = "fyagent-installer-actions"
version.workspace = true
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]
```

依赖应最小化，优先 `windows-sys` 或 `windows` 的明确 feature 集；不要依赖 Tauri 主应用。

### 3.3 `src-tauri/Cargo.lock`

由 Cargo 识别 workspace 后更新，使本地包块：

```text
fyagent = 0.2.1
fyagent-installer-actions = 0.2.1
```

不接受无关依赖升级。审查 `git diff`，依赖包版本变化必须回退或单独说明。

### 3.4 `src-tauri/tauri.conf.json`

删除：

```json
"version": "0.2.0"
```

保留 WiX template；后续若采用独立 UI Fragment，再加入 `fragmentPaths`。

### 3.5 `package.json`

删除：

```json
"version": "0.2.0"
```

增加：

```json
"private": true,
"version:get": "node scripts/version.mjs get",
"version:check": "node scripts/version.mjs check",
"version:set": "node scripts/version.mjs set",
"version:bump": "node scripts/version.mjs bump"
```

现有依赖版本和 `packageManager` 不属于应用版本，保持不变。

### 3.6 `scripts/version.mjs`（新增）

从本包 `reference/scripts/version.mjs` 复制到仓库 `scripts/version.mjs`，然后：

- 确认 local package allowlist 与最终 crate 名一致；
- 运行参考测试；
- 扩充回滚、bump、上限和结构异常测试；
- 在 Unix checkout 设置可执行位不是强制条件，因为通过 `node` 调用。

### 3.7 `tests/versionConsistency.test.ts`

当前文件存在硬编码：

```ts
const FYAGENT_V1_0_2_VERSION = "0.2.0";
```

不得改成另一个硬编码 `0.2.1`。选择之一：

1. 删除旧三文件相等测试，改由 `version.mjs check` 测试覆盖；或
2. 保留 Vitest 外壳，只执行脚本并断言退出码，不复制规则。

推荐方案 1 + 独立 Node test，避免同一契约维护两份。

### 3.8 `pnpm-lock.yaml`

预计无版本变更。若只编辑根 `package.json` 的 `private/scripts` 导致 pnpm 需要更新 importer metadata，可接受结构性最小 diff；不得出现依赖版本变化。

## 4. 第二阶段：新增原生目录校验 crate

### 4.1 建议文件结构

```text
src-tauri/installer-actions/
  Cargo.toml
  src/
    lib.rs              # MSI 导出入口、panic 边界
    msi.rs              # Property/Record/ProcessMessage 封装
    policy.rs           # 平台无关策略模型、错误枚举
    windows_path.rs     # 规范化、卷、handle、reparse point
    security.rs         # owner/DACL/ACE/Authz 检查
    messages.rs         # 稳定错误码到用户文案
  tests/
    policy_cases.rs     # 可在 Windows 执行的集成测试
```

如果规模较小，可合并文件，但必须保持：入口层、MSI 封装、策略核心、Windows API 访问分离。

### 4.2 `lib.rs`

导出两个 `extern "system"` 函数，名称与 WiX `DllEntry` 完全一致：

```text
ValidateFyAgentInstallDirUi
ValidateFyAgentInstallDirExecute
```

入口必须：

- 用 `catch_unwind` 阻止 Rust panic 穿越 FFI；
- 从 MSI session 读取 `INSTALLDIR`；
- 清空旧输出属性；
- 调用同一核心 validator；
- UI 策略拒绝时设置属性并返回 success；
- 系统/FFI 异常按阶段记录并 fail closed；
- 不修改文件系统或 ACL。

### 4.3 `policy.rs`

定义稳定结果：

```rust
pub struct ValidationResult {
    pub normalized_path: PathBuf,
    pub outcome: Result<(), PolicyError>,
}
```

`PolicyError` 使用稳定枚举，不把任意 Win32 文本直接展示给用户。

### 4.4 Windows API 模块

按 `04` 设计封装：

- `GetDriveTypeW`；
- `CreateFileW` + `FILE_FLAG_OPEN_REPARSE_POINT`；
- `GetFinalPathNameByHandleW`；
- `GetFileInformationByHandleEx`/attributes；
- `GetNamedSecurityInfoW` 或 handle 版本；
- token/Authz access check；
- well-known SID 比较。

所有 unsafe 块旁写明前置条件、buffer 生命周期和错误处理。

### 4.5 crate 测试

Windows 单元/集成测试必须创建受控临时目录和 ACL。需要管理员权限的测试单独标记，由 Windows VM job 运行；普通解析和错误映射测试可在非管理员上下文运行。

## 5. 第三阶段：替换 WiX 脚本和接入 UI

### 5.1 `src-tauri/wix/per-machine-main.wxs`

**删除**：

- 顶部 `InstallExecuteSequence` 旧 `ValidateInstallDirectory`；
- 顶部 `InstallUISequence` 旧调度；
- `Script="vbscript"` 的完整 CustomAction；
- `Scripting.FileSystemObject`、WMI、`GetSecurityDescriptor`、`Err.Raise`；
- 旧脚本专用属性/注释。

**新增/修改**：

- `<Binary Id="FyAgentInstallerActions" .../>`；
- UI 和 Execute 两个 Type 1 CustomAction；
- Type 19 终止动作；
- `INSTALLDIR`/结果属性的 Secure 声明；
- Execute Sequence 条件；
- 新 UI 引用；
- 升级目录恢复条件；
- 保留 HKLM InstallDir 和最终 ACL。

### 5.2 `src-tauri/wix/fyagent-install-dir-ui.wxs`（建议新增）

从项目当前 WiX v3 UI 源码版本复制必要 Fragment，并只修改 `InstallDirDlg` Next 事件链：

```text
SetTargetPath
WixUIValidatePath
InvalidDirDlg
ValidateFyAgentInstallDirUi
FyAgentUnsafeInstallDirDlg
VerifyReadyDlg
```

同时定义简单的错误对话框。不得复制与本项目无关的大量 UI；保留许可说明和上游来源版本。

### 5.3 `src-tauri/tauri.conf.json`

若 Tauri 当前版本支持并验证 fragmentPaths：

```json
"wix": {
  "template": "wix/per-machine-main.wxs",
  "fragmentPaths": ["wix/fyagent-install-dir-ui.wxs"]
}
```

若实际 bundler 不按预期合并，回退到主模板内嵌 Fragment。不得因工具限制退回 VBScript。

### 5.4 本地化资源

如果当前 MSI 只有单一语言，可先在 WiX 中使用现有安装器语言；但错误码和用户文案映射应独立，便于后续本地化。用户可见文案不得拼接 ACL/SID/原始系统错误。

## 6. 第四阶段：Windows 构建链

### 6.1 `scripts/windows-cross/build-windows-msi.sh`

当前脚本从 `tauri.conf.json` 读取版本，应改为调用：

```bash
APP_VERSION="$(pnpm --silent run version:get)"
pnpm run version:check
```

另外：

- 在 Tauri bundle 前构建目标架构动作 DLL；
- 设置 `FYAGENT_INSTALLER_ACTIONS_DLL` 绝对路径；
- 验证 PE Machine；
- 导出 `Binary`、`CustomAction`、`InstallUISequence`、`Dialog`、`ControlEvent` 等表；
- 断言无脚本动作；
- 把“原生 Windows 生命周期待验证”继续写入候选报告。

### 6.2 `.github/workflows/release.yml`

Windows 原生 job：

- 构建 x64/ARM64 helper；
- 把 DLL 路径传给 WiX template；
- 运行 MSI 静态验证；
- 签名 EXE/MSI；
- 执行原生生命周期测试；
- 资产名使用 `APP_VERSION`。

### 6.3 主 crate 类型

保留主 `src-tauri/Cargo.toml`：

```toml
crate-type = ["staticlib", "rlib"]
```

不要为了 Custom Action 给主应用添加 `cdylib`；独立 crate 已解决这一职责。

## 7. 第五阶段：跨平台发布契约

### 7.1 `.github/workflows/release.yml`

新增 `version-contract` job，并让所有 build jobs `needs` 它。

替换所有：

```text
VERSION=${GITHUB_REF_NAME}
$VERSION = $env:GITHUB_REF_NAME
```

为 `needs.version-contract.outputs.app_version`。

删除用于版本推导的 tag 清洗。标签只用于精确校验和 GitHub Release identity。

### 7.2 `.github/workflows/ci.yml`

在常规 CI 增加：

```bash
pnpm run version:check
node --test tests/version.test.mjs
```

Rust job覆盖整个 workspace：

```bash
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo clippy --workspace --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

非 Windows runner 需要确保 Windows-only helper crate通过 cfg/target 策略不会错误编译；必要时在 workspace default-members 或 CI target矩阵中显式控制。

### 7.3 `scripts/macos-cross/*`

重点修改：

- `project_metadata.py`；
- `preflight.py`；
- `build-package.sh`；
- 任何比较 package/Tauri/Cargo 三个版本的函数。

改为读取单一 `APP_VERSION`，同时保留对 bundle 内嵌版本的独立验收。

### 7.4 `scripts/generate-download-manifest.mjs`

输入应显式接受：

```text
APP_VERSION
RELEASE_TAG
SOURCE_SHA
assets directory
```

不再从 tag 派生版本。输出清单记录每项资产 SHA-256。

## 8. 第六阶段：测试文件

### 8.1 `tests/releaseWorkflow.test.ts`

更新断言：

- 存在 version-contract job；
- 所有平台使用 `APP_VERSION` output；
- 资产名不含 `v`；
- Windows job包含 helper 构建、静态/生命周期门禁；
- 不存在 `VERSION="${GITHUB_REF_NAME}"` 之类推导。

### 8.2 `tests/macosCrossWorkflow.test.ts`

删除三源版本比较断言，改为：

- 调用 `version:get/check`；
- 仍验证 macOS 工具链常量不被版本脚本修改；
- 验证 Info.plist 内嵌版本检查存在。

### 8.3 新增 WiX 静态测试

可放入：

```text
tests/windowsInstallerTemplate.test.ts
```

至少断言：

- 没有 `Script="vbscript"`；
- 没有 `Win32_LogicalFileSecuritySetting`；
- 有 Binary 和两个 Type 1 actions；
- UI/Execute 属性名一致；
- 卸载条件排除；
- 默认目录和 `ConfigurableDirectory` 保留。

字符串测试不能替代真实 MSI 表验证，但能阻止旧错误回归。

### 8.4 新增 Windows 生命周期 harness

建议：

```text
scripts/windows-installer-test/
  install-lifecycle.ps1
  inspect-msi.ps1
  fixtures/
```

脚本生成 verbose log、安装/升级/卸载、验证目录和注册表，并把日志作为 CI artifact 上传。

## 9. 不应修改的区域

本次禁止顺带修改：

- npm/Cargo 依赖版本；
- Rust/Node/pnpm 工具链版本；
- Tauri identifier、UpgradeCode/Product identity，除非升级测试证明现有值错误；
- 数据库 schema 或配置协议版本；
- 运行时业务逻辑；
- 旧 CHANGELOG/发布说明中的历史版本；
- CC Switch 品牌或安装模型；
- macOS/Linux 安装行为，除版本读取/验证外。

## 10. 推荐提交切片

下面是代码审查切片，不是用户可见版本拆分：

1. `build: establish FyAgent version single source`；
2. `test: add deterministic version command coverage`；
3. `feat(installer): add native install directory policy crate`；
4. `fix(msi): replace VBScript with native directory validation`；
5. `ci: enforce cross-platform version and Windows lifecycle contract`；
6. `docs: record 0.2.1 installer/version governance`。

每个提交保持可审查；最终 tag 仍为 `v0.2.1`。

## 11. 实施顺序与阻断点

```text
版本 workspace 迁移
  ↓（version:check 通过）
helper crate 骨架
  ↓（workspace tests 通过）
原生策略实现
  ↓（Windows Rust tests 通过）
WiX 接入
  ↓（MSI 编译和静态表检查通过）
UI/静默/升级生命周期
  ↓（真实 Windows 通过）
跨平台版本发布契约
  ↓（全部内嵌版本一致）
签名与发布
```

若任一阻断点失败，不得通过禁用校验、允许未知目录、恢复 VBScript、手工改产物名或跳过版本检查来推进。

## 12. 代码审查检查表

- [ ] 当前 FyAgent 应用版本字面量只有 Cargo workspace 一处；
- [ ] `version.mjs` 不做全局数字替换；
- [ ] 主应用和 helper crate均继承版本；
- [ ] WiX 中旧脚本完全删除；
- [ ] UI 和 Execute共享核心校验；
- [ ] 用户策略拒绝不会产生 1720；
- [ ] `/qn` 无法绕过；
- [ ] 卸载不被安全校验阻断；
- [ ] x64/ARM64 DLL与 MSI架构匹配；
- [ ] 所有平台资产版本由 version-contract output驱动；
- [ ] 原生 Windows 生命周期证据已上传；
- [ ] 没有无关依赖、历史文本或工具链版本 diff。
