# Component Guidelines

## Component Families

Business components are generally function components with a local props
interface and typed callback parameters. Many shared UI primitives that wrap
Radix or native elements use `React.forwardRef`; when a primitive forwards a
ref, retain the corresponding HTML attributes and give it a display name.
Small primitives that do not need a forwarded ref use simpler component
shapes. Reuse an existing primitive from `src/components/ui/` when it already
provides the required behavior.

```tsx
// src/components/ui/button.tsx
export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  },
);
```

Feature components use explicit domain inputs and `on...` callbacks. Optional
props receive defaults in the destructuring parameter when the component has a
stable fallback. `ProviderEmptyState` is a compact current example:

```tsx
// src/components/providers/ProviderEmptyState.tsx
interface ProviderEmptyStateProps {
  appId: AppId;
  onCreate?: () => void;
  onImport?: () => void;
}
```

## Composition and Styling

- When conditional Tailwind classes are needed, use `cn()` from
  `src/lib/utils.ts`. It applies `clsx` and `tailwind-merge` and is the helper
  used by the shared primitives.
- Shared visual variants use CVA where a primitive already exposes it, such as
  `buttonVariants` in `ui/button.tsx`. Business components otherwise use
  Tailwind utility classes directly.
- Theme tokens and the `.dark` selector are defined in `src/index.css`; keep
  new shared colors and focus behavior compatible with that token model.
- `ProviderForm` composes `Form`, `FormField`, `FormItem`, and `FormMessage`
  from the local UI layer with React Hook Form and a Zod resolver. Follow that
  composition when extending this form instead of duplicating its field/error
  plumbing.

## Responsive Top-Level Chrome

`TopLevelHeader` owns the stable shell order. Its app-switcher capacity slot is
the only flexible region and sits immediately before the fixed 40px trailing
primary-action slot. A top-level surface without a primary action passes
`trailingPrimaryActionEmpty` so an inert, `aria-hidden` placeholder preserves
geometry without creating a disabled or focusable control.

Context/P2 actions use `HeaderActionsOverflow` with a translated accessible
label. In constrained layout, retain the active app as a direct switcher control
and move other apps into More; compact icon-only controls still need an
accessible name.

## Text and Accessibility Patterns

- User-visible renderer text is obtained through `useTranslation()` and
  `t(...)`. The active locale registration is in `src/i18n/index.ts`; its
  current locale files are `en.json`, `ja.json`, `zh.json`, and `zh-TW.json`.
- Preserve the native props, forwarded ref, and focus-visible classes when
  changing UI primitives. `FormControl` also connects descriptions and errors
  with `aria-describedby`, so form changes should keep that relationship.
- Components with icon-only or stateful controls use translated ARIA labels in
  the existing feature code. Add labels where the surrounding pattern does,
  rather than replacing the native control with a non-semantic element.

## Evidence

- [src/components/ui/button.tsx](../../../src/components/ui/button.tsx)
  demonstrates CVA variants, `cn`, native button props, `asChild`, and
  `forwardRef`.
- [src/components/providers/ProviderEmptyState.tsx](../../../src/components/providers/ProviderEmptyState.tsx)
  demonstrates typed domain props, optional callbacks, and `useTranslation`.
- [src/components/providers/ProviderCard.tsx](../../../src/components/providers/ProviderCard.tsx)
  demonstrates defaulted optional props and callback wiring in a larger domain
  component.
- [src/components/ui/form.tsx](../../../src/components/ui/form.tsx) implements
  the shared React Hook Form composition and ARIA linkage.
- [src/components/topbar/TopLevelHeader.tsx](../../../src/components/topbar/TopLevelHeader.tsx)
  owns the sole flex-capacity slot and stable top-level order.
- [src/components/topbar/TrailingPrimaryActionSlot.tsx](../../../src/components/topbar/TrailingPrimaryActionSlot.tsx)
  preserves geometry without adding a keyboard stop.
