# 当前状态与根因分析

> **交付状态**：Observed historical baseline + implemented current state / 历史基线与当前实施状态均已核实
> **关联决策**：1–34、39–42、81–96、105–118
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 2026-08-07 输入快照（历史）

[Observed / 已核实的原始输入]

- 上传包根目录含约 1,509 个文件，不含 `.git`；
- `.node-version` 为 `22.12.0`；
- `rust-toolchain.toml` 为 `1.95`；
- `mise.toml` 重复声明 Node `22.12.0`、pnpm `10.12.3`、Python `3.12.8`、Rust `1.95.0`；
- `mise.toml` 仅有 5 个 task，全部服务本地交叉构建；
- CI 仅监听 `workflow_dispatch`；
- CI/Release 使用 Node 20 与 Rust `stable`；
- `package.json` 直接依赖 `cross-fetch ^4.1.0`；
- `tests/msw/tauriMocks.ts` 第 1 行导入 `cross-fetch/polyfill`。

## 2. 本地交叉构建传播面

本节记录退役前的根因。commit `e8954d97faed1b833a6bce6fb9477b4fc4e2fd83` 已删除这些活动入口；原生 Windows Release 验证器接管 MSI 安全断言。

| 传播面    | 退役前证据                                                     | 根因                                                          |
| --------- | -------------------------------------------------------------- | ------------------------------------------------------------- |
| 脚本      | `scripts/macos-cross/**`、`scripts/windows-cross/**`           | Linux/WSL 跨 OS 构建承担 SDK、Wine、WiX、osxcross、签名边界。 |
| mise      | 5 个 task；Rust 带 `llvm-tools` 与 4 个非宿主 target           | 发布专用工具被混入所有开发者的基础环境。                      |
| 安全      | 两个 macOS 脚本调用 `mise trust --yes`                         | 脚本替用户作出仓库信任决定。                                  |
| 测试      | `tests/macosCrossWorkflow.test.ts`                             | 主要检查文本包含，难以证明真实工具归属或 lock 结构。          |
| 文档/spec | 四份 README、development-environment、wsl-macos-cross-build 等 | 旧实现已成为活动规范，删除脚本不足以完成退役。                |

## 3. 工具链漂移

| 层         |    Node |   Rust |                              pnpm |
| ---------- | ------: | -----: | --------------------------------: |
| 本地       | 22.12.0 | 1.95.0 |                           10.12.3 |
| CI/Release |      20 | stable | 10.12.3（重复声明/部分 Corepack） |
| 目标       | 24.19.0 | 1.97.1 |                           10.12.3 |

根因不是“mise 与 Actions 安装方式不同”，而是**版本事实源重复且滚动值进入发布路径**。相同提交在不同时点可能解析到不同 Rust stable 或不同 runner OS。

[Observed / 已核实] commit `3d534710307d538e570c137231b1d80a64ac8ab7` 已将事实源收敛为 Node `24.19.0`、pnpm `10.12.3`、Rust `1.97.1`、Python `3.14.7` 和 lock 解析的 uv `0.12.2`；Linux x64 宿主验证以及 Windows x64/ARM64、Linux x64/ARM64、macOS Universal 发布原生 runner 证据已完成。Child 3 原验收字面要求的 Windows ARM64 locked uv-managed Python/Trellis smoke 也已由 [PR #8 run `31265504901`](https://github.com/NongHua123/fyagent/actions/runs/31265504901) 远程验证：x64 job `93122857985`、ARM64 job `93122858012` 与 Required job `93123992476` 均成功。

## 4. mise.lock 结构问题

[Observed / 已核实]

- `mise.lock` 有两个 Rust `1.95.0` 条目，options/targets 不同；
- pnpm 的 `windows-arm64` 条目实际指向 `pnpm-win-x64.exe`；
- Python 3.12.8 缺少 Windows ARM64 条目；
- 当前测试以字符串查找平台名，不能证明逐工具平台资产正确。

根因是 lockfile 被当作文本清单，而不是按工具、版本、options、平台、URL、checksum 解析的结构化派生文件。

[Observed / 已核实] 当前 `mise.lock` 已从空文件状态生成并对六个平台二次生成字节稳定，SHA-256 为 `5f0d9df527ec1fdaf5532726ba30d330c74872786ad0380783064a36ceeefd9d`。Windows ARM64 pnpm/uv 使用原生 ARM64 资产；Rust backend 不提供平台资产记录，因此仍由各原生 runner 证明 rustup toolchain。

## 5. CI 与 Release 缺口

[Observed / 已核实的原始输入]

- `ci.yml` 不随 PR/main 自动执行；
- 使用 `ubuntu-latest`、`windows-latest`、`macos-latest`；
- Actions 使用可移动标签而非完整 SHA；
- Release 手动预演只覆盖 macOS；
- Release 缺少 source SHA 已进入 main、Required CI 已成功、精确 10 资产、摘要和 provenance 的统一事务；
- `permissions: contents: write` 在 workflow 顶层过宽。

根因是发布矩阵已形成，但治理仍是“各 job 构建成功即发布”的流水线，而不是明确的来源资格、资产集合和最小权限事务。

[Observed / 已核实] commits `038675b3` 与 `94ff9ee9` 已实现自动 CI、安全 Labeler、稳定 `CI / Required`、五个原生目标、无签名 10 安装器/13 附件、强制 attestation 和私有 draft 到 stable 的失败关闭发布事务。D118 工程化修订通过 [PR #7](https://github.com/NongHua123/fyagent/pull/7) 合入 exact main commit `bde1370bbaffd345c3d9875708615eaf96140591`；[PR CI](https://github.com/NongHua123/fyagent/actions/runs/31258884239) 和 [main CI](https://github.com/NongHua123/fyagent/actions/runs/31259389682) 成功，包括 Windows x64/ARM64 MSI query native fixtures。[preflight](https://github.com/NongHua123/fyagent/actions/runs/31259905022) 和 [formal run](https://github.com/NongHua123/fyagent/actions/runs/31260931509) 的五原生目标、verify、attest 均成功，后者已发布 [stable/latest v0.3.0](https://github.com/NongHua123/fyagent/releases/tag/v0.3.0)。`merge_group` 依旧因个人账户与无保护治理而无法产生真实事件；D114 保持 live run=N/A，其 YAML/失败关闭静态合同及真实 PR/main/manual 替代证据已完整。

## 6. DEP0040 根因

2026-08-07 输入依赖链：

```text
cross-fetch@4.1.0
└─ node-fetch@2.7.0
   └─ whatwg-url@5.0.0
      └─ tr46@0.0.3
         └─ Node 内置 punycode
```

`cross-fetch/polyfill` 即使发现 `global.fetch` 已存在，也会在模块加载阶段引入 Node ponyfill 依赖图。Node 24 将部分弃用降为 application deprecation，默认可能不再打印 `node_modules` 中的警告，因此“输出干净”不能证明根因消失。

[Observed / 已核实] commit `4e407df4` 已删除直接依赖/import并重生成 lock；Node 24 原生 Fetch→MSW→Tauri mock 的成功、非 2xx 文本错误、204 空响应和跨 realm 行为均通过，依赖图与 suppression 扫描证明旧链路退出。Child 5 已完成并归档。

## 7. 文档与 Trellis 漂移

[Observed / 已核实的原始输入]

- 仓库总计存在大量 `mise exec --` 文本；活动 README、CONTRIBUTING、PR 模板、visual-baseline、specs 明确使用旧入口；
- `.trellis/workflow.md` 和项目操作型 skills 直接使用 `python3`；
- `.codex/hooks.json` 直接调用系统 `python3`；
- 活动 `wsl-macos-cross-build.md` 与目标架构直接冲突；
- 当前五个未归档任务状态为 planning/in_progress，但新的现代化计划将取代它们。

根因是命令入口、环境管理、长期规范和任务过程材料没有统一的生命周期边界。

[Observed / 已核实] hooks 与 80 个 canonical tasks 已实施；活动文档、长期 spec、workflow/skills 与设计包已迁移，closeout PR 原生证据已写回，最终 manifest 已重建并复验。Child 6 当前只剩 archive，parent 级 journal、final PR CI/merge、exact-main CI 和分支清理随后执行，因此状态仍为 **In progress**。

## 8. 上游关系

[Decision / 已决策]

```text
origin   NongHua123/fyagent          fetch+push
upstream farion1231/cc-switch        fetch only; push DISABLED
```

[Observed / 已核实] CC Switch `v3.19.2` 本身包含产品运行时 mise 可选兼容和大量产品改动。产品运行时兼容不等于 FyAgent 自身启动依赖 mise。

## 9. 结论

四个问题相互依赖：

依赖顺序已按设计执行：上游 merge → 交叉构建删除 → 开发环境 → CI/Release → DEP0040 → 文档/Trellis。实现 PR/main/manual、post-merge exact-main-SHA preflight、annotated tag、formal run、13 附件 stable Release 和独立 attestation 复核已完成，v0.3.0 可如实标记为 **Released / Verified**。[PR #8](https://github.com/NongHua123/fyagent/pull/8) 首次 run `31264604075` 在 head `623b6924e3b8682321b26aa69c15dc6f0b9f6f09` 上暴露 version-only `setup-uv` 于 Windows on ARM 选择 `win-amd64` 的根因：x64 job `93120609402` 成功，ARM64 job `93120609411` 与 Required `93121912798` 失败。commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` 改用官方完整 Python request 并强制 managed Python；修复 run `31265504901` 的 x64 `93122857985`、ARM64 `93122858012` 与 Required `93123992476` 全部成功，Child 3/D116 closeout smoke 已远程验证。最终设计包 manifest 随后按冻结字节重建并复验；当前只剩 task archive、journal、final PR CI/merge、exact-main CI 和分支清理。D114 的 N/A 例外仍不是 `merge_group` 成功运行证据。
