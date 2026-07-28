# Frontend Development Guidelines

These guidelines describe the renderer patterns observed in this checkout of
CC Switch. They are evidence-based reference material for changes under
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

## Guidelines

| Guide                                             | Use it for                                                                  |
| ------------------------------------------------- | --------------------------------------------------------------------------- |
| [Directory Structure](./directory-structure.md)   | Selecting the existing frontend layer and test location.                    |
| [Component Guidelines](./component-guidelines.md) | UI primitives, props, styling, translation, and form composition.           |
| [Hook Guidelines](./hook-guidelines.md)           | Naming, placement, effects, cleanup, and stateful hook APIs.                |
| [State Management](./state-management.md)         | React state, Context, TanStack Query keys, mutations, and persistence.      |
| [Type Safety](./type-safety.md)                   | Strict TypeScript, domain types, Zod schemas, and Tauri wire contracts.     |
| [Quality Guidelines](./quality-guidelines.md)     | Runnable checks, Vitest/MSW setup, translations, and accessible primitives. |

## Quality Check

For frontend code changes, run the checks applicable to the affected behavior:

```powershell
pnpm typecheck
pnpm format:check
pnpm test:unit
```

`pnpm lint` is mentioned in `CONTRIBUTING.md`, but it is not currently a
package script. Treat the package scripts as the reproducible command source
until that mismatch is resolved.

## Evidence

- [package.json](../../../package.json) defines the renderer tooling and
  runnable scripts.
- [src/main.tsx](../../../src/main.tsx) shows the renderer provider boundary.
- [CONTRIBUTING.md](../../../CONTRIBUTING.md) records the maintained
  contribution expectations, including strict TypeScript and translated UI
  text.
