# FyAgent 0.2.1 技术设计

## 架构边界

`src-tauri/Cargo.toml [workspace.package].version` 是版本真源。`scripts/version.mjs`
是唯一写入入口；Node、Python、Bash 和 workflow 通过 `pnpm --silent run version:get`
或 `version-contract` outputs 消费，不再解析 package/Tauri 版本字段。

`src-tauri/installer-actions` 是与主 Tauri crate 隔离的 Windows `cdylib`。它使用
`windows-sys 0.61` 的最小 MSI/security/filesystem feature 集；非 Windows 使用
`cfg`-gated no-op/stub 边界，使 `cargo --workspace` 可检查，而 Windows 才编译 MSI
FFI 与 ACL 实现。

## 安装器数据流

```text
INSTALLDIR (untrusted MSI property)
  -> WiX standard SetTargetPath / WixUIValidatePath
  -> ValidateFyAgentInstallDirUi
  -> stable result properties / recoverable dialog
  -> VerifyReadyDlg

INSTALLDIR (including /qn or deployment input)
  -> ValidateFyAgentInstallDirExecute
  -> same policy core
  -> AbortUnsafeFyAgentInstallDir (Type 19) when invalid
  -> InstallValidate / file copy
```

两个 FFI 入口都清空旧结果属性、`catch_unwind`、写入稳定错误码与非敏感用户文案。
UI 的策略拒绝返回 MSI success 以避免 1720；Execute 的 Type 19 action 负责显式
中止。内部错误和无法证明安全的情况一律 fail-closed。validator 不创建目录、测试
文件或 ACL，Installer 的现有事务继续创建并锁定目标目录。

升级/修复仅从 HKLM `InstallDir` 恢复路径，要求路径与既有 FYAgent 产品 marker
一致后才允许非空目录；不匹配显示重新安装指引。卸载条件排除 directory policy。

## 构建与发布数据流

每个 Windows build 先为当前 target 编译 helper DLL，检查 PE Machine，导出
`FYAGENT_INSTALLER_ACTIONS_DLL` 给 WiX bundling。结构检查增加 Binary、CustomAction、
InstallUISequence、ControlEvent 等 MSI 表，证明 DLL/序列实际进入候选 MSI。

release workflow 的 `version-contract` 验证 tag 类型与 `v${APP_VERSION}`，导出
`app_version`、`release_tag`、`source_sha`。平台 build 使用 outputs 命名资产并将
显式数据传给 manifest；保持 branch unsigned macOS 和 tag signing/notarization
分支不变。

## 兼容、回滚与安全取舍

- 保持主 crate 的 `staticlib` / `rlib`，不将 MSI DLL 变成 Tauri bundle resource。
- 保持 current `perMachine/elevated`、UpgradeCode/MajorUpgrade、legacy per-user 检测、
  HKLM registry 和最终 `PermissionEx`。
- 若 WiX fragment 合并在实际 v3 bundle 验证中不支持，采用同一逻辑的主模板内嵌；
  不恢复 VBScript/WMI 或引入 bootstrapper。
- 参考片段中的 `REMOVE~="ALL"` 卸载豁免适配为 `CostFinalize` 后的完整
  `INSTALLDIR` component action-state 集合。Linux candidate 和 Windows release
  gate 从实际 `Directory` / `Component` 表重算该集合；模板新增根目录组件会先使
  gate 失败，避免 mixed remove/add 被误判为纯卸载。
- `version:set` 预检失败不写入；写入中失败恢复所有已写文本。单次迁移出现问题可
  回滚 canonical Cargo、lockfile、脚本/readers 与 WiX/action crate 的同一提交。
- 下载 manifest 的已有入口/消费者必须先由测试和调用点确认；在不破坏旧必填字段
  的前提下增加 provenance 字段。
