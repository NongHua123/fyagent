import type { ReactNode } from "react";

interface TrailingPrimaryActionSlotProps {
  children?: ReactNode;
  empty?: boolean;
}

/**
 * The fixed trailing slot is intentionally present for every top-level app.
 * An inert child, rather than a disabled button, preserves the WorkBuddy
 * geometry without adding a focus stop or an accessible action.
 */
export function TrailingPrimaryActionSlot({
  children,
  empty = false,
}: TrailingPrimaryActionSlotProps) {
  return (
    <div
      className="flex h-10 w-10 shrink-0 items-center justify-end"
      data-testid="trailing-primary-action-slot"
    >
      {empty ? (
        <span
          aria-hidden="true"
          className="pointer-events-none h-8 w-8"
          data-testid="trailing-primary-action-placeholder"
        />
      ) : (
        children
      )}
    </div>
  );
}
