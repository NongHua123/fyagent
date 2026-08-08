# Contributing to FyAgent

Thank you for contributing. FyAgent is derived from CC Switch and maintains its
own product identity, licensing boundary, development toolchain, and release
process. Read the relevant Trellis specs before changing an owned contract.

## Ways to Contribute

- report reproducible defects or security concerns;
- improve provider/CLI/platform compatibility;
- add tests, translations, accessibility, or documentation;
- implement an approved Trellis task;
- review upstream CC Switch releases and propose a controlled merge.

Security reports should follow the repository security policy rather than a
public issue when disclosure would expose users.

## Development Setup

### Prerequisites

- Git;
- one global [mise](https://mise.jdx.dev/) installation meeting the repository
  minimum feature version;
- current-host native Tauri prerequisites reported by `system:check`;
- Git LFS where visual assets require it.

The repository declares Node, pnpm, Rust, uv, and uv-managed Python. Do not
substitute arbitrary system runtimes.

### First Setup

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

`mise trust` is a user security decision and is never run automatically by a
project script. `bootstrap` does not install privileged OS packages or change
Git remotes.

### Before a Pull Request

```bash
mise run check
```

This is the complete check for the current host. GitHub Required CI remains the
multi-platform merge authority.

Useful focused tasks:

```bash
mise run typecheck
mise run format:check
mise run test:unit
mise run test:i18n
mise run test:desktop:mock
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
mise run release:check
```

See `docs/fyagent/development/mise-tasks.md` for the canonical task catalog.
Do not document project work through `mise` 的旧式 exec 包装, direct pnpm/Cargo commands,
or system Python. GitHub Actions is the explicit exception because it does not
install mise.

## Local Build Boundary

```bash
mise run build:renderer
mise run build:binary
mise run build
mise run build:debug
```

These tasks build only the current host OS/architecture. Formal Release assets
are produced only by GitHub Actions. Local Linux/WSL-to-Windows or macOS
cross-builds are unsupported.

## Code and Test Expectations

- Preserve strict TypeScript and Rust warnings-as-errors at their owned gates.
- Add or update tests for observable behavior and failure paths.
- Use MSW/fakes for external/network/Tauri boundaries unless native evidence is
  specifically required.
- User-visible text must update all registered locales in one change.
- Preserve accessibility roles, keyboard/focus behavior, labels, and error text.
- Do not suppress Node deprecation warnings. Tests use Node native Fetch; do not
  reintroduce `cross-fetch/polyfill`.
- Modifying tasks such as formatting, icon generation, version apply, dependency
  updates, and visual-baseline updates must be invoked explicitly and reviewed.

## Trellis Workflow

Complex work requires a Trellis PRD/design/implementation plan and explicit
approval before implementation. Project Trellis operations use:

```bash
mise run trellis:context
mise run trellis:task -- <subcommand> [args]
mise run trellis:session:add -- [args]
```

A new durable engineering rule requires updating the owning `.trellis/spec/**`
file and its enforcing test. Do not rewrite historical design packages to make
old decisions look current.

## Upstream CC Switch Changes

`origin` is FyAgent's writable repository; `upstream` is read-only. Formal tag
merges use the `upstream:*` tasks, preserve a merge commit, record the full tag
SHA and MIT provenance, and stop before an automated commit or push. Conflict
resolution must preserve FyAgent identity and reviewed engineering contracts.

## Pull Request Requirements

A pull request should include:

- the problem and chosen solution;
- linked issue/Trellis task;
- exact tests and platform evidence;
- risk and rollback;
- docs/spec changes for durable contracts;
- upstream tag/SHA and conflict notes when applicable;
- Release asset/permission implications when applicable.

Use focused commits. The upstream merge commit, cross-build cleanup, toolchain,
CI/Release, dependency, and documentation changes should remain distinguishable.
Never commit secrets, certificates, API keys, personal `mise.local.*`, `.venv`,
or user data.

## Internationalization

Update all four locale files for user-visible strings and run:

```bash
mise run test:i18n
```

Do not rely on fallback text to hide a missing key.

## Licensing and Contribution Rights

Read `LICENSING.md`, `LICENSE`, and `THIRD_PARTY_NOTICES.md`. Directly merged CC
Switch material retains its MIT origin. FyAgent-owned contributions follow the
repository's published contribution and licensing terms. Do not remove upstream
notices while resolving conflicts.

---

# 贡献 FyAgent

感谢参与 FyAgent。FyAgent 基于 CC Switch 派生，但维护独立的产品身份、许可边界、
开发工具链和发布流程。修改长期工程契约前，请先阅读对应 Trellis spec。

## 可贡献内容

- 报告可复现问题或安全风险；
- 改进 Provider、CLI 和平台兼容；
- 补充测试、翻译、无障碍和文档；
- 实施已获批准的 Trellis task；
- 审查 CC Switch 正式版本并提出受控上游合并。

涉及用户风险的安全问题应按仓库安全策略私下报告。

## 开发环境

### 前置条件

- Git；
- 全局安装一份满足项目最低能力要求的 [mise](https://mise.jdx.dev/)；
- `system:check` 所列的当前宿主 Tauri 原生依赖；
- 视觉资源需要时安装 Git LFS。

Node、pnpm、Rust、uv 和 uv 管理的 Python 均由仓库声明，不要替换为任意系统版本。

### 首次初始化

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

`mise trust` 必须由用户显式执行，项目脚本不会自动信任。`bootstrap` 不安装提权系统包，
也不修改 Git remotes。

### 提交 Pull Request 前

```bash
mise run check
```

该命令是当前宿主的完整本地门禁；多平台合并权威仍是 GitHub Required CI。

常用聚焦任务：

```bash
mise run typecheck
mise run format:check
mise run test:unit
mise run test:i18n
mise run test:desktop:mock
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
mise run release:check
```

完整任务目录见 `docs/fyagent/development/mise-tasks.md`。活动开发文档不要使用
`mise` 的旧式 exec 包装、直接 pnpm/Cargo 或系统 Python；GitHub Actions 因不安装 mise 是明确例外。

## 本地构建边界

```bash
mise run build:renderer
mise run build:binary
mise run build
mise run build:debug
```

这些任务只构建当前宿主系统和架构。正式 Release 资产只能来自 GitHub Actions；
不支持 Linux/WSL 到 Windows 或 macOS 的本地交叉构建。

## 代码与测试要求

- 保持严格 TypeScript 和 Rust 质量门；
- 对可观察行为和失败路径补充测试；
- 外部网络/Tauri 边界优先使用 MSW/fake，除非明确需要原生平台证据；
- 用户可见文本一次性更新全部已注册语言；
- 保持键盘、焦点、角色、标签和错误信息等无障碍行为；
- 禁止抑制 Node 弃用告警；测试使用 Node 原生 Fetch，不得恢复 `cross-fetch/polyfill`；
- 格式化、图标、版本 apply、依赖升级和视觉基线更新均为显式修改型任务。

## Trellis 流程

复杂工作必须先形成 Trellis PRD、设计和实施计划，并在实现前取得明确批准。项目操作入口：

```bash
mise run trellis:context
mise run trellis:task -- <subcommand> [args]
mise run trellis:session:add -- [args]
```

形成长期工程规则时，必须更新对应 `.trellis/spec/**` 和执行测试；不要机械改写历史文档。

## 上游 CC Switch

`origin` 是 FyAgent 可写仓库，`upstream` 只读。正式标签合并使用 `upstream:*` 任务，
保留 merge commit、记录完整 SHA/MIT 来源，并在自动 commit/push 前停止。冲突裁决必须
保护 FyAgent 身份和已批准工程契约。

## Pull Request 要求

PR 应说明问题、方案、任务/Issue、测试与平台证据、风险和回退、文档/spec 更新，
以及适用时的上游 tag/SHA、冲突和 Release 权限/资产影响。提交应聚焦；上游 merge、
构建清理、工具链、CI/Release、依赖和文档改造应保持可区分。

禁止提交 secrets、证书、API Key、个人 `mise.local.*`、`.venv` 或用户数据。

## 国际化

修改用户可见文本时更新四份 locale，并运行：

```bash
mise run test:i18n
```

## 许可与贡献权利

阅读 `LICENSING.md`、`LICENSE` 和 `THIRD_PARTY_NOTICES.md`。直接合并的 CC Switch
代码保持 MIT 来源；FyAgent 自有贡献遵循仓库公布的许可/贡献条款。解决冲突时不得删除上游声明。
