# Quality Guidelines

## Reproducible Frontend Checks

The currently runnable frontend checks declared by `package.json` are:

```bash
mise exec -- pnpm typecheck
mise exec -- pnpm format:check
mise exec -- pnpm test:unit
```

Run local checks through the repository's
[mise environment](../backend/development-environment.md). Do not report a
frontend command as a successful project check unless `package.json` declares
it.

## Test Setup and Patterns

Vitest runs in `jsdom` and loads `tests/setupGlobals.ts` plus
`tests/setupTests.ts`. The shared setup installs Testing Library matchers,
initializes a minimal i18n instance, starts MSW, cleans up rendered trees, and
resets handlers/mocks after each test.

Component tests use React Testing Library (`render`, `screen`, events, and
role-based queries). Hook tests use `renderHook` and `act`. Tests that need
TanStack Query create a client with retries disabled so failures are immediate.

```tsx
// tests/utils/testQueryClient.ts
export const createTestQueryClient = () =>
  new QueryClient({ defaultOptions: { queries: { retry: false } } });
```

Test organization is primarily mirrored under `tests/components/`,
`tests/hooks/`, `tests/lib/`, `tests/config/`, and `tests/integration/`.
Use the closest existing test as the fixture/mocking model for the behavior
being changed; this repository has no documented universal coverage threshold.

## UI Text and Accessible Primitives

When a renderer change adds or changes user-visible text, use `t(...)` and
update the four locales registered by `src/i18n/index.ts`:

```text
src/i18n/locales/en.json
src/i18n/locales/ja.json
src/i18n/locales/zh.json
src/i18n/locales/zh-TW.json
```

Shared primitives already carry focus-visible styling and form ARIA linkage.
Preserve those properties when editing them, and test interactive behavior
through accessible roles where the nearby tests do so.

## Evidence

- [package.json](../../../package.json) defines the runnable type-check,
  formatting, and unit-test scripts.
- [vitest.config.ts](../../../vitest.config.ts) configures the `jsdom`
  environment and shared setup files.
- [tests/setupTests.ts](../../../tests/setupTests.ts) manages Testing Library,
  i18n, MSW, cleanup, and mock reset lifecycle.
- [Development Environment](../backend/development-environment.md) owns local
  runtime versions and command execution.
