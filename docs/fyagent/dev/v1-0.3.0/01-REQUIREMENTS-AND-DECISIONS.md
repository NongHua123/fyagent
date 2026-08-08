# 需求规格与决策基线

> **交付状态**：Release requirements verified / 发布需求已完成远程验收；closeout 与归档进行中
> **关联决策**：1–118（含历史、执行覆盖、已接受治理例外与当前执行纪律）
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 背景

[Observed / 已核实] 2026-08-07 输入基线同时存在：Linux/WSL 本地交叉构建、仅手动触发的 CI、Node/Rust 本地与 Actions 漂移、非规范的 mise 任务入口、系统 Python 直调、`cross-fetch` 引入的旧 `punycode` 依赖链，以及活动文档/Trellis 规范与目标架构冲突。

[Decision / 已决策] 本项目扩大为一个有序的现代化工作流：先完整合并 CC Switch `v3.19.2`，再清理交叉构建、重构开发环境和 Actions、根治 `DEP0040`、迁移文档与 Trellis。

## 2. 目标

| 需求域 | 目标                                                                      |
| ------ | ------------------------------------------------------------------------- |
| UP     | 可审计地完整合并上游稳定标签，保留 Git ancestry、FyAgent 身份和许可边界。 |
| XB     | 删除本地跨 OS/架构执行，只保留宿主原生命令和 Actions 原生验证/正式发布。  |
| ENV    | 统一 Node/Rust/pnpm/uv/Python 的声明、安装、锁定和诊断。                  |
| TASK   | 以 `mise run` 建立稳定、跨平台、可生成文档的仓库任务 API。                |
| CI     | 将 Actions 变成自动、严格、可复现的合并门禁。                             |
| REL    | 将 Actions 变成唯一正式发布来源，并形成 10 资产的失败关闭事务。           |
| DEP    | 从依赖图中移除旧 `punycode` 链路，建立原生 Fetch 行为和告警回归。         |
| DOC    | 使 README、CONTRIBUTING、PR 模板、Trellis 和 hooks 使用同一入口。         |
| GOV    | 保留历史、归档被取代任务、建立追踪矩阵和风险门槛。                        |

## 3. 非目标与当前排除项

- 不减少 Windows/Linux/macOS 的正式 Release 覆盖；
- 不删除 CC Switch 上游的产品运行时 mise 可选兼容；
- 不将 FyAgent 版本改为 `3.19.2`；
- 不建立本地正式发布或签名入口；
- v0.3.0 不实施 Windows/macOS 签名、公证、staple、签名凭据或 Release environment；
- 不配置 main/tag ruleset 或 branch protection；
- 不把旧 Trellis 任务伪装为已完成；
- 不重写历史版本化设计文档正文；
- 不复制上游 `v3.19.2` release notes 正文。

## 4. 功能性需求

### 4.1 上游同步（UP）

- **UP-001**：`upstream` fetch URL 必须为 `farion1231/cc-switch`，push URL 必须保持 `DISABLED`。
- **UP-002**：同步目标是正式 tag `v3.19.2`，实施时验证完整 commit SHA。
- **UP-003**：使用显式 merge commit；merge commit 只包含必要冲突裁决。
- **UP-004**：冲突必须按身份、上游共享逻辑、工程治理、FyAgent 专属功能四层裁决。
- **UP-005**：合并后的上游代码保留 MIT 来源；FyAgent 自有许可边界不回退。
- **UP-006**：后续删除上游 release-note 文件，但保留 provenance 台账与 CHANGELOG 摘要。

### 4.2 交叉构建清理（XB）

- **XB-001**：删除 `scripts/macos-cross/**`、`scripts/windows-cross/**` 及对应任务、测试、活动规范。
- **XB-002**：从基础 Rust 环境移除 `llvm-tools` 和只服务交叉构建的 Apple/Windows targets。
- **XB-003**：保留本机 `dev`/`build`，但不得通过标准 task 指定其他 OS/架构 target。
- **XB-004**：保留 Actions 的 Windows x64/ARM64、Linux x64/ARM64、macOS Universal。
- **XB-005**：任何项目脚本不得替用户执行 `mise trust --yes`。
- **XB-006**：本地标准开发、构建、测试、打包和验证命令只能针对当前宿主 OS/架构；子系统桥接、外来可执行文件、模拟器、复制工具链和本地暂存的非宿主产物都不能绕过该边界。
- **XB-007**：Windows、macOS、ARM64 及任何其他非宿主原生验收只能来自匹配的 GitHub Actions native runner；本地诊断结果不计入验收。

### 4.3 开发环境（ENV/TASK/PY）

- **ENV-001**：`.node-version=24.19.0`、`packageManager=pnpm@10.12.3`、`rust-toolchain.toml=1.97.1`。
- **ENV-002**：Actions 与 mise 消费标准文件，不在 workflow/mise.toml 重复硬编码 Node/Rust/pnpm。
- **ENV-003**：`mise.toml` 管理 `uv=latest`，`mise.lock` 固定当前批准的精确 uv 资产。
- **PY-001**：`.python-version=3.14.7`；`pyproject.toml` 为非包型开发环境；uv 只使用 managed Python。
- **PY-002**：uv 创建项目 `.venv`、维护 `uv.lock`、运行 Trellis 与 Codex hook。
- **TASK-001**：活动开发命令统一为 `mise run <canonical-task>`。
- **TASK-002**：`bootstrap` 不执行 trust、系统包安装、Git remote 修改、锁文件升级或发布。
- **TASK-003**：`env:check` 与 `system:check` strict、只读、失败关闭。
- **TASK-004**：修改型 task 默认预演/确认，不进入 `check`。
- **TASK-005**：生成 task 参考并在 CI 比较，防止命令文档漂移。
- **TASK-006**：常规 task 不隐式安装工具；`mise run bootstrap` 是显式准备锁定工具和依赖的唯一高级入口。

### 4.4 CI 与 Release（CI/REL）

- **CI-001**：监听 `pull_request`、`push(main)`、`merge_group`、`workflow_dispatch`。
- **CI-002**：Required/Release 禁止 `*-latest`；Actions 引用固定完整 SHA。
- **CI-003**：默认权限 `contents: read`；写权限仅授予必要 job。
- **CI-004**：稳定聚合 job 使用 `if: always()` 并明确拒绝 failure/cancelled/异常 skipped。
- **CI-005**：运行时验证每个平台实际 Node/pnpm/Rust 版本。
- **CI-006**：授权触发 Actions run 后，发起主流程同步等待整次 run 到 `completed`，随后只读取一次最终 run/job 结果；不得派后台/异步监控代理或重复轮询，且仅失败结果允许获取失败 job 日志。
- **REL-001**：正式 tag 必须与产品版本一致，属于 `origin/main`，并绑定同 SHA 的成功 `CI / Required`；仓库没有管理员保护规则，workflow-only 检查是已接受的残余风险。
- **REL-002**：正式发布精确包含 10 个安装资产。
- **REL-003**：Linux 在同架构 Ubuntu 22.04 容器构建，宿主使用明确的新 runner。
- **REL-004**：所有平台完成后才允许 publish job 获得 `contents: write`。
- **REL-005**：强制生成下载清单、SHA-256、构建元数据和 GitHub artifact attestation；任一缺失均阻止发布。
- **REL-006**：正式 Release 精确发布 13 个附件：10 个安装器、`download-manifest.json`、`build-metadata.json` 和 `artifact-attestation.sigstore.json`。

### 4.5 DEP0040（DEP）

- **DEP-001**：删除 `cross-fetch` 直接依赖和 `cross-fetch/polyfill` 导入。
- **DEP-002**：测试环境要求 Node 24 原生 `fetch/Headers/Request/Response`。
- **DEP-003**：增加 Native Fetch→MSW→Tauri mock 行为测试。
- **DEP-004**：普通 Node 测试使用 `--throw-deprecation`；聚焦探针增加 `--pending-deprecation`。
- **DEP-005**：禁止 warning suppression；依赖图证明旧链路退出。

### 4.6 文档与 Trellis（DOC/GOV）

- **DOC-001**：四份 README、CONTRIBUTING、PR 模板、Flatpak、视觉基线、Trellis workflow/skills 使用新入口。
- **DOC-002**：历史文档正文不改写，只加归档声明。
- **GOV-001**：建立 parent+6 child tasks；新长期规则写回 `.trellis/spec/`。
- **GOV-002**：当前五个未归档任务以 `superseded` 语义归档。
- **GOV-003**：交付使用证据标签、决策追踪和风险门槛。

## 5. 关键版本与事实源

| 工具    |                                 目标 | 事实源                        | Actions 消费方式                                                         |
| ------- | -----------------------------------: | ----------------------------- | ------------------------------------------------------------------------ |
| Node.js |                              24.19.0 | `.node-version`               | `node-version-file`                                                      |
| pnpm    |                              10.12.3 | `package.json#packageManager` | setup action读取 packageManager                                          |
| Rust    |                               1.97.1 | `rust-toolchain.toml`         | 经审核的 Rust setup action读取文件                                       |
| uv      | `latest` 选择器 / lock 解析为 0.12.2 | `mise.toml` + `mise.lock`     | contracts/frontend job 从 lock 解析版本并用固定 SHA 的 setup action 安装 |
| Python  |                               3.14.7 | `.python-version`             | 本地和需要合同测试的 Actions job 均由 uv managed Python 提供             |

## 6. 验收总则

- 所有 `D-001`–`D-118` 可追踪到设计、文件、子任务和证据状态；历史行由覆盖行解释，不篡改原时间线；
- 合并、配置、依赖和本地 CI/Release 合同已有真实提交证据；PR #7、same-SHA main CI、五原生目标 preflight、formal run、stable/latest Release、13 附件、独立重下载和 12-subject attestation 也已由真实远程证据满足；
- 不得用“默认无告警”“编译成功”或“某平台静态检查”替代明确的跨平台、无签名负向状态或依赖图验收；
- The project owner accepted D113/D114 on 2026-08-08：D113 确认 merge → main CI → exact-main/workflow-SHA preflight 的发布顺序；D114 确认真实 `merge_group` 在当前治理下为 N/A 的验证例外；
- D116 固化“本地仅宿主原生、非宿主只走 Actions native runner”；D117 固化“主流程同步等待整次 run，再一次读取结果，失败后才取日志”。本次 PR/main/preflight/formal 运行已按 D117 形成单一有序证据链，且非宿主结果全部来自匹配的 Actions native runner；
- `merge_group` 仍无法产生真实事件，且不得把静态 YAML 合同写成事件成功；D114 保持 live run=N/A 的已接受例外，其 YAML trigger、失败关闭静态合同与真实 PR/main/manual 替代证据已完整；
- 任一违反身份、许可、远程安全、正式发布来源或 strict 工具链契约的情况均是未来 Release 的 NO-GO；v0.3.0 的这些门禁已有通过证据，但无 ruleset/branch protection/Release environment 的已接受残余风险仍持续存在。
