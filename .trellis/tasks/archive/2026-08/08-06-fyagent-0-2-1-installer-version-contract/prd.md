# FyAgent 0.2.1 安全安装器与全局版本治理

## 目标

将 FyAgent 应用版本从 `0.2.0` 迁移至 `0.2.1`，以 Cargo workspace 为唯一
人工维护真源；以独立 Rust MSI Custom Action 替换会导致 Error 1720 的
VBScript/WMI 安装目录校验。保留当前机器级、提权、受保护目录和正式 Windows
运行时安全边界，并让所有构建/发布入口消费同一版本契约。

## 已确认事实

- 参考包 `docs/fyagent/dev/v1-0.2.1/` 是经批准的需求与验收规格，当前为
  未跟踪的原始输入；实施后必须原样纳入 Git，且 `MANIFEST.sha256` 必须保持有效。
- 当前版本仍同时写入 `package.json`、`src-tauri/Cargo.toml` 与
  `src-tauri/tauri.conf.json`；Windows/MSI 模板仍含内联 VBScript/WMI validator。
- 现有 `perMachine + elevated`、Program Files 默认目录、`WixUI_InstallDir`、
  HKLM `InstallDir`、最终 ACL、legacy per-user 阻断、正式 manifest 与签名边界
  是需要保留的既有行为。
- 本任务不创建 tag、不 push、不触发 GitHub Actions、不签名/公证、也不发布
  GitHub Release。

## 需求

### VER — 全局版本契约

- VER-001：`src-tauri/Cargo.toml [workspace.package].version = "0.2.1"` 是唯一
  人工维护的 FyAgent 应用版本；主 crate 与 installer-actions crate 均使用
  `version.workspace = true`。
- VER-002：`package.json` 不再含应用版本、必须为 `private: true`；
  `tauri.conf.json` 不再含版本字段，Tauri 从 Cargo package 继承。
- VER-003：提供 `version:get`、`version:check`、`version:set`、`version:bump`。
  脚本只改 canonical Cargo 值和本地 package lock 投影，验证 workspace、脚本、
  duplicate fields、tag、SemVer/MSI limits，并在写入失败时回滚。
- VER-004：Windows cross build、macOS metadata/preflight、release workflow、
  资产命名和下载 manifest 不再读取 package/Tauri 版本字段。

### INS / NAT — 安全目录与原生 Custom Action

- INS-001：保持机器级提权安装、Program Files 默认目录和标准目录选择页；用户仅可
  选择通过安全策略的本机固定磁盘 FyAgent 专用目录。
- INS-002：UI 和 Execute Sequence 必须通过同一 Rust policy 核心验证路径；UI
  策略拒绝留在目录页且不产生 Error 1720，Execute 必须阻止 `/qn`、企业部署和
  UI 后路径/ACL 变化的绕过。
- NAT-001：删除目录校验使用的 VBScript、WMI、`Err.Raise` 和脚本调度；新增独立
  Windows `cdylib`，导出 `ValidateFyAgentInstallDirUi` 与
  `ValidateFyAgentInstallDirExecute`。
- NAT-002：策略 fail-closed、只读且不依赖目标机脚本或外部可执行文件；检查路径、
  本地固定卷、重解析点、可信祖先、所有者/DACL/effective access、非空目录和
  既有 FyAgent 标识。
- INS-003：升级/修复恢复并重验受保护 HKLM `InstallDir`，不接受外部目录搬迁；
  卸载不运行目录准入校验。

### REL — 构建与发布版本契约

- REL-001：每个 Windows 架构在 bundling 前构建匹配的 helper DLL，通过
  `FYAGENT_INSTALLER_ACTIONS_DLL` 注入，验证 PE/MSI 架构与 Binary/CustomAction/
  sequence 表。
- REL-002：release workflow 增加 `version-contract`，输出 `app_version`、
  `release_tag`、`source_sha`；所有正式平台 job 使用这些 immutable outputs。
- REL-003：tag 只接受 `v${APP_VERSION}`；资产名使用不带 `v` 的应用版本；下载
  manifest 记录版本、tag、source SHA 和资产信息，同时保持现有调用方兼容。
- REL-004：不恢复 push/PR 自动 CI；保留当前签名、公证、branch unsigned macOS
  artifact 与 Windows release-manifest 选择边界。

## 验收标准

- [ ] `version:check`、版本 Node 测试、相关 Vitest/Python/Bash 测试及项目质量
  检查通过；Cargo.lock 只包含本地 workspace package 与必要 metadata 的预期变化。
- [ ] 当前应用版本只在 Cargo workspace 真源人工维护；所有下游读取/命名入口已
  迁移，且错误 tag/重复版本/缺失 workspace contract 都会失败。
- [ ] WiX 中不再含 FyAgent 目录校验的 VBScript/JScript/WMI；存在 Binary、两个
  Type 1 actions、UI 可恢复错误路径、Execute Type 19 终止路径和安全属性传播。
- [ ] helper crate 在非 Windows workspace 检查中安全编译，在 Windows target 上
  生成匹配 DLL；cross-build 结构检查覆盖新 MSI 表。
- [ ] 参考包以原始内容入库，`(cd docs/fyagent/dev/v1-0.2.1 && sha256sum -c
  MANIFEST.sha256)` 全部通过。
- [ ] 最终报告将 Windows x64/ARM64 原生 UI、silent、upgrade、repair、uninstall、
  ICE、签名和原生 macOS 验证标为发布阻断门禁，除非已有真实环境证据。

## 非目标与约束

- 不改应用运行时业务、协议/schema、依赖/工具链版本、主 crate 类型、UpgradeCode、
  legacy per-user 阻断、签名/公证策略或 CI 触发类型。
- 不将交叉构建或静态检查表述为真实 Windows 生命周期验收。
- `version:set` 不自动 commit、tag、push 或创建 release。
