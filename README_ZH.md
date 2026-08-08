<div align="center">

# FyAgent

### Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw 和 Hermes Agent 的全方位管理工具

[![Version](https://img.shields.io/github/v/release/NongHua123/fyagent?color=blue&label=version)](https://github.com/NongHua123/fyagent/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/NongHua123/fyagent/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![Downloads](https://img.shields.io/github/downloads/NongHua123/fyagent/total)](https://github.com/NongHua123/fyagent/releases/latest)

<a href="https://www.star-history.com/#NongHua123/fyagent&Date"><picture><source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/badge?repo=NongHua123/fyagent&theme=dark" /><img alt="Star History Rank" src="https://api.star-history.com/badge?repo=NongHua123/fyagent" width="196" height="55" /></picture></a>

### 🌐 项目仓库：**[GitHub](https://github.com/NongHua123/fyagent)**

[English](README.md) | 中文 | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [更新日志](CHANGELOG.md)

</div>

> [!WARNING]
> **FyAgent 0.3.0 的合同是公开、稳定、明确无签名的发布。** Windows
> 可执行文件和 MSI 的 Authenticode 状态为 `NotSigned`，macOS 应用没有
> Developer ID 签名或公证。安装前请阅读
> [v0.3.0 发布说明](docs/release-notes/v0.3.0-zh.md)并核对发布证据。正式
> workflow 尚未完成前，本提示不代表 Release 已经存在。

## 为什么选择 FyAgent？

现代 AI 编程依赖于 Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw 和 Hermes 等工具——但每个工具都有自己的配置格式。切换 API 供应商意味着手动编辑 JSON、TOML 或 `.env` 文件，而在多个工具之间缺乏一个统一管理 MCP, SKILLS 的方式。

**FyAgent** 为你提供一个桌面应用来管理所有支持的 AI 工具。无需手动编辑配置文件，你将获得一个可视化界面，一键将供应商导入应用，一键在不同的供应商之间进行切换，内置 50+ 供应商预设、统一的 MCP, SKILLS 管理以及系统托盘即时切换功能——所有操作都基于可靠的 SQLite 数据库和原子写入机制，保护你的配置不被损坏。

- **一个应用，八个工具** — 在单一界面中管理 Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw 和 Hermes
- **告别手动编辑** — 50+ 供应商预设，包括 AWS Bedrock、NVIDIA NIM 和社区中转服务；一键即可切换
- **统一 MCP, SKILLS 管理** — 一个面板管理 Claude、Codex、Gemini、Grok Build、OpenCode 和 Hermes 的 MCP, SKILLS, 支持双向同步
- **系统托盘快速切换** — 从托盘菜单即时切换供应商，无需打开完整应用
- **云同步** — 通过 Dropbox、OneDrive、iCloud 或 WebDAV 服务器在不同设备之间同步供应商数据
- **跨平台** — 基于 Tauri 2 构建的原生桌面应用，支持 Windows、macOS 和 Linux
- **小工具** - 内置首次安装登录确认、环境诊断、插件扩展同步等多种功能

## 界面预览

|                  主界面                   |                  添加供应商                  |
| :---------------------------------------: | :------------------------------------------: |
| ![主界面](assets/screenshots/main-zh.png) | ![添加供应商](assets/screenshots/add-zh.png) |

## 功能特性

[完整更新日志](CHANGELOG.md) | [v0.3.0 发布说明](docs/release-notes/v0.3.0-zh.md)

### 供应商管理

- **8 个支持工具，50+ 预设** — Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes；复制 key 即可一键导入
- **通用供应商** — 一份配置同步到 Claude Code、Codex 和 Gemini CLI
- 一键切换、系统托盘快速访问、拖拽排序、导入导出

### 代理与故障转移

- **本地代理热切换** — 格式转换、自动故障转移、熔断器、供应商健康监控和整流器
- **应用级代理接管** — 独立为 Claude、Codex、Gemini 或 Grok Build 配置代理，具体到单个供应商

### MCP、Prompts 与 Skills

- **统一 MCP 面板** — 管理 Claude、Codex、Gemini、Grok Build、OpenCode 和 Hermes 的 MCP 服务器，双向同步，支持 Deep Link 导入
- **Prompts** — Markdown 编辑器，跨应用同步（CLAUDE.md / AGENTS.md / GEMINI.md），回填保护
- **Skills** — 从 GitHub 仓库或 ZIP 文件一键安装，自定义仓库管理，支持软连接和文件复制

### 用量与成本追踪

- **用量仪表盘** — 跨供应商追踪支出、请求数和 Token 用量，趋势图表、详细请求日志和自定义模型定价

### 会话管理器与工作区

- 浏览、搜索和恢复支持的会话来源
- **工作区编辑器**（OpenClaw）— 编辑 Agent 文件（AGENTS.md、SOUL.md 等），支持 Markdown 预览

### 系统与平台

- **云同步** — 自定义配置目录（Dropbox、OneDrive、iCloud、坚果云、NAS）及 WebDAV 服务器同步
- **Deep Link** (`fyagent://`) — 通过 URL 一键导入供应商、MCP 服务器、提示词和技能
- 深色 / 浅色 / 跟随系统主题、开机自启、通过 GitHub Releases 手动更新、原子写入、自动备份、国际化（简中/繁中/英/日）

## 常见问题

<details>
<summary><strong>FyAgent 支持哪些 AI 工具？</strong></summary>

FyAgent 支持八个工具：**Claude Code**、**Claude Desktop**、**Codex**、**Gemini CLI**、**Grok Build**、**OpenCode**、**OpenClaw** 和 **Hermes**。每个工具都有专属的供应商预设和配置管理。

</details>

<details>
<summary><strong>切换供应商后需要重启终端吗？</strong></summary>

大多数工具需要重启终端或 CLI 工具才能使更改生效。例外的是 **Claude Code**，它目前支持供应商数据的热切换，无需重启。

</details>

<details>
<summary><strong>切换供应商之后我的插件配置怎么不见了？</strong></summary>

FyAgent 使用“通用配置片段”功能，在不同的供应商之间传递 Key 和请求地址之外的通用数据，您可以在“编辑供应商”菜单的“通用配置面板”里，点击“从当前供应商提取”，把所有的通用数据提取到通用配置中，之后在新建“供应商”的时候，只要勾选“应用通用配置”（默认勾选），就会把插件等数据写入到新的供应商配置中。您的所有配置项都会保存在运行本软件的时候，第一次导入的默认供应商里面，不会丢失。

</details>

<details>
<summary><strong>macOS 安装</strong></summary>

FyAgent 0.3.0 明确不签名、不公证，macOS 可能阻止首次启动。先尝试打开
FyAgent，然后使用 Apple 支持的“**系统设置 → 隐私与安全性 → 仍要打开**”流程并
确认提示。请先核对该版本的发布说明和证据；不要关闭 Gatekeeper，也不要移除隔离属性。

</details>

<details>
<summary><strong>为什么总有一个正在激活中的供应商无法删除？</strong></summary>

本软件的设计原则是“最小侵入性”，即使卸载本软件，也不会影响应用的正常使用。

所以系统总会保留一个正在激活中的配置，因为如果将所有配置全部删除，该应用将无法正常使用。如果你不经常使用某个对应的应用，可以在设置中关掉该应用的显示。如果你想切换回官方登录，可以参考下条。

</details>

<details>
<summary><strong>如何切换回官方登录？</strong></summary>

可以在预设供应商里面添加一个官方供应商。切换过去之后，执行一遍 Log out / Log in 流程，之后便可以在官方供应商和第三方供应商之间随意切换。CodeX 可以在不同官方供应商之间进行切换，方便多个 Plus 或者 Team 账号之间切换。

</details>

<details>
<summary><strong>我的数据存储在哪里？</strong></summary>

- **数据库**：`~/.fyagent/fyagent.db`（SQLite — 供应商、MCP、提示词、技能）
- **本地设置**：`~/.fyagent/settings.json`（设备级 UI 偏好设置）
- **备份**：`~/.fyagent/backups/`（自动轮换，保留最近 10 个）
- **SKILLS**：`~/.fyagent/skills/`（默认通过软链接连接到对应应用）
- **技能备份**：`~/.fyagent/skill-backups/`（卸载前自动创建，保留最近 20 个）

</details>

<details>
<summary><strong>Linux（Wayland + NVIDIA）：网页内容点不动、缩放后黑屏</strong></summary>

AppImage 会强制 `GDK_BACKEND=x11`（走 XWayland）以规避历史上的原生 Wayland 崩溃。但在较新的 Wayland + NVIDIA 环境下，这会导致网页内容区点不动（标题栏按钮仍可点）、窗口缩放后黑屏。可用内置的逃生开关切回原生 Wayland：

```bash
FYAGENT_GDK_BACKEND=wayland ./FyAgent-*.AppImage
```

如果你是从桌面图标启动的，请把它写进 `.desktop` 的 `Exec=` 行（如 `env FYAGENT_GDK_BACKEND=wayland /path/to/AppImage`），或在会话环境中设置。该变量是通用的：在 tiling Wayland 合成器（sway/Hyprland）下若出现点击失效，可反过来设 `FYAGENT_GDK_BACKEND=x11`。不设置则保持默认行为。

</details>

## 文档

如需了解各项功能的详细使用方法，请查阅 **[用户手册](docs/user-manual/zh/README.md)** — 涵盖供应商管理、MCP/Prompts/Skills、代理与故障转移等全部功能。

## 快速开始

### 基本使用

1. **添加供应商**：点击"添加供应商" → 选择预设或创建自定义配置
2. **切换供应商**：
   - 主界面：选择供应商 → 点击"启用"
   - 系统托盘：直接点击供应商名称（立即生效）
3. **生效方式**：重启终端或对应的 CLI 工具以应用更改（CLaude Code 无需重启）
4. **恢复官方登录**：添加"官方登录"预设，重启 CLI 工具后按照其登录/OAuth 流程操作

### MCP、Prompts、Skills 与会话

- **MCP**：点击"MCP"按钮 → 通过模板或自定义配置添加服务器 → 切换各应用同步开关
- **Prompts**：点击"Prompts" → 使用 Markdown 编辑器创建预设 → 激活后同步到 live 文件
- **Skills**：点击"Skills" → 浏览 GitHub 仓库 → 一键安装到支持的应用
- **会话**：点击"Sessions" → 浏览、搜索和恢复支持的会话来源

> **注意**：首次启动可以手动导入现有 CLI 工具配置作为默认供应商。

## 下载安装

### 系统要求

- **Windows**：Windows 10 及以上
- **macOS**：macOS 12 (Monterey) 及以上
- **Linux**：Ubuntu 22.04+ / Debian 11+ / Fedora 34+ 等主流发行版

### Windows 用户

从 [Releases](https://github.com/NongHua123/fyagent/releases) 页面下载：x64 Windows 使用
`FyAgent-0.3.0-Windows.msi`，ARM64 Windows 使用
`FyAgent-0.3.0-Windows-arm64.msi`。FyAgent 不再发布 Windows 绿色版 ZIP；
请使用受支持的全机器 MSI 安装路径。

> **无签名构建：** v0.3.0 可执行文件和 MSI 的 Authenticode 状态明确为
> `NotSigned`，Windows SmartScreen 可能显示警告。继续前请核对资产名称、manifest
> 和 attestation 证据；不要关闭 SmartScreen，也不要削弱组织管理的安全策略。

### macOS 用户

从 [Releases](https://github.com/NongHua123/fyagent/releases) 页面下载
`FyAgent-0.3.0-macOS.dmg`（推荐）或 `FyAgent-0.3.0-macOS.zip`。

> **无签名构建：** v0.3.0 没有 Developer ID 签名或公证。先尝试打开应用，
> 再使用“**系统设置 → 隐私与安全性 → 仍要打开**”并确认提示。请先核对 Release
> 证据；不要关闭 Gatekeeper 或移除隔离属性。

### Linux 用户

从 [Releases](https://github.com/NongHua123/fyagent/releases) 页面下载与当前
架构匹配的原生 Linux 构建：

- x64：`FyAgent-0.3.0-Linux-x86_64.AppImage`、
  `FyAgent-0.3.0-Linux-x86_64.deb` 或
  `FyAgent-0.3.0-Linux-x86_64.rpm`
- ARM64：`FyAgent-0.3.0-Linux-arm64.AppImage`、
  `FyAgent-0.3.0-Linux-arm64.deb` 或
  `FyAgent-0.3.0-Linux-arm64.rpm`

> **Flatpak**：官方 Release 不包含 Flatpak 包。如需使用，可从 `.deb` 自行构建 — 参见 [`flatpak/README.md`](flatpak/README.md)。

<details>
<summary><strong>v0.3.0 精确附件合同</strong></summary>

正式 Release 只允许上述 2 个 macOS 文件、2 个 Windows MSI 和 6 个 Linux
文件，共 **10 个安装资产**。另外只允许 3 个证据附件：包含每个安装资产 SHA-256
的 `download-manifest.json`、`build-metadata.json` 和
`artifact-attestation.sigstore.json`，所以总数必须为 **13**。workflow 会拒绝
缺失、重复、改名或额外文件。在正式 workflow 完成并独立复核前，这些内容只是发布
合同，不是已发布结果。

</details>

<details>
<summary><strong>架构总览</strong></summary>

### 设计原则

```
┌─────────────────────────────────────────────────────────────┐
│                    前端 (React + TS)                         │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │ Components  │  │    Hooks     │  │  TanStack Query  │    │
│  │   （UI）     │──│ （业务逻辑）   │──│   （缓存/同步）    │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
└────────────────────────┬────────────────────────────────────┘
                         │ Tauri IPC
┌────────────────────────▼────────────────────────────────────┐
│                  后端 (Tauri + Rust)                         │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │  Commands   │  │   Services   │  │  Models/Config   │    │
│  │ （API 层）   │──│  （业务层）    │──│    （数据）       │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

**核心设计模式**

- **SSOT**（单一事实源）：所有数据存储在 `~/.fyagent/fyagent.db`（SQLite）
- **双层存储**：SQLite 存储可同步数据，JSON 存储设备级设置
- **双向同步**：切换时写入 live 文件，编辑当前供应商时从 live 回填
- **原子写入**：临时文件 + 重命名模式防止配置损坏
- **并发安全**：Mutex 保护的数据库连接避免竞态条件
- **分层架构**：清晰分离（Commands → Services → DAO → Database）

**核心组件**

- **ProviderService**：供应商增删改查、切换、回填、排序
- **McpService**：MCP 服务器管理、导入导出、live 文件同步
- **ProxyService**：本地 Proxy 模式，支持热切换和格式转换
- **SessionManager**：全应用会话历史浏览
- **ConfigService**：配置导入导出、备份轮换
- **SpeedtestService**：API 端点延迟测量

</details>

<details>
<summary><strong>开发指南</strong></summary>

### 环境要求

- 全局安装 [mise](https://mise.jdx.dev/getting-started.html) 2026.8.0 或更高版本
- 安装 [Tauri 2.0 系统依赖](https://v2.tauri.app/start/prerequisites/)

仓库分别在 `.node-version`、`package.json`、`rust-toolchain.toml` 和
`.python-version` 中固定 Node.js 24.19.0、pnpm 10.12.3、Rust 1.97.1 和
Python 3.14.7。`mise.toml` 管理任务 API 与 uv selector；`mise.lock`、`uv.lock`
以及由 uv 管理的 `.venv` 锁定受批准的 Python 环境。Tauri CLI 随项目依赖安装。

检查仓库配置后，初始化开发环境：

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

`mise trust` 是开发者自己的安全决策，项目任务绝不会自动执行它。`bootstrap` 不安装
提权系统包、不修改 Git remote、不刷新 lock，也不发布。WSL 中禁止从
`/mnt/<drive>` 或 Windows shim 解析受管工具。完整 API 见生成的
[canonical task catalog](docs/fyagent/development/mise-tasks.md)。

### 宿主平台原生构建

本地开发和打包仅支持当前宿主操作系统。标准命令不接受其他操作系统或架构目标：

```bash
mise run dev
mise run build
```

FyAgent 安装包只由 GitHub Actions 在原生 Windows x64/ARM64、Linux x64/ARM64
和 macOS runner 上构建，其中 macOS job 生成 Universal 构建。本地从 Linux/WSL
打包 Windows 或 macOS 不属于受支持的发布路径。

### 开发命令

```bash
# 安装锁定依赖并检查环境
mise run bootstrap

# 开发模式（热重载）
mise run dev

# 类型检查
mise run typecheck

# 代码格式化
mise run format

# 检查代码格式
mise run format:check

# 运行前端单元测试
mise run test:unit

# 监听模式运行测试（推荐开发时使用）
mise run test:unit:watch

# 构建应用
mise run build

# 构建调试版本
mise run build:debug
```

### Rust 后端开发

```bash
# 格式化 Rust 代码
mise run rust:fmt

# 运行 clippy 检查
mise run rust:clippy

# 运行后端测试
mise run rust:test

# 运行特定测试
mise run rust:test test_name

# 提交 PR 前运行完整当前宿主门禁
mise run check
```

### 测试说明

**前端测试**：

- 使用 **vitest** 作为测试框架
- 使用 **MSW (Mock Service Worker)** 模拟 Tauri API 调用
- 使用 **@testing-library/react** 进行组件测试

**运行测试**：

```bash
# 运行所有测试
mise run test:unit

# 监听模式（自动重跑）
mise run test:unit:watch

# 完整前端门禁
mise run check:frontend
```

### 技术栈

**前端**：React 18 · TypeScript · Vite · TailwindCSS 3.4 · TanStack Query v5 · react-i18next · react-hook-form · zod · shadcn/ui · @dnd-kit

**后端**：Tauri 2.8 · Rust · serde · tokio · thiserror · tauri-plugin-process/dialog/store/log

**测试**：vitest · MSW · @testing-library/react

</details>

<details>
<summary><strong>项目结构</strong></summary>

```
├── src/                        # 前端 (React + TypeScript)
│   ├── components/
│   │   ├── providers/          # 供应商管理
│   │   ├── mcp/                # MCP 面板
│   │   ├── prompts/            # Prompts 管理
│   │   ├── skills/             # Skills 管理
│   │   ├── sessions/           # 会话管理器
│   │   ├── proxy/              # Proxy 模式面板
│   │   ├── openclaw/           # OpenClaw 配置面板
│   │   ├── settings/           # 设置（终端/备份/关于）
│   │   ├── deeplink/           # Deep Link 导入
│   │   ├── env/                # 环境变量管理
│   │   ├── universal/          # 跨应用配置
│   │   ├── usage/              # 用量统计
│   │   └── ui/                 # shadcn/ui 组件库
│   ├── hooks/                  # 自定义 hooks（业务逻辑）
│   ├── lib/
│   │   ├── api/                # Tauri API 封装（类型安全）
│   │   └── query/              # TanStack Query 配置
│   ├── locales/                # 翻译 (zh/zh-TW/en/ja)
│   ├── config/                 # 预设 (providers/mcp)
│   └── types/                  # TypeScript 类型定义
├── src-tauri/                  # 后端 (Rust)
│   └── src/
│       ├── commands/           # Tauri 命令层（按领域）
│       ├── services/           # 业务逻辑层
│       ├── database/           # SQLite DAO 层
│       ├── proxy/              # Proxy 模块
│       ├── session_manager/    # 会话管理
│       ├── deeplink/           # Deep Link 处理
│       └── mcp/                # MCP 同步模块
├── tests/                      # 前端测试
└── assets/                     # 截图
```

</details>

## 贡献

欢迎提交 Issue 反馈问题和建议！

提交 PR 前请确保：

- 运行完整当前宿主门禁：`mise run check`
- 开发时从 [canonical task catalog](docs/fyagent/development/mise-tasks.md)
  选择聚焦任务

新功能开发前，欢迎先开 Issue 讨论实现方案，不适合项目的功能性 PR 有可能会被关闭。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=NongHua123/fyagent&type=Date)](https://www.star-history.com/#NongHua123/fyagent&Date)

## License

FyAgent 是源码可用软件，并非 OSI 所定义的开源软件。FyAgent 自有的组件和修改部分采用
[PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt)
授权；商业使用须另行取得书面授权。源自 CC Switch 的部分仍采用 MIT 许可证。详见
[LICENSE](LICENSE)、[LICENSING.md](LICENSING.md) 和
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
