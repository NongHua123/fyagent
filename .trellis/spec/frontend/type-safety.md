# Type Safety

## Compiler Contract

The renderer compiles with TypeScript strict mode. `tsconfig.json` also enables
`noUnusedLocals`, `noUnusedParameters`, and `noFallthroughCasesInSwitch`, and
maps `@/*` to `src/*`. Type-check renderer and test code with
`mise exec -- pnpm typecheck`.

## Type Ownership

- Shared frontend domain interfaces and unions live in `src/types.ts` and the
  smaller `src/types/` modules. Components import those with `import type` when
  they only need a type.
- Most feature-level API facades in `src/lib/api/` expose explicit parameter
  and `Promise` return types around Tauri `invoke` calls. Prefer the relevant
  facade when extending a facade-driven feature, but follow the nearest
  established boundary: `src/main.tsx` and `src/hooks/useProxyStatus.ts` are
  existing direct-`invoke` paths, not a blanket pattern for unrelated features.
- Forms use Zod schemas in `src/lib/schemas/` and infer their form data from
  the schema rather than maintaining a parallel form-only interface.

```tsx
// src/lib/schemas/provider.ts
export const providerSchema = z.object({
  name: z.string(),
  websiteUrl: z.string().url().optional().or(z.literal("")),
  settingsConfig: z.string().min(1),
});

export type ProviderFormData = z.infer<typeof providerSchema>;
```

## Dynamic and Cross-Layer Data

The existing `Provider.settingsConfig` is deliberately dynamic
(`Record<string, any>`), and several UI paths parse editable JSON before
applying a narrow assertion. This means the repository does not currently
enforce a blanket ban on `any` or type assertions. For a stable new shape,
extend the existing domain type or a Zod schema; retain a narrow boundary for
provider-specific JSON instead of claiming it is universally validated.

The Tauri wire boundary uses camelCase names on the TypeScript side and
`serde(rename = "...")` on Rust fields where needed. A command payload change
must be checked against both sides of that boundary.

```rust
// src-tauri/src/provider.rs
#[serde(rename = "settingsConfig")]
pub settings_config: Value,
```

## Evidence

- [tsconfig.json](../../../tsconfig.json) defines strict renderer/test compiler
  options and the `@/*` alias.
- [src/lib/schemas/provider.ts](../../../src/lib/schemas/provider.ts) derives
  `ProviderFormData` from the runtime Zod schema.
- [src-tauri/src/provider.rs](../../../src-tauri/src/provider.rs) shows the
  serialized Rust companion for frontend provider data.
- [src/hooks/useProxyStatus.ts](../../../src/hooks/useProxyStatus.ts) shows an
  established direct-`invoke` boundary alongside the facade-based paths.
