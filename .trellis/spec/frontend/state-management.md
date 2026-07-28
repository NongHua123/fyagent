# State Management

The current renderer uses React local state and Context for UI state, plus
TanStack React Query for data read from or written to the Tauri backend. There
is no Zustand or Jotai dependency in `package.json`.

## State Categories

- **Local UI state:** components and feature hooks use `useState`, `useEffect`,
  and refs. `App.tsx` keeps the selected application and view locally, then
  persists those UI preferences in `localStorage`.
- **Small cross-tree UI state:** Context providers own values used by unrelated
  descendants. `ThemeProvider` owns the selected theme and its persistence;
  `UpdateProvider` owns update-check state. Both are composed in `main.tsx`.
- **Backend/resource state:** TanStack Query owns results obtained through
  `src/lib/api/*`. The shared `queryClient` provides the renderer defaults;
  feature query and mutation hooks live under both `src/lib/query/` and
  `src/hooks/`.

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

`localStorage` is currently used for renderer preferences such as theme, last
view, and dismissed update versions. Most feature data reaches native commands
through typed Tauri API facades and is frequently represented in the Query
cache; `main.tsx` retains a bootstrap-time direct `invoke` call. Keep the
renderer-preference boundary distinct from native configuration data when
extending existing behavior.

## Evidence

- [src/main.tsx](../../../src/main.tsx) composes `QueryClientProvider`,
  `ThemeProvider`, and `UpdateProvider` at the renderer root.
- [src/lib/query/queryClient.ts](../../../src/lib/query/queryClient.ts)
  defines the shared TanStack Query defaults.
- [src/hooks/useOpenClaw.ts](../../../src/hooks/useOpenClaw.ts) centralizes
  one resource family's keys, query hooks, mutations, and invalidation.
