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
|  |- providers/            # provider panels, dialogs, and local helpers
|  `- mcp/                  # MCP feature UI and co-located hooks
|- hooks/                   # cross-feature React hooks
|- contexts/                # small shared React Context providers
|- lib/
|  |- api/                  # typed Tauri command facades
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
- Feature-level Tauri calls are often grouped in `src/lib/api/`; existing
  bootstrap and proxy paths also call `invoke` directly in `src/main.tsx` and
  `src/hooks/useProxyStatus.ts`. Match the nearby ownership boundary rather
  than treating either pattern as application-wide. Keep query/cache behavior
  in `src/lib/query/` and runtime form validation in `src/lib/schemas/`.
- Follow the nearby naming family rather than imposing one global filename
  rule: domain components commonly use PascalCase (`ProviderCard.tsx`), while
  hooks use `use`-prefixed camelCase filenames (`useTauriEvent.ts`).

## Test Placement

Most renderer tests mirror their subject under `tests/` (`tests/components/`,
`tests/hooks/`, `tests/lib/`, and `tests/integration/`). A small number of
pure utilities have adjacent `*.test.ts` files under `src/utils/`. Match the
nearest existing test family for the code being changed.

## Evidence

- [src/main.tsx](../../../src/main.tsx) composes renderer-wide providers before
  rendering `App`.
- [src/components/providers/forms/ProviderForm.tsx](../../../src/components/providers/forms/ProviderForm.tsx)
  imports feature-local components and the `./hooks` barrel from one domain
  subtree.
- [src/lib/api/providers.ts](../../../src/lib/api/providers.ts) is the typed
  Tauri facade used by renderer hooks and components.
- [src/hooks/useProxyStatus.ts](../../../src/hooks/useProxyStatus.ts) is an
  existing feature-local direct-`invoke` path.
