import { AlertCircle, Loader2, RefreshCw, Search, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";

interface WorkBuddyExistingModelsCardProps {
  ids: string[];
  isLoading: boolean;
  isRefreshing: boolean;
  hasError: boolean;
  onRefresh: () => void;
}

/**
 * Existing IDs are a read-only, safe projection. Search is intentionally
 * local and immediate so it never creates a new backend request or reveals
 * the rest of the WorkBuddy model configuration.
 */
export function WorkBuddyExistingModelsCard({
  ids,
  isLoading,
  isRefreshing,
  hasError,
  onRefresh,
}: WorkBuddyExistingModelsCardProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const filteredIds = useMemo(() => {
    const query = search.trim().toLocaleLowerCase();
    if (!query) return ids;
    return ids.filter((id) => id.toLocaleLowerCase().includes(query));
  }, [ids, search]);

  return (
    <Card className="min-w-0">
      <CardHeader className="flex flex-row items-center justify-between gap-3 pb-3">
        <CardTitle className="min-w-0 text-base">
          {t("workbuddy.existingModels.title", { count: ids.length })}
        </CardTitle>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={isRefreshing}
          onClick={onRefresh}
        >
          <RefreshCw
            className={
              isRefreshing ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"
            }
          />
          {t("workbuddy.existingModels.refresh")}
        </Button>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="relative">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t("workbuddy.existingModels.searchPlaceholder")}
            aria-label={t("workbuddy.existingModels.searchPlaceholder")}
            className="pr-9 pl-9"
          />
          {search ? (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2"
              onClick={() => setSearch("")}
              aria-label={t("workbuddy.existingModels.clearSearch")}
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          ) : null}
        </div>

        {hasError ? (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>
              {t("workbuddy.existingModels.readFailed")}
            </AlertDescription>
          </Alert>
        ) : null}

        {isLoading && ids.length === 0 ? (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("workbuddy.existingModels.loading")}
          </p>
        ) : ids.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t("workbuddy.existingModels.empty")}
          </p>
        ) : filteredIds.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t("workbuddy.existingModels.noMatch")}
          </p>
        ) : (
          <ScrollArea
            className="h-56 rounded-md border border-border/70"
            aria-label={t("workbuddy.existingModels.list")}
          >
            <ul className="divide-y divide-border/70">
              {filteredIds.map((id) => (
                <li key={id} className="min-w-0 px-3 py-2">
                  <code className="block truncate font-mono text-xs">{id}</code>
                </li>
              ))}
            </ul>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
