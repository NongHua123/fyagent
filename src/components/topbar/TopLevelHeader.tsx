import type { CSSProperties, ReactNode } from "react";
import { MoreHorizontal } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { TrailingPrimaryActionSlot } from "./TrailingPrimaryActionSlot";

export interface HeaderActionDescriptor {
  id: string;
  label: string;
  icon: ReactNode;
  onSelect: () => void;
}

interface HeaderActionsOverflowProps {
  actions: readonly HeaderActionDescriptor[];
  label: string;
}

export function HeaderActionsOverflow({
  actions,
  label,
}: HeaderActionsOverflowProps) {
  if (actions.length === 0) return null;

  return (
    <div className="shrink-0" data-testid="top-level-actions-overflow">
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            aria-label={label}
            className="h-8 w-8 hover:bg-black/5 dark:hover:bg-white/5"
            size="icon"
            title={label}
            type="button"
            variant="ghost"
          >
            <MoreHorizontal aria-hidden="true" className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="z-[100]">
          {actions.map((action) => (
            <DropdownMenuItem key={action.id} onSelect={action.onSelect}>
              <span aria-hidden="true" className="flex h-4 w-4 items-center">
                {action.icon}
              </span>
              {action.label}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

interface TopLevelHeaderProps {
  leading: ReactNode;
  priorityControls?: ReactNode;
  contextActions?: readonly HeaderActionDescriptor[];
  appSwitcher: ReactNode;
  trailingPrimaryAction?: ReactNode;
  trailingPrimaryActionEmpty?: boolean;
  actionsLabel: string;
  compact?: boolean;
}

/**
 * Owns the stable visual order for application-level navigation. The capacity
 * slot is the only flexible region; keeping it immediately before the fixed
 * trailing action prevents provider/WorkBuddy switches from moving its edge.
 */
export function TopLevelHeader({
  leading,
  priorityControls,
  contextActions = [],
  appSwitcher,
  trailingPrimaryAction,
  trailingPrimaryActionEmpty = false,
  actionsLabel,
  compact = false,
}: TopLevelHeaderProps) {
  return (
    <div
      className={cn(
        "flex h-full min-w-0 items-center",
        compact ? "gap-1 px-3" : "gap-2 px-6",
      )}
      style={{ WebkitAppRegion: "no-drag" } as CSSProperties}
    >
      <div className="flex min-w-0 shrink-0 items-center gap-1">{leading}</div>
      {priorityControls ? (
        <div
          className="flex min-w-0 shrink-0 items-center gap-1.5"
          data-testid="top-level-priority-controls"
        >
          {priorityControls}
        </div>
      ) : null}
      <HeaderActionsOverflow actions={contextActions} label={actionsLabel} />
      <div
        className="flex min-w-0 flex-1 items-center justify-end"
        data-testid="app-switcher-capacity-slot"
      >
        {appSwitcher}
      </div>
      <TrailingPrimaryActionSlot empty={trailingPrimaryActionEmpty}>
        {trailingPrimaryAction}
      </TrailingPrimaryActionSlot>
    </div>
  );
}
