# Frontend Quality Guidelines

## Canonical checks

Use repository tasks:

```text
mise run typecheck
mise run format:check
mise run test
mise run check:frontend
```

`test` is read-only and includes unit, i18n parity, desktop mock, and visual preflight checks. Watch mode and visual-baseline update are interactive/mutating and never part of `check` or CI.

## Test environment

Vitest runs with jsdom and shared setup for Testing Library, i18n, MSW, cleanup, and mock reset. Component tests prefer accessible roles and user-visible behavior. Query tests disable retries unless retry behavior is the subject under test.

Desktop mock tests prove only the mocked Tauri contract. They do not prove a real window, installer, OS integration, signing, or publication.

## Native Fetch and Node deprecations

The repository-pinned Node runtime provides `fetch`, `Headers`, `Request`, and `Response`. Tests MUST NOT import `cross-fetch/polyfill` or add `cross-fetch`, `node-fetch`, `isomorphic-fetch`, `whatwg-fetch`, or an equivalent fallback solely to support an undeclared Node version.

A focused integration probe executes the real chain:

```text
Tauri invoke mock -> native fetch -> MSW handler -> Response parsing
```

Ordinary Node test/contract commands use `--throw-deprecation`. The native Fetch/MSW probe also enables `--pending-deprecation` so a dependency-level `DEP0040` regression is not hidden by Node's default application-deprecation filtering.

Forbidden suppression includes `NODE_NO_WARNINGS`, `--no-warnings`, `--no-deprecation`, `--disable-warning=DEP0040`, stderr filtering, and catch-and-ignore behavior.

The dependency contract verifies that the obsolete chain is absent:

```text
cross-fetch 4.1.0
└─ node-fetch 2.7.0
   └─ whatwg-url 5.0.0
      └─ tr46 0.0.3
```

Newer packages with the same names may remain when required by legitimate dependencies; checks use package name, version, and reverse-dependency origin rather than blanket string bans.

## Localization and accessibility

Every user-visible string uses the translation system. Registered locale files maintain exact leaf-key parity. Shared accessible primitives preserve focus-visible styling, labels, descriptions, and keyboard behavior.

## Tests Required

- strict TypeScript and formatting;
- full unit/i18n/desktop mock/visual preflight set;
- native Fetch globals contract and behavioral MSW probe;
- deprecation gates and no-suppression repository scan;
- lockfile/reverse-dependency proof that the old punycode chain exited;
- locale key parity and affected accessible interaction tests.
