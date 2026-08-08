# FyAgent v0.3.0 实施验证报告

> **状态**：Interim / 中间报告；正式发布当前 NO-GO
> **更新日期**：2026-08-08
> **证据边界**：只记录已发生的 Git、本地检查、明确的远程 pending 状态与已接受治理例外，不以静态 workflow 代替 Actions 或 Release 证据。

## 1. 当前结论

| 范围                                | 状态                                                       | 证据                                                                    |
| ----------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------- |
| Child 1 上游 v3.19.2 merge          | Implemented + locally verified + archived                  | `f4462765`、`487995e0`、`20a4bc65`                                      |
| Child 2 本地 cross-build 退役       | Implemented + locally verified + archived                  | `e8954d97`、`b8e50c4a`、`5e0dc678`                                      |
| Child 3 mise/uv/task/version        | Implemented + Linux x64 verified; remote platforms pending | `3d534710`、`8bd54f6b`；task 保持 `in_progress`                         |
| Child 4 CI/Labeler/unsigned Release | Implemented + locally verified; remote evidence pending    | `038675b3`、`94ff9ee9`、`2526588a`；task 保持 `in_progress`             |
| Child 5 DEP0040                     | Implemented + locally verified + archived                  | `4e407df4`、`e5c543f7`、`6be28965`                                      |
| Child 6 docs/Trellis                | Local migration verified; remote closeout pending          | `eb748f9c`、`58101230`、`1d3849e6`、`580c5efa`；task 保持 `in_progress` |
| Parent                              | Pending / NO-GO                                            | remote gates 与最终归档未完成；D113/D114 决策门禁已解除                 |

## 2. 已核实的实施事实

- 实施基线 `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`，恢复引用 `refs/backup/fyagent-v0.3.0-baseline`。
- 上游 tag object `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`，peeled commit `43eaf07355af145aebfee301801779e824d4c221`，merge base `28529620f438b2ed25c812f6364825d846a4a9d6`。
- 两父 merge commit `f4462765e9b3a2efd1deb13aabf3ce349166a058` 的第二父为上述 peeled commit。
- 产品版本事实源及两个本地 lock package 为 `0.3.0`；正式 tag 合同固定为 `v0.3.0`，但 tag 尚未创建。
- Node `24.19.0`、pnpm `10.12.3`、Rust `1.97.1`、Python `3.14.7`、lock 解析 uv `0.12.2`；当前本地原生执行只证明 Linux x64。
- 自动 CI、trusted-base Labeler、稳定 `CI / Required`、无签名五目标 Release、10 installer/13 attachment、mandatory attestation 和 draft→stable publish transaction 已通过本地静态/行为合同。
- `cross-fetch` 旧链路、polyfill import 与 warning suppression 已退出；Native Fetch→MSW→Tauri mock 四类行为已验证。
- D116 宿主原生本地边界已由共享 runtime planner 执行：canonical package/mise Tauri 与 Cargo 入口核对 process host 及绝对 rustc/rustdoc 身份，拒绝 caller compiler/wrapper/runner/linker/target、target-specific flags、loader/runtime injection 与 forwarded argv，扫描 Cargo config/includes 后显式 own 当前宿主 child env 与 target；Cargo test 的 no-shell runner 还核对原生格式和机器架构。D117 同步 whole-run 观察纪律已写入活动规范、Trellis workflow、任务工件和 v0.3.0 决策追踪；尚未由远程 run 实际演练 D117。

### 当前本地清理证据

- 为 Windows 诊断启动的精确 cargo/rustc 进程已停止，显式诊断临时目录已删除；当前进程复核未发现遗留 cargo/rustc 编译进程。
- `src-tauri/target/app` 与 `target/installer-actions` 已清理；清理前记录占用量分别为 4.1 GiB 和 57.4 MiB，当前路径复核均为不存在。
- `rustup target list --installed` 当前只返回 `x86_64-unknown-linux-gnu`。
- 先前本地 Windows Light/MSI 运行只用于诊断，不能引用为 Windows x64/ARM64 package、ICE、MSI 结构或生命周期验收。匹配的 Actions native runner 证据仍为 Pending。

详细命令、测试数量和修复记录分别位于各 child 的 `research/*evidence.md`，避免在本报告复制会漂移的日志。

## 3. Child 6 本地迁移验证

以下证据覆盖活动公开文档、长期 specs、项目 Trellis workflow/skills、本设计包
状态迁移和五个旧任务的 superseded archive；不替代远程 closeout：

- **PASS**：对本批次 39 个 Markdown 文件执行 Prettier check；
- **PASS**：`mise run tasks:docs:check`，生成的 task 参考与事实源逐字节一致；
- **PASS**：`mise run tasks:validate`，80 个 task metadata、docs contract、toolchain lock contract 全部通过；
- **PASS**：对 parent + 6 children 的真实/归档目录分别执行 `mise run trellis:validate -- <dir>`；
- **PASS**：活动 Trellis/spec 入口中无 `mise exec --` 或直接调用 `.trellis/scripts/*.py` 的残留；staged Markdown 的 299 个具体 `mise run` task 引用全部解析到 80-task catalog，workflow 的 6 个 finish-work 路由全部使用 `/trellis:finish-work`；历史 decision/冻结 overlay 不作为活动命令误报；
- **PASS**：内部 39 个文件的 49 个相对链接目标均存在；合并公开文档白名单后的最终共享 index 复核了 56 个现存 Markdown 文件与 178 个相对链接目标，另 3 项是有意删除的上游 release notes；
- **PASS**：`repository-overlay/` 为 111 个文件；diff 只包含 3 个中央冻结声明，108 个冻结 payload 未修改；
- **PASS**：工作树 diff whitespace check；
- **PASS**：共享 index 精确为 Child 6 的内部 39 项与公开文档 20 项两个批准白名单；不含 `task.json` 或 `tests/localBuildBoundary.test.ts`，cached diff whitespace check 通过。
- **PASS**：公开/内部两轮独立文档审查的全部 finding 已修复，最终无剩余 finding；
- **PASS**：五个旧任务按四 child 后 parent 归档，原状态与 superseded 语义保留；
- **PASS**：归档后无参 `trellis:validate` 覆盖四个活动任务，
  `miseTaskContract` 12/12、`release:check` 14 files/108 tests、Native Fetch 4/4。

### D116 runtime boundary 增量验证

- **PASS**：`localBuildBoundary.test.ts` 与 `miseTaskContract.test.ts` 共 37 个定向测试，覆盖六个 process OS/arch 映射、绝对 rustc/rustdoc 同 host/release/commit、Tauri/Cargo 当前宿主 argv/child-env plan、大小写无关 compiler/wrapper/runner/linker/target controls、target-specific/target-bearing flags、loader/runtime injection、递归 Cargo config/includes 和 fixed-operation forwarded argv 拒绝；
- **PASS**：真实 `mise run dev/build/build:binary/build:debug/check/rust:check/rust:clippy/rust:test` 拒绝 smoke，均在 Cargo/Tauri/frontend command 启动前因 caller target environment 非零退出；其中 `check` 的无子进程 guard 在 `env:check` 的 rustc 探测前失败关闭；
- **PASS**：真实 `pnpm dev` forwarded-argv 与 `pnpm build` target-environment 拒绝 smoke，均未启动 Tauri；
- **PASS**：真实 wrapper 配合临时 fake `rustc`/`rustdoc`/`pnpm`/`cargo` 的正常路径 smoke，记录到的 Tauri/Cargo argv 均只含核验后的当前 Linux x64 target；child env 固定绝对 compiler/rustdoc 并清空 wrappers/flags/injection controls，Cargo 以 TOML argv array 注入无 shell Node native runner；runner 的路径、symlink、ELF machine 与元字符 argv 回归通过，临时目录在测试结束时删除；
- **PASS**：`mise run rust:check`、`mise run rust:clippy` 及使用无匹配过滤器的 `mise run rust:test` 均通过真实当前宿主 wrapper 完成；Cargo 只使用 `--target x86_64-unknown-linux-gnu`，test binaries 全部经 TOML array no-shell runner 的路径/ELF machine 检查后执行；这不是打包或非宿主证据；
- **PASS**：`mise run typecheck`、`mise run tasks:validate`（80 tasks）、`mise run tasks:docs:check` 和 active task Trellis validation；
- **PASS**：决策登记与追踪矩阵严格唯一连续 1–117、入口文档引用 1–117、owned current docs 相对链接、canonical local cross-marker 与 active async/polling 反向扫描；
- **PASS**：owned code/docs targeted Prettier check 与工作树 whitespace diff check。

## 4. 远程待验证

以下项目尚未为当前最终候选形成满足验收的完整成功证据，因此状态必须保持 **Pending**。先前失败、诊断或过时 SHA 的 run、URL 与日志只能用于定位问题，不能替代下列门禁：

- 当前最终候选修复 PR 的自动 `CI / Required` 与安全 Labeler；
- merge 后 main push 的同名 Required check；
- Windows x64/ARM64、Linux x64/ARM64、macOS Universal 原生构建与运行时版本；
- `source_sha == GITHUB_SHA == GITHUB_WORKFLOW_SHA` 的 full-matrix unsigned preflight；
- 10 个 installer、两个 JSON evidence 和 Sigstore bundle；
- `v0.3.0` stable GitHub Release；
- 独立重下载、digest、架构、包结构、Windows `NotSigned`、macOS 无 Developer ID/notary 与 attestation 校验；
- closeout PR、最终 `MANIFEST.sha256`、剩余 child/parent archive 与 journal。

本轮没有触发、监控、轮询、重跑或取消 GitHub Actions。因而不存在可记录的 D117 远程观察结果；PR/main/manual/preflight/formal run 仍全部 Pending。

## 5. 已接受的治理决策

- The project owner accepted D113/D114 on 2026-08-08。
- **Confirmed — D113 preflight order**：原“待合入 main 的同一 SHA preflight”与 GitHub merge commit 及标准 artifact attestation 的 `GITHUB_SHA` provenance 不兼容。接受并保持的安全顺序为 merge → main CI → exact-main-SHA preflight → tag → formal Release；当前最终候选的成功 preflight 仍未产生，先前失败尝试不满足门禁。
- **Accepted verification exception — D114 merge_group**：仓库由个人账户拥有，且批准计划禁止 branch protection/ruleset。GitHub Merge Queue 无法在此组合下启用，因此不能产生真实 `merge_group` 事件证据。该项在当前治理下记为 N/A，而不是成功；接受的替代证据为 YAML trigger、失败关闭合同/静态测试和真实 PR/main/manual 运行，远程运行部分仍 Pending。
- **Confirmed — D116 host-native-only local execution**：本地 canonical 开发、构建、测试、打包和验证只针对当前宿主 OS/架构；package/mise wrapper 已在工具链启动前封闭 caller target env/argv，并显式固定核验后的当前 rustc host。低层 `pnpm tauri` 仅是 Actions/维护 leaf，不是本地标准入口；合同不声称拦截任意手写低层命令。本轮 Linux x64 runtime/清理证据不证明 Windows/macOS/ARM64。
- **Confirmed — D117 synchronous Actions observation**：授权触发后由发起主流程同步等待整次 run 到 `completed`，随后一次读取最终 run/job 结果；不派后台/异步 monitor、不重复轮询，仅失败时取失败 job 日志。本轮无远程 run，实际证据仍 Pending。

D113–D117 的当前决策已处置，但不替代 `4` 中的任何真实远程证据。PR/main/manual、原生矩阵、unsigned preflight、tag、Release、13 附件、attestation、独立复核与 closeout 未完成前，正式 Release 与 parent 归档仍为 NO-GO。

## 6. 原始设计包验证记录

2026-08-07 的只读输入包曾通过 134 个非 manifest 文件/135 个含 manifest 文件、JSON/JSONL 语法、1 parent+6 child overlay、decision 1–104、无 symlink/secrets 等静态检查。该记录只证明原始文档包，不证明当前修订字节或任何工程实施。现有 `MANIFEST.sha256` 保留为该历史快照，直到远程 closeout 后重新生成。
