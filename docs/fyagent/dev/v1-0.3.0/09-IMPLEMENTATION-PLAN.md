# 分阶段实施计划

> **交付状态**：Phases 0–5 implemented; Phase 6 in progress; Phase 7 remote evidence pending / 阶段 0–5 已实施，阶段 6 进行中，阶段 7 待远程验收
> **关联决策**：1–117
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 总体顺序

```text
Phase 0  建立真实 Git/远程/标签证据
Phase 1  完整 merge CC Switch v3.19.2
Phase 2  删除本地交叉构建
Phase 3  重构 mise/uv/Python/task API
Phase 4  重构 CI 与 Release
Phase 5  根治 DEP0040
Phase 6  迁移文档/Trellis，归档旧任务
Phase 7  最终 CI、预演和 Release 验收
```

任何阶段若违反 GO 条件不得用后续阶段“顺便修复”掩盖。

## 2. Trellis 任务树

| 任务                                     | 输入                 | 主要输出                                               | 依赖                           |
| ---------------------------------------- | -------------------- | ------------------------------------------------------ | ------------------------------ |
| Parent modernization                     | 决策 1–117           | 跨子任务契约、最终追踪                                 | 全部 child                     |
| merge-cc-switch-v3-19-2                  | 55173d2b、上游 tag   | 显式 merge commit、许可来源                            | Phase 0                        |
| remove-local-cross-builds                | merge 结果           | 删除脚本/task/spec/test                                | Child 1                        |
| redesign-mise-uv-development-environment | 标准版本选择         | mise/uv/task/lock 合同                                 | Child 2                        |
| modernize-ci-and-release                 | 新事实源与脚本       | workflow-only CI/Labeler/无签名 Release 实施与远程证据 | Child 3                        |
| eliminate-dep0040-punycode               | Node 24 + merge 结果 | 原生 Fetch、锁文件、探针                               | Child 3，可与 Child 4 部分并行 |
| migrate-docs-and-trellis-specs           | 前述最终命令/文件    | 活动 docs/spec/tasks/skills                            | 1–5 全部稳定后                 |

## 3. 实际提交序列与剩余边界

1. `f4462765` — `merge(upstream): merge CC Switch v3.19.2`（两父 merge）；
2. `e8954d97` — `build: remove local cross-platform builds`；
3. `3d534710` — `build: modernize mise and uv development environment`；
4. `038675b3` — `ci: restore automatic required checks and labeling`；
5. `94ff9ee9` — `ci: build unsigned release transaction`；
6. `4e407df4` — `test(node): remove cross-fetch and enforce native fetch`；
7. `eb748f9c` / `58101230` — Trellis 合同修复以及活动文档、长期 specs、决策追踪；
8. `1d3849e6` — 五个旧任务的 superseded archive；
9. `580c5efa` — 归档后活动任务统一验证回归；
10. 实现 PR 采用 GitHub merge commit 合入 `main`；
11. 正式 Release 后 closeout PR — 远程证据、剩余新任务归档、journal。

实际可按审查大小拆分，但 merge commit 不得混入 2–9。

D116 收口时已停止本地 Windows cargo/rustc 诊断进程，删除显式诊断临时目录，并清理 `src-tauri/target/app`（4.1 GiB）与 `target/installer-actions`（57.4 MiB）；当前 Rust target 仅 `x86_64-unknown-linux-gnu`。该清理只恢复本地宿主原生边界，先前 Light/MSI 输出不进入验收。

## 4. Phase 0 检查

- 工作树干净、基线 SHA/branch 与项目方信息一致；
- origin/upstream URL 与 push 边界正确；
- 上游 tag object、peeled commit、来源和可用的上游 tag 签名记录；该来源证据不构成 FyAgent v0.3.0 产物签名；
- 创建备份分支或可恢复引用；
- 评估 v3.19.1 ancestry 与真实 merge base；
- 将实际 commit set/diff stat 写入上游台账。

## 5. 各阶段验证

### 上游 merge

普通 CI 范围、品牌/许可/数据目录/版本差异审查。根据 49C，不另建产品专用 acceptance workflow，但不得省略冲突清单。

### 交叉构建删除

负向扫描、mise lock 结构、README/spec/task 引用、本机原生 build task。

### 开发环境

本地仅在当前宿主 OS/架构验证 mise install、uv managed Python、bootstrap、env:check 与 Trellis wrappers。Linux ARM64、macOS x64/ARM64、Windows x64/ARM64 等非宿主平台必须使用匹配的 GitHub Actions native runner；Windows ARM64 是明确远程验收目标。

### CI/Release

PR、main、manual 事件；聚合 gate 失败/取消/跳过模拟；post-merge exact-main-SHA unsigned 全矩阵预演；10 安装器/13 附件和 attestation 验证。每次已授权触发都由发起主流程同步等待整次 run 到 `completed`，随后一次读取最终 run/job 结果，仅失败时抓取失败 job 日志；不派后台/异步监控代理，不重复轮询。`merge_group` 静态合同已实现，但真实事件因个人账户且禁止 ruleset/branch protection 而无法产生。D114 已接受其为当前治理下 N/A 的验证例外；不得记为成功，替代验收仍需真实 PR/main/manual 运行。

### DEP0040

锁文件/`pnpm why`、Native Fetch/MSW、普通和 pending deprecation 探针。

### 文档/Trellis

生成 task docs、文档扫描、spec index、hooks 模拟输入、旧任务 superseded 元数据。

## 6. 回退点

- merge commit 可单独 revert；
- 交叉构建删除可单独 revert，但不应与 Actions 发布矩阵回退绑定；
- mise/uv 变更应整体回退版本文件、tasks、locks、hooks；
- CI 与 Release 分开提交，防止 CI 回退意外恢复不安全发布权限；
- `cross-fetch` 删除可单独 revert，但回退意味着重新接受 DEP0040 风险，需明确记录；
- 文档 overlay 只在实际配置稳定后应用。

## 7. 完成条件

- 所有 required CI 自动运行并稳定；
- 所有工具链实际版本与事实源一致；
- 活动仓库无本地 cross-build 入口；
- 所有本地标准开发/构建/测试/打包/验证严格停留在当前宿主 OS/架构；非宿主验收都有匹配的 Actions native runner 证据；
- 无 `cross-fetch` 旧链路或告警抑制；
- unsigned 全矩阵预演成功；
- 正式无签名流程成功产生精确 10 安装器/13 附件、provenance 与 manifest；
- 活动文档与 task 元数据一致；
- 风险登记达到 GO；任何尚未解决的 NO-GO 或真实远程证据缺口都会阻止发布与 parent 归档。D114 的已接受 N/A 例外本身不再要求 live `merge_group` 运行。

### 远程观察纪律

D117 不改变任何触发或发布授权。获得单次触发授权后，发起主流程必须保持同步并使用一个 whole-run wait 等到 `completed`；之后只读取一次最终 run/job 结果。成功不抓全量日志，失败才取失败 job 日志。不得将等待交给后台/异步代理，也不得用反复 `list`/`view`/job/check 查询拼接状态证据。

### Preflight 顺序覆盖

原计划中的“待合入 main 的同一 SHA preflight”与 GitHub merge commit 及标准 artifact attestation 的 `GITHUB_SHA` provenance 不兼容。已实施的安全顺序为：实现 PR merge → main 的 `CI / Required` 成功 → `source_sha == GITHUB_SHA == GITHUB_WORKFLOW_SHA` 的 manual preflight → tag → formal Release。The project owner accepted D113/D114 on 2026-08-08，其中 D113 明确接受该顺序；不得把 pre-merge candidate 标记为 trusted-main attestation provenance，也不得把决策接受写成 preflight 已运行。

D113/D114 继续有效；D116/D117 只收紧执行与取证方式。当前最终候选尚未形成满足门禁的 PR/main/manual、preflight、formal Release 与 closeout 完整成功证据；先前失败、诊断或过时 SHA 的运行不能升级为完成，整体仍为 Pending/NO-GO。
