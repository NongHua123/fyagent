# 当前状态与根因分析

> **交付状态**：Observed / 已核实 + Decision / 已决策  
> **关联决策**：1–34、39–42、81–96  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 仓库快照

[Observed / 已核实]

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

| 传播面 | 当前证据 | 根因 |
|---|---|---|
| 脚本 | `scripts/macos-cross/**`、`scripts/windows-cross/**` | Linux/WSL 跨 OS 构建承担 SDK、Wine、WiX、osxcross、签名边界。 |
| mise | 5 个 task；Rust 带 `llvm-tools` 与 4 个非宿主 target | 发布专用工具被混入所有开发者的基础环境。 |
| 安全 | 两个 macOS 脚本调用 `mise trust --yes` | 脚本替用户作出仓库信任决定。 |
| 测试 | `tests/macosCrossWorkflow.test.ts` | 主要检查文本包含，难以证明真实工具归属或 lock 结构。 |
| 文档/spec | 四份 README、development-environment、wsl-macos-cross-build 等 | 旧实现已成为活动规范，删除脚本不足以完成退役。 |

## 3. 工具链漂移

| 层 | Node | Rust | pnpm |
|---|---:|---:|---:|
| 本地 | 22.12.0 | 1.95.0 | 10.12.3 |
| CI/Release | 20 | stable | 10.12.3（重复声明/部分 Corepack） |
| 目标 | 24.19.0 | 1.97.1 | 10.12.3 |

根因不是“mise 与 Actions 安装方式不同”，而是**版本事实源重复且滚动值进入发布路径**。相同提交在不同时点可能解析到不同 Rust stable 或不同 runner OS。

## 4. mise.lock 结构问题

[Observed / 已核实]

- `mise.lock` 有两个 Rust `1.95.0` 条目，options/targets 不同；
- pnpm 的 `windows-arm64` 条目实际指向 `pnpm-win-x64.exe`；
- Python 3.12.8 缺少 Windows ARM64 条目；
- 当前测试以字符串查找平台名，不能证明逐工具平台资产正确。

根因是 lockfile 被当作文本清单，而不是按工具、版本、options、平台、URL、checksum 解析的结构化派生文件。

## 5. CI 与 Release 缺口

[Observed / 已核实]

- `ci.yml` 不随 PR/main 自动执行；
- 使用 `ubuntu-latest`、`windows-latest`、`macos-latest`；
- Actions 使用可移动标签而非完整 SHA；
- Release 手动预演只覆盖 macOS；
- Release 缺少 source SHA 已进入 main、Required CI 已成功、精确 10 资产、摘要和 provenance 的统一事务；
- `permissions: contents: write` 在 workflow 顶层过宽。

根因是发布矩阵已形成，但治理仍是“各 job 构建成功即发布”的流水线，而不是明确的来源资格、资产集合和最小权限事务。

## 6. DEP0040 根因

当前依赖链：

```text
cross-fetch@4.1.0
└─ node-fetch@2.7.0
   └─ whatwg-url@5.0.0
      └─ tr46@0.0.3
         └─ Node 内置 punycode
```

`cross-fetch/polyfill` 即使发现 `global.fetch` 已存在，也会在模块加载阶段引入 Node ponyfill 依赖图。Node 24 将部分弃用降为 application deprecation，默认可能不再打印 `node_modules` 中的警告，因此“输出干净”不能证明根因消失。

## 7. 文档与 Trellis 漂移

[Observed / 已核实]

- 仓库总计存在大量 `mise exec --` 文本；活动 README、CONTRIBUTING、PR 模板、visual-baseline、specs 明确使用旧入口；
- `.trellis/workflow.md` 和项目操作型 skills 直接使用 `python3`；
- `.codex/hooks.json` 直接调用系统 `python3`；
- 活动 `wsl-macos-cross-build.md` 与目标架构直接冲突；
- 当前五个未归档任务状态为 planning/in_progress，但新的现代化计划将取代它们。

根因是命令入口、环境管理、长期规范和任务过程材料没有统一的生命周期边界。

## 8. 上游关系

[Decision / 已决策]

```text
origin   NongHua123/fyagent          fetch+push
upstream farion1231/cc-switch        fetch only; push DISABLED
```

[Observed / 已核实] CC Switch `v3.19.2` 本身包含产品运行时 mise 可选兼容和大量产品改动。产品运行时兼容不等于 FyAgent 自身启动依赖 mise。

## 9. 结论

四个问题相互依赖：

1. 必须先合并上游，避免在即将冲突的旧基线上重构；
2. 再删除交叉构建和重构开发环境；
3. CI/Release 必须消费新的事实源和 task/contract；
4. `DEP0040` 修复必须基于 Node 24 且作为明确的上游差异；
5. 最后统一文档/Trellis，才能防止旧架构回流。
