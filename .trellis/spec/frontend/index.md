# Frontend Development Guidelines

These guidelines describe the renderer patterns observed in this checkout of
FyAgent. They are evidence-based reference material for changes under
`src/` and related renderer tests, not a proposed frontend redesign.

## Pre-Development Checklist

Before changing renderer code:

1. Read the guideline that owns the change and inspect the nearest existing
   feature or primitive.
2. Locate the existing Tauri API facade, query hook, type, schema, and test
   family before creating another one.
3. Classify state as local UI state, Context state, or backend/resource state.
4. For user-visible text, locate the matching keys in all four registered
   locale files before adding a literal string.
5. For a backend payload change, inspect both the TypeScript facade and the
   matching `src-tauri/` serialization/command code.
6. For an application-brand icon change, read the shared
   [Application Brand Asset Contract](../backend/application-brand-assets.md)
   before regenerating Tauri or About assets.
7. For product names, storage keys, serialized markers, deep links, or public
   source/install links, read the shared
   [Application Identity Contract](../backend/application-identity.md).
8. Run local tooling through the shared
   [Development Environment Contract](../backend/development-environment.md).

## Guidelines

| Guide                                                              | Use it for                                                                                   |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------- |
| [Directory Structure](./directory-structure.md)                    | Selecting the existing frontend layer and test location.                                     |
| [Component Guidelines](./component-guidelines.md)                  | UI primitives, props, styling, translation, and form composition.                            |
| [Hook Guidelines](./hook-guidelines.md)                            | Naming, placement, effects, cleanup, and stateful hook APIs.                                 |
| [State Management](./state-management.md)                          | React state, Context, TanStack Query keys, mutations, and persistence.                       |
| [Type Safety](./type-safety.md)                                    | Strict TypeScript, domain types, Zod schemas, and Tauri wire contracts.                      |
| [Quality Guidelines](./quality-guidelines.md)                      | Core and desktop-contract checks, Vitest/MSW setup, translations, and accessible primitives. |
| [Application Brand Assets](../backend/application-brand-assets.md) | Cross-platform generated icons and renderer About reuse.                                     |
| [Application Identity](../backend/application-identity.md)         | FyAgent-owned runtime identity and factual repository/provenance boundaries.                 |

## Quality Check

For frontend code changes, run the checks applicable to the affected behavior:

```bash
mise run typecheck
mise run format:check
mise run test:unit
```

For desktop-shell, responsive-header, window-layout, or acceptance changes,
also run the mock and visual-preflight checks in
[Quality Guidelines](./quality-guidelines.md); they do not replace real native
desktop or installer evidence.

`package.json` remains the source of package-level frontend scripts. The
repository-level entrypoint is the generated `mise run` task API; Node.js and
pnpm versions come from `.node-version` and `package.json#packageManager`, not
from duplicate declarations in `mise.toml`.

## Evidence

- [package.json](../../../package.json) defines the renderer tooling and
  runnable scripts.
- [src/main.tsx](../../../src/main.tsx) shows the renderer provider boundary.
- [CONTRIBUTING.md](../../../CONTRIBUTING.md) records the maintained
  contribution expectations, including strict TypeScript and translated UI
  text.
- [Development Environment](../backend/development-environment.md) defines the
  required local mise command boundary.
