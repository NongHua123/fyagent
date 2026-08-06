import { useEffect, useMemo, useRef, useState } from "react";
import type { VisibleApps } from "@/types";
import { ProviderIcon } from "@/components/ProviderIcon";
import { WorkBuddyIcon } from "@/components/BrandIcons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import type { TopLevelAppId } from "@/types/topLevelApp";
import { useTranslation } from "react-i18next";
import { Monitor, MoreHorizontal, Terminal } from "lucide-react";
import type { WindowLayoutMode } from "@/lib/layout/windowLayoutConstants";

const APP_BADGE_ICON: Partial<
  Record<TopLevelAppId, { icon: typeof Terminal; offsetY?: number }>
> = {
  claude: { icon: Terminal },
  "claude-desktop": { icon: Monitor, offsetY: 0.5 },
};

const ALL_APPS: TopLevelAppId[] = [
  "claude",
  "claude-desktop",
  "codex",
  "workbuddy",
  "gemini",
  "grokbuild",
  "opencode",
  "openclaw",
  "hermes",
];
const STORAGE_KEY = "fyagent-last-app";
const APP_BUTTON_WIDTH = 48;
const APP_BUTTON_GAP = 4;
const APP_SWITCHER_HORIZONTAL_PADDING = 8;
const APP_SWITCHER_MORE_WIDTH = 32;

interface AppSwitcherProps {
  activeApp: TopLevelAppId;
  onSwitch: (app: TopLevelAppId) => void;
  visibleApps?: VisibleApps;
  layoutMode?: WindowLayoutMode;
}

interface CapacityResult {
  directApps: TopLevelAppId[];
  overflowApps: TopLevelAppId[];
}

/**
 * Reserves space for the overflow trigger before calculating direct items. This
 * keeps a resize from oscillating between one extra app and a hidden trigger.
 */
export function getAppSwitcherCapacity(
  apps: readonly TopLevelAppId[],
  activeApp: TopLevelAppId,
  availableWidth: number,
): CapacityResult {
  if (apps.length <= 1) {
    return { directApps: [...apps], overflowApps: [] };
  }

  const requiredWidthWithoutOverflow =
    APP_SWITCHER_HORIZONTAL_PADDING +
    apps.length * APP_BUTTON_WIDTH +
    (apps.length - 1) * APP_BUTTON_GAP;
  if (availableWidth >= requiredWidthWithoutOverflow) {
    return { directApps: [...apps], overflowApps: [] };
  }

  const usableWidth = Math.max(
    0,
    availableWidth -
      APP_SWITCHER_HORIZONTAL_PADDING -
      APP_SWITCHER_MORE_WIDTH -
      APP_BUTTON_GAP,
  );
  const directAppCount = Math.max(
    1,
    Math.floor(usableWidth / (APP_BUTTON_WIDTH + APP_BUTTON_GAP)),
  );
  const directApps = new Set<TopLevelAppId>([activeApp]);

  for (const app of apps) {
    if (directApps.size >= directAppCount) break;
    directApps.add(app);
  }

  return {
    directApps: apps.filter((app) => directApps.has(app)),
    overflowApps: apps.filter((app) => !directApps.has(app)),
  };
}

export function AppSwitcher({
  activeApp,
  onSwitch,
  visibleApps,
  layoutMode = "normal",
}: AppSwitcherProps) {
  const { t } = useTranslation();
  const capacityRef = useRef<HTMLDivElement>(null);
  const [availableWidth, setAvailableWidth] = useState(0);

  useEffect(() => {
    const capacityElement = capacityRef.current;
    if (!capacityElement) return;

    const updateCapacity = (nextWidth?: number) => {
      const width = nextWidth ?? capacityElement.clientWidth;
      setAvailableWidth((previousWidth) =>
        previousWidth === width ? previousWidth : width,
      );
    };

    updateCapacity();
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      updateCapacity(entry?.contentRect.width);
    });
    observer.observe(capacityElement);
    return () => observer.disconnect();
  }, []);

  const handleSwitch = (app: TopLevelAppId) => {
    if (app === activeApp) return;
    localStorage.setItem(STORAGE_KEY, app);
    onSwitch(app);
  };

  const appIconName: Record<Exclude<TopLevelAppId, "workbuddy">, string> = {
    claude: "claude",
    "claude-desktop": "claude",
    codex: "openai",
    gemini: "gemini",
    grokbuild: "grok",
    opencode: "opencode",
    openclaw: "openclaw",
    hermes: "hermes",
  };
  const appDisplayName: Record<TopLevelAppId, string> = {
    claude: "Claude Code",
    "claude-desktop": "Claude Desktop",
    codex: "Codex",
    workbuddy: t("apps.workbuddy"),
    gemini: "Gemini",
    grokbuild: "Grok Build",
    opencode: "OpenCode",
    openclaw: "OpenClaw",
    hermes: "Hermes",
  };

  const appsToShow = useMemo(() => {
    const configuredApps = ALL_APPS.filter((app) => {
      if (!visibleApps) return true;
      return app === "workbuddy"
        ? (visibleApps.workbuddy ?? true)
        : visibleApps[app];
    });

    // Settings changes are asynchronous. Preserve the current item through the
    // render that precedes App's visibility fallback so P0 navigation never
    // briefly vanishes.
    return configuredApps.includes(activeApp)
      ? configuredApps
      : [activeApp, ...configuredApps];
  }, [activeApp, visibleApps]);

  const { directApps, overflowApps } =
    layoutMode === "normal"
      ? { directApps: appsToShow, overflowApps: [] }
      : getAppSwitcherCapacity(appsToShow, activeApp, availableWidth);
  const iconSize = 20;
  const actionsLabel = t("common.actions");

  const renderAppIcon = (
    app: TopLevelAppId,
    options?: { decorative?: boolean },
  ) => {
    const badgeConfig = APP_BADGE_ICON[app];
    const BadgeIcon = badgeConfig?.icon;
    const isActive = activeApp === app;

    const icon = (
      <span className="relative inline-flex shrink-0">
        {app === "workbuddy" ? (
          <WorkBuddyIcon size={iconSize} />
        ) : (
          <ProviderIcon
            icon={appIconName[app]}
            name={appDisplayName[app]}
            size={iconSize}
          />
        )}
        {BadgeIcon ? (
          <span
            aria-hidden="true"
            className={cn(
              "absolute -bottom-0.5 -right-0.5 flex h-[11px] w-[11px] items-center justify-center rounded-[3px] border",
              isActive
                ? "border-border bg-background text-foreground"
                : "border-background bg-muted text-muted-foreground group-hover:bg-background group-hover:text-foreground",
            )}
          >
            <BadgeIcon
              className="h-[8px] w-[8px]"
              strokeWidth={2.5}
              style={
                badgeConfig?.offsetY
                  ? { transform: `translateY(${badgeConfig.offsetY}px)` }
                  : undefined
              }
            />
          </span>
        ) : null}
      </span>
    );

    return options?.decorative ? <span aria-hidden="true">{icon}</span> : icon;
  };

  return (
    <div
      className="min-w-0 flex-1"
      data-testid="app-switcher-capacity"
      ref={capacityRef}
    >
      <div className="flex min-w-0 items-center justify-end gap-1 rounded-xl bg-muted p-1">
        {directApps.map((app) => {
          const isActive = activeApp === app;
          return (
            <button
              key={app}
              aria-label={appDisplayName[app]}
              aria-pressed={isActive}
              className={cn(
                "group inline-flex h-8 w-11 shrink-0 items-center justify-center rounded-md text-sm font-medium transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                isActive
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:bg-background/50 hover:text-foreground",
              )}
              onClick={() => handleSwitch(app)}
              title={appDisplayName[app]}
              type="button"
            >
              {renderAppIcon(app)}
            </button>
          );
        })}
        {overflowApps.length > 0 ? (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                aria-label={actionsLabel}
                className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-background/50 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                data-testid="app-switcher-more"
                title={actionsLabel}
                type="button"
              >
                <MoreHorizontal aria-hidden="true" className="h-4 w-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="z-[100]">
              {overflowApps.map((app) => {
                const isActive = activeApp === app;
                return (
                  <DropdownMenuItem
                    aria-current={isActive ? "page" : undefined}
                    key={app}
                    onSelect={() => handleSwitch(app)}
                  >
                    {renderAppIcon(app, { decorative: true })}
                    {appDisplayName[app]}
                  </DropdownMenuItem>
                );
              })}
            </DropdownMenuContent>
          </DropdownMenu>
        ) : null}
      </div>
    </div>
  );
}
