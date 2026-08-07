# 分阶段实施计划

> **交付状态**：Proposed / 拟实施  
> **关联决策**：1–104  
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

| 任务 | 输入 | 主要输出 | 依赖 |
|---|---|---|---|
| Parent modernization | 决策 1–104 | 跨子任务契约、最终追踪 | 全部 child |
| merge-cc-switch-v3-19-2 | 55173d2b、上游 tag | 显式 merge commit、许可来源 | Phase 0 |
| remove-local-cross-builds | merge 结果 | 删除脚本/task/spec/test | Child 1 |
| redesign-mise-uv-development-environment | 标准版本选择 | mise/uv/task/lock 合同 | Child 2 |
| modernize-ci-and-release | 新事实源与脚本 | CI/Release/ruleset 设计实施 | Child 3 |
| eliminate-dep0040-punycode | Node 24 + merge 结果 | 原生 Fetch、锁文件、探针 | Child 3，可与 Child 4 部分并行 |
| migrate-docs-and-trellis-specs | 前述最终命令/文件 | 活动 docs/spec/tasks/skills | 1–5 全部稳定后 |

## 3. 建议提交序列

1. `merge(upstream): merge cc-switch v3.19.2`（两父 merge commit）；
2. `chore(build): remove local cross-platform build toolchains`；
3. `build(dev): establish mise task and uv python environment`；
4. `ci: enforce pinned multi-platform required checks`；
5. `ci(release): make actions the sole release artifact source`；
6. `test(node): remove cross-fetch and enforce native fetch deprecations`；
7. `docs(trellis): migrate task API and long-lived specifications`；
8. `chore(trellis): archive superseded tasks`（使用 no-commit 后单独明确提交）。

实际可按审查大小拆分，但 merge commit 不得混入 2–8。

## 4. Phase 0 检查

- 工作树干净、基线 SHA/branch 与项目方信息一致；
- origin/upstream URL 与 push 边界正确；
- tag 完整 SHA、签名/来源（若可用）记录；
- 创建备份分支或可恢复引用；
- 评估 v3.19.1 ancestry 与真实 merge base；
- 将实际 commit set/diff stat 写入上游台账。

## 5. 各阶段验证

### 上游 merge

普通 CI 范围、品牌/许可/数据目录/版本差异审查。根据 49C，不另建产品专用 acceptance workflow，但不得省略冲突清单。

### 交叉构建删除

负向扫描、mise lock 结构、README/spec/task 引用、本机原生 build task。

### 开发环境

在 Linux x64/ARM64、macOS x64/ARM64、Windows x64/ARM64 中按可用 runner/设备验证 mise install、uv managed Python、bootstrap、env:check、Trellis wrappers。Windows ARM64 是明确验收目标。

### CI/Release

PR、main、merge queue 事件；聚合 gate 失败/取消/跳过模拟；unsigned 全矩阵预演；signed 受保护环境预演；10 资产验证。

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
- 无 `cross-fetch` 旧链路或告警抑制；
- unsigned 全矩阵预演成功；
- 正式/签名流程在受控环境成功产生精确资产与 provenance/manifest；
- 活动文档与 task 元数据一致；
- 风险登记达到 GO 或明确的 GO WITH CONDITIONS。
