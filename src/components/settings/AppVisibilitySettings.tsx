import { useTranslation } from "react-i18next";
import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ToggleRow } from "@/components/ui/toggle-row";
import { cn } from "@/lib/utils";
import { ProviderIcon } from "@/components/ProviderIcon";
import { WorkBuddyIcon } from "@/components/BrandIcons";
import type { SettingsFormState } from "@/hooks/useSettings";
import type { VisibleApps } from "@/types";
import type { TopLevelAppId } from "@/types/topLevelApp";

interface AppVisibilitySettingsProps {
  settings: SettingsFormState;
  onChange: (updates: Partial<SettingsFormState>) => void;
}

type AppConfig =
  | {
      id: Exclude<TopLevelAppId, "workbuddy">;
      icon: string;
      nameKey: string;
    }
  | {
      id: "workbuddy";
      nameKey: string;
    };

const APP_CONFIG: AppConfig[] = [
  { id: "claude", icon: "claude", nameKey: "apps.claudeCode" },
  {
    id: "claude-desktop",
    icon: "claude",
    nameKey: "apps.claudeDesktop",
  },
  { id: "codex", icon: "openai", nameKey: "apps.codex" },
  { id: "workbuddy", nameKey: "apps.workbuddy" },
  { id: "gemini", icon: "gemini", nameKey: "apps.gemini" },
  { id: "grokbuild", icon: "grok", nameKey: "apps.grokbuild" },
  { id: "opencode", icon: "opencode", nameKey: "apps.opencode" },
  { id: "openclaw", icon: "openclaw", nameKey: "apps.openclaw" },
  { id: "hermes", icon: "hermes", nameKey: "apps.hermes" },
];

export function AppVisibilitySettings({
  settings,
  onChange,
}: AppVisibilitySettingsProps) {
  const { t } = useTranslation();

  const visibleApps: VisibleApps = settings.visibleApps ?? {
    claude: true,
    "claude-desktop": true,
    codex: true,
    workbuddy: true,
    gemini: true,
    grokbuild: true,
    opencode: true,
    openclaw: true,
    hermes: true,
  };

  // Count how many apps are currently visible
  const resolvedVisibleApps = {
    ...visibleApps,
    workbuddy: visibleApps.workbuddy ?? true,
  };

  const visibleCount =
    Object.values(resolvedVisibleApps).filter(Boolean).length;

  const handleToggle = (appId: TopLevelAppId) => {
    const isCurrentlyVisible =
      appId === "workbuddy"
        ? resolvedVisibleApps.workbuddy
        : resolvedVisibleApps[appId];
    // Prevent disabling the last visible app
    if (isCurrentlyVisible && visibleCount <= 1) return;

    onChange({
      visibleApps: {
        ...resolvedVisibleApps,
        [appId]: !isCurrentlyVisible,
      },
    });
  };

  return (
    <section className="space-y-2">
      <header className="space-y-1">
        <h3 className="text-sm font-medium">
          {t("settings.appVisibility.title")}
        </h3>
        <p className="text-xs text-muted-foreground">
          {t("settings.appVisibility.description")}
        </p>
      </header>
      <div className="flex flex-wrap gap-1 rounded-md border border-border-default bg-background p-1">
        {APP_CONFIG.map((app) => {
          const isVisible =
            app.id === "workbuddy"
              ? resolvedVisibleApps.workbuddy
              : resolvedVisibleApps[app.id];
          // Disable button if this is the last visible app
          const isDisabled = isVisible && visibleCount <= 1;

          return (
            <AppButton
              key={app.id}
              active={isVisible}
              disabled={isDisabled}
              onClick={() => handleToggle(app.id)}
              icon={app.id === "workbuddy" ? undefined : app.icon}
              isWorkBuddy={app.id === "workbuddy"}
              name={t(app.nameKey)}
            >
              {t(app.nameKey)}
            </AppButton>
          );
        })}
      </div>
      <ToggleRow
        icon={<FolderOpen className="h-4 w-4 text-emerald-500" />}
        title={t("settings.appVisibility.showProfileSwitcher")}
        description={t("settings.appVisibility.showProfileSwitcherDescription")}
        checked={settings.showProfileSwitcher ?? true}
        onCheckedChange={(value) => onChange({ showProfileSwitcher: value })}
      />
    </section>
  );
}

interface AppButtonProps {
  active: boolean;
  disabled?: boolean;
  onClick: () => void;
  icon: string | undefined;
  isWorkBuddy?: boolean;
  name: string;
  children: React.ReactNode;
}

function AppButton({
  active,
  disabled,
  onClick,
  icon,
  isWorkBuddy,
  name,
  children,
}: AppButtonProps) {
  return (
    <Button
      type="button"
      onClick={onClick}
      disabled={disabled}
      size="sm"
      variant={active ? "default" : "ghost"}
      className={cn(
        "min-w-[90px] w-auto gap-1.5 px-3",
        active
          ? "shadow-sm"
          : "text-muted-foreground hover:text-foreground hover:bg-muted",
      )}
    >
      {isWorkBuddy ? (
        <WorkBuddyIcon size={14} />
      ) : (
        <ProviderIcon icon={icon} name={name} size={14} />
      )}
      {children}
    </Button>
  );
}
