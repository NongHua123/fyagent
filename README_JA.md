<div align="center">

# FyAgent

### Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes Agent のオールインワン管理ツール

[![Version](https://img.shields.io/github/v/release/NongHua123/fyagent?color=blue&label=version)](https://github.com/NongHua123/fyagent/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/NongHua123/fyagent/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![Downloads](https://img.shields.io/github/downloads/NongHua123/fyagent/total)](https://github.com/NongHua123/fyagent/releases/latest)

<a href="https://www.star-history.com/#NongHua123/fyagent&Date"><picture><source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/badge?repo=NongHua123/fyagent&theme=dark" /><img alt="Star History Rank" src="https://api.star-history.com/badge?repo=NongHua123/fyagent" width="196" height="55" /></picture></a>

### 🌐 プロジェクトリポジトリ：**[GitHub](https://github.com/NongHua123/fyagent)**

[English](README.md) | [中文](README_ZH.md) | 日本語 | [Deutsch](README_DE.md) | [Changelog](CHANGELOG.md)

</div>

> [!WARNING]
> **FyAgent 0.3.0 は、公開・安定版でありながら意図的に未署名とするリリース契約です。**
> Windows の実行ファイルと MSI の Authenticode 状態は `NotSigned`、macOS
> アプリは Developer ID 署名も公証もありません。インストール前に
> [v0.3.0 リリースノート](docs/release-notes/v0.3.0-ja.md)と公開証拠を確認して
> ください。正式 workflow が完了するまでは、この案内は Release の公開済みを
> 意味しません。

## FyAgent を選ぶ理由

最新の AI コーディングは Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes などのツールに依存していますが、各ツールの設定形式はバラバラです。API プロバイダを切り替えるたびに JSON、TOML、`.env` ファイルを手動で編集する必要があり、複数ツール間で MCP や Skills を統一的に管理する手段もありません。

**FyAgent** は、対応する AI ツールを 1 つのデスクトップアプリで一元管理できます。設定ファイルを手作業で編集する代わりに、ワンクリックでプロバイダをインポートし、瞬時に切り替えられるビジュアルインターフェースを提供します。50 以上の組み込みプリセット、統一 MCP・Skills 管理、システムトレイからの即時切り替え機能を搭載。すべてはアトミック書き込みによる信頼性の高い SQLite データベースに支えられており、設定の破損を防ぎます。

- **1 つのアプリで 8 つのツール** -- Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes を単一インターフェースで管理
- **手動編集は不要** -- AWS Bedrock、NVIDIA NIM、コミュニティリレーなど 50 以上のプロバイダプリセットを内蔵。選んで切り替えるだけ
- **統一 MCP・Skills 管理** -- 1 つのパネルで Claude、Codex、Gemini、Grok Build、OpenCode、Hermes の MCP サーバーと Skills を双方向同期で管理
- **システムトレイでクイック切り替え** -- トレイメニューから即座にプロバイダを切り替え。アプリを開く必要なし
- **クラウド同期** -- Dropbox、OneDrive、iCloud、または WebDAV サーバー経由でデバイス間のプロバイダデータを同期
- **クロスプラットフォーム** -- Tauri 2 で構築された Windows、macOS、Linux 対応のネイティブデスクトップアプリ
- **便利ツール内蔵** -- 初回起動時のログイン確認、環境診断、プラグイン拡張の同期など、さまざまなユーティリティを搭載

## スクリーンショット

|                  メイン画面                   |                  プロバイダ追加                  |
| :-------------------------------------------: | :----------------------------------------------: |
| ![メイン画面](assets/screenshots/main-ja.png) | ![プロバイダ追加](assets/screenshots/add-ja.png) |

## 特長

[完全な更新履歴](CHANGELOG.md) | [v0.3.0 リリースノート](docs/release-notes/v0.3.0-ja.md)

### プロバイダ管理

- **8 つの対応ツール、50 以上のプリセット** -- Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes。キーをコピーしてワンクリックでインポート
- **ユニバーサルプロバイダ** -- 1 つの設定を Claude Code、Codex、Gemini CLI に同期
- ワンクリック切り替え、システムトレイクイックアクセス、ドラッグ＆ドロップ並び替え、インポート/エクスポート

### プロキシ & フェイルオーバー

- **ローカルプロキシのホットスイッチ** -- フォーマット変換、自動フェイルオーバー、サーキットブレーカー、プロバイダヘルスモニタリング、リクエストレクティファイア
- **アプリレベルのテイクオーバー** -- Claude、Codex、Gemini、Grok Build を個別にプロキシ経由でルーティング、プロバイダ単位で設定可能

### MCP、Prompts & Skills

- **統一 MCP パネル** -- Claude、Codex、Gemini、Grok Build、OpenCode、Hermes の MCP サーバーを管理、双方向同期、Deep Link インポート対応
- **Prompts** -- Markdown エディタ、クロスアプリ同期（CLAUDE.md / AGENTS.md / GEMINI.md）、バックフィル保護
- **Skills** -- GitHub リポジトリまたは ZIP ファイルからワンクリックインストール、カスタムリポジトリ管理、シンボリックリンクとファイルコピーに対応

### 使用量 & コストトラッキング

- **使用量ダッシュボード** -- プロバイダ横断で支出・リクエスト数・トークン使用量を追跡、トレンドチャート、詳細リクエストログ、カスタムモデル価格設定

### Session Manager & ワークスペース

- 対応するセッションソースの会話履歴を閲覧・検索・復元
- **ワークスペースエディタ**（OpenClaw）-- エージェントファイル（AGENTS.md、SOUL.md など）を Markdown プレビュー付きで編集

### システム & プラットフォーム

- **クラウド同期** -- カスタム設定ディレクトリ（Dropbox、OneDrive、iCloud、NAS）および WebDAV サーバー同期
- **Deep Link** (`fyagent://`) -- URL 経由でプロバイダ、MCP サーバー、Prompts、Skills をワンクリックインポート
- ダーク / ライト / システムテーマ、自動起動、GitHub Releases からの手動更新、アトミック書き込み、自動バックアップ、多言語対応（簡体中文/繁體中文/英/日）

## よくある質問

<details>
<summary><strong>FyAgent はどの AI ツールに対応していますか？</strong></summary>

FyAgent は **Claude Code**、**Claude Desktop**、**Codex**、**Gemini CLI**、**Grok Build**、**OpenCode**、**OpenClaw**、**Hermes** の 8 つのツールに対応しています。各ツールに専用のプロバイダプリセットと設定管理が用意されています。

</details>

<details>
<summary><strong>プロバイダを切り替えた後、ターミナルの再起動は必要ですか？</strong></summary>

ほとんどのツールでは、はい。変更を反映するにはターミナルまたは CLI ツールを再起動してください。ただし **Claude Code** は例外で、現在プロバイダデータのホットスイッチに対応しており、再起動は不要です。

</details>

<details>
<summary><strong>プロバイダを切り替えた後、プラグイン設定が消えてしまいました。どうすればよいですか？</strong></summary>

FyAgent には「共有設定スニペット」機能があり、APIキーやエンドポイント以外の共通データをプロバイダ間で引き継ぐことができます。「プロバイダ編集」→「共有設定パネル」→「現在のプロバイダから抽出」をクリックして、すべての共通データを保存してください。新しいプロバイダを作成する際に「共有設定を適用」にチェック（デフォルトで有効）を入れれば、プラグインなどのデータが新しいプロバイダ設定に含まれます。すべての設定項目は、アプリ初回起動時にインポートされたデフォルトプロバイダに保存されており、失われることはありません。

</details>

<details>
<summary><strong>macOS のインストールについて</strong></summary>

FyAgent 0.3.0 は意図的に未署名・未公証なので、macOS が初回起動をブロックする
場合があります。一度 FyAgent を開こうとした後、Apple がサポートする
**システム設定 → プライバシーとセキュリティ → このまま開く**の手順で確認して
ください。先に該当 Release のノートと証拠を検証し、Gatekeeper の無効化や隔離
属性の削除は行わないでください。

</details>

<details>
<summary><strong>現在アクティブなプロバイダを削除できないのはなぜですか？</strong></summary>

FyAgent は「最小限の介入」という設計原則に従っています。アプリをアンインストールしても、CLI ツールは正常に動作し続けます。すべての設定を削除すると対応する CLI ツールが使用できなくなるため、システムは常にアクティブな設定を 1 つ保持します。特定の CLI ツールをあまり使用しない場合は、設定で非表示にできます。公式ログインに戻す方法は、次の質問をご覧ください。

</details>

<details>
<summary><strong>公式ログインに戻すにはどうすればよいですか？</strong></summary>

プリセットリストから公式プロバイダを追加してください。切り替え後、ログアウト／ログインのフローを実行すれば、以降は公式プロバイダとサードパーティプロバイダを自由に切り替えられます。Codex では異なる公式プロバイダ間の切り替えに対応しており、複数の Plus アカウントや Team アカウントの切り替えに便利です。

</details>

<details>
<summary><strong>データはどこに保存されますか？</strong></summary>

- **データベース**: `~/.fyagent/fyagent.db`（SQLite -- プロバイダ、MCP、Prompts、Skills）
- **ローカル設定**: `~/.fyagent/settings.json`（デバイスレベルの UI 設定）
- **バックアップ**: `~/.fyagent/backups/`（自動ローテーション、最新 10 件を保持）
- **Skills**: `~/.fyagent/skills/`（デフォルトでシンボリックリンクにより対応アプリに接続）
- **Skill バックアップ**: `~/.fyagent/skill-backups/`（アンインストール前に自動作成、最新 20 件を保持）

</details>

<details>
<summary><strong>Linux（Wayland + NVIDIA）：Web コンテンツがクリックできない・リサイズで黒画面になる</strong></summary>

AppImage は過去のネイティブ Wayland クラッシュを避けるため `GDK_BACKEND=x11`（XWayland）を強制します。新しい Wayland + NVIDIA 環境ではこれが原因で Web コンテンツ領域がクリックできなくなり（タイトルバーのボタンは動作します）、リサイズ時に黒画面になることがあります。内蔵のエスケープハッチでネイティブ Wayland に戻せます：

```bash
FYAGENT_GDK_BACKEND=wayland ./FyAgent-*.AppImage
```

デスクトップアイコンから起動する場合は、`.desktop` の `Exec=` 行に追記するか（例：`env FYAGENT_GDK_BACKEND=wayland /path/to/AppImage`）、セッション環境で設定してください。この変数は汎用です：タイル型 Wayland コンポジタ（sway/Hyprland）でクリックが効かない場合は、逆に `FYAGENT_GDK_BACKEND=x11` を試してください。未設定の場合は既定の動作のままです。

</details>

## ドキュメント

各機能の詳しい使い方については、**[ユーザーマニュアル](docs/user-manual/ja/README.md)** をご覧ください。プロバイダ管理、MCP/Prompts/Skills、プロキシとフェイルオーバーなど、すべての機能を網羅しています。

## クイックスタート

### 基本的な使い方

1. **プロバイダ追加**: 「Add Provider」をクリック → プリセットを選ぶかカスタム設定を作成
2. **プロバイダ切り替え**:
   - メイン UI: プロバイダを選択 → 「Enable」をクリック
   - システムトレイ: プロバイダ名をクリック（即時反映）
3. **反映**: ターミナルまたは対応する CLI ツールを再起動して適用（Claude Code は再起動不要）
4. **公式設定に戻す**: 「Official Login」プリセットを追加し、CLI ツールを再起動してログイン/OAuth フローを実行

### MCP、Prompts、Skills & Sessions

- **MCP**: 「MCP」ボタンをクリック → テンプレートまたはカスタム設定でサーバーを追加 → アプリごとの同期をトグルで切り替え
- **Prompts**: 「Prompts」をクリック → Markdown エディタでプリセットを作成 → 有効化してライブファイルに同期
- **Skills**: 「Skills」をクリック → GitHub リポジトリを閲覧 → 対応アプリへワンクリックでインストール
- **Sessions**: 「Sessions」をクリック → 対応するセッションソースの会話履歴を閲覧・検索・復元

> **補足**: 初回起動時に、既存の CLI ツール設定を手動でインポートしてデフォルトプロバイダとして使用できます。

## ダウンロード & インストール

### システム要件

- **Windows**: Windows 10 以上
- **macOS**: macOS 12 (Monterey) 以上
- **Linux**: Ubuntu 22.04+ / Debian 11+ / Fedora 34+ など主要ディストリビューション

### Windows ユーザー

x64 Windows では `FyAgent-0.3.0-Windows.msi`、ARM64 Windows では
`FyAgent-0.3.0-Windows-arm64.msi` を
[Releases](https://github.com/NongHua123/fyagent/releases) から取得してください。
Windows ポータブル ZIP は配布せず、サポート対象の全マシン MSI パスを使用します。

> **未署名ビルド:** v0.3.0 の実行ファイルと MSI は Authenticode `NotSigned`
> であり、Windows SmartScreen が警告する場合があります。続行前に資産名、manifest、
> attestation 証拠を確認してください。SmartScreen を無効化したり、組織管理の
> セキュリティポリシーを弱めたりしないでください。

### macOS ユーザー

[Releases](https://github.com/NongHua123/fyagent/releases) から
`FyAgent-0.3.0-macOS.dmg`（推奨）または `FyAgent-0.3.0-macOS.zip` を
ダウンロードしてください。

> **未署名ビルド:** v0.3.0 は Developer ID 署名も公証もありません。一度アプリを
> 開こうとした後、**システム設定 → プライバシーとセキュリティ → このまま開く**を
> 使用して確認してください。Release 証拠を先に確認し、Gatekeeper の無効化や隔離
> 属性の削除は行わないでください。

### Linux ユーザー

[Releases](https://github.com/NongHua123/fyagent/releases) から現在の
アーキテクチャに対応するネイティブ Linux ビルドを取得してください：

- x64: `FyAgent-0.3.0-Linux-x86_64.AppImage`、
  `FyAgent-0.3.0-Linux-x86_64.deb`、
  `FyAgent-0.3.0-Linux-x86_64.rpm`
- ARM64: `FyAgent-0.3.0-Linux-arm64.AppImage`、
  `FyAgent-0.3.0-Linux-arm64.deb`、
  `FyAgent-0.3.0-Linux-arm64.rpm`

> **Flatpak**：公式リリースには含まれていません。`.deb` から自分でビルドできます — 手順は [`flatpak/README.md`](flatpak/README.md) を参照してください。

<details>
<summary><strong>v0.3.0 の正確な添付ファイル契約</strong></summary>

正式 Release に許可されるインストーラーは、上記 macOS 2、Windows MSI 2、
Linux 6 の **10 ファイル**だけです。証拠添付は、各インストーラーの SHA-256 を含む
`download-manifest.json`、`build-metadata.json`、
`artifact-attestation.sigstore.json` の 3 ファイルだけで、合計は **13** です。
workflow は欠落、重複、改名、余分なファイルを拒否します。正式 workflow と公開後の
独立検証が完了するまでは、これは公開済み結果ではなくリリース契約です。

</details>

<details>
<summary><strong>アーキテクチャ概要</strong></summary>

### 設計原則

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

**コア設計パターン**

- **SSOT** (Single Source of Truth): すべてのデータを `~/.fyagent/fyagent.db`（SQLite）に集約
- **二層ストレージ**: 同期データは SQLite、デバイスデータは JSON
- **双方向同期**: 切り替え時はライブファイルへ書き込み、編集時はアクティブプロバイダから逆同期
- **アトミック書き込み**: 一時ファイル + rename パターンで設定破損を防止
- **並行安全**: Mutex で保護された DB 接続でレースコンディションを防止
- **レイヤードアーキテクチャ**: Commands → Services → DAO → Database を明確に分離

**主要コンポーネント**

- **ProviderService**: プロバイダの CRUD、切り替え、バックフィル、ソート
- **McpService**: MCP サーバー管理、インポート/エクスポート、ライブファイル同期
- **ProxyService**: ローカル Proxy モードのホットスイッチとフォーマット変換
- **SessionManager**: 対応する全アプリの会話履歴閲覧
- **ConfigService**: 設定のインポート/エクスポート、バックアップローテーション
- **SpeedtestService**: API エンドポイントの遅延計測

</details>

<details>
<summary><strong>開発ガイド</strong></summary>

### 開発環境

- グローバルにインストールした [mise](https://mise.jdx.dev/getting-started.html)
  2026.8.0 以降
- [Tauri 2.0 のシステム要件](https://v2.tauri.app/start/prerequisites/)

Node.js 24.19.0、pnpm 10.12.3、Rust 1.97.1、Python 3.14.7 はそれぞれ
`.node-version`、`package.json`、`rust-toolchain.toml`、`.python-version` に
固定されています。`mise.toml` はタスク API と uv selector を管理し、`mise.lock`、
`uv.lock`、uv 管理の `.venv` が承認済み Python 環境を固定します。Tauri CLI は
プロジェクト依存関係としてインストールされます。

リポジトリ設定を確認した後、開発環境を初期化します：

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

`mise trust` は開発者自身のセキュリティ判断であり、プロジェクトタスクが自動実行する
ことはありません。`bootstrap` は権限昇格が必要なシステムパッケージの導入、Git remote
変更、lock 更新、公開を行いません。WSL では `/mnt/<drive>` または Windows shim の
管理対象ツールを使用しないでください。完全な API は生成済みの
[canonical task catalog](docs/fyagent/development/mise-tasks.md) を参照してください。

### ホストネイティブビルド

ローカル開発とパッケージングは、現在のホスト OS のみをサポートします。標準コマンドは
別の OS やアーキテクチャのターゲットを受け付けません：

```bash
mise run dev
mise run build
```

FyAgent のインストーラーは、GitHub Actions のネイティブな Windows x64/ARM64、
Linux x64/ARM64、macOS runner でのみ構築されます。macOS job は Universal build を
生成します。Linux/WSL から Windows または macOS をローカルでパッケージする方法は、
サポート対象のリリース経路ではありません。

### 開発コマンド

```bash
# ロック済み依存関係を導入し環境を検証
mise run bootstrap

# ホットリロード付き開発モード
mise run dev

# 型チェック
mise run typecheck

# コード整形
mise run format

# フォーマット検証
mise run format:check

# フロントエンド単体テスト
mise run test:unit

# ウォッチモード（開発に推奨）
mise run test:unit:watch

# アプリをビルド
mise run build

# デバッグビルド
mise run build:debug
```

### Rust バックエンド開発

```bash
# Rust コード整形
mise run rust:fmt

# clippy チェック
mise run rust:clippy

# バックエンドテスト
mise run rust:test

# 特定テストのみ実行
mise run rust:test test_name

# PR 前に現在のホスト向け全ゲートを実行
mise run check
```

### テストガイド

**フロントエンドテスト**:

- テストフレームワークに **vitest** を使用
- **MSW (Mock Service Worker)** で Tauri API 呼び出しをモック
- コンポーネントテストに **@testing-library/react** を採用

**テスト実行**:

```bash
# 全テストを実行
mise run test:unit

# ウォッチモード（自動再実行）
mise run test:unit:watch

# フロントエンド全ゲート
mise run check:frontend
```

### 技術スタック

**フロントエンド**: React 18 · TypeScript · Vite · TailwindCSS 3.4 · TanStack Query v5 · react-i18next · react-hook-form · zod · shadcn/ui · @dnd-kit

**バックエンド**: Tauri 2.8 · Rust · serde · tokio · thiserror · tauri-plugin-process/dialog/store/log

**テスト**: vitest · MSW · @testing-library/react

</details>

<details>
<summary><strong>プロジェクト構成</strong></summary>

```
├── src/                        # フロントエンド (React + TypeScript)
│   ├── components/
│   │   ├── providers/          # プロバイダ管理
│   │   ├── mcp/                # MCP パネル
│   │   ├── prompts/            # Prompts 管理
│   │   ├── skills/             # Skills 管理
│   │   ├── sessions/           # Session Manager
│   │   ├── proxy/              # Proxy モードパネル
│   │   ├── openclaw/           # OpenClaw 設定パネル
│   │   ├── settings/           # 設定 (Terminal/Backup/About)
│   │   ├── deeplink/           # Deep Link インポート
│   │   ├── env/                # 環境変数管理
│   │   ├── universal/          # クロスアプリ設定
│   │   ├── usage/              # 使用量統計
│   │   └── ui/                 # shadcn/ui コンポーネントライブラリ
│   ├── hooks/                  # カスタムフック（ビジネスロジック）
│   ├── lib/
│   │   ├── api/                # Tauri API ラッパー（型安全）
│   │   └── query/              # TanStack Query 設定
│   ├── locales/                # 翻訳 (zh/zh-TW/en/ja)
│   ├── config/                 # プリセット (providers/mcp)
│   └── types/                  # TypeScript 型定義
├── src-tauri/                  # バックエンド (Rust)
│   └── src/
│       ├── commands/           # Tauri コマンド層（ドメイン別）
│       ├── services/           # ビジネスロジック層
│       ├── database/           # SQLite DAO 層
│       ├── proxy/              # Proxy モジュール
│       ├── session_manager/    # セッション管理
│       ├── deeplink/           # Deep Link 処理
│       └── mcp/                # MCP 同期モジュール
├── tests/                      # フロントエンドテスト
└── assets/                     # スクリーンショット
```

</details>

## 貢献

Issue や提案を歓迎します！

PR を送る前に以下をご確認ください：

- 現在のホスト向け全ゲートを実行: `mise run check`
- 開発中は [canonical task catalog](docs/fyagent/development/mise-tasks.md) から
  集中的なタスクを選択

新機能の場合は、PR を送る前に Issue でディスカッションしてください。プロジェクトに合わない機能の PR はクローズされる場合があります。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=NongHua123/fyagent&type=Date)](https://www.star-history.com/#NongHua123/fyagent&Date)

## ライセンス

FyAgent はソースを利用可能なソフトウェアであり、OSI が定義するオープンソースではありません。
FyAgent に帰属するコンポーネントおよび変更部分には
[PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt)
が適用されます。商用利用には別途書面による許可が必要です。CC Switch 由来の部分は
MIT ライセンスのままです。詳しくは [LICENSE](LICENSE)、[LICENSING.md](LICENSING.md)、
および [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) を参照してください。
