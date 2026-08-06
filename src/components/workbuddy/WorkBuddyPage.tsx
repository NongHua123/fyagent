import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertCircle,
  CheckSquare,
  Eye,
  EyeOff,
  Loader2,
  RefreshCw,
  Save,
  Search,
  Square,
  X,
} from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { WorkBuddyIcon } from "@/components/BrandIcons";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  getWorkBuddyError,
  type WorkBuddyError,
  type WorkBuddySaveModelsRequest,
  workBuddyApi,
} from "@/lib/api/workbuddy";
import { settingsApi } from "@/lib/api/settings";
import {
  useWorkBuddyModelIdsQuery,
  useWorkBuddyStatusQuery,
  workBuddyKeys,
} from "@/lib/query/workbuddy";
import { cn } from "@/lib/utils";
import { WorkBuddyDuplicateConflictDialog } from "./WorkBuddyDuplicateConflictDialog";
import { WorkBuddyExistingModelsCard } from "./WorkBuddyExistingModelsCard";
import { WorkBuddyStatusCard } from "./WorkBuddyStatusCard";
import { hasWorkBuddyRemoteHttpWarning } from "./urlSafety";

const WORKBUDDY_WEBSITE_URL = "https://www.workbuddy.cn/";

const parseManualModelIds = (value: string): string[] =>
  value
    .split(/\r?\n/)
    .map((modelId) => modelId.trim())
    .filter(Boolean);

const errorKeyByCode: Partial<Record<WorkBuddyError["code"], string>> = {
  WORKBUDDY_INVALID_URL: "workbuddy.error.invalidUrl",
  WORKBUDDY_API_KEY_REQUIRED: "workbuddy.error.apiKeyRequired",
  WORKBUDDY_FETCH_HTTP_ERROR: "workbuddy.error.fetchHttp",
  WORKBUDDY_FETCH_TIMEOUT: "workbuddy.error.fetchTimeout",
  WORKBUDDY_FETCH_REDIRECT_REJECTED: "workbuddy.error.fetchRedirectRejected",
  WORKBUDDY_FETCH_RESPONSE_TOO_LARGE: "workbuddy.error.fetchResponseTooLarge",
  WORKBUDDY_FETCH_INVALID_SCHEMA: "workbuddy.error.fetchInvalidSchema",
  WORKBUDDY_CONFIG_READ_FAILED: "workbuddy.error.configReadFailed",
  WORKBUDDY_CONFIG_INVALID_JSON: "workbuddy.error.configInvalidJson",
  WORKBUDDY_CONFIG_ROOT_NOT_ARRAY: "workbuddy.error.configUnsupportedRoot",
  WORKBUDDY_CONFIG_ROOT_UNSUPPORTED: "workbuddy.error.configUnsupportedRoot",
  WORKBUDDY_CONFIG_MODELS_NOT_ARRAY: "workbuddy.error.configModelsNotArray",
  WORKBUDDY_CONFIG_INVALID_ENTRY: "workbuddy.error.configInvalidEntry",
  WORKBUDDY_CONFIG_NO_TARGET_MODELS: "workbuddy.error.configNoTargetModels",
  WORKBUDDY_CONFIG_CONCURRENT_MODIFICATION:
    "workbuddy.error.configConcurrentModification",
  WORKBUDDY_CONFIG_BACKUP_FAILED: "workbuddy.error.configBackupFailed",
  WORKBUDDY_CONFIG_WRITE_FAILED: "workbuddy.error.configWriteFailed",
  WORKBUDDY_CONFIG_DUPLICATE_TARGET: "workbuddy.error.configDuplicateTarget",
  WORKBUDDY_CONFIG_EXISTING_TARGET: "workbuddy.error.configDuplicateTarget",
  WORKBUDDY_OVERWRITE_TOKEN_INVALID: "workbuddy.error.overwriteTokenInvalid",
  WORKBUDDY_OVERWRITE_TOKEN_EXPIRED: "workbuddy.error.overwriteTokenExpired",
  WORKBUDDY_INTERNAL_ERROR: "workbuddy.error.internal",
};

interface PendingOverwriteSave {
  request: WorkBuddySaveModelsRequest;
  token: string;
  existingIds: string[];
}

interface WorkBuddyErrorPresentation {
  message: string;
  httpStatus?: number;
  redactedSummary?: string;
}

function WorkBuddyErrorDescription({
  error,
}: {
  error: WorkBuddyErrorPresentation;
}) {
  const { t } = useTranslation();

  return (
    <div className="space-y-1">
      <p>{error.message}</p>
      {error.httpStatus !== undefined ? (
        <p className="text-xs">
          {t("workbuddy.error.httpStatus", { status: error.httpStatus })}
        </p>
      ) : null}
      {error.redactedSummary ? (
        <p className="break-words text-xs">
          {t("workbuddy.error.redactedSummary", {
            summary: error.redactedSummary,
          })}
        </p>
      ) : null}
    </div>
  );
}

export function WorkBuddyPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const statusQuery = useWorkBuddyStatusQuery();
  const existingModelIdsQuery = useWorkBuddyModelIdsQuery();
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [allowNoApiKey, setAllowNoApiKey] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [clearExistingApiKeys, setClearExistingApiKeys] = useState(false);
  const [manualModels, setManualModels] = useState("");
  const [fetchedModels, setFetchedModels] = useState<string[]>([]);
  const [selectedModelIds, setSelectedModelIds] = useState<Set<string>>(
    () => new Set(),
  );
  const [hasFetchedModels, setHasFetchedModels] = useState(false);
  const [fetchedModelsTruncated, setFetchedModelsTruncated] = useState(false);
  const [remoteSearch, setRemoteSearch] = useState("");
  const [apiKeyError, setApiKeyError] = useState<string | null>(null);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [fetchError, setFetchError] =
    useState<WorkBuddyErrorPresentation | null>(null);
  const [saveError, setSaveError] = useState<WorkBuddyErrorPresentation | null>(
    null,
  );
  const [isFetching, setIsFetching] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isOpeningWebsite, setIsOpeningWebsite] = useState(false);
  const [pendingOverwriteSave, setPendingOverwriteSave] =
    useState<PendingOverwriteSave | null>(null);
  const fetchSequenceRef = useRef(0);
  const apiKeyRef = useRef("");
  const isMountedRef = useRef(true);

  useEffect(() => {
    apiKeyRef.current = apiKey;
  }, [apiKey]);

  useEffect(() => {
    isMountedRef.current = true;
    const clearApiKeyReference = () => {
      apiKeyRef.current = "";
      fetchSequenceRef.current += 1;
    };

    window.addEventListener("pagehide", clearApiKeyReference);
    return () => {
      isMountedRef.current = false;
      window.removeEventListener("pagehide", clearApiKeyReference);
      clearApiKeyReference();
    };
  }, []);

  const existingModelIds = existingModelIdsQuery.data?.ids ?? [];
  const existingModelIdSet = useMemo(
    () => new Set(existingModelIds),
    [existingModelIds],
  );
  const selectedFetchedModelIds = useMemo(
    () => fetchedModels.filter((modelId) => selectedModelIds.has(modelId)),
    [fetchedModels, selectedModelIds],
  );
  const manualModelIds = useMemo(
    () => parseManualModelIds(manualModels),
    [manualModels],
  );
  const targetModelIds = useMemo(
    () => new Set([...selectedFetchedModelIds, ...manualModelIds]),
    [manualModelIds, selectedFetchedModelIds],
  );
  const hasTargetModels = targetModelIds.size > 0;
  const hasExistingTarget = useMemo(
    () =>
      [...targetModelIds].some((modelId) => existingModelIdSet.has(modelId)),
    [existingModelIdSet, targetModelIds],
  );
  const showClearExistingApiKeys = !apiKey.trim() && hasExistingTarget;
  const remoteHttpWarning = hasWorkBuddyRemoteHttpWarning(baseUrl);
  const filteredFetchedModels = useMemo(() => {
    const query = remoteSearch.trim().toLocaleLowerCase();
    if (!query) return fetchedModels;
    return fetchedModels.filter((modelId) =>
      modelId.toLocaleLowerCase().includes(query),
    );
  }, [fetchedModels, remoteSearch]);
  const selectedFilteredModelCount = useMemo(
    () =>
      filteredFetchedModels.filter((modelId) => selectedModelIds.has(modelId))
        .length,
    [filteredFetchedModels, selectedModelIds],
  );

  useEffect(() => {
    if (!showClearExistingApiKeys) setClearExistingApiKeys(false);
  }, [showClearExistingApiKeys]);

  const refreshConfiguration = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: workBuddyKeys.status() }),
      queryClient.invalidateQueries({ queryKey: workBuddyKeys.modelIds() }),
    ]);
  }, [queryClient]);

  const setKey = useCallback(
    (value: string) => {
      apiKeyRef.current = value;
      setApiKey(value);
      if (value.trim() || allowNoApiKey) setApiKeyError(null);
    },
    [allowNoApiKey],
  );

  const requireApiKey = useCallback((): boolean => {
    if (allowNoApiKey || apiKeyRef.current.trim()) {
      setApiKeyError(null);
      return true;
    }

    setApiKeyError(t("workbuddy.validation.apiKeyRequired"));
    return false;
  }, [allowNoApiKey, t]);

  const getSafeErrorMessage = useCallback(
    (
      error: unknown,
    ): {
      presentation: WorkBuddyErrorPresentation;
      structured: WorkBuddyError | null;
    } => {
      const structured = getWorkBuddyError(error);
      if (!structured) {
        return {
          presentation: { message: t("workbuddy.error.internal") },
          structured: null,
        };
      }

      const fallbackKey =
        errorKeyByCode[structured.code] ?? "workbuddy.error.internal";
      const { httpStatus, redactedSummary } = structured.details;
      return {
        presentation: {
          message: t(structured.messageKey, {
            defaultValue: t(fallbackKey),
          }),
          ...(typeof httpStatus === "number" && Number.isInteger(httpStatus)
            ? { httpStatus }
            : {}),
          ...(redactedSummary ? { redactedSummary } : {}),
        },
        structured,
      };
    },
    [t],
  );

  const buildSaveRequest = useCallback(
    (overwriteToken?: string): WorkBuddySaveModelsRequest => ({
      baseUrl,
      apiKey: apiKeyRef.current,
      allowNoApiKey,
      selectedModelIds: selectedFetchedModelIds,
      manualModelIds,
      clearExistingApiKeys,
      expectedRevision:
        existingModelIdsQuery.data?.revision ??
        statusQuery.data?.revision ??
        null,
      ...(overwriteToken ? { overwriteToken } : {}),
    }),
    [
      allowNoApiKey,
      baseUrl,
      clearExistingApiKeys,
      existingModelIdsQuery.data?.revision,
      manualModelIds,
      selectedFetchedModelIds,
      statusQuery.data?.revision,
    ],
  );

  const saveRequest = useCallback(
    async (request: WorkBuddySaveModelsRequest): Promise<void> => {
      setIsSaving(true);
      setSaveError(null);
      try {
        const result = await workBuddyApi.saveModels(request);
        if (result.state === "saved") {
          setPendingOverwriteSave(null);
          await refreshConfiguration();
          toast.success(t("workbuddy.saveSuccess"));
          return;
        }

        if (result.state === "overwrite_confirmation_required") {
          // The immutable snapshot prevents confirmation from saving later
          // edits. The opaque token is renderer-memory only and consumed once.
          setPendingOverwriteSave({
            request: { ...request },
            token: result.token,
            existingIds: result.existingIds,
          });
          return;
        }

        setPendingOverwriteSave(null);
        await refreshConfiguration();
        const presentation = {
          message: t("workbuddy.error.configConcurrentModification"),
        };
        setSaveError(presentation);
        toast.error(presentation.message);
      } catch (error) {
        const { presentation, structured } = getSafeErrorMessage(error);
        const mustRefresh =
          structured?.code === "WORKBUDDY_CONFIG_CONCURRENT_MODIFICATION" ||
          structured?.code === "WORKBUDDY_OVERWRITE_TOKEN_INVALID" ||
          structured?.code === "WORKBUDDY_OVERWRITE_TOKEN_EXPIRED";

        if (mustRefresh) {
          setPendingOverwriteSave(null);
          await refreshConfiguration();
        }

        setSaveError(presentation);
        toast.error(presentation.message);
      } finally {
        if (isMountedRef.current) setIsSaving(false);
      }
    },
    [getSafeErrorMessage, refreshConfiguration, t],
  );

  const handleFetch = async (): Promise<void> => {
    if (!requireApiKey()) return;

    const sequence = fetchSequenceRef.current + 1;
    fetchSequenceRef.current = sequence;
    setFetchError(null);
    setIsFetching(true);

    try {
      const result = await workBuddyApi.fetchModels({
        baseUrl,
        apiKey: apiKeyRef.current,
        allowNoApiKey,
      });
      if (sequence !== fetchSequenceRef.current) return;

      setFetchedModels(result.models);
      setSelectedModelIds(new Set(result.models));
      setFetchedModelsTruncated(result.truncated);
      setHasFetchedModels(true);
    } catch (error) {
      if (sequence !== fetchSequenceRef.current) return;
      const { presentation } = getSafeErrorMessage(error);
      setFetchError(presentation);
      toast.error(presentation.message);
    } finally {
      if (isMountedRef.current && sequence === fetchSequenceRef.current) {
        setIsFetching(false);
      }
    }
  };

  const handleSave = (): void => {
    if (!requireApiKey()) return;
    if (!hasTargetModels) {
      setModelsError(t("workbuddy.validation.modelRequired"));
      return;
    }

    setModelsError(null);
    void saveRequest(buildSaveRequest());
  };

  const toggleFetchedModel = (modelId: string, checked: boolean): void => {
    setSelectedModelIds((current) => {
      const next = new Set(current);
      if (checked) next.add(modelId);
      else next.delete(modelId);
      return next;
    });
    setModelsError(null);
  };

  const selectFilteredModels = (): void => {
    setSelectedModelIds((current) => {
      const next = new Set(current);
      filteredFetchedModels.forEach((modelId) => next.add(modelId));
      return next;
    });
    setModelsError(null);
  };

  const clearFilteredModels = (): void => {
    setSelectedModelIds((current) => {
      const next = new Set(current);
      filteredFetchedModels.forEach((modelId) => next.delete(modelId));
      return next;
    });
  };

  const handleOpenWebsite = async (): Promise<void> => {
    setIsOpeningWebsite(true);
    try {
      await settingsApi.openExternal(WORKBUDDY_WEBSITE_URL);
    } catch {
      // Keep the platform error private: it may include a system path or
      // browser detail and does not help the user resolve this fixed action.
      toast.error(t("workbuddy.status.downloadFailed"));
    } finally {
      if (isMountedRef.current) setIsOpeningWebsite(false);
    }
  };

  return (
    <div
      className="w-full min-w-0 space-y-5 px-4 py-6 sm:px-6 lg:px-8"
      style={{ scrollbarGutter: "stable" }}
    >
      <header className="flex min-w-0 items-start gap-3">
        <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-border/70 bg-muted/50">
          <WorkBuddyIcon size={24} />
        </span>
        <div className="min-w-0 space-y-1">
          <h1 className="text-xl font-semibold">{t("workbuddy.title")}</h1>
          <p className="text-sm text-muted-foreground">
            {t("workbuddy.description")}
          </p>
        </div>
      </header>

      <div className="grid min-w-0 gap-5 xl:grid-cols-2">
        <WorkBuddyStatusCard
          status={statusQuery.data}
          isLoading={statusQuery.isLoading}
          isOpeningWebsite={isOpeningWebsite}
          onOpenWebsite={() => void handleOpenWebsite()}
        />
        <WorkBuddyExistingModelsCard
          ids={existingModelIds}
          isLoading={existingModelIdsQuery.isLoading}
          isRefreshing={existingModelIdsQuery.isFetching}
          hasError={existingModelIdsQuery.isError}
          onRefresh={() => void existingModelIdsQuery.refetch()}
        />
      </div>

      <Card className="min-w-0">
        <CardHeader className="pb-4">
          <CardTitle className="text-base">
            {t("workbuddy.connection.title")}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="space-y-2">
            <Label htmlFor="workbuddy-base-url">{t("workbuddy.baseUrl")}</Label>
            <Input
              id="workbuddy-base-url"
              name="workbuddy-base-url"
              type="url"
              value={baseUrl}
              onChange={(event) => setBaseUrl(event.target.value)}
              placeholder="https://api.example.com/v1"
              autoComplete="off"
              spellCheck={false}
              aria-describedby={
                remoteHttpWarning ? "workbuddy-http-warning" : undefined
              }
            />
            {remoteHttpWarning ? (
              <Alert id="workbuddy-http-warning" variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertTitle>{t("workbuddy.httpWarning.title")}</AlertTitle>
                <AlertDescription>
                  {t("workbuddy.httpWarning.description")}
                </AlertDescription>
              </Alert>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="workbuddy-api-key">{t("workbuddy.apiKey")}</Label>
            <div className="flex gap-2">
              <Input
                id="workbuddy-api-key"
                name="workbuddy-api-key"
                type={showApiKey ? "text" : "password"}
                value={apiKey}
                onChange={(event) => setKey(event.target.value)}
                aria-invalid={Boolean(apiKeyError)}
                aria-describedby={
                  apiKeyError ? "workbuddy-api-key-error" : undefined
                }
                autoComplete="off"
              />
              <Button
                type="button"
                variant="outline"
                size="icon"
                aria-label={
                  showApiKey
                    ? t("workbuddy.hideApiKey")
                    : t("workbuddy.showApiKey")
                }
                title={
                  showApiKey
                    ? t("workbuddy.hideApiKey")
                    : t("workbuddy.showApiKey")
                }
                onClick={() => setShowApiKey((visible) => !visible)}
              >
                {showApiKey ? (
                  <EyeOff className="h-4 w-4" />
                ) : (
                  <Eye className="h-4 w-4" />
                )}
              </Button>
            </div>
            {apiKeyError ? (
              <p
                id="workbuddy-api-key-error"
                className="text-sm text-destructive"
                role="alert"
              >
                {apiKeyError}
              </p>
            ) : null}
          </div>

          <div className="flex items-start gap-3 rounded-md border border-border/70 p-3">
            <Switch
              id="workbuddy-allow-no-api-key"
              checked={allowNoApiKey}
              onCheckedChange={(checked) => {
                setAllowNoApiKey(checked);
                if (checked) setApiKeyError(null);
              }}
              aria-label={t("workbuddy.allowNoApiKey")}
            />
            <div className="space-y-1">
              <Label
                htmlFor="workbuddy-allow-no-api-key"
                className="cursor-pointer"
              >
                {t("workbuddy.allowNoApiKey")}
              </Label>
              <p className="text-xs text-muted-foreground">
                {t("workbuddy.allowNoApiKeyHint")}
              </p>
            </div>
          </div>

          {showClearExistingApiKeys ? (
            <label className="flex cursor-pointer items-start gap-2 text-sm">
              <Checkbox
                checked={clearExistingApiKeys}
                onCheckedChange={(checked) =>
                  setClearExistingApiKeys(checked === true)
                }
                aria-label={t("workbuddy.clearExistingApiKeys")}
              />
              <span className="space-y-1">
                <span className="block font-medium">
                  {t("workbuddy.clearExistingApiKeys")}
                </span>
                <span className="block text-xs text-muted-foreground">
                  {t("workbuddy.clearExistingApiKeysHint")}
                </span>
              </span>
            </label>
          ) : null}

          <Button
            type="button"
            disabled={isFetching}
            onClick={() => void handleFetch()}
          >
            {isFetching ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
            {t("workbuddy.fetchModels")}
          </Button>
          {fetchError ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>{t("workbuddy.fetchFailed")}</AlertTitle>
              <AlertDescription>
                <WorkBuddyErrorDescription error={fetchError} />
              </AlertDescription>
            </Alert>
          ) : null}
        </CardContent>
      </Card>

      <Card className="min-w-0">
        <CardHeader className="space-y-3 pb-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <CardTitle className="text-base">
              {t("workbuddy.remoteModels.title")}
            </CardTitle>
            {hasFetchedModels ? (
              <p className="text-sm text-muted-foreground">
                {t("workbuddy.remoteModels.selectionSummary", {
                  selected: selectedFilteredModelCount,
                  total: filteredFetchedModels.length,
                })}
              </p>
            ) : null}
          </div>
          {hasFetchedModels ? (
            <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
              <div className="relative min-w-0 flex-1">
                <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={remoteSearch}
                  onChange={(event) => setRemoteSearch(event.target.value)}
                  placeholder={t("workbuddy.remoteModels.searchPlaceholder")}
                  aria-label={t("workbuddy.remoteModels.searchPlaceholder")}
                  className="pr-9 pl-9"
                />
                {remoteSearch ? (
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2"
                    onClick={() => setRemoteSearch("")}
                    aria-label={t("workbuddy.remoteModels.clearSearch")}
                  >
                    <X className="h-3.5 w-3.5" />
                  </Button>
                ) : null}
              </div>
              <div className="flex shrink-0 gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={filteredFetchedModels.length === 0}
                  onClick={selectFilteredModels}
                >
                  <CheckSquare className="h-4 w-4" />
                  {t("workbuddy.remoteModels.selectCurrent")}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={filteredFetchedModels.length === 0}
                  onClick={clearFilteredModels}
                >
                  <Square className="h-4 w-4" />
                  {t("workbuddy.remoteModels.deselectCurrent")}
                </Button>
              </div>
            </div>
          ) : null}
          {fetchedModelsTruncated ? (
            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>{t("workbuddy.models.truncatedTitle")}</AlertTitle>
              <AlertDescription>
                {t("workbuddy.models.truncatedDescription")}
              </AlertDescription>
            </Alert>
          ) : null}
        </CardHeader>
        <CardContent className="space-y-5">
          {hasFetchedModels ? (
            fetchedModels.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {t("workbuddy.models.emptyFetch")}
              </p>
            ) : filteredFetchedModels.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {t("workbuddy.remoteModels.noMatch")}
              </p>
            ) : (
              <ScrollArea
                className="h-64 rounded-md border border-border/70"
                aria-label={t("workbuddy.models.fetchedList")}
              >
                <ul className="divide-y divide-border/70">
                  {filteredFetchedModels.map((modelId) => {
                    const checked = selectedModelIds.has(modelId);
                    const checkboxId = `workbuddy-model-${modelId}`;
                    const isExisting = existingModelIdSet.has(modelId);
                    return (
                      <li key={modelId}>
                        <label
                          htmlFor={checkboxId}
                          className={cn(
                            "flex cursor-pointer items-center gap-3 px-3 py-2 text-sm hover:bg-muted/50",
                            !checked && "text-muted-foreground",
                          )}
                        >
                          <Checkbox
                            id={checkboxId}
                            checked={checked}
                            onCheckedChange={(value) =>
                              toggleFetchedModel(modelId, value === true)
                            }
                          />
                          <code className="min-w-0 flex-1 truncate font-mono text-xs">
                            {modelId}
                          </code>
                          {isExisting ? (
                            <Badge variant="secondary" className="shrink-0">
                              {t("workbuddy.remoteModels.existing")}
                            </Badge>
                          ) : null}
                        </label>
                      </li>
                    );
                  })}
                </ul>
              </ScrollArea>
            )
          ) : (
            <p className="text-sm text-muted-foreground">
              {t("workbuddy.models.notFetched")}
            </p>
          )}

          <div className="space-y-2">
            <Label htmlFor="workbuddy-manual-models">
              {t("workbuddy.models.manual")}
            </Label>
            <Textarea
              id="workbuddy-manual-models"
              name="workbuddy-manual-models"
              value={manualModels}
              onChange={(event) => {
                setManualModels(event.target.value);
                setModelsError(null);
              }}
              placeholder={t("workbuddy.models.manualPlaceholder")}
              autoComplete="off"
              aria-describedby={
                modelsError ? "workbuddy-models-error" : undefined
              }
            />
            <p className="text-xs text-muted-foreground">
              {t("workbuddy.models.manualHint")}
            </p>
            {modelsError ? (
              <p
                id="workbuddy-models-error"
                className="text-sm text-destructive"
                role="alert"
              >
                {modelsError}
              </p>
            ) : null}
          </div>

          {saveError ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>{t("workbuddy.saveFailed")}</AlertTitle>
              <AlertDescription>
                <WorkBuddyErrorDescription error={saveError} />
              </AlertDescription>
            </Alert>
          ) : null}

          <Button type="button" disabled={isSaving} onClick={handleSave}>
            {isSaving ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Save className="h-4 w-4" />
            )}
            {isSaving ? t("workbuddy.saving") : t("workbuddy.save")}
          </Button>
        </CardContent>
      </Card>

      <WorkBuddyDuplicateConflictDialog
        existingIds={pendingOverwriteSave?.existingIds ?? []}
        isOpen={Boolean(pendingOverwriteSave)}
        isSaving={isSaving}
        onCancel={() => setPendingOverwriteSave(null)}
        onConfirm={() => {
          if (!pendingOverwriteSave) return;
          void saveRequest({
            ...pendingOverwriteSave.request,
            overwriteToken: pendingOverwriteSave.token,
          });
        }}
      />
    </div>
  );
}
