# Application Brand Asset Contract

## 1. Scope / Trigger

Read this contract before changing the FyAgent application icon, regenerating
Tauri icons, changing the About icon, or editing the macOS tray template. It
does not apply to provider, partner, Claude, OpenAI, screenshot, or DMG
background assets.

The application icon crosses renderer, Tauri bundle, Flatpak, Windows shell,
and macOS menu-bar boundaries. A valid change updates every consumer from one
approved source while preserving the established FyAgent application identity
and unrelated artwork.

## 2. Signatures

The current source and generation entry point are:

```text
source:  assets/fyagent.png
format:  PNG, 1024x1024, RGBA with transparency
command: pnpm tauri icon assets/fyagent.png --output src-tauri/icons
```

The direct consumers are:

```text
src-tauri/tauri.conf.json                         Tauri bundle icon list
flatpak/com.fyagent.desktop.yml                  128x128 Flatpak icon
src/assets/icons/app-icon.png                     renderer About icon
src-tauri/src/lib.rs                              embedded macOS 3x tray template
src-tauri/icons/tray/macos/statusTemplate.png     1x template
src-tauri/icons/tray/macos/statusTemplate@2x.png  2x template
src-tauri/icons/tray/macos/statusbar_template_3x.png 3x template
```

## 3. Contracts

- Preserve the approved source bytes in `assets/fyagent.png`; do not redraw,
  recolor, crop, or recomposite the color application icon.
- Use the repository's Tauri CLI to generate the standard desktop, Windows
  Store, Android, and iOS files. Do not hand-maintain parallel resizers for
  those outputs.
- Keep every existing generated path, including `64x64.png`, unless a reviewed
  Tauri/toolchain migration explicitly changes the inventory.
- Copy `src-tauri/icons/32x32.png` byte-for-byte to
  `src/assets/icons/app-icon.png` for the About surface.
- macOS template images are the technical monochrome exception to the color
  preservation rule. Crop to the source alpha bounds, fit proportionally in
  an 18pt content box centered on a 24pt canvas, and emit black RGBA at 24,
  48, and 72 pixels. Preserve antialiased alpha; Tauri/macOS supplies the
  light/dark rendered color.
- Do not change `src-tauri/icons/dmg-background.png`, provider or partner
  artwork, screenshots, the established FyAgent `identifier`, deep-link
  schemes, data directories, internal package names, or `LICENSE` as part of a
  future icon-only update. The 2026 clean-break rename is an application
  identity change, not an icon-generation rule.

## 4. Validation & Error Matrix

| Condition                                                                         | Required result                                                                     |
| --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Source is missing, not square 1024px RGBA, or lacks transparency                  | Stop before generation                                                              |
| Preserved source differs from the approved input                                  | Reject the change                                                                   |
| A previously tracked generated icon path is missing                               | Reject the inventory                                                                |
| A generated PNG, ICO, or ICNS container cannot be decoded                         | Reject the output                                                                   |
| About icon differs from generated `32x32.png`                                     | Reject the renderer asset                                                           |
| Tray template has the wrong size, non-black visible RGB, or no partial alpha      | Reject the template                                                                 |
| Provider, partner, screenshot, or DMG background appears in the diff              | Remove it from the icon change                                                      |
| Static/build checks pass but native shell or Dock appearance is unobserved        | Keep native visual acceptance pending                                               |
| A regenerated ICNS container differs byte-for-byte but decoded sizes/pixels match | Accept only with decoded-image evidence; container bytes are not a stable assertion |

## 5. Good / Base / Bad Cases

- Good: one approved RGBA source regenerates all Tauri outputs, the About copy
  matches 32px exactly, the three tray templates pass their mask contract, and
  only application-brand files change.
- Base: a future approved source replaces `assets/fyagent.png`; the same
  generation and validation flow runs without changing consumer paths.
- Bad: only `icon.ico` is replaced, the color bitmap is embedded as a macOS
  template, or a broad image-directory rewrite modifies provider artwork.

## 6. Tests Required

- Decode the source and all generated PNG files; assert dimensions, mode, and
  alpha behavior.
- Enumerate ICO sizes and assert the expected Windows frames. Decode ICNS sizes
  through 1024px; compare decoded content rather than raw ICNS bytes when
  testing regeneration determinism.
- Assert the About file is byte-identical to `32x32.png`, all configured paths
  resolve, and the Flatpak source points to the generated 128px icon.
- Assert each tray template size, visible RGB, alpha range, and centered content
  bounds.
- Compare the diff/inventory against the pre-change checkout and assert the
  exclusion assets are unchanged.
- Run `pnpm format:check`, `pnpm typecheck`, `pnpm build:renderer`,
  `cargo check --manifest-path src-tauri/Cargo.toml`, and a desktop bundle build
  appropriate to the host platform.
- Keep Windows installer/shortcut/taskbar/window and macOS Finder/Dock/app
  switcher/menu-bar inspection as explicit manual acceptance with screenshots.

## 7. Wrong vs Correct

Wrong:

```text
Copy one PNG over icon.png and assume every package surface inherits it.
```

Correct:

```text
Preserve the approved source, run the Tauri generator, derive the About and
macOS template assets, validate every consumer, then perform native visual
acceptance separately.
```
