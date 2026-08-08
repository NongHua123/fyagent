# DEP0040 remediation evidence

Date: 2026-08-08
Branch: `codex/fyagent-v0.3.0`
Implementation commit: `4e407df4`

## Dependency and runtime result

- Removed the direct `cross-fetch@4.1.0` dependency and `cross-fetch/polyfill` import without adding another Fetch polyfill or direct `undici` dependency.
- Regenerated `pnpm-lock.yaml` with pnpm `10.12.3`; frozen offline installation succeeds.
- The obsolete `cross-fetch -> node-fetch@2.7.0 -> whatwg-url@5.0.0 -> tr46@0.0.3` path is absent from both the lockfile and actual `pnpm why --json` graph.
- The legitimate jsdom path remains and is origin-checked as `jsdom@25.0.1 -> whatwg-url@14.2.0 -> tr46@5.1.1 -> punycode@2.3.1`.
- The machine-readable DEP0040 report verifies exact Node `24.19.0`, native `fetch`/`Headers`/`Request`/`Response`, 519 active module files, 2,812 module specifiers, canonical pnpm package/snapshot agreement, reviewed reverse origins, and 68 executable configuration surfaces without suppression.

## Behavior and deprecation result

- Native Fetch through MSW and the Tauri mock passes four focused cases: JSON success with invocation evidence, non-2xx text error, HTTP 204 mapped to `undefined`, and cross-realm JSDOM `Headers` with reset/non-leakage evidence.
- The focused command runs with `--pending-deprecation --throw-deprecation`; ordinary unit, i18n, desktop mock, and visual commands run with `--throw-deprecation`.
- The dependency check treats the Node pending probe as supplemental evidence only. Lock and reverse-dependency graph checks are the fail-closed proof because Node `24.19.0` did not reliably surface the historical node_modules warning in the preliminary probe.
- Executable configuration scanning rejects warning suppression, including common statically composed JavaScript strings and shell/PowerShell/batch/Python script surfaces.

## Verification

- DEP0040 JSON report: 7/7 checks passed.
- Focused Native Fetch: 4/4 tests passed.
- DEP0040 and CI contracts: 13/13 tests passed.
- CI-safe Release contracts with a failing `mise` PATH probe: 67/67 tests passed.
- Local Release contracts: 106/106 tests plus the focused 4/4 probe passed.
- Full unit suite: 140 files / 963 tests passed.
- Desktop mock: 7/7; locale parity: 3/3; visual preflight remained read-only and passed.
- `mise run typecheck`, formatting checks, frozen lock verification, task validation, and Trellis validation passed.
- The final second `mise run check` exited 0, including Rust format/check/Clippy and 2,627 Rust tests.
- Independent Trellis review fixed the exact-Node precondition, AST parse fail-closed behavior, pnpm package/snapshot structural checks, reverse-origin/alias validation, suppression composition coverage, and cross-realm cleanup evidence.

The first full check observed one unrelated download-rate timing flake. Its focused file reran 34/34 successfully, and the second complete check passed; no unrelated implementation was added.

## Remaining parent gates

Real Windows, macOS, Linux ARM64, PR/main Actions, and formal Release evidence remain parent-level gates. They do not change the completed dependency, behavior, deprecation, and suppression acceptance criteria for this child.
