# 外部来源登记

> **交付状态**：Observed / 已核实  
> **关联决策**：7–19、23–34、39–42、57–65、81–87、101  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

技术问题优先采用官方文档、官方发布页和上游仓库。本文档不复制长篇上游内容。

| # | 来源 | 支撑事实 | 访问日期 |
|---:|---|---|---|
| 1 | [Node.js 24.19.0 release](https://nodejs.org/en/blog/release/v24.19.0) | Node 24.19.0 LTS 版本与发布日期。 | 2026-08-07 |
| 2 | [Node.js release schedule](https://nodejs.org/en/about/previous-releases) | Node LTS/EOL 状态。 | 2026-08-07 |
| 3 | [Node deprecations](https://nodejs.org/api/deprecations.html#dep0040-nodepunycode-module) | DEP0040、application/runtime deprecation 与 pending/throw 行为。 | 2026-08-07 |
| 4 | [Node globals / Fetch](https://nodejs.org/api/globals.html#fetch) | 原生 Fetch API 状态。 | 2026-08-07 |
| 5 | [Rust 1.97.1](https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/) | Rust 1.97.1 和 LLVM miscompilation 修复。 | 2026-08-07 |
| 6 | [Python 3.14.7](https://www.python.org/downloads/release/python-3147/) | Python 3.14.7 维护版本。 | 2026-08-07 |
| 7 | [uv releases](https://github.com/astral-sh/uv/releases) | uv 当前版本/平台资产。 | 2026-08-07 |
| 8 | [uv projects](https://docs.astral.sh/uv/guides/projects/) | pyproject/.python-version/.venv/uv.lock 项目模型。 | 2026-08-07 |
| 9 | [uv Python versions](https://docs.astral.sh/uv/concepts/python-versions/) | uv managed Python。 | 2026-08-07 |
| 10 | [uv CLI](https://docs.astral.sh/uv/reference/cli/) | --locked、--no-sync、--offline 等。 | 2026-08-07 |
| 11 | [uv settings](https://docs.astral.sh/uv/reference/settings/) | python-preference、python-downloads、required-version。 | 2026-08-07 |
| 12 | [mise tasks](https://mise.jdx.dev/tasks/) | mise task 运行环境与任务模型。 | 2026-08-07 |
| 13 | [mise task configuration](https://mise.jdx.dev/tasks/task-configuration.html) | depends、usage、confirm、includes 等。 | 2026-08-07 |
| 14 | [mise lock](https://mise.jdx.dev/dev-tools/mise-lock.html) | latest selector 与 lockfile 精确解析。 | 2026-08-07 |
| 15 | [mise trust](https://mise.jdx.dev/cli/trust.html) | 用户显式信任边界。 | 2026-08-07 |
| 16 | [mise settings](https://mise.jdx.dev/configuration/settings.html) | auto_install、lockfile、idiomatic files。 | 2026-08-07 |
| 17 | [GitHub runner images](https://github.com/actions/runner-images) | 明确 runner 标签、latest 迁移和更新频率。 | 2026-08-07 |
| 18 | [macOS 14 deprecation](https://github.com/actions/runner-images/issues/13518) | macOS 14 runner 退役计划。 | 2026-08-07 |
| 19 | [Ubuntu 22 deprecation](https://github.com/actions/runner-images/issues/14254) | Ubuntu 22 runner 退役计划。 | 2026-08-07 |
| 20 | [GitHub secure use](https://docs.github.com/en/actions/reference/security/secure-use) | 完整 action SHA 与最小权限。 | 2026-08-07 |
| 21 | [GitHub merge_group event](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#merge_group) | Merge Queue required workflow 事件。 | 2026-08-07 |
| 22 | [GitHub job containers](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/run-jobs-in-a-container) | Linux job 容器。 | 2026-08-07 |
| 23 | [GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations) | provenance/attestation。 | 2026-08-07 |
| 24 | [Tauri AppImage](https://v2.tauri.app/distribute/appimage/) | 旧 glibc 基线、容器/Actions 建议、ARM 原生构建。 | 2026-08-07 |
| 25 | [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) | 各宿主原生开发依赖。 | 2026-08-07 |
| 26 | [actions/setup-node](https://github.com/actions/setup-node) | node-version-file 和缓存能力。 | 2026-08-07 |
| 27 | [pnpm/action-setup](https://github.com/pnpm/action-setup) | packageManager 版本解析。 | 2026-08-07 |
| 28 | [CC Switch v3.19.2](https://github.com/farion1231/cc-switch/releases/tag/v3.19.2) | 上游 tag、短 SHA、发布日期、无 schema migration 声明；公开 commit 页面确认完整 SHA `43eaf07355af145aebfee301801779e824d4c221`。 | 2026-08-07 |
| 29 | [CC Switch repository](https://github.com/farion1231/cc-switch) | 上游仓库身份和 MIT 来源。 | 2026-08-07 |
| 30 | [Git merge](https://git-scm.com/docs/git-merge) | --no-ff、--no-commit 行为。 | 2026-08-07 |
| 31 | [Git remote](https://git-scm.com/docs/git-remote) | fetch/push URL 检查。 | 2026-08-07 |
| 32 | [pnpm why](https://pnpm.io/cli/why) | 反向依赖图。 | 2026-08-07 |
| 33 | [pnpm install](https://pnpm.io/cli/install) | --frozen-lockfile。 | 2026-08-07 |
| 34 | [Cargo check](https://doc.rust-lang.org/cargo/commands/cargo-check.html) | --locked。 | 2026-08-07 |

## 使用规则

- 版本/runner/Action 等会变化的信息在实施 PR 中重新验证；
- Action 完整 SHA、容器 digest、上游完整 40 位 SHA 不能从本设计包的短标签推断；
- 若官方资料与本设计发生变化，以实施时官方资料为输入，并通过新的决策/PR 记录偏差。
