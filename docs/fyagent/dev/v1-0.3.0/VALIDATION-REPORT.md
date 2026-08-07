# FyAgent v0.3.0 实施验证报告

> **状态**：Interim / 中间报告；正式发布当前 NO-GO
> **更新日期**：2026-08-08
> **证据边界**：只记录已发生的 Git、本地检查和明确的远程 pending/blocked 状态，不以静态 workflow 代替 Actions 或 Release 证据。

## 1. 当前结论

| 范围                                | 状态                                                       | 证据                                                            |
| ----------------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------- |
| Child 1 上游 v3.19.2 merge          | Implemented + locally verified + archived                  | `f4462765`、`487995e0`、`20a4bc65`                              |
| Child 2 本地 cross-build 退役       | Implemented + locally verified + archived                  | `e8954d97`、`b8e50c4a`、`5e0dc678`                              |
| Child 3 mise/uv/task/version        | Implemented + Linux x64 verified; remote platforms pending | `3d534710`、`8bd54f6b`；task 保持 `in_progress`                 |
| Child 4 CI/Labeler/unsigned Release | Implemented + locally verified; remote evidence pending    | `038675b3`、`94ff9ee9`、`2526588a`；task 保持 `in_progress`     |
| Child 5 DEP0040                     | Implemented + locally verified + archived                  | `4e407df4`、`e5c543f7`、`6be28965`                              |
| Child 6 docs/Trellis                | Internal migration locally verified; closeout pending      | 本报告第 3 节；公开文档、旧任务 archive、remote closeout 待收口 |
| Parent                              | Pending / NO-GO                                            | remote gates、两个决策门禁和最终归档未完成                      |

## 2. 已核实的实施事实

- 实施基线 `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`，恢复引用 `refs/backup/fyagent-v0.3.0-baseline`。
- 上游 tag object `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`，peeled commit `43eaf07355af145aebfee301801779e824d4c221`，merge base `28529620f438b2ed25c812f6364825d846a4a9d6`。
- 两父 merge commit `f4462765e9b3a2efd1deb13aabf3ce349166a058` 的第二父为上述 peeled commit。
- 产品版本事实源及两个本地 lock package 为 `0.3.0`；正式 tag 合同固定为 `v0.3.0`，但 tag 尚未创建。
- Node `24.19.0`、pnpm `10.12.3`、Rust `1.97.1`、Python `3.14.7`、lock 解析 uv `0.12.2`；当前本地原生执行只证明 Linux x64。
- 自动 CI、trusted-base Labeler、稳定 `CI / Required`、无签名五目标 Release、10 installer/13 attachment、mandatory attestation 和 draft→stable publish transaction 已通过本地静态/行为合同。
- `cross-fetch` 旧链路、polyfill import 与 warning suppression 已退出；Native Fetch→MSW→Tauri mock 四类行为已验证。

详细命令、测试数量和修复记录分别位于各 child 的 `research/*evidence.md`，避免在本报告复制会漂移的日志。

## 3. Child 6 内部迁移验证

以下证据只覆盖长期 specs、项目 Trellis workflow/skills 和本设计包的内部状态迁移；不替代公开文档 worker、旧任务归档或远程 closeout：

- **PASS**：对本批次 39 个 Markdown 文件执行 Prettier check；
- **PASS**：`mise run tasks:docs:check`，生成的 task 参考与事实源逐字节一致；
- **PASS**：`mise run tasks:validate`，80 个 task metadata、docs contract、toolchain lock contract 全部通过；
- **PASS**：对 parent + 6 children 的真实/归档目录分别执行 `mise run trellis:validate -- <dir>`；
- **PASS**：活动 Trellis/spec 入口中无 `mise exec --` 或直接调用 `.trellis/scripts/*.py` 的残留；staged Markdown 的 299 个具体 `mise run` task 引用全部解析到 80-task catalog，workflow 的 6 个 finish-work 路由全部使用 `/trellis:finish-work`；历史 decision/冻结 overlay 不作为活动命令误报；
- **PASS**：内部 39 个文件的 49 个相对链接目标均存在；合并公开文档白名单后的最终共享 index 复核了 56 个现存 Markdown 文件与 178 个相对链接目标，另 3 项是有意删除的上游 release notes；
- **PASS**：`repository-overlay/` 为 111 个文件；diff 只包含 3 个中央冻结声明，108 个冻结 payload 未修改；
- **PASS**：工作树 diff whitespace check；
- **PASS**：共享 index 精确为 Child 6 的内部 39 项与公开文档 20 项两个批准白名单；不含 `task.json` 或 `tests/localBuildBoundary.test.ts`，cached diff whitespace check 通过。

## 4. 远程待验证

以下项目没有 run/Release URL、下载产物或远程日志，因此状态必须保持 **Pending**：

- 实现 PR 的自动 `CI / Required` 与安全 Labeler；
- merge 后 main push 的同名 Required check；
- Windows x64/ARM64、Linux x64/ARM64、macOS Universal 原生构建与运行时版本；
- `source_sha == GITHUB_SHA == GITHUB_WORKFLOW_SHA` 的 full-matrix unsigned preflight；
- 10 个 installer、两个 JSON evidence 和 Sigstore bundle；
- `v0.3.0` stable GitHub Release；
- 独立重下载、digest、架构、包结构、Windows `NotSigned`、macOS 无 Developer ID/notary 与 attestation 校验；
- closeout PR、最终 `MANIFEST.sha256`、剩余 child/parent archive 与 journal。

## 5. 结构性阻塞与待决策偏差

- **Blocked — merge_group**：仓库由个人账户拥有，且批准计划禁止 branch protection/ruleset。GitHub Merge Queue 无法在此组合下启用，因此不能产生真实 `merge_group` 事件证据。workflow 的静态 trigger 合同不是远程事件成功。
- **Pending acceptance — preflight order**：原“待合入 main 的同一 SHA preflight”与 GitHub merge commit 及标准 artifact attestation 的 `GITHUB_SHA` provenance 不兼容。已实现的安全顺序为 merge → main CI → exact-main-SHA preflight → tag → formal Release。

任一项未处置都阻止正式 Release 与 parent 归档。

## 6. 原始设计包验证记录

2026-08-07 的只读输入包曾通过 134 个非 manifest 文件/135 个含 manifest 文件、JSON/JSONL 语法、1 parent+6 child overlay、decision 1–104、无 symlink/secrets 等静态检查。该记录只证明原始文档包，不证明当前修订字节或任何工程实施。现有 `MANIFEST.sha256` 保留为该历史快照，直到远程 closeout 后重新生成。
