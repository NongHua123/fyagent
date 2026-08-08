# Flatpak Diagnostic Build Guide

This directory contains the `com.fyagent.desktop` Flatpak manifest. It converts
a current-host Linux DEB into a local diagnostic `.flatpak`; Flatpak is **not**
one of the ten formal FyAgent 0.3.0 installer assets and this path does not
produce Release evidence.

## Prerequisites

Review and initialize the repository development environment:

```bash
mise trust
mise run bootstrap
mise run system:check
```

`mise trust` is an explicit developer decision and no task runs it
automatically. Install Flatpak tooling and the GNOME 46 runtime with the host
package manager. For Ubuntu/Debian:

```bash
sudo apt install flatpak flatpak-builder
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install -y --user flathub org.gnome.Platform//46 org.gnome.Sdk//46
```

## Local Diagnostic Build

Run the canonical native build on a Linux host:

```bash
mise run build
```

Locate the DEB under `src-tauri/target/release/bundle/deb/`, verify that exactly
one package for the current host architecture was generated, and copy that
reviewed file to `flatpak/fyagent.deb`. Do not select a package with `head`, a
wildcard that could hide duplicates, or an opposite-architecture target.

Build and export the local diagnostic package:

```bash
flatpak-builder --force-clean --user --disable-cache --repo flatpak-repo flatpak-build flatpak/com.fyagent.desktop.yml
flatpak build-bundle --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo flatpak-repo FyAgent-Linux.flatpak com.fyagent.desktop
```

Install and run it locally:

```bash
flatpak install --user ./FyAgent-Linux.flatpak
flatpak run com.fyagent.desktop
```

## Permission Boundary

The current manifest grants `--filesystem=home` for compatibility with managed
CLI configuration files and the directory-override feature. A Flathub or
security-hardening change must replace it with reviewed least-privilege paths
and prove user-data and CLI compatibility. For example:

```yaml
- --filesystem=~/.fyagent:create
- --filesystem=~/.claude:create
- --filesystem=~/.claude.json
- --filesystem=~/.codex:create
- --filesystem=~/.gemini:create
- --filesystem=~/.config/opencode:create
- --filesystem=~/.openclaw:create
```

Flatpak's `:create` modifier applies to directories, not files, so
`~/.claude.json` requires a separately reviewed creation strategy. Permission
changes must not silently narrow existing data-path or backup compatibility.
