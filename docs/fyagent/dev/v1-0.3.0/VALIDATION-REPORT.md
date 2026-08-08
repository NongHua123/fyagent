# FyAgent v0.3.0 实施验证报告

> **状态**：Released and independently verified; repository closeout in progress / v0.3.0 已发布并独立复核，仓库收尾进行中
> **更新日期**：2026-08-09
> **证据边界**：只记录已发生的 Git、本地检查、Actions/Release 真实 URL 与独立下载复核；closeout PR native smoke 已远程验证，最终设计包 manifest 已重建并复验；归档、journal、final PR CI/merge、exact-main CI 和分支清理仍明确标为 pending。

## 1. 当前结论

| 范围                                | 状态                                                                         | 证据                                                                                                                           |
| ----------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Child 1 上游 v3.19.2 merge          | Implemented + locally verified + archived                                    | `f4462765`、`487995e0`、`20a4bc65`                                                                                             |
| Child 2 本地 cross-build 退役       | Implemented + locally verified + archived                                    | `e8954d97`、`b8e50c4a`、`5e0dc678`                                                                                             |
| Child 3 mise/uv/task/version        | Implemented + native remotely verified                                       | `3d534710`、`8bd54f6b`、`4645668d`；closeout run `31265504901` 的 x64/ARM64/Required 全成功                                    |
| Child 4 CI/Labeler/unsigned Release | Implemented + remotely verified                                              | PR/main/preflight/formal runs `31258884239` / `31259389682` / `31259905022` / `31260931509`；Release `367220197`               |
| Child 5 DEP0040                     | Implemented + locally verified + archived                                    | `4e407df4`、`e5c543f7`、`6be28965`                                                                                             |
| Child 6 docs/Trellis                | Migration verified; closeout pending                                         | `eb748f9c`、`58101230`、`1d3849e6`、`580c5efa`；本文档写集已记录 Release 证据，但 task 仍 `in_progress`                        |
| v0.3.0 Release                      | **Released / Verified**                                                      | [stable/latest Release](https://github.com/NongHua123/fyagent/releases/tag/v0.3.0)；exact 13 attachments；12 subjects verified |
| Parent                              | Release + native gate + final manifest complete; repository closeout pending | 仍需 child/parent archive → journal → final PR CI/merge → exact-main CI/branch cleanup                                         |

## 2. 已核实的实施事实

- 实施基线 `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`，恢复引用 `refs/backup/fyagent-v0.3.0-baseline`。
- 上游 tag object `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`，peeled commit `43eaf07355af145aebfee301801779e824d4c221`，merge base `28529620f438b2ed25c812f6364825d846a4a9d6`。
- 两父 merge commit `f4462765e9b3a2efd1deb13aabf3ce349166a058` 的第二父为上述 peeled commit。
- 产品版本事实源及两个本地 lock package 为 `0.3.0`；annotated `v0.3.0` tag object 是 `e6706d4bdc33a184cf641204574df1fc2962ca4c`，peeled/source commit 是 `bde1370bbaffd345c3d9875708615eaf96140591`。
- Node `24.19.0`、pnpm `10.12.3`、Rust `1.97.1`、Python `3.14.7`、lock 解析 uv `0.12.2`；本地原生执行只证明 Linux x64，Windows/Linux/macOS 发布结果来自匹配的 Actions native runner。
- 自动 CI、trusted-base Labeler、稳定 `CI / Required`、无签名五目标 Release、10 installer/13 attachment、mandatory attestation 和 draft→stable publish transaction 不仅通过本地静态/行为合同，也由真实 PR/main/preflight/formal 运行验证。
- `cross-fetch` 旧链路、polyfill import 与 warning suppression 已退出；Native Fetch→MSW→Tauri mock 四类行为已验证。
- D116 宿主原生本地边界已由共享 runtime planner 执行：canonical package/mise Tauri 与 Cargo 入口核对 process host 及绝对 rustc/rustdoc 身份，拒绝 caller compiler/wrapper/runner/linker/target、target-specific flags、loader/runtime injection 与 forwarded argv，扫描 Cargo config/includes 后显式 own 当前宿主 child env 与 target；Cargo test 的 no-shell runner 还核对原生格式和机器架构。D117 已在 PR/main/preflight/formal 四条远程证据链上实际使用：同步 whole-run wait 至 `completed`，再一次读取最终结果，没有后台/异步监控或频繁轮询。

### 当前本地清理证据

- 为 Windows 诊断启动的精确 cargo/rustc 进程已停止，显式诊断临时目录已删除；当前进程复核未发现遗留 cargo/rustc 编译进程。
- `src-tauri/target/app` 与 `target/installer-actions` 已清理；清理前记录占用量分别为 4.1 GiB 和 57.4 MiB，当前路径复核均为不存在。
- `rustup target list --installed` 当前只返回 `x86_64-unknown-linux-gnu`。
- 先前本地 Windows Light/MSI 运行只用于诊断，不引用为 Windows x64/ARM64 package、ICE、MSI 结构或生命周期验收。有效证据是 PR/main 的 x64/ARM64 native MSI query fixtures 以及 preflight/formal 的匹配 Windows native build/verify jobs。

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
- **PASS**：决策登记与追踪矩阵严格唯一连续 1–118、owned current docs 相对链接、canonical local cross-marker 与 active async/polling 反向扫描；
- **PASS**：owned code/docs targeted Prettier check 与工作树 whitespace diff check。

### Closeout 首批写集本地验证

- **PASS**：`ciWorkflow.test.ts` 9/9，覆盖 exact-seven Required topology、
  Windows x64/ARM64 runner matrix、锁定 setup-uv/Python、native platform
  断言、Trellis JSON smoke 与 MSI fixture 顺序；
- **PASS**：当前 Linux x64 宿主通过相同的
  `uv run --locked --no-sync python .trellis/scripts/task.py list --json`
  协议返回 4 个活动 task；该结果只证明协议，不冒充 Windows 原生证据；
- **PASS**：`mise run release:check` 最终为 16 files / 231 tests，Native
  Fetch 4/4；首轮所有 15 files / 224 tests 已通过后 Vitest worker 出现一次
  `ENODATA` unhandled read，单次有界复跑完整通过，未修改代码或弱化门禁；
- **PASS**：`mise run check` 完整退出 0，包括 142 files / 1088 frontend
  tests、Linux x64 Rust fmt/check/strict Clippy/tests、task/docs/lock/hook、
  version 与 release contracts；未执行非宿主 target；
- **PASS**：typecheck、format、80-task validation、task docs byte-check、
  `version:check --tag v0.3.0`、四个活动 task 的 Trellis validation、42-file
  whitespace diff、36 个变更 Markdown 的 21 个相对链接以及 D1–D118/
  traceability 连续唯一性；
- **REMOTE VERIFIED**：本机没有独立 `actionlint` 可执行文件，本地也未替代
  Windows 原生执行；closeout PR 的完整 run `31265504901` 已由 GitHub 接受并使
  Windows x64/ARM64 两腿与 aggregate Required 全部成功。

## 4. 远程 CI 与发布证据

| 节点                          | 结果     | 证据                                                                                                                                                                  |
| ----------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D118 implementation PR        | Merged   | [PR #7](https://github.com/NongHua123/fyagent/pull/7)；最终 head `d8c26b70c83fa6e4286d02549c0c383db4f5a318`                                                           |
| PR `CI / Required`            | Success  | [run `31258884239`](https://github.com/NongHua123/fyagent/actions/runs/31258884239)；Windows Installer Query x64/ARM64 native fixtures 成功                           |
| implementation merge          | Success  | merge commit/source `bde1370bbaffd345c3d9875708615eaf96140591`                                                                                                        |
| main `CI / Required`          | Success  | [run `31259389682`](https://github.com/NongHua123/fyagent/actions/runs/31259389682)；formal metadata 绑定该 run attempt 1 的唯一 `CI / Required`                      |
| exact-main unsigned preflight | Success  | [run `31259905022`](https://github.com/NongHua123/fyagent/actions/runs/31259905022)；Windows x64/ARM64、Linux x64/ARM64、macOS Universal 全成功，publish skipped      |
| annotated tag                 | Verified | object `e6706d4bdc33a184cf641204574df1fc2962ca4c`；peeled commit 与 source/main 相同                                                                                  |
| formal Release                | Success  | [run `31260931509`](https://github.com/NongHua123/fyagent/actions/runs/31260931509)；eligibility、五 native targets、verify、attest、publish 全成功                   |
| public Release                | Released | [v0.3.0](https://github.com/NongHua123/fyagent/releases/tag/v0.3.0)；Release ID `367220197`；`draft=false`、`prerelease=false`、latest；发布于 `2026-08-08T14:24:16Z` |

Closeout [PR #8](https://github.com/NongHua123/fyagent/pull/8) 的原生证据链为：

- 首次 [run `31264604075`](https://github.com/NongHua123/fyagent/actions/runs/31264604075)，head `623b6924e3b8682321b26aa69c15dc6f0b9f6f09`：[x64 `93120609402`](https://github.com/NongHua123/fyagent/actions/runs/31264604075/job/93120609402) 成功，[ARM64 `93120609411`](https://github.com/NongHua123/fyagent/actions/runs/31264604075/job/93120609411) 与 [Required `93121912798`](https://github.com/NongHua123/fyagent/actions/runs/31264604075/job/93121912798) 失败；
- commit [`4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd`](https://github.com/NongHua123/fyagent/commit/4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd) 修复后，[run `31265504901`](https://github.com/NongHua123/fyagent/actions/runs/31265504901) 的 [x64 `93122857985`](https://github.com/NongHua123/fyagent/actions/runs/31265504901/job/93122857985)、[ARM64 `93122858012`](https://github.com/NongHua123/fyagent/actions/runs/31265504901/job/93122858012) 与 [Required `93123992476`](https://github.com/NongHua123/fyagent/actions/runs/31265504901/job/93123992476) 均成功。

上述已授权 run 均由发起主流程同步等待 whole run 到 `completed`，再一次读取最终 run/job 结果；没有后台/异步 monitor 或频繁轮询。首次 PR run `31258303784` 暴露 empty cleanup accumulator 的 PowerShell 参数绑定根因，修复后才以 `31258884239` 作为成功验收。closeout 首次 run `31264604075` 则证明 version-only `setup-uv` 在 Windows on ARM 选择 `win-amd64`，因此 ARM64 native-platform 断言和 aggregate Required 正确失败关闭；commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` 改用 uv 官方完整 `implementation-version-os-arch-libc` request 并执行 `uv sync --locked --managed-python`，修复 run `31265504901` 才作为 Child 3/D116 成功验收。两次都没有以兼容分支或完整 preflight 重试隐藏问题。

## 5. 13 附件与独立复核

| Attachment                            | Size (bytes) | SHA-256                                                            |
| ------------------------------------- | -----------: | ------------------------------------------------------------------ |
| `FyAgent-0.3.0-macOS.dmg`             |   30,387,916 | `15ef5a55ebb51d87ea05ff5ae8619e41f6ff462dbb0091d5763ccb29a44ea6dc` |
| `FyAgent-0.3.0-macOS.zip`             |   26,779,994 | `3a3994885cea25e4a9fb593afd74b177fe6461baa565d08b51f85b17dbf048f0` |
| `FyAgent-0.3.0-Windows.msi`           |   13,209,600 | `6d9979ced9770159a2f592b7882572802765947b4bcb6027e8a02dbb990ea56f` |
| `FyAgent-0.3.0-Windows-arm64.msi`     |   12,500,992 | `58dd45663a1eb2bee570dec150eaca6706ebfbbfc819c0e4522d07d57c7f572d` |
| `FyAgent-0.3.0-Linux-x86_64.AppImage` |   91,634,168 | `0b070880df1b1f252cfe16685a650992fe427fbca7a7a205d53e49dede38c166` |
| `FyAgent-0.3.0-Linux-x86_64.deb`      |   12,946,262 | `b1040048e3eeea4345fd2f5c819640d2b62486a9c52c9d6a8436cab471bc380a` |
| `FyAgent-0.3.0-Linux-x86_64.rpm`      |   12,946,280 | `55330c1ebaed5fe57310b36c9762e50b01175725b74d93d8a1af1af4a8621541` |
| `FyAgent-0.3.0-Linux-arm64.AppImage`  |   89,201,160 | `110b7f8dc5d140cf2f39342ce0a64d7c1b8c05e80ab0fa274261776b7ea7251e` |
| `FyAgent-0.3.0-Linux-arm64.deb`       |   12,386,266 | `7f56b30eaa7b58065b7e0a500c647ae334a0c4d2293238dfa038f4c7676def3a` |
| `FyAgent-0.3.0-Linux-arm64.rpm`       |   12,383,557 | `1083470e31db398b94944ac9319656a83d50bcd3af147a1592638f12ccc2c843` |
| `download-manifest.json`              |        3,866 | `d1d81b973aea506d369e21b385ee60b993b88121b946cf68dc254e825b4abea1` |
| `build-metadata.json`                 |        4,015 | `7ae0631b77059d05a8866ec9602f8afc7f8493a092f6c32a3e4a161a3fc98079` |
| `artifact-attestation.sigstore.json`  |       12,683 | `4802f1e9b5eca3eb0cc2a03530b86057d79e9d5828a97615d8ea5e5430ce0576` |

独立复核重新下载了全部 13 附件，exact allowlist 通过；`download-manifest.json` 中的 10 个 installer 文件名、尺寸、SHA-256 和 URL 与实际下载 bytes 一致。`build-metadata.json` 绑定 `.github/workflows/ci.yml`、run `31259389682` attempt 1、`CI / Required`、success，且 source/workflow/formal identity 与 `bde1370...` 一致。

独立验证从 GitHub CLI 官方 Release 取得 v2.97.0 Linux amd64 二进制并核对官方 checksums，再使用下载的本地 bundle、`--repo NongHua123/fyagent`、`--signer-workflow NongHua123/fyagent/.github/workflows/release.yml`、`--source-digest bde1370bbaffd345c3d9875708615eaf96140591`、`--source-ref refs/tags/v0.3.0` 和 `--deny-self-hosted-runners` 验证了全部 12 个 subjects。临时 Release 下载和 CLI 目录在复核后已删除且确认不存在。Windows/macOS 无签名状态来自匹配 native formal jobs 的失败关闭验证，没有在 Linux 本地跨 OS 重放。

## 6. 已接受的治理决策与剩余门禁

- **Confirmed — D113 preflight order**：实际顺序已按 merge → main CI → exact-main-SHA preflight → tag → formal Release 完成；每个节点都有独立真实证据。
- **Accepted verification exception — D114 merge_group**：个人账户仓库且不配置 branch protection/ruleset，GitHub Merge Queue 无法产生 live `merge_group`。该项仍为 N/A，不是成功；YAML trigger、失败关闭合同/静态测试和真实 PR/main/manual 替代证据已完整。
- **Verified — D116 host-native-only local execution**：本地 canonical 入口仍只针对当前宿主；五个发布目标的非宿主证据全部来自匹配 native Actions jobs，closeout x64/ARM64 locked uv/Python/Trellis smoke 也由 run `31265504901` 的匹配 runner 完成。
- **Confirmed — D117 synchronous Actions observation**：PR/main/preflight/formal 及 closeout 首败/修复 run 均按同步 whole-run wait → 一次最终读取执行；未使用后台/异步 monitor 或频繁轮询。
- **Verified — D118 shift-left engineering**：PR/main x64/ARM64 native MSI query fixtures 通过，exact-main preflight 和 formal 五目标紧随成功；未选择兼容分支隐藏未知根因。
- **Residual risk**：main/tag 无 ruleset/branch protection，Release 无 environment；该 workflow-only 风险仍按 D109 接受，不宣称管理员保护。

v0.3.0 发布门禁的结论是 **Released / Verified**。仓库级 closeout 的 `Windows Native Contracts` x64/ARM64 gate 也已由 run `31265504901` 远程验证；`MANIFEST.sha256` 已在设计包字节冻结后确定性重建并复验。当前在同一 PR 按 Child 3 → Child 4 → Child 6 → parent 归档，随后记录 journal。该完整系列还必须通过 final PR CI 才能 merge；exact-main CI 复验成功后才清理除 `main` 外的本地/origin 分支。

## 7. 原始设计包验证记录

2026-08-07 的只读输入包曾通过 134 个非 manifest 文件/135 个含 manifest 文件、JSON/JSONL 语法、1 parent+6 child overlay、decision 1–104、无 symlink/secrets 等静态检查。该记录只证明原始文档包，不证明当前修订字节或任何工程实施。closeout `MANIFEST.sha256` 已在最终文档字节稳定后重新生成，并以 134 个非 manifest 普通文件、路径全集一致、0 symlink 和逐项 `sha256sum -c` 通过重新建立当前完整性证据。
