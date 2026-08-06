# 12 — Codex 执行 Runbook

## 1. 用途

本文可直接作为自动化开发代理的实施约束。它分为：

- **一次性 `0.2.1` 改造**：安装器修复 + 全局版本治理；
- **以后更新 FyAgent 版本**：只运行版本脚本，不搜索替换。

## 2. 全局不可违反约束

1. 目标版本是 **FyAgent 全局应用版本 `0.2.1`**，覆盖 Windows、macOS、Linux；不是 Windows 专用版本。
2. 本次只发布一个小版本，不创建“临时无校验版”。
3. Windows 保持 `perMachine + elevated`，默认 Program Files。
4. 用户可以选择其他安全的本机固定磁盘专用目录。
5. 不允许安装到用户 profile、Desktop、Downloads、AppData、Temp、网络盘、移动盘或重解析路径。
6. 删除 VBScript/WMI，不通过修复旧脚本交付。
7. UI 和 Execute Sequence 调用同一个 Rust 校验核心。
8. 不改变主应用为 per-user，不拆 privileged broker。
9. `src-tauri/Cargo.toml [workspace.package].version` 是唯一 FyAgent 版本真源。
10. 不修改依赖版本、工具链版本、schema/config/protocol 版本或历史文本。
11. 不通过全仓库搜索替换更新版本。
12. 不自动提交、打标签、推送或发布。
13. 不跳过真实 Windows MSI 生命周期测试。

## 3. 一次性 `0.2.1` 实施任务

### 3.1 开始前

```bash
git status --short
git rev-parse HEAD
pnpm install --frozen-lockfile
pnpm run test:unit
cargo test --manifest-path src-tauri/Cargo.toml
```

若工作树已有用户改动：

- 不覆盖、不 reset；
- 识别与本任务的重叠文件；
- 在最终报告中区分用户原改动和本次改动。

记录当前源码关键事实：

```bash
rg -n '"version"|version = "0\.2\.0"|ValidateInstallDirectory|Script="vbscript"|GetSecurityDescriptor|GITHUB_REF_NAME' \
  package.json src-tauri scripts .github tests
```

这一步只用于迁移盘点；迁移完成后的版本更新不得再使用搜索替换。

### 3.2 建立版本单一真源

1. 修改 `src-tauri/Cargo.toml`：
   - 添加 `[workspace]`；
   - members 为 `.` 和 `installer-actions`；
   - 添加 `[workspace.package] version = "0.2.1"`；
   - 主 package 改为 `version.workspace = true`；
   - 不改依赖版本和现有 crate type。
2. 创建 `src-tauri/installer-actions/Cargo.toml`，使用 `version.workspace=true`、`publish=false`、`cdylib`。
3. 删除 `src-tauri/tauri.conf.json.version`。
4. 删除 `package.json.version`，增加 `private: true` 和四个版本命令。
5. 从文档包复制 `reference/scripts/version.mjs` 到 `scripts/version.mjs`。
6. 从文档包复制/扩展 `reference/tests/version.test.mjs` 到 `tests/version.test.mjs`。
7. 让 Cargo 更新 workspace lockfile；只接受本地包/必要 workspace metadata diff。
8. 删除或重写 `tests/versionConsistency.test.ts`，不得硬编码 `0.2.1`。
9. 运行：

```bash
node --test tests/version.test.mjs
pnpm run version:check
pnpm --silent run version:get
```

最后一条必须只输出：

```text
0.2.1
```

### 3.3 实现原生目录策略 crate

按 `04-NATIVE-INSTALL-DIR-VALIDATOR-DESIGN.md` 实现：

```text
src-tauri/installer-actions/src/lib.rs
src-tauri/installer-actions/src/msi.rs
src-tauri/installer-actions/src/policy.rs
src-tauri/installer-actions/src/windows_path.rs
src-tauri/installer-actions/src/security.rs
src-tauri/installer-actions/src/messages.rs
```

最低要求：

- `extern "system"` 导出：
  - `ValidateFyAgentInstallDirUi`
  - `ValidateFyAgentInstallDirExecute`
- `catch_unwind` 包裹 FFI；
- 读取 `INSTALLDIR`，每次清空旧结果属性；
- UI 策略拒绝设置属性并返回 success；
- Execute 使用相同核心重验；
- 使用 handle-based 最终路径、reparse 和安全描述符检查；
- 检查本地固定磁盘、ACL、所有者、父目录删除/抢先创建风险；
- 无法证明安全则拒绝；
- 校验只读；
- 用户消息来自固定映射；
- 详细错误只写 MSI 日志。

不要：

- 创建“测试文件”判断可写性；
- 修改 ACL；
- 使用 PowerShell/WMI/VBScript；
- 只按路径字符串前缀判断安全；
- 只扫描 allow ACE 而不计算有效权限；
- 让 panic 跨 FFI。

### 3.4 替换 WiX

在 `src-tauri/wix/per-machine-main.wxs`：

1. 删除完整旧 `ValidateInstallDirectory` Script CustomAction；
2. 删除顶部 UI/Execute 旧调度；
3. 加入 helper DLL Binary；
4. 定义两个 Type 1 CustomAction；
5. 定义 Execute 拒绝用 Type 19 action；
6. 设置/保护所需 Public Properties；
7. 保留：
   - `WIXUI_INSTALLDIR=INSTALLDIR`；
   - Program Files 默认目录；
   - `ConfigurableDirectory="INSTALLDIR"`；
   - HKLM InstallDir；
   - 最终目录 PermissionEx；
8. 定制 InstallDirDlg Next 事件：
   - SetTargetPath；
   - WixUIValidatePath；
   - 标准 InvalidDirDlg；
   - UI native validator；
   - FyAgent 简单错误 dialog；
   - VerifyReadyDlg；
9. Execute validator 在 `CostFinalize` 后、`InstallValidate` 前；
10. 排除完整卸载；
11. 升级/修复恢复并重验旧 HKLM 目录，不提供搬迁。

优先创建：

```text
src-tauri/wix/fyagent-install-dir-ui.wxs
```

并通过 `fragmentPaths` 接入。若实际 Tauri/WiX 合并不兼容，把同一逻辑内嵌到主模板；不要恢复脚本。

### 3.5 构建接线

修改 Windows 构建流程：

```bash
cargo build -p fyagent-installer-actions \
  --manifest-path src-tauri/Cargo.toml \
  --target <x86_64-or-aarch64-pc-windows-msvc> \
  --release
```

在 Tauri bundle 前设置：

```text
FYAGENT_INSTALLER_ACTIONS_DLL=<absolute target dll path>
```

验证：

- PE Machine 与目标一致；
- WiX 能找到 DLL；
- MSI Binary/CustomAction 表正确；
- x64 和 ARM64 分别构建，不复用 DLL。

### 3.6 改造发布版本契约

在 `.github/workflows/release.yml`：

1. 增加 `version-contract` job；
2. 精确校验 tag；
3. 输出 `app_version/release_tag/source_sha`；
4. 所有平台 job `needs` 该 job；
5. 资产命名使用 `APP_VERSION`，不含 `v`；
6. 删除各处 `VERSION=${GITHUB_REF_NAME}` 和清洗推导；
7. Windows 正式 job增加原生生命周期门禁；
8. 生成 manifest 时传入明确版本、tag、SHA。

修改：

```text
scripts/windows-cross/build-windows-msi.sh
scripts/macos-cross/project_metadata.py
scripts/macos-cross/preflight.py
scripts/macos-cross/build-package.sh（以及实际读取版本的其他文件）
scripts/generate-download-manifest.mjs
.github/workflows/ci.yml
tests/releaseWorkflow.test.ts
tests/macosCrossWorkflow.test.ts
```

所有构建脚本只允许通过：

```bash
pnpm --silent run version:get
```

或 CI output 获取 `APP_VERSION`。

### 3.7 测试

至少运行：

```bash
pnpm run version:check
node --test tests/version.test.mjs
pnpm run typecheck
pnpm run format:check
pnpm run test:unit
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo clippy --workspace --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

Windows 原生环境：

```cmd
msiexec /i FyAgent-0.2.1-Windows.msi /L*V install.log
```

完成 `10-TEST-AND-ACCEPTANCE-PLAN.md` 中的默认目录、安全 D 盘、不安全目录、`/qn`、竞态、升级、修复、卸载和架构矩阵。

### 3.8 最终静态检查

```bash
rg -n 'Script="vbscript"|Win32_LogicalFileSecuritySetting|Scripting\.FileSystemObject|GetSecurityDescriptor\(\)' \
  src-tauri/wix tests scripts .github
```

预期：无产品实现命中；允许设计文档或明确的负向测试字符串。

```bash
rg -n '"version"\s*:\s*"0\.2\.1"|version\s*=\s*"0\.2\.1"' \
  package.json src-tauri scripts .github tests
```

预期：当前 FyAgent 应用版本字面量只在：

```text
src-tauri/Cargo.toml [workspace.package]
```

测试 fixture 中可有示例，但不得成为实际项目状态源。

## 4. Codex 最终报告格式

实施完成时按以下结构报告：

```markdown
## 结果
- FyAgent 全局版本：0.2.1
- 单一真源：src-tauri/Cargo.toml [workspace.package]
- Windows 安装器：可选择安全目录，旧 VBScript 已删除

## 主要变更
- <文件/模块和行为>

## 版本脚本
- version:get: ...
- version:check: ...
- version:set dry-run: ...

## 测试
- <命令>: PASS/FAIL
- Windows x64 lifecycle: PASS/未执行（原因）
- Windows ARM64 lifecycle: PASS/未执行（原因）

## 产物验证
- MSI ProductVersion: ...
- macOS bundle version: ...
- Linux package version: ...
- Tag/asset/manifest: ...

## 未完成或风险
- 仅列真实未完成项，不隐藏失败
```

不能把“源码编译”描述为“Windows 安装生命周期已通过”。

## 5. 以后执行“更新 FyAgent 版本号为 X.Y.Z”

### 5.1 唯一标准流程

收到指令：

> 更新 FyAgent 版本号为 0.2.2

直接执行：

```bash
git status --short
pnpm run version:check
pnpm run version:set -- 0.2.2
pnpm run version:check
git diff -- src-tauri/Cargo.toml src-tauri/Cargo.lock
```

然后运行与版本文件相关的测试：

```bash
node --test tests/version.test.mjs
```

如果本次同时准备发布，再运行完整 CI；如果只是版本变更，不自动打标签。

### 5.2 禁止动作

后续更新版本时不要：

- 全仓库搜索 `0.2.1` 并替换；
- 编辑 `package.json.version`；
- 编辑 `tauri.conf.json.version`；
- 手工改 release workflow 中的版本；
- 修改依赖版本；
- 修改旧 CHANGELOG/发布文档；
- 修改数据库/schema/protocol 版本；
- 创建 Git tag；
- 自动提交或推送。

### 5.3 脚本失败时

若 `version:check` 或 `version:set` 失败：

1. 不绕过脚本；
2. 不手工改多个文件；
3. 读取错误，确认是否有人破坏单一真源结构；
4. 只修复契约结构；
5. 重新运行 `version:check`；
6. 再执行 `version:set`；
7. 报告导致契约漂移的文件。

### 5.4 Dry run

在不确定目标版本或工作树有复杂改动时：

```bash
pnpm run version:set -- 0.2.2 --dry-run
```

预期只报告：

```text
src-tauri/Cargo.toml
src-tauri/Cargo.lock
```

出现其他文件即视为脚本设计回归。

### 5.5 自动递增

只有用户明确说“patch/minor/major 递增”时使用：

```bash
pnpm run version:bump -- patch
```

用户给了确切版本时始终使用 `version:set`，不要自行推断或改写目标。

## 6. 以后发布标签

版本更新提交合并并通过全部门禁后，发布负责人单独执行：

```bash
APP_VERSION="$(pnpm --silent run version:get)"
pnpm run version:check -- --tag "v${APP_VERSION}"
git tag -s "v${APP_VERSION}" -m "FyAgent ${APP_VERSION}"
```

是否推送由明确发布指令决定。Codex 未获授权不得执行 tag/push/release。

## 7. 完成判定

一次性 `0.2.1` 改造只有在以下全部满足时完成：

- 版本单一真源和脚本落库；
- 安装器动作 crate继承全局版本；
- 旧 VBScript/WMI完全删除；
- 用户可选择安全目录；
- UI/静默/升级校验一致；
- x64/ARM64原生 DLL匹配；
- Windows 生命周期通过；
- Windows/macOS/Linux内嵌版本为 `0.2.1`；
- release tag/资产/manifest契约通过；
- 没有无关依赖或历史文本修改。
