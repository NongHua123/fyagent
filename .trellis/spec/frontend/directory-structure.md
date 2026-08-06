# Directory Structure

The renderer is a single React application rooted in `src/` and hosted by the
Tauri desktop shell. Keep new frontend code in the existing layer that owns
its responsibility; this repository does not use a route-per-folder or a
feature-package layout.

## Current Layout

```text
src/
|- main.tsx                 # renderer bootstrap and provider composition
|- App.tsx                  # application shell and top-level view selection
|- components/
|  |- ui/                   # reusable Radix/Tailwind primitives
|  |- theme-provider.tsx    # renderer-wide Theme Context provider
|  |- topbar/               # stable responsive application chrome
|  |- providers/            # provider panels, dialogs, and local helpers
|  `- mcp/                  # MCP feature UI and co-located hooks
|- hooks/                   # cross-feature React hooks
|- lib/
|  |- api/                  # typed Tauri command facades
|  |- layout/               # pure window-layout policy and host-sync hook
|  |- query/                # TanStack Query client and resource hooks
|  `- schemas/              # Zod form schemas
|- config/                  # static application and provider presets
|- i18n/ and icons/          # locale registration and bundled icon assets
|- types/ and types.ts      # shared domain and feature types
`- utils/                   # non-React helpers
```

## Placement Rules Observed in the Codebase

- Put reusable primitives in `src/components/ui/`. The files are lower-case,
  following the shadcn/Radix-style family already present there.
- Put domain UI with its owning feature. Provider forms, their shared
  subcomponents, and their private hooks are co-located under
  `src/components/providers/forms/`.
- Keep a hook in `src/hooks/` when it is reused outside one feature. A hook
  used only by a feature may sit with that feature, as
  `src/components/providers/forms/hooks/useApiKeyState.ts` does.
- Feature-level Tauri calls are generally grouped in `src/lib/api/`. Keep direct
  `invoke` narrowly scoped to renderer bootstrap or native-only UI utilities,
  such as `src/main.tsx`, `src/components/theme-provider.tsx`, and
  `src/components/DatabaseUpgrade.tsx`; do not treat those exceptions as a
  general feature-call pattern. Keep query/cache behavior in `src/lib/query/`
  and runtime form validation in `src/lib/schemas/`.
- Follow the nearby naming family rather than imposing one global filename
  rule: domain components commonly use PascalCase (`ProviderCard.tsx`), while
  hooks use `use`-prefixed camelCase filenames (`useTauriEvent.ts`).

## Test Placement

Most renderer tests mirror their subject under `tests/` (`tests/components/`,
`tests/hooks/`, `tests/lib/`, and `tests/integration/`). A small number of
pure utilities have adjacent `*.test.ts` files under `src/utils/`. Match the
nearest existing test family for the code being changed.

`tests/desktop-acceptance/` owns mock-only desktop acceptance contracts and
fixtures. `tests/e2e/visual-baselines/` owns the candidate-only visual-baseline
manifest and LFS-backed assets; it is not a locally runnable real-desktop E2E
runner.

## Evidence

- [src/main.tsx](../../../src/main.tsx) composes renderer-wide providers before
  rendering `App`.
- [src/components/providers/forms/ProviderForm.tsx](../../../src/components/providers/forms/ProviderForm.tsx)
  imports feature-local components and the `./hooks` barrel from one domain
  subtree.
- [src/lib/api/providers.ts](../../../src/lib/api/providers.ts) is the typed
  Tauri facade used by renderer hooks and components.
- [src/lib/api/proxy.ts](../../../src/lib/api/proxy.ts) is the typed proxy
  facade consumed by `useProxyStatus`.
- [src/components/theme-provider.tsx](../../../src/components/theme-provider.tsx)
  shows a narrow native-only direct-`invoke` boundary.
