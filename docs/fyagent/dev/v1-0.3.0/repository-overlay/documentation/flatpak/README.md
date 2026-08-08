# Flatpak Build Guide

This directory contains the `com.fyagent.desktop` Flatpak manifest. Local work
converts a current-host Linux `.deb` into a diagnostic `.flatpak`; it is not a
formal FyAgent Release artifact.

## Prerequisites

Initialize the repository development environment:

```bash
mise trust
mise run bootstrap
mise run system:check
```

Install Flatpak tooling and the required GNOME runtime with the host package
manager. For Ubuntu/Debian:

```bash
sudo apt install flatpak flatpak-builder
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install -y --user flathub org.gnome.Platform//46 org.gnome.Sdk//46
```

## Local Diagnostic Build

Build the current-host Debian bundle through the canonical native build task:

```bash
mise run build --bundles deb
```

The task implementation must validate that `deb` is a current-host bundle flag,
not a cross-OS target. Copy the produced package:

```bash
cp "$(find src-tauri/target/release/bundle -name '*.deb' | head -n 1)" flatpak/fyagent.deb
```

Build and export:

```bash
flatpak-builder --force-clean --user --disable-cache --repo flatpak-repo flatpak-build flatpak/com.fyagent.desktop.yml
flatpak build-bundle --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo flatpak-repo FyAgent-Linux.flatpak com.fyagent.desktop
```

Install/run:

```bash
flatpak install --user ./FyAgent-Linux.flatpak
flatpak run com.fyagent.desktop
```

## Permissions

The current manifest grants `--filesystem=home` for compatibility with CLI
configuration and directory overrides. A Flathub/security-hardening change must
replace it with reviewed least-privilege paths and verify creation semantics.
For example:

```yaml
  - --filesystem=~/.fyagent:create
  - --filesystem=~/.claude:create
  - --filesystem=~/.claude.json
  - --filesystem=~/.codex:create
  - --filesystem=~/.gemini:create
  - --filesystem=~/.config/opencode:create
  - --filesystem=~/.openclaw:create
```

Flatpak `:create` applies to directories, not files; `~/.claude.json` needs a
separate creation strategy. Permission changes require explicit user-data and
CLI compatibility tests.
