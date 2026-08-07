# FyAgent 上游同步、工具链与发布现代化设计包

> **交付状态**：Proposed / 拟实施（文档交付已生成）  
> **关联决策**：1–104  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 目的

本 ZIP 是对项目方提供的 FyAgent 基线进行只读分析后形成的**需求、设计、Trellis 规划和文档 overlay**。它为后续工程实施建立可追溯的工作边界，但不是可直接运行的源码补丁，也不表示任何 Git 合并、测试、CI 或 Release 已经完成。

## 2. 授权边界

[Decision / 已决策] 本次只允许生成文档和 ZIP。未执行：

- `git fetch`、`git merge`、`git commit`、`git tag`、`git push`；
- 产品源码、`mise.toml`、锁文件、GitHub Actions 或仓库 ruleset 的实际修改；
- Node/Rust/uv/Python 安装、测试、签名、公证或发布；
- 上游 release note 的复制分发。

## 3. 证据标签

| 标签 | 含义 |
|---|---|
| `[Observed / 已核实]` | 来自上传源码、官方文档或上游公开资料的事实。 |
| `[Decision / 已决策]` | 访谈中由项目方确认的选择。 |
| `[Proposed / 拟实施]` | 目标设计或完整文档草稿，尚未应用到真实仓库。 |
| `[Pending Verification / 待验证]` | 必须在含 `.git` 的真实仓库、Actions runner、签名环境或目标平台中验证。 |

## 4. 基线

- [Decision / 已决策] FyAgent 基线：`55173d2b`，分支 `feature/fyagent-v1`。
- [Observed / 已核实] 上传压缩包不含 `.git`，因此无法独立验证 ancestry。
- [Observed / 已核实] 上游目标：`farion1231/cc-switch` 的 `v3.19.2`，GitHub 发布页显示短提交 `43eaf07`，其公开 commit 页面为 `43eaf07355af145aebfee301801779e824d4c221`，发布日期 2026-08-06。
- [Pending Verification / 待验证] 实施前仍必须在真实仓库通过 `git rev-parse v3.19.2^{commit}` 核验 tag 实际解析到 `43eaf07355af145aebfee301801779e824d4c221`。

## 5. 权威层级

1. `decisions/DECISION-REGISTER.md`：确认决策；
2. `01`–`10` 主文档：需求、设计、实施和风险；
3. `repository-overlay/trellis/`：拟直接应用的 Trellis artifacts/spec/workflow/skills；
4. `repository-overlay/documentation/`：拟直接替换的活动开发文档；
5. `implementation-map/`：逐文件、提交、CI、Release 资产契约；
6. `sources/`：外部来源、上传工件登记和上游 provenance。

代码、配置和 workflow 仅在主文档中给出精确目标结构，不在 overlay 中伪装为已实施文件。

## 6. 阅读路径

- 先读 `01-REQUIREMENTS-AND-DECISIONS.md` 与 `02-CURRENT-STATE-AND-ROOT-CAUSE.md`；
- 按 `03`→`07` 理解技术方案；
- 用 `09-IMPLEMENTATION-PLAN.md` 组织后续 PR；
- 用 `10-RISKS-AND-ACCEPTANCE.md` 判断 GO/NO-GO；
- 用 `implementation-map/CONFIG-AND-WORKFLOW-TARGET-SNIPPETS.md` 审阅尚未实施的配置/workflow 目标；
- 应用 overlay 前先审阅 `repository-overlay/README.md`。

## 7. 关键结果

- 完整合并 CC Switch `v3.19.2`，保留 ancestry 与 FyAgent 独立身份；
- 删除本地 Linux/WSL→Windows/macOS 交叉构建，不削减 Actions Release 矩阵；
- 本地统一为 `mise run <task>`；mise 管理 uv，uv 独占管理 Python 3.14.7 和 `.venv`；
- Node 24.19.0、Rust 1.97.1、pnpm 10.12.3 使用标准文件为事实源；
- CI 使用明确 runner、固定 Action SHA、最小权限和稳定 Required gate；
- Linux Release 在新宿主的同架构 Ubuntu 22.04 容器中构建；
- 删除 `cross-fetch` polyfill，使用 Node 原生 Fetch，并用弃用探针防止 `DEP0040` 回流；
- 更新活动 README、CONTRIBUTING、Trellis specs/workflow/skills；旧任务以 superseded 语义归档。

## 8. 完整性

根目录 `MANIFEST.sha256` 在最终打包步骤生成，覆盖本目录中的全部交付文件（清单文件自身除外，避免自引用）。
