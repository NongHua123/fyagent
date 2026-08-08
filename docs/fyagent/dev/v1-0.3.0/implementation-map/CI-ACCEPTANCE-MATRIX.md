# GitHub Actions CI 验收矩阵

> **交付状态**：Implemented and locally verified; remote runs pending, merge_group live run N/A by accepted exception / 已实施并完成本地验证，远程运行待验，merge_group live run 按已接受例外为 N/A
> **关联决策**：13–19、24、49、70、81–87、105–115
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

[Observed / 已核实] commit `038675b3` 已冻结六个 Required dependency job 与一个稳定 aggregate；下表描述真实 workflow，不再是建议。修改 job ID 必须同步 pure-Node evaluator 与合同测试。

## 1. 触发与聚合

```yaml
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  merge_group:
    types: [checks_requested]
  workflow_dispatch:

permissions:
  contents: read
```

稳定 context：`CI / Required`。它使用 `if: always()`，读取全部 required `needs.*.result`，只接受合同允许的 `success`；`failure`、`cancelled` 与非预期 `skipped` 必须失败。整个 required workflow 不使用会让 check 永久 Pending 的顶层 path filter。

## 2. 已实施 job 矩阵

| Job / 稳定责任                       | Runner         | 安装                         | 主要检查                                                                                                                                                       | Secrets/权限                 |               Required                |
| ------------------------------------ | -------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | :-----------------------------------: |
| `contracts`                          | `ubuntu-24.04` | Node/pnpm；按检查需要 Rust   | 标准版本文件唯一性；mise/uv/lock 结构；task metadata/DAG；workflow runner/action/permission 策略；活动文档漂移；cross-build/告警抑制负向扫描；Release 静态合同 | `contents: read`；无 secrets |                   ✓                   |
| `frontend`                           | `ubuntu-24.04` | Node 24.19.0、pnpm 10.12.3   | frozen install；typecheck；format check；unit；i18n；普通 Node `--throw-deprecation`                                                                           | 同上                         |                   ✓                   |
| `desktop-acceptance-contract`        | `ubuntu-24.04` | Node/pnpm                    | desktop mock；visual preflight；Native Fetch→MSW probe；聚焦 `--pending-deprecation --throw-deprecation`                                                       | 同上                         |                   ✓                   |
| `backend-linux`                      | `ubuntu-24.04` | Rust 1.97.1 + rustfmt/clippy | fmt check；workspace/all-target check；clippy `-D warnings`；locked tests                                                                                      | 同上                         |                   ✓                   |
| `backend-windows`                    | `windows-2022` | 同一标准文件工具链           | 运行时版本；`FYAGENT_WINDOWS_MANIFEST=test`；check/clippy/test；平台路径/编码合同                                                                              | 同上                         |                   ✓                   |
| `backend-macos`                      | `macos-15`     | 同一标准文件工具链           | 运行时版本；check/clippy/test；macOS 条件编译合同                                                                                                              | 同上                         |                   ✓                   |
| `required`（显示名 `CI / Required`） | `ubuntu-24.04` | 无业务工具要求               | 枚举精确六个 dependency 结果；输出机器可读总结；只接受全 `success`                                                                                             | `contents: read`             | 外部稳定 context；当前无 ruleset 强制 |

### 平台/耗时调整规则

- 前端纯逻辑测试可以只在 Linux 运行；Node/pnpm **实际版本验证**仍在所有相关平台 job 执行。
- Rust fmt 可以只在 Linux执行一次；Rust check/test 必须覆盖 Linux、Windows、macOS 条件编译面。Clippy 是否三平台执行可按真实兼容性/耗时调整，但至少 Linux 阻断，且 Windows/macOS 的编译警告不得被静默忽略。
- Windows ARM64 的本地开发合同不强制在每个 PR 使用昂贵 runner；至少由定期/手动 smoke 或 Release preflight 验证，并在发布前成为阻断证据。
- CI 不安装 mise。`mise run tasks:validate` 属于本地 canonical task；CI 运行同一底层 Node 合同解析器并校验提交的 task 元数据/生成文档。不得因此在 YAML 重复版本事实。

## 3. 运行时版本证据

每个相关平台记录并严格比较：

```text
node --version  == v24.19.0
pnpm --version  == 10.12.3
rustc --version == 1.97.1
rustfmt/clippy  == rust-toolchain.toml 对应组件
```

值从 `.node-version`、`package.json#packageManager`、`rust-toolchain.toml` 解析；检查脚本不得再硬编码一套常量。

## 4. Required gate 组合测试

至少为聚合逻辑增加以下单元/合同案例：

| 依赖结果                                          | 预期                                                            |
| ------------------------------------------------- | --------------------------------------------------------------- |
| 全部 `success`                                    | success                                                         |
| 任一 `failure`                                    | failure                                                         |
| 任一 `cancelled`                                  | failure                                                         |
| 任一非合同允许的 `skipped`                        | failure                                                         |
| workflow/job 名重构但 exact dependency set 未同步 | contract failure                                                |
| merge queue `merge_group` 事件                    | 静态 trigger 已覆盖；D114 接受当前治理下 live run=N/A，不是成功 |

## 5. 非 CI 证据

- 当前宿主 `mise run check` 不是多平台 CI 的替代品；
- Tauri 安装包、Windows/macOS 无签名负向状态、Linux glibc 基线和精确 10 资产属于 Release preflight/formal workflow；签名与公证不是 v0.3.0 门禁；
- 根据决策 49，不建立上游 merge 专用产品验收 workflow，但合并 PR 仍执行上述 Required CI 和冲突/许可审查。
- 仓库不配置 ruleset/branch protection；`CI / Required` 是 workflow 输出，不是管理员强制的 merge policy。该残余风险已接受，不能宣称受保护。
- 本地 21 个 CI/Labeler 合同测试、CI-safe 42 个 contract/898 个 app tests、全量 137 files/936 tests 和 actionlint 已通过；真实 PR/main/manual/Labeler run 仍 Pending。
- The project owner accepted D113/D114 on 2026-08-08。D114 确认 `merge_group` 因个人账户仓库且禁止保护规则而无法产生真实运行，在当前治理下记为 N/A 而非成功；接受的替代证据是 YAML trigger、失败关闭合同/静态测试及真实 PR/main/manual 运行，不以 manual dispatch 冒充 `merge_group`。
