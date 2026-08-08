# FyAgent 上游同步、工具链与发布现代化设计包

> **交付状态**：Implementation in progress / 实施中
> **关联决策**：1–115（含历史、执行覆盖与已接受治理例外）
> **证据边界**：Child 1、2、5 已实施、本地验证并归档；Child 3、4 已实施且本地验证，远程证据待补；Child 6 实施中；parent 与正式 Release closeout 待完成。

## 1. 目的

本目录最初是 2026-08-07 对输入快照进行只读分析后形成的需求、设计、Trellis 规划和文档 overlay。项目方于 2026-08-08 批准真实实施；本目录现作为原始输入意图、覆盖决策、实施提交、本地验证和远程 closeout 的审计工作区。冻结的 overlay payload 仅保留来源记录，不再是可应用补丁。

## 2. 执行授权与当前边界

[Decision / 已决策] 2026-08-08 的执行授权覆盖了原“只生成文档和 ZIP”的边界，并批准代码、配置、Git、Actions、PR、tag、Release 和 Trellis 归档。当前真实状态是：

- 上游两父 merge、交叉构建退役、mise/uv/task API、CI/Labeler、无签名 Release workflow 和 DEP0040 根因修复已经落地并完成本地合同验证；
- 产品版本已更新为 `0.3.0`，正式 tag 固定为 `v0.3.0`，但 tag 与 GitHub Release 尚未创建；
- v0.3.0 明确不签名、不公证、不 staple，不使用签名 secrets 或 Release environment；
- 仓库公开，但不配置 branch/tag ruleset、branch protection 或 Release environment；来源资格仅由 workflow 失败关闭检查约束，该残余风险已接受；
- The project owner accepted D113/D114 on 2026-08-08：D113 确认 post-merge exact-main/workflow-SHA preflight 顺序；D114 确认当前个人仓库/无保护治理下真实 `merge_group` 运行不适用（N/A）的验证例外；
- 真实 PR/main/full-matrix/preflight/13 附件/attestation/Release URL 证据仍待远程执行，不得从本地测试推断；
- `merge_group` YAML trigger 与失败关闭合同已实现；个人账户仓库且禁止保护规则使 Merge Queue 无法启用，因此不会有真实事件成功证据。D114 接受以该静态合同加后续真实 PR/main/manual 运行作为替代证据，不得把 N/A 写成成功运行。

## 3. 证据标签

| 标签                              | 含义                                                                                   |
| --------------------------------- | -------------------------------------------------------------------------------------- |
| `[Observed / 已核实]`             | 来自上传源码、官方文档或上游公开资料的事实。                                           |
| `[Decision / 已决策]`             | 访谈中由项目方确认的选择。                                                             |
| `[Proposed / 拟实施]`             | 仅描述 2026-08-07 原始设计快照；若与 2026-08-08 覆盖决策或真实实现冲突，以后两者为准。 |
| `[Pending Verification / 待验证]` | 必须在含 `.git` 的真实仓库、Actions runner、正式发布或目标平台中验证。                 |

## 4. 基线

- [Observed / 已核实] 实施基线为 `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`；恢复引用为 `refs/backup/fyagent-v0.3.0-baseline`。
- [Observed / 已核实] 上游 tag object 为 `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`，peeled commit 为 `43eaf07355af145aebfee301801779e824d4c221`，merge base 为 `28529620f438b2ed25c812f6364825d846a4a9d6`。
- [Observed / 已核实] 两父 merge commit 为 `f4462765e9b3a2efd1deb13aabf3ce349166a058`；第二父正是上游 peeled commit。

## 5. 权威层级

1. 用户批准的 2026-08-08 实施计划与 `research/execution-authority.md`；
2. 当前代码、workflow、测试和 `.trellis/spec/` 可执行合同；
3. `decisions/DECISION-REGISTER.md` 的历史决策及其覆盖登记；
4. `01`–`10` 主文档、`implementation-map/` 与真实 validation evidence；
5. `sources/` 中的输入来源与上游 provenance；
6. `repository-overlay/` 仅为冻结的 2026-08-07 设计快照，不得整包应用。

代码、配置和 workflow 的当前事实以真实仓库树及长期 spec 为准。

## 6. 阅读路径

- 先读 `01-REQUIREMENTS-AND-DECISIONS.md` 与 `02-CURRENT-STATE-AND-ROOT-CAUSE.md`；
- 按 `03`→`07` 理解技术方案；
- 用 `09-IMPLEMENTATION-PLAN.md` 组织后续 PR；
- 用 `10-RISKS-AND-ACCEPTANCE.md` 判断 GO/NO-GO；
- 用 `implementation-map/CONFIG-AND-WORKFLOW-TARGET-SNIPPETS.md` 对照“原始目标→实际实现”；
- 阅读 `repository-overlay/README.md` 的冻结声明，不应用其中 payload。

## 7. 关键结果

- 完整合并 CC Switch `v3.19.2`，保留 ancestry 与 FyAgent 独立身份；
- 删除本地 Linux/WSL→Windows/macOS 交叉构建，不削减 Actions Release 矩阵；
- 本地统一为 `mise run <task>`；mise 管理 uv，uv 独占管理 Python 3.14.7 和 `.venv`；
- Node 24.19.0、Rust 1.97.1、pnpm 10.12.3 使用标准文件为事实源；
- CI/Labeler 已使用明确 runner、固定 Action SHA、最小权限和稳定 Required gate；
- Linux Release 在新宿主的同架构 Ubuntu 22.04 容器中构建；
- 已删除 `cross-fetch` polyfill，使用 Node 原生 Fetch，并用行为、依赖图和弃用探针防止 `DEP0040` 回流；
- 活动 README、CONTRIBUTING、Trellis specs/workflow/skills 已完成本地迁移；五个旧任务已以 `superseded` 语义归档，剩余新任务只在真实远程 closeout 证据完整后归档。

## 8. 完整性

根目录现有 `MANIFEST.sha256` 是原始设计包快照，当前修订期间必然 stale。只有正式 Release 和 closeout 证据落地后才重新生成，覆盖本目录中除清单自身外的最终文件；在此之前不得把它作为当前字节完整性证据。
