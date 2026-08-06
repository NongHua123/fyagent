import { ExternalLink, FileText, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { WorkBuddyStatus } from "@/lib/api/workbuddy";

interface WorkBuddyStatusCardProps {
  status?: WorkBuddyStatus;
  isLoading: boolean;
  isOpeningWebsite: boolean;
  onOpenWebsite: () => void;
}

/**
 * The status card receives only the renderer-safe status projection. It never
 * needs the full WorkBuddy document or credentials to render its summary.
 */
export function WorkBuddyStatusCard({
  status,
  isLoading,
  isOpeningWebsite,
  onOpenWebsite,
}: WorkBuddyStatusCardProps) {
  const { t } = useTranslation();

  return (
    <Card className="min-w-0">
      <CardHeader className="flex flex-row items-center justify-between gap-3 pb-3">
        <CardTitle className="flex min-w-0 items-center gap-2 text-base">
          <FileText className="h-4 w-4 shrink-0" />
          {t("workbuddy.status.title")}
        </CardTitle>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={isOpeningWebsite}
          onClick={onOpenWebsite}
          title={t("workbuddy.status.downloadHint")}
        >
          {isOpeningWebsite ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <ExternalLink className="h-3.5 w-3.5" />
          )}
          {t("workbuddy.status.download")}
        </Button>
      </CardHeader>
      <CardContent
        className="space-y-2 text-sm"
        aria-live="polite"
        aria-busy={isLoading}
      >
        {isLoading && !status ? (
          <p className="flex items-center gap-2 text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("workbuddy.status.loading")}
          </p>
        ) : status ? (
          <dl className="grid grid-cols-2 gap-x-4 gap-y-3">
            <div className="col-span-2 min-w-0">
              <dt className="text-xs text-muted-foreground">
                {t("workbuddy.status.path")}
              </dt>
              <dd className="truncate font-mono text-xs" title={status.path}>
                {status.path}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">
                {t("workbuddy.status.exists")}
              </dt>
              <dd>
                {status.exists
                  ? t("workbuddy.status.existsYes")
                  : t("workbuddy.status.existsNo")}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">
                {t("workbuddy.status.modelCount")}
              </dt>
              <dd>{status.modelCount}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">
                {t("workbuddy.status.backup")}
              </dt>
              <dd>
                {status.backupExists
                  ? t("workbuddy.status.backupYes")
                  : t("workbuddy.status.backupNo")}
              </dd>
            </div>
          </dl>
        ) : (
          <p className="text-sm text-muted-foreground">
            {t("workbuddy.status.unavailable")}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
