# Hook Guidelines

## Location and Naming

Custom hooks are exported functions whose names start with `use`. General hooks
live in `src/hooks/`; hooks private to a feature may be co-located beneath that
feature and re-exported from a local barrel. Do not move a feature-private hook
to `src/hooks/` unless a second feature needs it.

```tsx
// src/hooks/useDebouncedValue.ts
export function useDebouncedValue<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedValue(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);
  return debouncedValue;
}
```

`src/components/providers/forms/hooks/index.ts` is the local barrel for the
provider-form hooks. `src/components/mcp/useMcpValidation.ts` is another
feature-local hook that stays beside its MCP UI.

## Effects and Native Events

Hooks that create timers, observers, or native listeners return the cleanup
from their effect. The Tauri event wrapper also guards an asynchronously
created unsubscribe function so it is released even when unmount happens
before `listen()` resolves.

```tsx
// Pattern from src/hooks/useTauriEvent.ts
useEffect(() => {
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  // await listen(...), then retain or immediately call the unlisten function
  return () => {
    disposed = true;
    unlisten?.();
  };
}, [eventName]);
```

## Stateful Hook Shape

Hooks keep feature-specific transient state and expose named values plus
handlers. `useApiKeyState` returns `apiKey`, `setApiKey`,
`handleApiKeyChange`, and `showApiKey`; it synchronizes a form field with an
editable JSON configuration while retaining the feature's validation rules.
Use a named return object when a hook exposes multiple related values.

Resource hooks that use TanStack Query retain their query key and invalidation
logic with the owning domain. For example, `useOpenClaw.ts` groups its hooks
and shared keys together instead of scattering key literals across components.

## Evidence

- [src/hooks/useDebouncedValue.ts](../../../src/hooks/useDebouncedValue.ts)
  shows a generic hook with an effect cleanup.
- [src/hooks/useTauriEvent.ts](../../../src/hooks/useTauriEvent.ts) handles
  asynchronous native subscription setup and teardown.
- [src/components/providers/forms/hooks/useApiKeyState.ts](../../../src/components/providers/forms/hooks/useApiKeyState.ts)
  demonstrates a co-located form-state hook with typed inputs and a named
  return object.
