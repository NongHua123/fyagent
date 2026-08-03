import type { VisibleApps } from "@/types";
import { ProviderIcon } from "@/components/ProviderIcon";
import { WorkBuddyIcon } from "@/components/BrandIcons";
import { cn } from "@/lib/utils";
import type { TopLevelAppId } from "@/types/topLevelApp";
import { useTranslation } from "react-i18next";
import { Monitor, Terminal } from "lucide-react";

const APP_BADGE_ICON: Partial<
  Record<TopLevelAppId, { icon: typeof Terminal; offsetY?: number }>
> = {
  claude: { icon: Terminal },
  "claude-desktop": { icon: Monitor, offsetY: 0.5 },
};

interface AppSwitcherProps {
  activeApp: TopLevelAppId;
  onSwitch: (app: TopLevelAppId) => void;
  visibleApps?: VisibleApps;
  compact?: boolean;
}

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

export function AppSwitcher({
  activeApp,
  onSwitch,
  visibleApps,
  compact,
}: AppSwitcherProps) {
  const { t } = useTranslation();

  const handleSwitch = (app: TopLevelAppId) => {
    if (app === activeApp) return;
    localStorage.setItem(STORAGE_KEY, app);
    onSwitch(app);
  };
  const iconSize = 20;
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

  // Filter apps based on visibility settings (default all visible)
  const appsToShow = ALL_APPS.filter((app) => {
    if (!visibleApps) return true;
    return app === "workbuddy"
      ? (visibleApps.workbuddy ?? true)
      : visibleApps[app];
  });

  return (
    <div className="inline-flex bg-muted rounded-xl p-1 gap-1">
      {appsToShow.map((app) => {
        const badgeConfig = APP_BADGE_ICON[app];
        const BadgeIcon = badgeConfig?.icon;
        const isActive = activeApp === app;
        return (
          <button
            key={app}
            type="button"
            onClick={() => handleSwitch(app)}
            aria-pressed={isActive}
            className={cn(
              "group inline-flex items-center px-3 h-8 rounded-md text-sm font-medium transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
              isActive
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground hover:bg-background/50",
            )}
          >
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
              {BadgeIcon && (
                <span
                  className={cn(
                    "absolute -bottom-0.5 -right-0.5 flex items-center justify-center rounded-[3px] border h-[11px] w-[11px]",
                    isActive
                      ? "bg-background border-border text-foreground"
                      : "bg-muted border-background text-muted-foreground group-hover:bg-background group-hover:text-foreground",
                  )}
                  aria-hidden="true"
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
              )}
            </span>
            <span
              className={cn(
                "transition-all duration-200 whitespace-nowrap overflow-hidden",
                compact
                  ? "max-w-0 opacity-0 ml-0"
                  : "max-w-[120px] opacity-100 ml-2",
              )}
            >
              {appDisplayName[app]}
            </span>
          </button>
        );
      })}
    </div>
  );
}
