# FyAgent 0.2.1 安装器与全局版本治理文档包

本交付包面向 FyAgent 源码基线 `cc-switch-feature-fyagent-v1(1)(2).zip`，包含 Windows MSI 安装目录选择修复、原生目录安全校验、跨平台应用版本单一真源、版本一键更新脚本、CI 发布契约、实施变更地图与验收计划。

## 已确定方案

- 本次作为 **一个 FyAgent 补丁版本 `0.2.1`** 交付，不拆分临时版和正式版。
- `0.2.1` 是 FyAgent 的**全局应用版本**，同时约束 Windows、macOS、Linux、运行时版本显示、发布标签与下载清单；不是 Windows 安装器的独立子版本。
- Windows 保持 `perMachine + elevated`，默认安装到 Program Files。
- 保留 WiX 标准安装目录选择页，用户可以选择其他符合安全策略的本机固定磁盘目录。
- 删除当前内联 VBScript/WMI 校验，改用 x64/ARM64 原生 Rust DLL Custom Action。
- 目录校验在 UI“下一步”和 Execute Sequence 中各执行一次，并共享同一核心策略。
- FyAgent 应用版本的单一真源设为 `src-tauri/Cargo.toml` 的 `[workspace.package].version`。
- `src-tauri/tauri.conf.json` 不再保存重复版本；Tauri 从 Cargo 包版本继承。
- `package.json` 不再保存 FyAgent 应用版本，并明确设置 `private: true`。
- 后续更新版本仅执行 `pnpm run version:set -- X.Y.Z`，禁止全仓库搜索替换。

## 目录

- `docs/fyagent/dev/v1-0.2.1/`：需求、设计、实施与验收文档。
- `reference/scripts/version.mjs`：一键版本更新参考实现。
- `reference/tests/version.test.mjs`：脚本参考测试。
- `reference/snippets/`：Cargo、Tauri、package.json、WiX 与 CI 接入片段。
- `PACKAGE-METADATA.md`：输入基线、哈希和交付范围。
- `MANIFEST.sha256`：交付包内文件校验清单。

## 使用方式

先阅读 `docs/fyagent/dev/v1-0.2.1/README.md`。实施人员按 `09-IMPLEMENTATION-CHANGE-MAP.md` 执行；Codex 可直接使用 `12-CODEX-EXECUTION-RUNBOOK.md` 的约束与步骤。

参考脚本在当前文档包中用于审阅和测试。正式落库时应复制为：

```text
scripts/version.mjs
tests/version.test.mjs
```

随后将 `reference/snippets/package.json.versioning.json` 中的脚本项合并到项目 `package.json`。

## 交付边界

本包是方案需求与设计交付，不直接修改用户上传的源码，也不声称原生目录校验 DLL 已在真实 Windows 环境编译通过。参考版本脚本已通过随包单元测试；Windows MSI 生命周期与 ACL 安全测试仍属于实施后的发布门禁。
