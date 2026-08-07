# Eliminate DEP0040 punycode root cause — PRD

## Goal

Remove the obsolete test dependency chain that triggers Node DEP0040 and prove native Fetch/MSW behavior under strict deprecation gates.

## Scope

- remove `cross-fetch/polyfill` import and direct dependency
- use Node 24 Fetch globals
- regenerate pnpm lock normally
- prove old reverse-dependency chain exits
- add native Fetch/MSW behavior probe
- add deprecation and no-suppression contracts

## Constraints

- no replacement polyfill or direct undici dependency
- no warning suppression
- do not treat Node 24 default silence as proof
- keep upstream merge commit untouched

## Acceptance Criteria

- [ ] package/source/lock no longer contain the obsolete path
- [ ] native Fetch globals and real MSW request pass
- [ ] ordinary `--throw-deprecation` tests pass
- [ ] pending+throw focused probe passes
- [ ] repository suppression scan is clean

## Evidence Boundary

This task begins in `planning`. Nothing in this artifact claims the merge, configuration, tests, CI, or Release has already been completed. Pending platform/Git evidence must be attached during implementation.
