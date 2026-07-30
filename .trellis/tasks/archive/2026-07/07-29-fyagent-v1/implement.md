# FyAgent V1 实施计划

## Preconditions

- 用户对本次最终规划摘要作出后续明确批准。
- 创建并记录 `feature/fyagent-v1` 集成分支（该名称由 V1 文档明确要求），保持
  `main` 不被本轮直接修改。
- 复核当前 commit、工具链、镜像 schema/checksum，并取得受控的匿名 Windows
  Publisher/manifest 签名 fixture；若证据无法获得，先完成不依赖它的契约与测试，
  但不得放宽 identity gate 或宣称 Windows 完整。
- 在每次修改常量或现有值前按项目指南先执行精确 `rg` 搜索，避免遗漏镜像、品牌、
  updater 或 CLI 生命周期的联动位置。

## Windows Publisher Evidence Record

- 2026-07-29，使用本机 Windows PowerShell 5.1 的只读
  `Get-AppxPackage` / `Get-AppxPackageManifest` 查询到当前用户已安装包：
  `OpenAI.Codex_26.721.4979.0_x64__2p2nqsd0c76g0`；其 Identity 为
  `Name=OpenAI.Codex`、`Publisher=CN=50BDFD77-8903-4850-9FFE-6E8522F64D5B`、
  `Version=26.721.4979.0`、`ProcessorArchitecture=x64`。
- 同一系统查询返回 `SignatureKind=Store`、`Status=Ok`、
  `IsDevelopmentMode=False`，且 Publisher ID 为 `2p2nqsd0c76g0`。
- 同日只读核验 agentsmirror 的 `/latest/manifest` 与 `/latest/checksums`：其 x64
  `packageMoniker`、版本和 Package Family Name 后缀与上述已安装包一致；MSIX 文件名
  `OpenAI.Codex_26.721.4979.0_x64__2p2nqsd0c76g0.Msix` 的 SHA-256 为
  `f0c1d75045952a11a581d34f28f595d1d110fb13f8f7e5c5201802ed2bbd7093`。
- 未下载、部署或修改完整生产 MSIX。代码仅采用该精确 Publisher 作为 allowlist，
  仍由 PackageManager 在部署时执行 Windows 签名/信任链验证；Publisher 变化继续
  fail closed，必须重新取得并审阅同等证据后才可更新常量。

## Ordered Execution

1. **M0 — Baseline and fixtures** (`fyagent-v1-core`)
   - Record baseline/toolchain in task evidence; add redacted manifest/checksum and package metadata
     fixtures only, never complete MSIX/DMG.
   - Add schema v5 parser fixtures including wrong/missing/cross-architecture data and checksum
     malformed cases.
   - Run baseline typecheck/unit/Rust tests once and record pre-existing failures separately.

2. **M1 — Domain contracts and job state** (`fyagent-v1-core`)
   - Create `src-tauri/src/codex_desktop/{types,error,platform,mod}.rs` and service Job modules.
   - Implement platform versions, canonical release ID, stable errors/redaction, safe unsupported
     adapter, legal transitions, sequence snapshots, single-job mutex and cancel/install race.
   - Add pure Rust unit tests and Rust/TS JSON contract fixture.
   - Review gate: all platform-specific details remain out of shared DTOs; ordinary request has only
     `expected_release_id`.

3. **M2 — Source, download and shared validation** (`fyagent-v1-core`)
   - Implement fixed endpoint selection, raw manifest checksum-before-parse, schema validation,
     descriptor cache/revalidation, manual HTTPS redirect policy, retries, progress/cancel,
     bounded `.part` writes, size/hash validation, three-times disk preflight and safe cleanup.
   - Extend existing proxy facilities only via a scoped installer client builder, preserving existing
     singleton behavior.
   - Validate with mock transport/filesystem/clock; no Internet-dependent tests.
   - Review gate: no external URL from metadata reaches download/API/diagnostics unredacted.

4. **M3 — Windows adapter** (`fyagent-v1-windows`, after M1 contract)
   - Build bounded MSIX manifest parser and exact package identity/Publisher/arch/version/min-OS
     checks using fixtures.
   - Implement fakeable PackageManager local inventory, current-user local `file://` deploy,
     progress/HRESULT mapping, post-check and AUMID launch.
   - Add restricted all-users headless/elevation tests separate from normal commands.
   - Review gate: ordinary UI/IPC cannot pass scope/URL/path or trigger UAC; no direct exe,
     PowerShell/winget or force shutdown.

5. **M4 — macOS adapter** (`fyagent-v1-macos`, after M1 contract)
   - Build standard-directory scanning, Bundle ID/Team/signature/Gatekeeper/arm64 validation and
     fake command runner.
   - Implement DMG mount guard, unique bundle discovery, target policy, permission-only fallback,
     same-volume staging/backup/swap/restore, post-check and verified-path launch.
   - Add fixture and fake command/filesystem test matrix without writing real Applications.
   - Review gate: no name-based identity, xattr/codesign modification, sudo/helper, kill or
     `open -a` behavior.

6. **M5 — Service and IPC integration** (`fyagent-v1-core`, after M2 and platform trait work)
   - Add `CodexDesktopService` orchestration, AppState lifecycle, event emitter, temp cleanup and
     seven thin commands.
   - Register shared modules only in `fyagent-v1-integration` after isolated code is reviewed.
   - Add fake source/platform service tests for success, metadata change, retries, cancellation,
     failures, post-install mismatch, local launch during remote failure and stale events.

7. **M6 — Frontend installer experience** (`fyagent-v1-ui`, after IPC DTO fixture)
   - Add types, invoke API, Query, Hook, Card, minimal Provider page mount and zh/en keys.
   - Test state matrix, event/query race, cancel boundaries, exactly-once toast, remote/local
     separation, Linux/Intel behavior, accessibility and absence of prohibited controls.
   - In the local-older/update-available state, expose only the `Update Codex` primary action;
     do not render a secondary launch action.
   - Review gate: `App.tsx` contains only mounting, no invoke/state machine/platform logic.

8. **M7 — Codex CLI read-only behavior** (`fyagent-v1-integration`)
   - Centralize lifecycle capability decision; reject Codex write operations in Rust, delete/hide
     front-end actions/commands and exclude Codex from bulk lists while preserving other tools.
   - Add regression tests for UI and direct command behavior.

9. **M8 — FyAgent branding and updater removal** (`fyagent-v1-integration`)
   - Change visible product/window names, remove visible upstream links, updater plugin/config/
     command/UI imports, capability permissions and update artifact generation in the release
     workflow, leaving legal notices and internal identity.
   - Replace the database-version recovery branch's updater actions with a translated, no-network
     incompatibility/support message; retain the existing safety block and do not mutate the
     database.
   - Search all registered locales and current updater consumers before patching; audit output is
     reviewed rather than relying on a broad replace.

10. **M9 — Exit protection and diagnostics** (`fyagent-v1-integration`)
    - Attach installer-state-aware exit guard to the existing close coordination point, not a second
      global listener.
    - Finalize copy-safe diagnostics, log opener, warning semantics, progress throttling and stale
      temp cleanup tests.

11. **M10 — Integration quality gate** (`fyagent-v1-integration`)
    - Rebase/cherry-pick in Core → Windows → macOS → UI order, then add shared registrations and
      resolve contracts intentionally.
    - Run all required commands, per-platform compilation where CI/config permits, format checks,
      static `rg` audit and a code-quality/security review.
    - Repair only issues traceable to V1 requirements; do not fold unrelated refactors into the
      integration branch.

12. **M11 — Manual acceptance package** (`fyagent-v1-integration`, human-owned execution)
    - Prepare `14-MANUAL-ACCEPTANCE.md` records for Windows x64, Windows ARM64, macOS Apple
      Silicon and the mainland-network scenario.
    - Mark them pending human execution. Never automate real install, UAC, `/Applications` writes,
      uninstall or process termination.

## Validation Matrix

| Layer | Required evidence |
| --- | --- |
| Frontend | `pnpm install --frozen-lockfile`, `pnpm typecheck`, `pnpm format:check`, `pnpm test:unit` |
| Rust shared | `cargo fmt --check --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml` |
| Platform | Windows/macOS CI compilation and fixture/mock suites; Linux shared build remains clean |
| Static safety | Exact audit for non-agentsmirror production URLs, custom inputs, all-users IPC, lifecycle commands, updater, visible CC Switch branding, unsafe platform fallbacks and secrets in diagnostics |
| Manual | Prepared acceptance records only; actual platform signing is explicitly pending a human |

Windows ARM64 is an explicit integration sub-gate: obtain and record a suitable target `cargo check`
or runner evidence after target-specific dependencies are added. The existing x64 Windows CI result
does not satisfy this requirement by inference.

## Rollback Points

- Each milestone is an atomic commit. If M3/M4 fails, revert only its owned commit(s) and retain a
  truthful unsupported state while fixing the platform.
- Shared registration/Cargo/updater/brand patches are deferred to integration to avoid partial
  coupling; revert them as one coherent patch if startup regression occurs.
- Runtime installer failure never mutates database/settings or official package contents; macOS
  swap has a same-run backup restore path and cleanup is bounded to validated temp/staging paths.
