# 逐文件变更矩阵

> **交付状态**：Implemented and released inventory; repository closeout pending / 已实施并发布，仓库收尾待完成
> **关联决策**：1–118
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

本表由原始目标矩阵迁移为当前实施 inventory。A–E 中已有实现 commit 的行表示已执行；PR/main/preflight/formal Release 已由真实运行和附件证据验证。closeout PR 自身的 native smoke、合并、archive/journal 和分支清理仍以 Pending 为准，不能从文件存在推断完成。

## A. 上游合并与来源

| 路径/系统                                  | Action                  | 所有者    | 必须保持的合同                                                                   | 未来证据                                        |
| ------------------------------------------ | ----------------------- | --------- | -------------------------------------------------------------------------------- | ----------------------------------------------- |
| Git graph / integration branch             | 新建显式 merge commit   | Child 1   | `v3.19.2` ancestry、两父提交、merge commit 不混入现代化改造                      | `git show --pretty=raw`、`merge-base`、冲突台账 |
| Git remotes                                | 只读核验                | Child 1   | `origin` 可写；`upstream` fetch 指向 CC Switch、push=`DISABLED`                  | `git remote get-url{,--push}` 日志              |
| `CHANGELOG.md`                             | 更新                    | Child 1/6 | 只记录“合入上游 v3.19.2”及 FyAgent 后续差异，不冒充上游 release notes            | 文档 diff                                       |
| `THIRD_PARTY_NOTICES.md` / licensing files | 更新                    | Child 1   | 新合入 CC Switch 派生代码保留 MIT 来源；FyAgent 混合许可边界不回退               | 许可审查记录                                    |
| `docs/release-notes/v3.19.2-{en,ja,zh}.md` | merge 后由 Child 6 删除 | Child 1/6 | merge commit 忠实合入；后续不作为 FyAgent 发布说明保留                           | 两个独立提交的 diff                             |
| `docs/upstream/cc-switch-v3.19.2.md`       | 新增                    | Child 6   | 记录上游 repo/tag/full SHA/merge SHA/许可/FyAgent 差异，不复制完整 release notes | provenance 文档                                 |

## B. 本地交叉构建清理

| 路径                                             | Action                 | 所有者    | 未来验证                              |
| ------------------------------------------------ | ---------------------- | --------- | ------------------------------------- |
| `scripts/macos-cross/bootstrap-host.sh`          | 删除                   | Child 2   | 负向文件/引用扫描                     |
| `scripts/macos-cross/build-package.sh`           | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/build-universal-dmg.sh`     | 删除                   | Child 2   | 同上；自动 `mise trust --yes` 消失    |
| `scripts/macos-cross/cmake/FindOpenSSL.cmake`    | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/constants.sh`               | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/lib.sh`                     | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/macos-cross-env.sh`         | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/make-dmg.sh`                | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/make_app.py`                | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/preflight.py`               | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/project_metadata.py`        | 删除                   | Child 2   | 同上                                  |
| `scripts/macos-cross/provision-toolchains.sh`    | 删除                   | Child 2   | 自动 trust 负向扫描                   |
| `scripts/macos-cross/tests/test_macos_cross.py`  | 删除                   | Child 2   | 不保留失效测试                        |
| `scripts/macos-cross/udif.magic`                 | 删除                   | Child 2   | 无残留消费者                          |
| `scripts/macos-cross/verify_artifacts.py`        | 删除                   | Child 2   | 无残留消费者                          |
| `scripts/windows-cross/build-windows-msi.sh`     | 删除                   | Child 2   | 无 cargo-xwin/Wine/WiX 本地入口       |
| `tests/macosCrossWorkflow.test.ts`               | 删除并以新合同测试替代 | Child 2/3 | 结构化 lock/task/cross-build 负向合同 |
| `.trellis/spec/backend/wsl-macos-cross-build.md` | 删除活动规范           | Child 6   | backend index 不再引用；Git 历史保留  |
| 旧 cross-build 输出目录/ignore 规则              | 清理                   | Child 2   | 仓库内仅保留真实仍使用项              |

## C. 工具链、mise、uv 与任务 API

| 路径                          | Action                      | 所有者      | 关键合同                                                                                                          |
| ----------------------------- | --------------------------- | ----------- | ----------------------------------------------------------------------------------------------------------------- |
| `.node-version`               | 更新为 `24.19.0`            | Child 3     | Node 唯一版本事实源                                                                                               |
| `package.json#packageManager` | 保持 `pnpm@10.12.3`         | Child 3     | pnpm 唯一版本事实源                                                                                               |
| `rust-toolchain.toml`         | 更新为 `1.97.1`             | Child 3     | `minimal` + rustfmt/clippy；无交叉 target/llvm-tools                                                              |
| `mise.toml`                   | 重构                        | Child 3     | 启用标准版本文件、声明 `uv=latest`、include tasks；不重复 Node/pnpm/Rust/Python                                   |
| `mise.lock`                   | 用批准 mise 版本重新生成    | Child 3     | Node/pnpm/Rust/uv 按工具/平台/options/checksum 结构化验证；无旧 Rust options                                      |
| `.python-version`             | 新增 `3.14.7`               | Child 3     | Python 精确开发基线，仅由 uv 消费                                                                                 |
| `pyproject.toml`              | 新增                        | Child 3     | 非包型开发环境；`requires-python >=3.14,<3.15`；managed-only Python                                               |
| `uv.lock`                     | 新增并由 uv 生成            | Child 3     | Python 依赖锁；普通运行 `--locked`                                                                                |
| `.gitignore`                  | 更新                        | Child 3     | `.venv/`、`mise.local.*`、`mise.*.local.*`                                                                        |
| `.mise/tasks/core.toml`       | 新增并收紧 native wrapper   | Child 3     | bootstrap/env/system/deps；check 首步 guard；build/dev 固定宿主且拒绝转发                                         |
| `.mise/tasks/frontend.toml`   | 新增                        | Child 3     | dev/build/type/format/test 映射                                                                                   |
| `.mise/tasks/rust.toml`       | 新增并收紧 native wrapper   | Child 3     | fmt 不选 target；check/clippy/test locked 且显式固定当前宿主                                                      |
| `.mise/tasks/python.toml`     | 新增                        | Child 3     | uv sync/lock/add/remove/with/tool/run                                                                             |
| `.mise/tasks/trellis.toml`    | 新增                        | Child 3     | 薄包装现有 Python CLI，详细参数由 argparse 所有                                                                   |
| `.mise/tasks/upstream.toml`   | 新增                        | Child 3     | check/fetch/audit/merge:prepare/abort 安全边界                                                                    |
| `.mise/tasks/hooks.toml`      | 新增                        | Child 3     | Codex no-sync/offline wrappers                                                                                    |
| `.mise/tasks/contracts.toml`  | 新增                        | Child 3     | task/docs/toolchain/workflow/release 合同                                                                         |
| `scripts/tasks/*.mjs`         | 新增                        | Child 3/4/5 | `host-native.mjs` 统一六宿主、绝对 rustc/rustdoc、Cargo config 扫描及 compiler/runner/linker/env/argv/target 合同 |
| `package.json#scripts`        | 保留低层 leaf、收紧标准入口 | Child 3     | `pnpm dev/build` 进入 host-native wrapper；`pnpm tauri` 仅 Actions/维护                                           |
| `.codex/hooks.json`           | 已实施                      | Child 3     | 仅引用 `mise run --silent codex:hook:*`；locked/no-sync/offline 与协议测试已通过                                  |

## D. DEP0040

| 路径                                         | Action                             | 所有者  | 关键合同                                                                 |
| -------------------------------------------- | ---------------------------------- | ------- | ------------------------------------------------------------------------ |
| `package.json`                               | 删除 `cross-fetch`                 | Child 5 | 不新增替代 polyfill/direct undici                                        |
| `tests/msw/tauriMocks.ts`                    | 删除 `cross-fetch/polyfill` import | Child 5 | 使用 Node 24 原生 Web API                                                |
| `pnpm-lock.yaml`                             | 用 pnpm 10.12.3 正常重解析         | Child 5 | 旧 `cross-fetch → node-fetch@2.7.0 → whatwg-url@5 → tr46@0.0.3` 路径退出 |
| Native Fetch/MSW probe（实际路径实施时确定） | 新增                               | Child 5 | 真实 request interception + pending/throw deprecation                    |
| Node warning contract script                 | 新增                               | Child 5 | 禁止 suppression；普通 `--throw-deprecation`                             |

## E. GitHub Actions 与仓库级管理配置

| 路径/系统                             | Action                      | 所有者  | 关键合同                                                                                                                                         |
| ------------------------------------- | --------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `.github/workflows/ci.yml`            | 已重写并收紧                | Child 4 | PR/main/merge_group/manual；exact seven Required dependencies；`Windows Native Contracts` x64/ARM64 locked uv/Python/Trellis smoke + MSI fixture |
| `.github/workflows/release.yml`       | 已重写                      | Child 4 | unsigned preflight/formal 两模式、5 target groups、10 installers/13 attachments、mandatory attestation、失败关闭 publish；远程已验证             |
| `.github/workflows/labeler.yml`       | 最小化权限并固定 Action SHA | Child 4 | 只授予必要 PR 权限                                                                                                                               |
| 所有 `.github/workflows/*.yml`        | Action SHA/权限/runner 审计 | Child 4 | 无 `*-latest`、滚动工具链、可移动 action refs、顶层过宽写权限                                                                                    |
| Branch protection / main ruleset      | 明确不配置                  | Parent  | 已接受 workflow-only 残余风险；不得宣称 `CI / Required` 被管理员强制                                                                             |
| Tag ruleset                           | 明确不配置                  | Parent  | workflow 精确限制 `v0.3.0`；正式 tag 发布后不移动/删除是操作政策而非管理员保护                                                                   |
| Release environment / signing secrets | 明确不配置/不引用           | Child 4 | v0.3.0 无签名、公证、staple；未来签名另开任务                                                                                                    |

## F. 活动文档与 Trellis

| 路径                                                        | Action                          | 所有者         | 关键合同                                                                           |
| ----------------------------------------------------------- | ------------------------------- | -------------- | ---------------------------------------------------------------------------------- |
| `README.md`, `README_ZH.md`, `README_JA.md`, `README_DE.md` | 更新开发章节                    | Child 6        | canonical bootstrap/check/native build；不重复完整 task 表                         |
| `CONTRIBUTING.md`                                           | 更新                            | Child 6        | 双语贡献流程、spec/PR/上游/Release 边界                                            |
| `.github/pull_request_template.md`                          | 更新                            | Child 6        | 自适应双语证据清单                                                                 |
| `flatpak/README.md`                                         | 更新                            | Child 6        | 非正式资产边界与 canonical tasks                                                   |
| `tests/e2e/visual-baselines/README.md`                      | 更新                            | Child 6        | preflight/update/evidence 明确分离                                                 |
| `.trellis/workflow.md`                                      | 更新                            | Child 6        | 项目操作使用 `mise run trellis:*`                                                  |
| 项目操作型 `.agents/skills/trellis-*`                       | 更新                            | Child 6        | 不直接调用系统 Python；通用 trellis-meta 不批量改写                                |
| `.agents/skills/fyagent-trellis/SKILL.md`                   | 新增薄入口                      | Child 6        | 只选择 canonical mise/Trellis 路径，不复制通用技能                                 |
| `.trellis/spec/backend/development-environment.md`          | 已由前序 child 重写             | Child 3        | 标准版本文件、mise/uv/Python/locks/strict checks；Child 6 不覆盖                   |
| `.trellis/spec/backend/task-runner-contract.md`             | 已由前序 child 新增             | Child 3        | task API、参数、副作用、弃用、文档生成；Child 6 不覆盖                             |
| `.trellis/spec/backend/upstream-sync.md`                    | 已由前序 child 新增             | Child 1        | fork/upstream、merge、许可和可选 runtime mise；Child 6 不覆盖                      |
| `.trellis/spec/backend/development-hooks.md`                | 已由前序 child 新增             | Child 3        | Trellis/Codex wrapper 与可见降级；Child 6 不覆盖                                   |
| `.trellis/spec/backend/github-release-workflow.md`          | 已由前序 child 重写             | Child 4        | Required/Release 长期合同；Child 6 不覆盖                                          |
| `.trellis/spec/backend/windows-release-boundary.md`         | 已由前序 child 重写             | Child 4        | native MSI、manifest 分层、无签名/安装安全；Child 6 不覆盖                         |
| backend 品牌/版本/config/deeplink specs                     | 状态与 canonical 命令收口       | Child 6        | 保留身份/schema 16/数据路径/深链安全，只迁移 0.3.0 与任务入口                      |
| `.trellis/spec/frontend/quality-guidelines.md`              | Child 5 行为合同 + Child 6 收口 | Child 5/6      | Native Fetch、DEP0040、测试证据边界；Child 6 不重写已实施行为                      |
| backend/frontend `index.md`                                 | 更新                            | Child 6        | 保留所有仍有效规范并加入新规范；移除退役项                                         |
| 新 parent + 6 child tasks                                   | 已物化并按序执行                | Parent/Child 6 | Child1/2/5 已归档；Child3/4/6 与 parent 在 closeout PR native smoke 通过后按序归档 |
| 5 个旧 task 目录                                            | `1d3849e6` 已归档为 superseded  | Child 6        | 原文/原始状态保留、四 child 后旧 parent、不称旧需求已完成                          |
| `docs/fyagent/dev/v1-0.*` 入口                              | 仅加归档声明                    | Child 6        | 历史正文不改写                                                                     |

## G. 明确保留、不应误删

- `src-tauri/src/commands/misc.rs`、`src-tauri/src/codex_config.rs` 中来自上游的可选产品运行时 mise/CLI 兼容，除非真实上游 merge 产生有证据的冲突；
- macOS runner 上的 Universal Binary；
- Windows x64/ARM64、Linux x64/ARM64、macOS Universal 正式目标；
- FyAgent 品牌、bundle/deep-link/data path、`FYAGENT_*`、独立 `0.3.0` 产品版本和混合许可边界；
- 历史 Trellis/版本化设计正文和 Git ancestry。
