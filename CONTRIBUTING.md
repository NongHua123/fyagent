# Contributing to FyAgent

> [中文版本](#贡献指南)

Thank you for your interest in contributing to FyAgent! Please read our [Code of Conduct](./CODE_OF_CONDUCT.md) before participating.

## How to Contribute

There are many ways to contribute:

- **Report bugs** — Found something broken? [Open a bug report](https://github.com/NongHua123/fyagent/issues/new?template=bug_report.yml).
- **Suggest features** — Have an idea? [Submit a feature request](https://github.com/NongHua123/fyagent/issues/new?template=feature_request.yml).
- **Improve docs** — Spot a typo or missing info? [Report a doc issue](https://github.com/NongHua123/fyagent/issues/new?template=doc_issue.yml).
- **Contribute code** — Fix bugs or implement features via pull requests.
- **Translate** — Help us improve English, Simplified Chinese, Traditional Chinese, and Japanese translations.

> **Security vulnerabilities**: Please do NOT use public issues. See our [Security Policy](./SECURITY.md) instead.

## Development Setup

### Prerequisites

- [mise](https://mise.jdx.dev/getting-started.html) 2026.8.0 or newer,
  installed globally
- [Tauri 2.0 prerequisites](https://v2.tauri.app/start/prerequisites/)

The repository's `mise.toml` is the single source of truth for Node.js, pnpm,
Python, and Rust/Cargo versions. Tauri CLI is installed as a project dependency.
After reviewing the config, trust it once and install the pinned tools:

```bash
mise trust
mise install
```

The examples below use `mise exec --` so they also work without shell
activation. You may omit that prefix in an intentionally mise-activated shell.

### Quick Start

```bash
# Install dependencies
mise exec -- pnpm install --frozen-lockfile

# Start development server with hot reload
mise exec -- pnpm dev
```

### Useful Commands

| Command | Description |
|---------|-------------|
| `mise exec -- pnpm dev` | Start dev server (hot reload) |
| `mise exec -- pnpm build` | Production build |
| `mise exec -- pnpm typecheck` | TypeScript type checking |
| `mise exec -- pnpm test:unit` | Run unit tests |
| `mise exec -- pnpm format` | Format code (Prettier) |
| `mise exec -- pnpm format:check` | Check code formatting |

For Rust backend:

```bash
mise exec -- cargo fmt --manifest-path src-tauri/Cargo.toml
mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml
```

## Code Style

- **Frontend**: Prettier for formatting and strict TypeScript (`mise exec -- pnpm typecheck`)
- **Backend**: `mise exec -- cargo fmt` for formatting, `mise exec -- cargo clippy` for linting
- **Tauri 2.0**: Command names must use camelCase

Run all checks before submitting:

```bash
mise exec -- pnpm typecheck
mise exec -- pnpm format:check
mise exec -- pnpm test:unit
mise exec -- cargo fmt --check --manifest-path src-tauri/Cargo.toml
mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml
```

## Pull Request Guidelines

1. **Open an issue first** for new features — PRs for features that are not a good fit may be closed.
2. **Fork and branch** — Create a feature branch from `main` (e.g., `feat/my-feature` or `fix/issue-123`).
3. **Keep PRs focused** — One feature or fix per PR. Avoid unrelated changes.
4. **Follow the PR template** — Fill in the summary, related issue, and checklist.

### PR Checklist

- [ ] `mise exec -- pnpm typecheck` passes
- [ ] `mise exec -- pnpm format:check` passes
- [ ] `mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` passes (if Rust code changed)
- [ ] Updated i18n files if user-facing text changed

### Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(provider): add support for new provider
fix(tray): resolve menu not updating after switch
docs(readme): update installation instructions
ci: add format check workflow
chore(deps): update dependencies
```

## AI-Assisted Contributions

We welcome AI-assisted contributions, but **the responsibility stays with you**. AI tools lower the cost of writing code — they do not lower the cost of reviewing it. Maintainers are not obligated to clean up AI-generated output.

By submitting a PR, you agree to the following:

1. **You have read and understood your code.** You must be able to explain any line in your PR. If you cannot, it is not ready for review.
2. **You have tested it yourself.** Every change must be verified locally — not just "it looks right." Do not submit code for platforms or features you cannot test.
3. **PRs must be small and focused.** One issue, one PR. Large, sprawling, multi-topic PRs will be closed.
4. **Open an issue first.** Drive-by PRs with no prior discussion — especially AI-generated ones — may be closed without review.
5. **Maintainers may close without explanation.** PRs that appear to be unreviewed AI output — hallucinated fixes, unnecessary refactors, bulk changes with no context — may be closed at the maintainer's discretion.

**In short**: AI is a tool, not a substitute for understanding. Use it to help you contribute better, not to shift work onto maintainers.

## Licensing and contribution rights

By submitting a contribution, you represent that you have the right to
authorize your own contribution under this repository's licensing model. Do
not submit code, assets, or data under incompatible terms. Identify the source
and license of any third-party material included with a contribution.

FyAgent plans to offer commercial licensing for FyAgent-owned code. Until a
legally reviewed and deployed contributor license agreement or explicit
relicensing process exists, a pull request does not automatically transfer
copyright. Maintainers should not merge substantial external contributions
that would affect commercial licensing capacity until that process exists.

These requirements do not remove or alter the attribution or MIT licensing of
CC Switch-derived portions, including the original attribution to Jason Young.

## Internationalization (i18n)

FyAgent maintains four locale resources. When modifying user-facing text:

1. Update **all four** locale files:
   - `src/i18n/locales/en.json`
   - `src/i18n/locales/ja.json`
   - `src/i18n/locales/zh.json`
   - `src/i18n/locales/zh-TW.json`
2. Use the `t()` function from i18next for all UI text.
3. Never hardcode user-facing strings.

## Questions?

- [Open a question](https://github.com/NongHua123/fyagent/issues/new?template=question.yml)
- [GitHub Discussions](https://github.com/NongHua123/fyagent/discussions)

---

# 贡献指南

> [English Version](#contributing-to-fyagent)

感谢你对 FyAgent 的贡献兴趣！参与之前请阅读我们的[行为准则](./CODE_OF_CONDUCT.md)。

## 如何贡献

你可以通过多种方式参与贡献：

- **报告 Bug** — 发现问题？[提交 Bug 报告](https://github.com/NongHua123/fyagent/issues/new?template=bug_report.yml)。
- **建议功能** — 有想法？[提交功能请求](https://github.com/NongHua123/fyagent/issues/new?template=feature_request.yml)。
- **改进文档** — 发现错误或缺失？[报告文档问题](https://github.com/NongHua123/fyagent/issues/new?template=doc_issue.yml)。
- **贡献代码** — 通过 Pull Request 修复 Bug 或实现新功能。
- **翻译** — 帮助改进英文、简体中文、繁体中文和日文翻译。

> **安全漏洞**：请不要使用公开 Issue 报告。请参阅我们的[安全策略](./SECURITY.md)。

## 开发环境搭建

### 前提条件

- 全局安装 [mise](https://mise.jdx.dev/getting-started.html) 2026.8.0 或更高版本
- [Tauri 2.0 开发环境](https://v2.tauri.app/start/prerequisites/)

仓库中的 `mise.toml` 是 Node.js、pnpm、Python 和 Rust/Cargo 版本的单一事实源；
Tauri CLI 作为项目依赖安装。检查配置后，信任一次并安装固定的开发工具：

```bash
mise trust
mise install
```

下列示例使用 `mise exec --`，未配置 shell 激活时也能使用仓库固定的工具。
若当前 shell 已明确激活 mise，可以省略该前缀。

### 快速开始

```bash
# 安装依赖
mise exec -- pnpm install --frozen-lockfile

# 启动开发服务器（热重载）
mise exec -- pnpm dev
```

### 常用命令

| 命令 | 说明 |
|------|------|
| `mise exec -- pnpm dev` | 启动开发服务器（热重载） |
| `mise exec -- pnpm build` | 构建生产版本 |
| `mise exec -- pnpm typecheck` | TypeScript 类型检查 |
| `mise exec -- pnpm test:unit` | 运行单元测试 |
| `mise exec -- pnpm format` | 格式化代码（Prettier） |
| `mise exec -- pnpm format:check` | 检查代码格式 |

Rust 后端命令：

```bash
mise exec -- cargo fmt --manifest-path src-tauri/Cargo.toml
mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml
```

## 代码规范

- **前端**：使用 Prettier 格式化和严格 TypeScript（`mise exec -- pnpm typecheck`）
- **后端**：使用 `mise exec -- cargo fmt` 格式化、`mise exec -- cargo clippy` 检查
- **Tauri 2.0**：命令名必须使用 camelCase

提交前运行所有检查：

```bash
mise exec -- pnpm typecheck
mise exec -- pnpm format:check
mise exec -- pnpm test:unit
mise exec -- cargo fmt --check --manifest-path src-tauri/Cargo.toml
mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml
```

## Pull Request 指南

1. **先开 Issue 讨论** — 新功能请先开 Issue，不适合项目方向的 PR 可能会被关闭。
2. **Fork 并创建分支** — 从 `main` 创建功能分支（如 `feat/my-feature` 或 `fix/issue-123`）。
3. **保持 PR 专注** — 每个 PR 只做一件事，避免无关改动。
4. **遵循 PR 模板** — 填写概述、关联 Issue 和检查清单。

### PR 检查清单

- [ ] `mise exec -- pnpm typecheck` 通过
- [ ] `mise exec -- pnpm format:check` 通过
- [ ] `mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` 通过（如修改了 Rust 代码）
- [ ] 如修改了用户可见文本，已更新国际化文件

### 提交信息规范

我们使用 [Conventional Commits](https://www.conventionalcommits.org/)：

```
feat(provider): add support for new provider
fix(tray): resolve menu not updating after switch
docs(readme): update installation instructions
ci: add format check workflow
chore(deps): update dependencies
```

## AI 辅助贡献

我们欢迎 AI 辅助的贡献，但**责任始终在你身上**。AI 工具降低了写代码的成本，但并没有降低 review 的成本。维护者没有义务替你清理 AI 的产出。

提交 PR 即表示你同意以下规则：

1. **你已阅读并理解了你的代码。** 你必须能解释 PR 中的每一行。如果做不到，说明还没准备好提交 review。
2. **你已亲自测试过。** 每个改动都必须在本地验证——而不是"看起来对"。不要提交你自己无法测试的平台或功能的代码。
3. **PR 必须小而聚焦。** 一个 Issue 对应一个 PR。大而散、跨多个主题的 PR 会被直接关闭。
4. **先开 Issue 讨论。** 没有事先讨论的"路过式 PR"——尤其是 AI 生成的——可能会被直接关闭。
5. **维护者可以直接关闭。** 看起来是未经审阅的 AI 产出的 PR——虚构的修复、不必要的重构、缺乏上下文的批量改动——维护者可自行决定关闭。

**一句话总结**：AI 是工具，不是理解力的替代品。用它来帮助你更好地贡献，而不是把工作转移给维护者。

## 许可与贡献权利

提交贡献即表示你确认有权按本仓库的许可模式授权你自己的贡献。请勿提交采用不兼容条款的
代码、资产或数据；贡献中包含第三方材料时，请说明其来源和许可证。

FyAgent 计划为 FyAgent 自有代码提供商业许可。在经过法律审查并部署贡献者许可协议或明确的
再许可流程之前，Pull Request 不会自动转让版权。维护者在该流程建立前不应合并会影响商业许可
能力的重大外部贡献。

这些要求不会移除或改变 CC Switch 衍生部分的署名或 MIT 许可，包括对原作者 Jason Young 的
署名。

## 国际化（i18n）

FyAgent 维护四份 locale 资源。修改用户可见文本时：

1. **同时更新四份**语言文件：
   - `src/i18n/locales/en.json`
   - `src/i18n/locales/ja.json`
   - `src/i18n/locales/zh.json`
   - `src/i18n/locales/zh-TW.json`
2. 所有 UI 文本使用 i18next 的 `t()` 函数。
3. 不要硬编码用户可见的字符串。

## 有疑问？

- [提问](https://github.com/NongHua123/fyagent/issues/new?template=question.yml)
- [GitHub 讨论区](https://github.com/NongHua123/fyagent/discussions)
