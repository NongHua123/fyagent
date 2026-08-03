import {
  AlertCircle,
  CheckCircle2,
  Download,
  FolderOpen,
  Loader2,
  Play,
  RefreshCw,
  ShieldCheck,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { CodexIcon } from "@/components/BrandIcons";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { useCodexDesktopInstaller } from "@/hooks/useCodexDesktopInstaller";
import type {
  InstallerPrimaryAction,
  InstallerViewState,
} from "@/types/codexDesktop";

const stateMessageKeys: Record<InstallerViewState, string> = {
  hidden: "codexDesktop.state.hidden",
  checking: "codexDesktop.state.checking",
  unsupported_architecture: "codexDesktop.state.unsupportedArchitecture",
  ambiguous: "codexDesktop.state.ambiguous",
  ready_install: "codexDesktop.state.readyInstall",
  ready_update: "codexDesktop.state.updateAvailable",
  ready_launch: "codexDesktop.state.upToDate",
  local_newer: "codexDesktop.state.localNewer",
  remote_unavailable: "codexDesktop.state.remoteUnavailable",
  remote_unavailable_installed: "codexDesktop.state.remoteUnavailableInstalled",
  job_checking: "codexDesktop.state.checking",
  job_preflight: "codexDesktop.state.preflight",
  job_downloading: "codexDesktop.state.downloading",
  job_verifying_download: "codexDesktop.state.verifyingDownload",
  job_installing: "codexDesktop.state.installing",
  job_verifying_installation: "codexDesktop.state.verifyingInstallation",
  succeeded: "codexDesktop.state.succeeded",
  failed: "codexDesktop.state.failed",
  cancelled: "codexDesktop.state.cancelled",
};

const primaryActionKeys: Record<
  Exclude<InstallerPrimaryAction, null>,
  string
> = {
  install: "codexDesktop.actions.install",
  update: "codexDesktop.actions.update",
  launch: "codexDesktop.actions.launch",
  retry: "codexDesktop.actions.retry",
  refresh: "codexDesktop.actions.refresh",
};

function formatBytes(value: number | null | undefined): string | null {
  if (value == null || !Number.isFinite(value) || value < 0) return null;
  const units = ["B", "KB", "MB", "GB"];
  let unitIndex = 0;
  let amount = value;
  while (amount >= 1024 && unitIndex < units.length - 1) {
    amount /= 1024;
    unitIndex += 1;
  }
  return (
    amount.toLocaleString(undefined, {
      maximumFractionDigits: unitIndex === 0 ? 0 : 1,
    }) +
    " " +
    units[unitIndex]
  );
}

function PrimaryActionIcon({
  action,
  pending,
}: {
  action: Exclude<InstallerPrimaryAction, null>;
  pending: boolean;
}) {
  if (pending) return <Loader2 className="h-4 w-4 animate-spin" />;
  switch (action) {
    case "install":
    case "update":
      return <Download className="h-4 w-4" />;
    case "launch":
      return <Play className="h-4 w-4" />;
    case "retry":
    case "refresh":
      return <RefreshCw className="h-4 w-4" />;
  }
}

export function CodexDesktopInstallerCard() {
  const { t } = useTranslation();
  const installer = useCodexDesktopInstaller();
  const { state, error, progress, localVersion, remoteVersion, primaryAction } =
    installer;

  if (state === "hidden") {
    return null;
  }

  const isWorking = state.startsWith("job_");
  const progressPercent =
    progress?.percent == null
      ? null
      : Math.max(0, Math.min(100, Math.round(progress.percent)));
  const showDownloadBytes = state === "job_downloading";
  const completedText = showDownloadBytes
    ? formatBytes(progress?.current)
    : null;
  const totalText = showDownloadBytes ? formatBytes(progress?.total) : null;
  const speedText = showDownloadBytes
    ? formatBytes(progress?.bytesPerSecond)
    : null;
  const primaryPending = installer.primaryDisabled && Boolean(primaryAction);
  const errorMessage =
    error &&
    t(error.messageKey, {
      defaultValue: error.details.redactedMessage ?? error.code,
    });
  const errorDetails = error
    ? [
        error.code,
        error.details.redactedMessage,
        error.details.platformErrorCode,
      ]
        .filter(Boolean)
        .join("\n")
    : null;

  return (
    <Card className="overflow-hidden border-border/80 bg-card">
      <CardHeader className="space-y-3 pb-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex min-w-0 items-start gap-3">
            <span
              aria-hidden="true"
              className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-border/70 bg-muted/50"
            >
              <CodexIcon size={22} />
            </span>
            <div className="min-w-0 space-y-1">
              <CardTitle className="text-base">
                {t("codexDesktop.title")}
              </CardTitle>
              <CardDescription className="leading-relaxed">
                {t("codexDesktop.description")}
              </CardDescription>
            </div>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="shrink-0"
            aria-label={t("codexDesktop.actions.refresh")}
            title={t("codexDesktop.actions.refresh")}
            disabled={installer.isRefreshing || isWorking}
            onClick={() => void installer.refresh()}
          >
            <RefreshCw
              className={
                "h-4 w-4 " + (installer.isRefreshing ? "animate-spin" : "")
              }
            />
          </Button>
        </div>
        <p className="flex items-center gap-2 text-xs text-muted-foreground">
          <ShieldCheck className="h-3.5 w-3.5 shrink-0" />
          {t("codexDesktop.source")}
        </p>
      </CardHeader>

      <CardContent className="space-y-4">
        <dl className="grid gap-2 text-sm sm:grid-cols-2">
          <div className="flex min-w-0 items-center justify-between gap-3 rounded-md bg-muted/40 px-3 py-2">
            <dt className="text-muted-foreground">
              {t("codexDesktop.details.localVersion")}
            </dt>
            <dd className="truncate font-medium tabular-nums">
              {localVersion ?? t("common.notInstalled")}
            </dd>
          </div>
          <div className="flex min-w-0 items-center justify-between gap-3 rounded-md bg-muted/40 px-3 py-2">
            <dt className="text-muted-foreground">
              {t("codexDesktop.details.latestVersion")}
            </dt>
            <dd className="truncate font-medium tabular-nums">
              {remoteVersion ?? t("codexDesktop.details.unavailable")}
            </dd>
          </div>
        </dl>

        <div
          className="flex items-center gap-2 text-sm text-muted-foreground"
          aria-live="polite"
        >
          {isWorking ? (
            <Loader2 className="h-4 w-4 shrink-0 animate-spin" />
          ) : state === "succeeded" ? (
            <CheckCircle2 className="h-4 w-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
          ) : state === "failed" || state === "ambiguous" ? (
            <AlertCircle className="h-4 w-4 shrink-0 text-destructive" />
          ) : (
            <ShieldCheck className="h-4 w-4 shrink-0" />
          )}
          <span>
            {t(stateMessageKeys[state], {
              version: remoteVersion,
              defaultValue: state,
            })}
          </span>
        </div>

        {progress && (
          <div className="space-y-2">
            <div className="flex items-center justify-between gap-3 text-xs text-muted-foreground">
              <span>{t("codexDesktop.details.progress")}</span>
              <span className="shrink-0 tabular-nums">
                {progressPercent == null ? null : progressPercent + "%"}
                {completedText
                  ? " · " + completedText + (totalText ? " / " + totalText : "")
                  : null}
                {speedText ? " · " + speedText + "/s" : null}
              </span>
            </div>
            <div
              className="h-2 w-full overflow-hidden rounded-full bg-muted"
              role="progressbar"
              aria-label={t("codexDesktop.details.progress")}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={progressPercent ?? undefined}
            >
              <div
                className={
                  "h-full rounded-full bg-blue-500 transition-[width] duration-200 " +
                  (progressPercent == null ? "w-1/3 animate-pulse" : "")
                }
                style={
                  progressPercent == null
                    ? undefined
                    : { width: String(progressPercent) + "%" }
                }
              />
            </div>
          </div>
        )}

        {error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertTitle>{t("codexDesktop.error.title")}</AlertTitle>
            <AlertDescription className="space-y-2">
              <p>{errorMessage}</p>
              <details className="font-mono text-xs">
                <summary className="cursor-pointer select-none">
                  {t("codexDesktop.error.details")}
                </summary>
                <pre className="mt-2 overflow-x-auto whitespace-pre-wrap break-words rounded bg-background/60 p-2">
                  {errorDetails}
                </pre>
              </details>
            </AlertDescription>
          </Alert>
        )}
      </CardContent>

      <CardFooter className="flex flex-wrap items-center gap-2">
        {primaryAction && (
          <Button
            type="button"
            disabled={installer.primaryDisabled}
            onClick={() => void installer.runPrimaryAction()}
          >
            <PrimaryActionIcon
              action={primaryAction}
              pending={primaryPending}
            />
            {t(primaryActionKeys[primaryAction])}
          </Button>
        )}
        {installer.canCancel && (
          <Button
            type="button"
            variant="outline"
            onClick={() => void installer.cancel()}
          >
            <X className="h-4 w-4" />
            {t("codexDesktop.actions.cancel")}
          </Button>
        )}
        {error && (
          <>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => void installer.copyErrorDetails()}
            >
              {t("codexDesktop.actions.copyErrorDetails")}
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => void installer.openLogs()}
            >
              <FolderOpen className="h-3.5 w-3.5" />
              {t("codexDesktop.actions.openLogDirectory")}
            </Button>
          </>
        )}
      </CardFooter>
    </Card>
  );
}
