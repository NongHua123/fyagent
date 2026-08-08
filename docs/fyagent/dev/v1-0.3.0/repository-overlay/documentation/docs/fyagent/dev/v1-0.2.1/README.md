> **Historical / archived design**
>
> This directory records the design state at the time it was created. It is not
> authoritative for the current development environment, canonical mise tasks,
> CI/Release workflow, or local build policy. Use the current README,
> `docs/fyagent/development/mise-tasks.md`, and `.trellis/spec/**` for active
> contracts. The historical body below is intentionally preserved.

# FyAgent 0.2.1：Windows 安装目录选择与全局版本治理

> 目标版本：`0.2.1`  
> 核查日期：2026-08-06  
> 适用源码：用户上传的 `cc-switch-feature-fyagent-v1(1)(2).zip`  
> 文档状态：已确认方案，待实施

## 1. 目标

在同一个 FyAgent `0.2.1` 补丁版本中完成两项工程改造：

1. 修复 Windows MSI 在目录选择页出现前报 Error 1720 的问题，并让用户能够选择安全的安装目录；
2. 把 FyAgent 应用版本从多个重复字段收敛到一个单一真源，通过脚本一键更新，并在所有平台构建与发布时强制校验。

## 2. 最终决策摘要

| 主题 | 决策 |
|---|---|
| 发布版本 | 单个补丁版本 `0.2.1` |
| 版本含义 | FyAgent 全局应用版本，不是 Windows 专用版本 |
| Windows 安装范围 | 保持 `perMachine`、`elevated` |
| 默认目录 | 当前架构的 Program Files 下 `FyAgent` |
| 目录选择 | 保留并定制 `WixUI_InstallDir` |
| 自定义校验 | 删除 VBScript/WMI，新增 Rust `cdylib` Type 1 Custom Action |
| 可选目录边界 | 本机固定磁盘、路径无重解析点、父目录受管理员控制、FyAgent 专用目录 |
| UI 错误 | 简单可理解；技术细节只写 MSI verbose 日志 |
| 静默安装 | Execute Sequence 重验，不能绕过 |
| 版本单一真源 | `src-tauri/Cargo.toml` → `[workspace.package].version` |
| Tauri 版本 | 删除 `tauri.conf.json.version`，由 Cargo 继承 |
| npm 元数据 | 删除 `package.json.version`，设置 `private: true` |
| 本地 Rust 包 | 主程序和安装器动作 crate 使用 `version.workspace = true` |
| Cargo.lock | 生成投影；脚本只更新 FyAgent 本地包条目，不碰依赖版本 |
| 版本命令 | `pnpm run version:set -- X.Y.Z` / `version:bump` / `version:check` |
| 标签 | 精确为 `vX.Y.Z` |
| 正式 MSI | Windows 原生 runner 构建和生命周期测试；交叉构建只作为候选 |

## 3. 文档索引

| 文档 | 用途 |
|---|---|
| [01-REQUIREMENTS.md](./01-REQUIREMENTS.md) | 产品、功能、安全、版本与发布需求基线 |
| [02-CURRENT-STATE-AND-ROOT-CAUSE.md](./02-CURRENT-STATE-AND-ROOT-CAUSE.md) | 当前源码证据、Error 1720 根因和版本散布分析 |
| [03-INSTALLER-UX-AND-DIRECTORY-POLICY.md](./03-INSTALLER-UX-AND-DIRECTORY-POLICY.md) | 用户流程、目录准入规则和错误文案 |
| [04-NATIVE-INSTALL-DIR-VALIDATOR-DESIGN.md](./04-NATIVE-INSTALL-DIR-VALIDATOR-DESIGN.md) | Rust DLL、Win32 路径/ACL 算法、错误模型与测试接口 |
| [05-WIX-MSI-INTEGRATION-DESIGN.md](./05-WIX-MSI-INTEGRATION-DESIGN.md) | WiX UI 事件、Execute Sequence、升级与构建接线 |
| [06-FYAGENT-VERSIONING-REQUIREMENTS.md](./06-FYAGENT-VERSIONING-REQUIREMENTS.md) | FyAgent 全局版本语义、变更规则与范围边界 |
| [07-SINGLE-SOURCE-AND-VERSION-SCRIPT-DESIGN.md](./07-SINGLE-SOURCE-AND-VERSION-SCRIPT-DESIGN.md) | Cargo 单一真源、一键脚本和迁移设计 |
| [08-RELEASE-WORKFLOW-VERSION-CONTRACT.md](./08-RELEASE-WORKFLOW-VERSION-CONTRACT.md) | CI 标签契约、产物命名与跨平台内嵌版本验证 |
| [09-IMPLEMENTATION-CHANGE-MAP.md](./09-IMPLEMENTATION-CHANGE-MAP.md) | 精确文件变更、依赖顺序与提交边界 |
| [10-TEST-AND-ACCEPTANCE-PLAN.md](./10-TEST-AND-ACCEPTANCE-PLAN.md) | 单元、MSI、Windows 生命周期和发布验收矩阵 |
| [11-ADR-RISKS-AND-REFERENCES.md](./11-ADR-RISKS-AND-REFERENCES.md) | ADR、备选方案、剩余风险和官方资料 |
| [12-CODEX-EXECUTION-RUNBOOK.md](./12-CODEX-EXECUTION-RUNBOOK.md) | 可直接交给 Codex 的实施约束与后续版本指令 |

## 4. 推荐实施顺序

```text
锁定 01 的需求与非目标
  ↓
按 07 先建立全局版本单一真源和脚本
  ↓
按 04 建立独立 installer-actions crate 与原生校验核心
  ↓
按 05 删除旧脚本并接入 WiX UI/Execute Sequence
  ↓
按 08 改造跨平台构建与发布版本契约
  ↓
按 10 完成 Windows 原生生命周期和跨平台版本验收
```

先做版本治理的原因是：新增的安装器动作 crate 可从第一天继承同一版本，避免实现过程中再次引入重复字段。

## 5. 版本更新后的标准操作

初次迁移完成后，后续将 FyAgent 更新为任意稳定版本只执行：

```bash
pnpm run version:set -- 0.2.2
pnpm run version:check
git diff -- src-tauri/Cargo.toml src-tauri/Cargo.lock
```

也可以按语义自动递增：

```bash
pnpm run version:bump -- patch
pnpm run version:bump -- minor
pnpm run version:bump -- major
```

脚本不自动修改 CHANGELOG、不提交、不打标签、不推送。这些是发布决策，不应与机械版本更新耦合。

## 6. 完成定义

`0.2.1` 只有同时满足下列条件才能发布：

- Windows 完整 UI 能安装到默认目录和安全的自定义目录；
- 不安全目录显示友好提示并停留在目录页；
- `/qn INSTALLDIR=...` 不能绕过校验；
- MSI 中不存在 FyAgent 自定义 VBScript/JScript/WMI 校验；
- x64 与 ARM64 MSI 均嵌入对应架构的原生动作 DLL；
- 从 `0.2.0` 升级后保留既有安全目录；
- `src-tauri/Cargo.toml` 只有一个 FyAgent 版本字面量；
- `package.json` 与 `tauri.conf.json` 不再保存重复应用版本；
- Windows、macOS、Linux 正式产物的内嵌版本和文件名均为 `0.2.1`；
- 发布标签严格为 `v0.2.1`，所有产物来自同一提交；
- `pnpm run version:check`、参考脚本测试、原生 Windows 生命周期测试全部通过。

## 目录约束

本交付包的全部内容均位于：

```text
docs/fyagent/dev/v1-0.2.1/
```

配套脚本、测试与配置片段位于该目录下的 `reference/`，包元数据与校验清单也位于同一目录。
