# State Management

The current renderer uses React local state and Context for UI state, plus
TanStack React Query for data read from or written to the Tauri backend. There
is no Zustand or Jotai dependency in `package.json`.

## State Categories

- **Local UI state:** components and feature hooks use `useState`, `useEffect`,
  and refs. `App.tsx` keeps the selected application and view locally, then
  persists those UI preferences in `localStorage`.
- **Small cross-tree UI state:** Context providers own values used by unrelated
  descendants. `ThemeProvider` owns the selected theme and its persistence and
  is composed in `main.tsx`. The host updater is intentionally not renderer
  state: V1 has no `UpdateProvider`, updater query, or updater capability.
- **Backend/resource state:** TanStack Query owns results obtained through
  `src/lib/api/*`. The shared `queryClient` provides the renderer defaults;
  feature query and mutation hooks live under both `src/lib/query/` and
  `src/hooks/`.

## Host-Synchronized UI State

`src/lib/layout/useWindowLayoutMode.ts` keeps a renderer state value for
rendering, but accepts the validated native `layout-mode-changed` work-area mode
when available. The normal/constrained policy remains pure and testable in
`src/lib/layout/`; browser previews and fake tests fall back to renderer width.
Do not put this host-synchronized state in the Query cache or persist it as an
application preference.

## Query Key and Invalidation Pattern

When several hooks share a resource family, centralize their query keys in the
domain module and invalidate the affected keys from successful mutations.
`useOpenClaw.ts` is the clearest current example:

```tsx
export const openclawKeys = {
  env: ["openclaw", "env"] as const,
  health: ["openclaw", "health"] as const,
};

return useMutation({
  mutationFn: (env: OpenClawEnvConfig) => openclawApi.setEnv(env),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: openclawKeys.env });
    queryClient.invalidateQueries({ queryKey: openclawKeys.health });
  },
});
```

Not every existing query key is centralized: `src/lib/query/queries.ts` also
uses short array keys directly for resources such as providers and settings.
Match the local resource module instead of introducing a new application-wide
key factory.

## Persistence Boundary

`localStorage` is currently used for renderer preferences such as theme and
last view. Most feature data reaches native commands through typed Tauri API
facades and is frequently represented in the Query cache; `main.tsx` retains a
bootstrap-time direct `invoke` call. Keep the renderer-preference boundary
distinct from native configuration data when extending existing behavior.

## Host Updater Boundary

FyAgent V1 removes the host application's upstream updater end to end. Do not
add a global update context, update-check query, background download action, or
DatabaseUpgrade-to-updater bridge. `main.tsx` may render `DatabaseUpgrade` when
the native host reports `db_version_too_new`, but that recovery surface must
remain a local, no-network support or controlled-distribution prompt and must
not mutate the database.

## Evidence

- [src/main.tsx](../../../src/main.tsx) composes `QueryClientProvider` and
  `ThemeProvider` at the renderer root and renders the database-too-new
  recovery branch without an updater provider.
- [src/lib/query/queryClient.ts](../../../src/lib/query/queryClient.ts)
  defines the shared TanStack Query defaults.
- [src/hooks/useOpenClaw.ts](../../../src/hooks/useOpenClaw.ts) centralizes
  one resource family's keys, query hooks, mutations, and invalidation.
- [src/lib/layout/useWindowLayoutMode.ts](../../../src/lib/layout/useWindowLayoutMode.ts)
  accepts a validated native layout mode while retaining a renderer fallback.
