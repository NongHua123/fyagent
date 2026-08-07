<div align="center">

# FyAgent

### The All-in-One Manager for Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw & Hermes Agent

[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

English | [中文](README_ZH.md) | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [Changelog](CHANGELOG.md)

</div>

## Why FyAgent?

Modern AI-powered coding relies on tools like Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, and Hermes — but each has its own configuration format. Switching API providers means manually editing JSON, TOML, or `.env` files, and there is no unified way to manage MCP and Skills across multiple tools.

**FyAgent** gives you a single desktop app to manage all supported AI tools. Instead of editing config files by hand, you get a visual interface to import providers with one click, switch between them instantly, with 50+ built-in provider presets, unified MCP and Skills management, and system tray quick switching — all backed by a reliable SQLite database with atomic writes that protect your configs from corruption.

- **One App, Eight Tools** — Manage Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, and Hermes from a single interface
- **No More Manual Editing** — 50+ provider presets including AWS Bedrock, NVIDIA NIM, and community relays; just pick and switch
- **Unified MCP & Skills Management** — One panel to manage MCP servers and Skills across Claude, Codex, Gemini, Grok Build, OpenCode, and Hermes with bidirectional sync
- **System Tray Quick Switch** — Switch providers instantly from the tray menu, no need to open the full app
- **Cloud Sync** — Sync provider data across devices via Dropbox, OneDrive, iCloud, or WebDAV servers
- **Cross-Platform** — Native desktop app for Windows, macOS, and Linux, built with Tauri 2
- **Built-in Utilities** — Includes various utilities for first-launch login confirmation, signature bypass, plugin extension sync, and more

## Screenshots

|                  Main Interface                   |                  Add Provider                  |
| :-----------------------------------------------: | :--------------------------------------------: |
| ![Main Interface](assets/screenshots/main-en.png) | ![Add Provider](assets/screenshots/add-en.png) |

## Features

[Full Changelog](CHANGELOG.md) | [Release Notes](docs/release-notes/v3.16.1-en.md)

### Provider Management

- **8 supported tools, 50+ presets** — Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, Hermes; copy your key and import with one click
- **Universal providers** — One config syncs to Claude Code, Codex, and Gemini CLI
- One-click switching, system tray quick access, drag-and-drop sorting, import/export

### Proxy & Failover

- **Local proxy with hot-switching** — Format conversion, auto-failover, circuit breaker, provider health monitoring, and request rectifier
- **App-level takeover** — Independently proxy Claude, Codex, Gemini, or Grok Build, down to individual providers

### MCP, Prompts & Skills

- **Unified MCP panel** — Manage MCP servers across Claude, Codex, Gemini, Grok Build, OpenCode, and Hermes with bidirectional sync and Deep Link import
- **Prompts** — Markdown editor with cross-app sync (CLAUDE.md / AGENTS.md / GEMINI.md) and backfill protection
- **Skills** — One-click install from GitHub repos or ZIP files, custom repository management, with symlink and file copy support

### Usage & Cost Tracking

- **Usage dashboard** — Track spending, requests, and tokens with trend charts, detailed request logs, and custom per-model pricing

### Session Manager & Workspace

- Browse, search, and restore conversation history across supported session sources
- **Workspace editor** (OpenClaw) — Edit agent files (AGENTS.md, SOUL.md, etc.) with Markdown preview

### System & Platform

- **Cloud sync** — Custom config directory (Dropbox, OneDrive, iCloud, NAS) and WebDAV server sync
- **Deep Link** (`fyagent://`) — Import providers, MCP servers, prompts, and skills via URL
- Dark / Light / System theme, auto-launch, manual release downloads (no in-app auto-update), atomic writes, auto-backups, i18n (zh/zh-TW/en/ja)

## FAQ

<details>
<summary><strong>Which AI tools does FyAgent support?</strong></summary>

FyAgent supports eight tools: **Claude Code**, **Claude Desktop**, **Codex**, **Gemini CLI**, **Grok Build**, **OpenCode**, **OpenClaw**, and **Hermes**. Each tool has dedicated provider presets and configuration management.

</details>

<details>
<summary><strong>Do I need to restart the terminal after switching providers?</strong></summary>

For most tools, yes — restart your terminal or the CLI tool for changes to take effect. The exception is **Claude Code**, which currently supports hot-switching of provider data without a restart.

</details>

<details>
<summary><strong>My plugin configuration disappeared after switching providers — what happened?</strong></summary>

FyAgent provides a "Shared Config Snippet" feature to pass common data (beyond API keys and endpoints) between providers. Go to "Edit Provider" → "Shared Config Panel" → click "Extract from Current Provider" to save all common data. When creating a new provider, check "Write Shared Config" (enabled by default) to include plugin data in the new provider. All your configuration items are preserved in the default provider imported when you first launched the app.

</details>

<details>
<summary><strong>macOS installation</strong></summary>

Download the `.dmg` asset published with the release you are installing. Signing and notarization are release-specific; verify the evidence published for that exact build before relying on a direct-install claim.

</details>

<details>
<summary><strong>Why can't I delete the currently active provider?</strong></summary>

FyAgent follows a "minimal intrusion" design principle — even if you uninstall the app, your CLI tools will continue to work normally. The system always keeps one active configuration, because deleting all configurations would make the corresponding CLI tool unusable. If you rarely use a specific CLI tool, you can hide it in Settings. To switch back to official login, see the next question.

</details>

<details>
<summary><strong>How do I switch back to official login?</strong></summary>

Add an official provider from the preset list. After switching to it, run the Log out / Log in flow, and then you can freely switch between the official provider and third-party providers. Codex supports switching between different official providers, making it easy to switch between multiple Plus or Team accounts.

</details>

<details>
<summary><strong>Where is my data stored?</strong></summary>

- **Database**: `~/.fyagent/fyagent.db` (SQLite — providers, MCP, prompts, skills)
- **Local settings**: `~/.fyagent/settings.json` (device-level UI preferences)
- **Backups**: `~/.fyagent/backups/` (auto-rotated, keeps 10 most recent)
- **Skills**: `~/.fyagent/skills/` (symlinked to corresponding apps by default)
- **Skill Backups**: `~/.fyagent/skill-backups/` (created automatically before uninstall, keeps 20 most recent)

</details>

<details>
<summary><strong>Linux (Wayland + NVIDIA): clicks don't register and the window black-screens on resize</strong></summary>

The AppImage forces `GDK_BACKEND=x11` (XWayland) to avoid a historical native-Wayland crash. On newer Wayland + NVIDIA setups this can leave the web content area unclickable (the title-bar buttons still work) and black-screen on resize. Launch with the opt-in escape hatch to switch back to native Wayland:

```bash
FYAGENT_GDK_BACKEND=wayland ./FyAgent-*.AppImage
```

If you launch from a desktop icon, add it to the `.desktop` `Exec=` line (e.g. `env FYAGENT_GDK_BACKEND=wayland /path/to/AppImage`) or set it in your session environment. The variable is generic: on tiling Wayland compositors (sway/Hyprland) where clicks don't register, try `FYAGENT_GDK_BACKEND=x11` instead. Leaving it unset keeps the default behavior.

</details>

## Documentation

For detailed guides on every feature, check out the **[User Manual](docs/user-manual/en/README.md)** — covering provider management, MCP/Prompts/Skills, proxy & failover, and more.

## Quick Start

### Basic Usage

1. **Add Provider**: Click "Add Provider" → Choose a preset or create custom configuration
2. **Switch Provider**:
   - Main UI: Select provider → Click "Enable"
   - System Tray: Click provider name directly (instant effect)
3. **Takes Effect**: Restart your terminal or the corresponding CLI tool to apply changes (Claude Code does not require a restart)
4. **Back to Official**: Add an "Official Login" preset, restart the CLI tool, then follow its login/OAuth flow

### MCP, Prompts, Skills & Sessions

- **MCP**: Click the "MCP" button → Add servers via templates or custom config → Toggle per-app sync
- **Prompts**: Click "Prompts" → Create presets with Markdown editor → Activate to sync to live files
- **Skills**: Click "Skills" → Browse GitHub repos → One-click install to supported apps
- **Sessions**: Click "Sessions" → Browse, search, and restore conversation history across supported session sources

> **Note**: On first launch, you can manually import existing CLI tool configs as the default provider.

## Download & Installation

### System Requirements

- **Windows**: Windows 10 and above
- **macOS**: macOS 12 (Monterey) and above
- **Linux**: Ubuntu 22.04+ / Debian 11+ / Fedora 34+ and other mainstream distributions

### Windows Users

Download the latest `FyAgent-{version}-Windows.msi` installer from the [Releases](https://github.com/NongHua123/fyagent/releases) page. FyAgent no longer publishes a Windows Portable ZIP because the supported Windows application runs through the protected per-machine installation path.

### macOS Users

Download `FyAgent-{version}-macOS.dmg` (recommended) or `.zip` from the [Releases](https://github.com/NongHua123/fyagent/releases) page.

> **Note**: Signing and notarization are release-specific. Check the evidence published with the exact FyAgent build before expecting macOS to allow a direct install.

### Linux Users

Download the latest Linux build, including builds usable on Arch Linux, from the [Releases](https://github.com/NongHua123/fyagent/releases) page:

- `FyAgent-{version}-Linux-{arch}.deb` (Debian/Ubuntu)
- `FyAgent-{version}-Linux-{arch}.rpm` (Fedora/RHEL/openSUSE)
- `FyAgent-{version}-Linux-{arch}.AppImage` (portable; choose `x86_64` or `arm64` for `{arch}`)

> **Flatpak**: Not included in official releases. You can build it yourself from the `.deb` — see [`flatpak/README.md`](flatpak/README.md) for instructions.

<details>
<summary><strong>Architecture Overview</strong></summary>

### Design Principles

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (React + TS)                    │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │ Components  │  │    Hooks     │  │  TanStack Query  │    │
│  │   (UI)      │──│ (Bus. Logic) │──│   (Cache/Sync)   │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
└────────────────────────┬────────────────────────────────────┘
                         │ Tauri IPC
┌────────────────────────▼────────────────────────────────────┐
│                  Backend (Tauri + Rust)                     │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │  Commands   │  │   Services   │  │  Models/Config   │    │
│  │ (API Layer) │──│ (Bus. Layer) │──│     (Data)       │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

**Core Design Patterns**

- **SSOT** (Single Source of Truth): All data stored in `~/.fyagent/fyagent.db` (SQLite)
- **Dual-layer Storage**: SQLite for syncable data, JSON for device-level settings
- **Dual-way Sync**: Write to live files on switch, backfill from live when editing active provider
- **Atomic Writes**: Temp file + rename pattern prevents config corruption
- **Concurrency Safe**: Mutex-protected database connection avoids race conditions
- **Layered Architecture**: Clear separation (Commands → Services → DAO → Database)

**Key Components**

- **ProviderService**: Provider CRUD, switching, backfill, sorting
- **McpService**: MCP server management, import/export, live file sync
- **ProxyService**: Local proxy mode with hot-switching and format conversion
- **SessionManager**: Conversation history browsing across supported session sources
- **ConfigService**: Config import/export, backup rotation
- **SpeedtestService**: API endpoint latency measurement

</details>

<details>
<summary><strong>Development Guide</strong></summary>

### Environment Requirements

- [mise](https://mise.jdx.dev/getting-started.html) 2026.8.0 or newer,
  installed globally
- [Tauri 2.0 system prerequisites](https://v2.tauri.app/start/prerequisites/)

`mise.toml` is the single source of truth for local Node.js, pnpm, Python, and
Rust/Cargo versions. The Tauri CLI is installed with the project dependencies;
do not install or select separate tool versions from broad minimum-version
requirements.

After reviewing the repository config, initialize the development environment:

```bash
mise trust
mise install
mise exec -- pnpm install --frozen-lockfile
```

The commands below use `mise exec --` explicitly so they resolve the pinned
tools even when shell activation is not configured. In an intentionally
mise-activated shell, the prefix may be omitted. WSL must not resolve managed
tools from `/mnt/<drive>` or Windows shims.

### Native Platform Builds

Local development and packaging support only the current host operating
system. The standard commands do not accept another OS or architecture target:

```bash
mise exec -- pnpm dev
mise exec -- pnpm build
```

FyAgent installers are built only by GitHub Actions on native Windows x64 and
ARM64, Linux x64 and ARM64, and macOS runners. The macOS job produces the
Universal build. Local Linux/WSL-to-Windows or macOS packaging is not a
supported release path.

### Development Commands

```bash
# Install dependencies
mise exec -- pnpm install --frozen-lockfile

# Dev mode (hot reload)
mise exec -- pnpm dev

# Type check
mise exec -- pnpm typecheck

# Format code
mise exec -- pnpm format

# Check code format
mise exec -- pnpm format:check

# Run frontend unit tests
mise exec -- pnpm test:unit

# Run tests in watch mode (recommended for development)
mise exec -- pnpm test:unit:watch

# Build application
mise exec -- pnpm build

# Build debug version
mise exec -- pnpm tauri build --debug
```

### Rust Backend Development

```bash
# Format Rust code
mise exec -- cargo fmt --manifest-path src-tauri/Cargo.toml

# Run clippy checks
mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml

# Run backend tests
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml

# Run specific tests
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml test_name

# Run tests with test-hooks feature
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml --features test-hooks
```

### Testing Guide

**Frontend Testing**:

- Uses **vitest** as test framework
- Uses **MSW (Mock Service Worker)** to mock Tauri API calls
- Uses **@testing-library/react** for component testing

**Running Tests**:

```bash
# Run all tests
mise exec -- pnpm test:unit

# Watch mode (auto re-run)
mise exec -- pnpm test:unit:watch

# With coverage report
mise exec -- pnpm test:unit --coverage
```

### Tech Stack

**Frontend**: React 18 · TypeScript · Vite · TailwindCSS 3.4 · TanStack Query v5 · react-i18next · react-hook-form · zod · shadcn/ui · @dnd-kit

**Backend**: Tauri 2.8 · Rust · serde · tokio · thiserror · tauri-plugin-process/dialog/store/log

**Testing**: vitest · MSW · @testing-library/react

</details>

<details>
<summary><strong>Project Structure</strong></summary>

```
├── src/                        # Frontend (React + TypeScript)
│   ├── components/
│   │   ├── providers/          # Provider management
│   │   ├── mcp/                # MCP panel
│   │   ├── prompts/            # Prompts management
│   │   ├── skills/             # Skills management
│   │   ├── sessions/           # Session Manager
│   │   ├── proxy/              # Proxy mode panel
│   │   ├── openclaw/           # OpenClaw config panels
│   │   ├── settings/           # Settings (Terminal/Backup/About)
│   │   ├── deeplink/           # Deep Link import
│   │   ├── env/                # Environment variable management
│   │   ├── universal/          # Cross-app configuration
│   │   ├── usage/              # Usage statistics
│   │   └── ui/                 # shadcn/ui component library
│   ├── hooks/                  # Custom hooks (business logic)
│   ├── lib/
│   │   ├── api/                # Tauri API wrapper (type-safe)
│   │   └── query/              # TanStack Query config
│   ├── locales/                # Translations (zh/zh-TW/en/ja)
│   ├── config/                 # Presets (providers/mcp)
│   └── types/                  # TypeScript definitions
├── src-tauri/                  # Backend (Rust)
│   └── src/
│       ├── commands/           # Tauri command layer (by domain)
│       ├── services/           # Business logic layer
│       ├── database/           # SQLite DAO layer
│       ├── proxy/              # Proxy module
│       ├── session_manager/    # Session management
│       ├── deeplink/           # Deep Link handling
│       └── mcp/                # MCP sync module
├── tests/                      # Frontend tests
└── assets/                     # Screenshots
```

</details>

## Contributing

Issues and suggestions are welcome!

Before submitting PRs, please ensure:

- Pass type check: `mise exec -- pnpm typecheck`
- Pass format check: `mise exec -- pnpm format:check`
- Pass unit tests: `mise exec -- pnpm test:unit`

For new features, please open an issue for discussion before submitting a PR. PRs for features that are not a good fit for the project may be closed.

## License

FyAgent is source-available software, not open source as defined by the Open
Source Initiative. FyAgent-owned components and modifications are licensed
under the [PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt).
Commercial use requires separate written authorization. CC Switch-derived
portions remain under the MIT License. See [LICENSE](LICENSE),
[LICENSING.md](LICENSING.md), and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
