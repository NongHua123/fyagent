<div align="center">

# FyAgent

### Der All-in-One-Manager für Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw & Hermes Agent

[![Version](https://img.shields.io/github/v/release/NongHua123/fyagent?color=blue&label=version)](https://github.com/NongHua123/fyagent/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/NongHua123/fyagent/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![Downloads](https://img.shields.io/github/downloads/NongHua123/fyagent/total)](https://github.com/NongHua123/fyagent/releases/latest)

<a href="https://www.star-history.com/#NongHua123/fyagent&Date"><picture><source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/badge?repo=NongHua123/fyagent&theme=dark" /><img alt="Star History Rank" src="https://api.star-history.com/badge?repo=NongHua123/fyagent" width="196" height="55" /></picture></a>

### 🌐 Projekt-Repository: **[GitHub](https://github.com/NongHua123/fyagent)**

[English](README.md) | [中文](README_ZH.md) | [日本語](README_JA.md) | Deutsch | [Changelog](CHANGELOG.md)

</div>

## Warum FyAgent?

Modernes KI-gestütztes Programmieren stützt sich auf Werkzeuge wie Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw und Hermes — doch jedes hat sein eigenes Konfigurationsformat. Der Wechsel des API-Anbieters bedeutet, JSON-, TOML- oder `.env`-Dateien von Hand zu bearbeiten, und es gibt keine einheitliche Möglichkeit, MCP und Skills über mehrere Werkzeuge hinweg zu verwalten.

**FyAgent** gibt Ihnen eine einzige Desktop-App, um alle unterstützten KI-Werkzeuge zu verwalten. Statt Konfigurationsdateien von Hand zu bearbeiten, erhalten Sie eine visuelle Oberfläche, um Anbieter mit einem Klick zu importieren und sofort zwischen ihnen zu wechseln — mit 50+ integrierten Anbieter-Presets, einheitlicher MCP- und Skills-Verwaltung und schnellem Umschalten über das System-Tray. Das Ganze gestützt auf eine zuverlässige SQLite-Datenbank mit atomaren Schreibvorgängen, die Ihre Konfigurationen vor Beschädigung schützen.

- **Eine App, acht Werkzeuge** — Verwalten Sie Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw und Hermes über eine einzige Oberfläche
- **Kein manuelles Bearbeiten mehr** — 50+ Anbieter-Presets einschließlich AWS Bedrock, NVIDIA NIM und Community-Relays; einfach auswählen und umschalten
- **Einheitliche MCP- & Skills-Verwaltung** — Ein Panel zur Verwaltung von MCP-Servern und Skills für Claude, Codex, Gemini, Grok Build, OpenCode und Hermes mit bidirektionaler Synchronisierung
- **Schnellumschaltung über System-Tray** — Wechseln Sie Anbieter sofort über das Tray-Menü, ohne die vollständige App öffnen zu müssen
- **Cloud-Synchronisierung** — Synchronisieren Sie Anbieterdaten geräteübergreifend über Dropbox, OneDrive, iCloud oder WebDAV-Server
- **Plattformübergreifend** — Native Desktop-App für Windows, macOS und Linux, gebaut mit Tauri 2
- **Integrierte Hilfsprogramme** — Enthält diverse Hilfsprogramme für die Login-Bestätigung beim Erststart, das Umgehen von Signaturen, die Synchronisierung von Plugin-Erweiterungen und mehr

## Screenshots

|                  Hauptoberfläche                   |                  Anbieter hinzufügen                  |
| :------------------------------------------------: | :---------------------------------------------------: |
| ![Hauptoberfläche](assets/screenshots/main-en.png) | ![Anbieter hinzufügen](assets/screenshots/add-en.png) |

## Funktionen

[Vollständiges Changelog](CHANGELOG.md) | [Release Notes](docs/release-notes/v3.16.1-en.md)

### Anbieterverwaltung

- **8 unterstützte Werkzeuge, 50+ Presets** — Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, Hermes; Schlüssel kopieren und mit einem Klick importieren
- **Universelle Anbieter** — Eine Konfiguration synchronisiert sich mit Claude Code, Codex und Gemini CLI
- Umschaltung mit einem Klick, Schnellzugriff über System-Tray, Sortierung per Drag-and-drop, Import/Export

### Proxy & Failover

- **Lokaler Proxy mit Hot-Switching** — Formatkonvertierung, automatisches Failover, Circuit Breaker, Anbieter-Health-Monitoring und Request-Rectifier
- **Übernahme auf App-Ebene** — Claude, Codex, Gemini oder Grok Build unabhängig über den Proxy leiten, bis hinunter auf einzelne Anbieter

### MCP, Prompts & Skills

- **Einheitliches MCP-Panel** — Verwalten Sie MCP-Server für Claude, Codex, Gemini, Grok Build, OpenCode und Hermes mit bidirektionaler Synchronisierung und Deep-Link-Import
- **Prompts** — Markdown-Editor mit App-übergreifender Synchronisierung (CLAUDE.md / AGENTS.md / GEMINI.md) und Backfill-Schutz
- **Skills** — Installation mit einem Klick aus GitHub-Repositorys oder ZIP-Dateien, Verwaltung eigener Repositorys, mit Unterstützung für Symlinks und Dateikopien

### Nutzungs- & Kostenverfolgung

- **Nutzungs-Dashboard** — Verfolgen Sie Ausgaben, Anfragen und Token mit Trenddiagrammen, detaillierten Anfrageprotokollen und eigener Preisgestaltung pro Modell

### Session Manager & Workspace

- Gesprächsverlauf aus unterstützten Sitzungsquellen durchsuchen, suchen und wiederherstellen
- **Workspace-Editor** (OpenClaw) — Bearbeiten Sie Agent-Dateien (AGENTS.md, SOUL.md usw.) mit Markdown-Vorschau

### System & Plattform

- **Cloud-Synchronisierung** — Eigenes Konfigurationsverzeichnis (Dropbox, OneDrive, iCloud, NAS) und WebDAV-Server-Synchronisierung
- **Deep Link** (`fyagent://`) — Importieren Sie Anbieter, MCP-Server, Prompts und Skills per URL
- Dunkles / Helles / System-Theme, automatischer Start, manuelle Updates über GitHub Releases, atomare Schreibvorgänge, automatische Backups, i18n (zh/zh-TW/en/ja)

## FAQ

<details>
<summary><strong>Welche KI-Werkzeuge unterstützt FyAgent?</strong></summary>

FyAgent unterstützt acht Werkzeuge: **Claude Code**, **Claude Desktop**, **Codex**, **Gemini CLI**, **Grok Build**, **OpenCode**, **OpenClaw** und **Hermes**. Jedes Werkzeug verfügt über dedizierte Anbieter-Presets und Konfigurationsverwaltung.

</details>

<details>
<summary><strong>Muss ich das Terminal nach einem Anbieterwechsel neu starten?</strong></summary>

Bei den meisten Werkzeugen ja — starten Sie Ihr Terminal oder das CLI-Werkzeug neu, damit die Änderungen wirksam werden. Die Ausnahme ist **Claude Code**, das derzeit das Hot-Switching von Anbieterdaten ohne Neustart unterstützt.

</details>

<details>
<summary><strong>Meine Plugin-Konfiguration ist nach einem Anbieterwechsel verschwunden — was ist passiert?</strong></summary>

FyAgent bietet eine Funktion „Gemeinsames Konfigurations-Snippet", um gemeinsame Daten (über API-Schlüssel und Endpunkte hinaus) zwischen Anbietern weiterzugeben. Gehen Sie zu „Anbieter bearbeiten" → „Panel für gemeinsame Konfiguration" → klicken Sie auf „Aus aktuellem Anbieter extrahieren", um alle gemeinsamen Daten zu speichern. Aktivieren Sie beim Anlegen eines neuen Anbieters die Option „Gemeinsame Konfiguration schreiben" (standardmäßig aktiviert), um die Plugin-Daten in den neuen Anbieter aufzunehmen. Alle Ihre Konfigurationspunkte bleiben im Standardanbieter erhalten, der beim ersten Start der App importiert wurde.

</details>

<details>
<summary><strong>Installation unter macOS</strong></summary>

Laden Sie das mit dem jeweiligen Release veröffentlichte `.dmg` herunter. Codesignatur und Notarisierung sind releasespezifisch; prüfen Sie vor der Installation die für genau diesen FyAgent-Build veröffentlichten Nachweise.

</details>

<details>
<summary><strong>Warum kann ich den aktuell aktiven Anbieter nicht löschen?</strong></summary>

FyAgent folgt dem Designprinzip der „minimalen Eingriffstiefe" — selbst wenn Sie die App deinstallieren, funktionieren Ihre CLI-Werkzeuge weiterhin normal. Das System behält immer eine aktive Konfiguration bei, da das Löschen aller Konfigurationen das entsprechende CLI-Werkzeug unbrauchbar machen würde. Wenn Sie ein bestimmtes CLI-Werkzeug selten verwenden, können Sie es in den Einstellungen ausblenden. Wie Sie zurück zum offiziellen Login wechseln, erfahren Sie in der nächsten Frage.

</details>

<details>
<summary><strong>Wie wechsle ich zurück zum offiziellen Login?</strong></summary>

Fügen Sie einen offiziellen Anbieter aus der Preset-Liste hinzu. Führen Sie nach dem Wechsel den Abmelde-/Anmelde-Vorgang aus; anschließend können Sie frei zwischen dem offiziellen Anbieter und Drittanbietern wechseln. Codex unterstützt den Wechsel zwischen verschiedenen offiziellen Anbietern, was das Umschalten zwischen mehreren Plus- oder Team-Konten erleichtert.

</details>

<details>
<summary><strong>Wo werden meine Daten gespeichert?</strong></summary>

- **Datenbank**: `~/.fyagent/fyagent.db` (SQLite — Anbieter, MCP, Prompts, Skills)
- **Lokale Einstellungen**: `~/.fyagent/settings.json` (gerätebezogene UI-Einstellungen)
- **Backups**: `~/.fyagent/backups/` (automatisch rotiert, behält die 10 neuesten)
- **Skills**: `~/.fyagent/skills/` (standardmäßig per Symlink mit den entsprechenden Apps verbunden)
- **Skill-Backups**: `~/.fyagent/skill-backups/` (vor der Deinstallation automatisch erstellt, behält die 20 neuesten)

</details>

<details>
<summary><strong>Linux (Wayland + NVIDIA): Klicks im Webinhalt reagieren nicht, schwarzer Bildschirm beim Größenändern</strong></summary>

Das AppImage erzwingt `GDK_BACKEND=x11` (XWayland), um einen historischen nativen Wayland-Absturz zu vermeiden. Auf neueren Wayland-+-NVIDIA-Systemen kann das dazu führen, dass der Webinhalt nicht anklickbar ist (die Titelleisten-Schaltflächen funktionieren weiterhin) und das Fenster beim Größenändern schwarz wird. Starten Sie mit dem optionalen Notausgang, um zu nativem Wayland zu wechseln:

```bash
FYAGENT_GDK_BACKEND=wayland ./FyAgent-*.AppImage
```

Wenn Sie über ein Desktop-Symbol starten, fügen Sie es der `Exec=`-Zeile der `.desktop`-Datei hinzu (z. B. `env FYAGENT_GDK_BACKEND=wayland /pfad/zum/AppImage`) oder setzen Sie es in Ihrer Sitzungsumgebung. Die Variable ist generisch: Auf Tiling-Wayland-Compositors (sway/Hyprland), bei denen Klicks nicht reagieren, versuchen Sie umgekehrt `FYAGENT_GDK_BACKEND=x11`. Bleibt sie ungesetzt, bleibt das Standardverhalten erhalten.

</details>

## Dokumentation

Ausführliche Anleitungen zu jeder Funktion finden Sie im **[Benutzerhandbuch](docs/user-manual/en/README.md)** — es deckt Anbieterverwaltung, MCP/Prompts/Skills, Proxy & Failover und mehr ab.

## Schnellstart

### Grundlegende Verwendung

1. **Anbieter hinzufügen**: Klicken Sie auf „Add Provider" → Wählen Sie ein Preset oder erstellen Sie eine eigene Konfiguration
2. **Anbieter wechseln**:
   - Hauptoberfläche: Anbieter auswählen → auf „Enable" klicken
   - System-Tray: Anbietername direkt anklicken (sofort wirksam)
3. **Wirksam werden**: Starten Sie Ihr Terminal oder das entsprechende CLI-Werkzeug neu, um die Änderungen anzuwenden (Claude Code erfordert keinen Neustart)
4. **Zurück zum Offiziellen**: Fügen Sie ein „Official Login"-Preset hinzu, starten Sie das CLI-Werkzeug neu und folgen Sie dann seinem Login-/OAuth-Vorgang

### MCP, Prompts, Skills & Sessions

- **MCP**: Klicken Sie auf die Schaltfläche „MCP" → Server über Vorlagen oder eigene Konfiguration hinzufügen → Synchronisierung pro App umschalten
- **Prompts**: Klicken Sie auf „Prompts" → Presets mit dem Markdown-Editor erstellen → Aktivieren, um mit den Live-Dateien zu synchronisieren
- **Skills**: Klicken Sie auf „Skills" → GitHub-Repositorys durchsuchen → mit einem Klick in unterstützte Apps installieren
- **Sessions**: Klicken Sie auf „Sessions" → Gesprächsverlauf aus unterstützten Sitzungsquellen durchsuchen, suchen und wiederherstellen

> **Hinweis**: Beim Erststart können Sie bestehende CLI-Werkzeug-Konfigurationen manuell als Standardanbieter importieren.

## Download & Installation

### Systemanforderungen

- **Windows**: Windows 10 und höher
- **macOS**: macOS 12 (Monterey) und höher
- **Linux**: Ubuntu 22.04+ / Debian 11+ / Fedora 34+ und andere gängige Distributionen

### Windows-Nutzer

Laden Sie das neueste Installationsprogramm `FyAgent-v{version}-Windows.msi` von der Seite [Releases](https://github.com/NongHua123/fyagent/releases) herunter. FyAgent veröffentlicht kein portables Windows-ZIP mehr; verwenden Sie den MSI-Installer für den unterstützten installationsweiten Pfad.

### macOS-Nutzer

Laden Sie `FyAgent-v{version}-macOS.dmg` (empfohlen) oder `.zip` von der Seite [Releases](https://github.com/NongHua123/fyagent/releases) herunter.

> **Hinweis**: Der Status von Codesignatur und Notarisierung ist releasespezifisch. Prüfen Sie vor der Installation die für den heruntergeladenen FyAgent-Build veröffentlichten Nachweise.

### Linux-Nutzer

Laden Sie den neuesten Linux-Build, einschließlich der unter Arch Linux nutzbaren Builds, von der Seite [Releases](https://github.com/NongHua123/fyagent/releases) herunter:

- `FyAgent-v{version}-Linux-{arch}.deb` (Debian/Ubuntu)
- `FyAgent-v{version}-Linux-{arch}.rpm` (Fedora/RHEL/openSUSE)
- `FyAgent-v{version}-Linux-{arch}.AppImage` (portabel; für `{arch}` `x86_64` oder `arm64` wählen)

> **Flatpak**: Nicht in den offiziellen Releases enthalten. Sie können es selbst aus dem `.deb` bauen — eine Anleitung finden Sie unter [`flatpak/README.md`](flatpak/README.md).

<details>
<summary><strong>Architekturüberblick</strong></summary>

### Designprinzipien

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

**Kern-Designmuster**

- **SSOT** (Single Source of Truth): Alle Daten werden in `~/.fyagent/fyagent.db` (SQLite) gespeichert
- **Zweischichtiger Speicher**: SQLite für synchronisierbare Daten, JSON für gerätebezogene Einstellungen
- **Bidirektionale Synchronisierung**: Schreiben in Live-Dateien beim Umschalten, Backfill aus den Live-Dateien beim Bearbeiten des aktiven Anbieters
- **Atomare Schreibvorgänge**: Das Muster aus temporärer Datei + Umbenennen verhindert die Beschädigung von Konfigurationen
- **Nebenläufigkeitssicher**: Eine durch Mutex geschützte Datenbankverbindung vermeidet Race Conditions
- **Geschichtete Architektur**: Klare Trennung (Commands → Services → DAO → Database)

**Schlüsselkomponenten**

- **ProviderService**: Anbieter-CRUD, Umschaltung, Backfill, Sortierung
- **McpService**: Verwaltung von MCP-Servern, Import/Export, Synchronisierung von Live-Dateien
- **ProxyService**: Lokaler Proxy-Modus mit Hot-Switching und Formatkonvertierung
- **SessionManager**: Durchsuchen des Gesprächsverlaufs über alle unterstützten Apps hinweg
- **ConfigService**: Konfigurations-Import/-Export, Backup-Rotation
- **SpeedtestService**: Messung der Latenz von API-Endpunkten

</details>

<details>
<summary><strong>Entwicklungsleitfaden</strong></summary>

### Entwicklungsumgebung

FyAgent verwendet lokal genau eine globale [mise](https://mise.jdx.dev/)-Installation
für Werkzeugversionen und die Repository-Tasks. Prüfe die Projektkonfiguration,
bevor du ihr ausdrücklich vertraust:

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

Vor einem Pull Request:

```bash
mise run check
```

`bootstrap` bereitet Node/Rust/uv gemäß der persönlichen mise-Richtlinie vor,
synchronisiert die eingefrorenen pnpm-Abhängigkeiten und die von uv gesperrte
Python-Umgebung und führt strikte Umgebungs-/Task-Prüfungen aus. Ist Auto-Install
persönlich deaktiviert, führe vorher `mise install` aus.

| Werkzeug | Projektbasis | Verbindliche Quelle |
| --- | --- | --- |
| Node.js | `24.19.0` | `.node-version` |
| pnpm | `10.12.3` | `package.json#packageManager` |
| Rust | `1.97.1` | `rust-toolchain.toml` |
| uv | `latest`, freigegebener Build im Lock | `mise.toml` + `mise.lock` |
| Python | `3.14.7`, ausschließlich von uv verwaltet | `.python-version` |

Native Tauri-Compiler, WebView und Bibliotheken bleiben Aufgabe des
Host-Paketmanagers. `mise run system:check` meldet fehlende Voraussetzungen und
offizielle Installationshinweise, installiert aber keine privilegierten Pakete.

### Kanonische Entwicklungs-Tasks

```bash
mise run dev
mise run dev:renderer
mise run build:renderer
mise run build:binary
mise run build
mise run build:debug

mise run typecheck
mise run format:check
mise run test
mise run rust:check
mise run rust:clippy
mise run rust:test
```

Gepflegte lokale Projektoperationen verwenden `mise run <task>`. Die ausführliche
Referenz steht in [docs/fyagent/development/mise-tasks.md](docs/fyagent/development/mise-tasks.md).

### Native Builds und formelle Releases

Lokale Builds unterstützen nur das aktuelle Host-Betriebssystem und dessen
Architektur. Die früheren lokalen Linux/WSL-Cross-Builds für Windows und macOS
werden entfernt. Native Entwicklung und Diagnose-Bundles auf Windows, macOS und
Linux bleiben unterstützt.

Formelle Windows-x64/ARM64-, Linux-x64/ARM64- und macOS-Universal-Artefakte
werden ausschließlich von GitHub Actions gebaut, signiert/notarisiert, geprüft
und veröffentlicht. Ein lokales Bundle ist kein formelles FyAgent-Release-Asset.

### Rust-Backend und Tests

```bash
mise run rust:fmt
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
mise run rust:test <filter>

mise run test:unit
mise run test:unit:watch
mise run test:i18n
mise run test:desktop:mock
mise run test:desktop:visual:preflight
```

Mock- und Visual-Preflight-Prüfungen sind kein nativer Installer-Nachweis.
Visual-Baselines werden nur durch einen expliziten, ändernden Task mit geprüfter
Kandidaten-Evidenz aktualisiert.

### Tech-Stack

**Frontend**: React 18 · TypeScript · Vite · TailwindCSS 3.4 · TanStack Query v5 · react-i18next · react-hook-form · zod · shadcn/ui · @dnd-kit

**Backend**: Tauri 2.8 · Rust · serde · tokio · thiserror · tauri-plugin-process/dialog/store/log

**Testing**: vitest · MSW · @testing-library/react

</details>

<details>
<summary><strong>Projektstruktur</strong></summary>

```
├── src/                        # Frontend (React + TypeScript)
│   ├── components/
│   │   ├── providers/          # Anbieterverwaltung
│   │   ├── mcp/                # MCP-Panel
│   │   ├── prompts/            # Prompts-Verwaltung
│   │   ├── skills/             # Skills-Verwaltung
│   │   ├── sessions/           # Session Manager
│   │   ├── proxy/              # Proxy-Modus-Panel
│   │   ├── openclaw/           # OpenClaw-Konfigurationspanels
│   │   ├── settings/           # Einstellungen (Terminal/Backup/About)
│   │   ├── deeplink/           # Deep-Link-Import
│   │   ├── env/                # Verwaltung von Umgebungsvariablen
│   │   ├── universal/          # App-übergreifende Konfiguration
│   │   ├── usage/              # Nutzungsstatistik
│   │   └── ui/                 # shadcn/ui-Komponentenbibliothek
│   ├── hooks/                  # Eigene Hooks (Geschäftslogik)
│   ├── lib/
│   │   ├── api/                # Tauri-API-Wrapper (typsicher)
│   │   └── query/              # TanStack-Query-Konfiguration
│   ├── locales/                # Übersetzungen (zh/zh-TW/en/ja)
│   ├── config/                 # Presets (providers/mcp)
│   └── types/                  # TypeScript-Definitionen
├── src-tauri/                  # Backend (Rust)
│   └── src/
│       ├── commands/           # Tauri-Befehlsschicht (nach Domäne)
│       ├── services/           # Geschäftslogikschicht
│       ├── database/           # SQLite-DAO-Schicht
│       ├── proxy/              # Proxy-Modul
│       ├── session_manager/    # Sitzungsverwaltung
│       ├── deeplink/           # Deep-Link-Verarbeitung
│       └── mcp/                # MCP-Synchronisierungsmodul
├── tests/                      # Frontend-Tests
└── assets/                     # Screenshots
```

</details>

## Mitwirken

Issues und Vorschläge sind willkommen!

Bitte stellen Sie vor dem Einreichen von PRs Folgendes sicher:

- Typprüfung besteht: `mise run typecheck`
- Formatprüfung besteht: `mise run format:check`
- Unit-Tests bestehen: `mise run test:unit`

Eröffnen Sie für neue Funktionen bitte vor dem Einreichen eines PR ein Issue zur Diskussion. PRs für Funktionen, die nicht gut zum Projekt passen, können geschlossen werden.

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=NongHua123/fyagent&type=Date)](https://www.star-history.com/#NongHua123/fyagent&Date)

## Lizenz

FyAgent ist quellverfügbar, aber nicht im Sinne der Open Source Initiative
Open Source. Für FyAgent-eigene Komponenten und Änderungen gilt die
[PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt).
Für die kommerzielle Nutzung ist eine gesonderte schriftliche Genehmigung
erforderlich. Von CC Switch abgeleitete Teile bleiben unter der MIT-Lizenz.
Details finden sich in [LICENSE](LICENSE), [LICENSING.md](LICENSING.md) und
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
