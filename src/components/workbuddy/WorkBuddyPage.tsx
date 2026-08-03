import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertCircle,
  CheckSquare,
  Eye,
  EyeOff,
  FileText,
  Loader2,
  RefreshCw,
  Save,
  Square,
} from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { WorkBuddyIcon } from "@/components/BrandIcons";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  getWorkBuddyError,
  type WorkBuddyError,
  type WorkBuddySaveModelsRequest,
  workBuddyApi,
} from "@/lib/api";
import { useWorkBuddyStatusQuery, workBuddyKeys } from "@/lib/query";
import { cn } from "@/lib/utils";
import { WorkBuddyDuplicateConflictDialog } from "./WorkBuddyDuplicateConflictDialog";
import { hasWorkBuddyRemoteHttpWarning } from "./urlSafety";

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
  WORKBUDDY_CONFIG_ROOT_NOT_ARRAY: "workbuddy.error.configRootNotArray",
  WORKBUDDY_CONFIG_INVALID_ENTRY: "workbuddy.error.configInvalidEntry",
  WORKBUDDY_CONFIG_NO_TARGET_MODELS: "workbuddy.error.configNoTargetModels",
  WORKBUDDY_CONFIG_CONCURRENT_MODIFICATION:
    "workbuddy.error.configConcurrentModification",
  WORKBUDDY_CONFIG_BACKUP_FAILED: "workbuddy.error.configBackupFailed",
  WORKBUDDY_CONFIG_WRITE_FAILED: "workbuddy.error.configWriteFailed",
  WORKBUDDY_INTERNAL_ERROR: "workbuddy.error.internal",
  WORKBUDDY_CONFIG_DUPLICATE_TARGET: "workbuddy.error.configDuplicateTarget",
};

interface PendingDuplicateSave {
  request: WorkBuddySaveModelsRequest;
  duplicates: NonNullable<WorkBuddyError["details"]["duplicateIds"]>;
}

interface WorkBuddyErrorPresentation {
  message: string;
  httpStatus?: number;
  redactedSummary?: string;
  invalidEntryIndex?: number;
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
      {error.invalidEntryIndex !== undefined ? (
        <p className="text-xs">
          {t("workbuddy.error.invalidEntryIndex", {
            index: error.invalidEntryIndex,
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
  const [apiKeyError, setApiKeyError] = useState<string | null>(null);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [fetchError, setFetchError] =
    useState<WorkBuddyErrorPresentation | null>(null);
  const [saveError, setSaveError] = useState<WorkBuddyErrorPresentation | null>(
    null,
  );
  const [isFetching, setIsFetching] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [pendingDuplicateSave, setPendingDuplicateSave] =
    useState<PendingDuplicateSave | null>(null);
  const fetchSequenceRef = useRef(0);
  const apiKeyRef = useRef("");
  const isMountedRef = useRef(true);

  useEffect(() => {
    apiKeyRef.current = apiKey;
  }, [apiKey]);

  useEffect(() => {
    isMountedRef.current = true;

    return () => {
      // The key is intentionally component-only and is never persisted or
      // copied into a query cache. Clear the last retained value on unmount.
      isMountedRef.current = false;
      fetchSequenceRef.current += 1;
      apiKeyRef.current = "";
    };
  }, []);

  const selectedFetchedModelIds = useMemo(
    () => fetchedModels.filter((modelId) => selectedModelIds.has(modelId)),
    [fetchedModels, selectedModelIds],
  );
  const manualModelIds = useMemo(
    () => parseManualModelIds(manualModels),
    [manualModels],
  );
  const hasTargetModels = useMemo(
    () => new Set([...selectedFetchedModelIds, ...manualModelIds]).size > 0,
    [manualModelIds, selectedFetchedModelIds],
  );
  const remoteHttpWarning = hasWorkBuddyRemoteHttpWarning(baseUrl);

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
      const { httpStatus, redactedSummary, invalidEntryIndex } =
        structured.details;
      return {
        presentation: {
          message: t(structured.messageKey, {
            defaultValue: t(fallbackKey),
          }),
          ...(typeof httpStatus === "number" && Number.isInteger(httpStatus)
            ? { httpStatus }
            : {}),
          ...(redactedSummary ? { redactedSummary } : {}),
          ...(typeof invalidEntryIndex === "number" &&
          Number.isInteger(invalidEntryIndex) &&
          invalidEntryIndex >= 0
            ? { invalidEntryIndex }
            : {}),
        },
        structured,
      };
    },
    [t],
  );

  const buildSaveRequest = useCallback(
    (duplicatePolicy?: "reject" | "updateAll"): WorkBuddySaveModelsRequest => ({
      baseUrl,
      apiKey: apiKeyRef.current,
      allowNoApiKey,
      selectedModelIds: selectedFetchedModelIds,
      manualModelIds,
      clearExistingApiKeys,
      expectedRevision: statusQuery.data?.revision ?? null,
      ...(duplicatePolicy ? { duplicatePolicy } : {}),
    }),
    [
      allowNoApiKey,
      baseUrl,
      clearExistingApiKeys,
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
        await workBuddyApi.saveModels(request);
        apiKeyRef.current = "";
        setApiKey("");
        setClearExistingApiKeys(false);
        setPendingDuplicateSave(null);
        await queryClient.invalidateQueries({
          queryKey: workBuddyKeys.status(),
        });
        toast.success(t("workbuddy.saveSuccess"));
      } catch (error) {
        const { presentation, structured } = getSafeErrorMessage(error);
        const isInitialDuplicateConflict =
          structured?.code === "WORKBUDDY_CONFIG_DUPLICATE_TARGET" &&
          request.duplicatePolicy !== "updateAll";

        if (isInitialDuplicateConflict) {
          // Keep a full, immutable snapshot so the confirmation cannot save a
          // later-edited URL/key/model set by accident.
          setPendingDuplicateSave({
            request: { ...request },
            duplicates: structured.details.duplicateIds ?? [],
          });
        } else {
          setPendingDuplicateSave(null);
          setSaveError(presentation);
          toast.error(presentation.message);
        }
      } finally {
        setIsSaving(false);
      }
    },
    [getSafeErrorMessage, queryClient, t],
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
    void saveRequest(buildSaveRequest("reject"));
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

  return (
    <div className="mx-auto w-full max-w-4xl space-y-5 px-4 py-6 sm:px-6">
      <header className="flex items-start gap-3">
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

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <FileText className="h-4 w-4" />
            {t("workbuddy.status.title")}
          </CardTitle>
        </CardHeader>
        <CardContent
          className="space-y-2 text-sm"
          aria-live="polite"
          aria-busy={statusQuery.isLoading}
        >
          {statusQuery.isLoading ? (
            <p className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t("workbuddy.status.loading")}
            </p>
          ) : statusQuery.data ? (
            <dl className="grid gap-x-4 gap-y-2 sm:grid-cols-[minmax(0,1fr)_auto]">
              <div className="min-w-0">
                <dt className="text-xs text-muted-foreground">
                  {t("workbuddy.status.path")}
                </dt>
                <dd
                  className="truncate font-mono text-xs"
                  title={statusQuery.data.path}
                >
                  {statusQuery.data.path}
                </dd>
              </div>
              <div className="flex gap-5 sm:items-end">
                <div>
                  <dt className="text-xs text-muted-foreground">
                    {t("workbuddy.status.exists")}
                  </dt>
                  <dd>
                    {statusQuery.data.exists
                      ? t("workbuddy.status.existsYes")
                      : t("workbuddy.status.existsNo")}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs text-muted-foreground">
                    {t("workbuddy.status.modelCount")}
                  </dt>
                  <dd>{statusQuery.data.modelCount}</dd>
                </div>
              </div>
            </dl>
          ) : (
            <p className="text-sm text-muted-foreground">
              {t("workbuddy.status.unavailable")}
            </p>
          )}
        </CardContent>
      </Card>

      <Card>
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

          {allowNoApiKey && !apiKey.trim() ? (
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

      <Card>
        <CardHeader className="space-y-3 pb-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <CardTitle className="text-base">
              {t("workbuddy.models.title")}
            </CardTitle>
            {hasFetchedModels ? (
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setSelectedModelIds(new Set(fetchedModels));
                    setModelsError(null);
                  }}
                >
                  <CheckSquare className="h-4 w-4" />
                  {t("workbuddy.models.selectAll")}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setSelectedModelIds(new Set())}
                >
                  <Square className="h-4 w-4" />
                  {t("workbuddy.models.deselectAll")}
                </Button>
              </div>
            ) : null}
          </div>
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
            fetchedModels.length > 0 ? (
              <div
                className="max-h-64 divide-y overflow-y-auto rounded-md border border-border/70"
                aria-label={t("workbuddy.models.fetchedList")}
              >
                {fetchedModels.map((modelId) => {
                  const checked = selectedModelIds.has(modelId);
                  const checkboxId = `workbuddy-model-${modelId}`;
                  return (
                    <label
                      key={modelId}
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
                      <code className="min-w-0 truncate font-mono text-xs">
                        {modelId}
                      </code>
                    </label>
                  );
                })}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">
                {t("workbuddy.models.emptyFetch")}
              </p>
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
        duplicates={pendingDuplicateSave?.duplicates ?? []}
        isOpen={Boolean(pendingDuplicateSave)}
        isSaving={isSaving}
        onCancel={() => setPendingDuplicateSave(null)}
        onConfirm={() => {
          if (!pendingDuplicateSave) return;
          const request = {
            ...pendingDuplicateSave.request,
            duplicatePolicy: "updateAll" as const,
          };
          void saveRequest(request);
        }}
      />
    </div>
  );
}
