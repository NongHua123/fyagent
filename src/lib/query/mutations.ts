import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { providersApi, sessionsApi, settingsApi, type AppId } from "@/lib/api";
import type { DeleteSessionOptions } from "@/lib/api/sessions";
import type {
  CodexProviderMutationWarning,
  SwitchResult,
} from "@/lib/api/providers";
import type { Provider, SessionMeta, Settings } from "@/types";
import { extractErrorMessage } from "@/utils/errorUtils";
import { generateUUID } from "@/utils/uuid";
import { openclawKeys } from "@/hooks/useOpenClaw";
import { invalidateHermesProviderCaches } from "@/hooks/useHermes";
import { proxyKeys } from "@/lib/query/proxy";
import { usageKeys } from "@/lib/query/usage";
import {
  CODEX_OFFICIAL_PROVIDER_ID,
  GROKBUILD_OFFICIAL_PROVIDER_ID,
} from "@/utils/providerCapabilities";

/**
 * Renderer-facing summary for a completed provider write. `liveConfigChanged`
 * is intentionally supplied only by the Codex-aware backend mutation IPCs;
 * it is never inferred from the selected provider or a button label.
 */
export interface ProviderMutationOutcome<T> {
  value: T;
  liveConfigChanged: boolean;
  warningCodes?: CodexProviderMutationWarning[];
}

const codexMutationWarningMessage = (
  t: ReturnType<typeof useTranslation>["t"],
  operation: "added" | "saved",
  warnings: CodexProviderMutationWarning[] | undefined,
): string | null => {
  if (!warnings?.length) return null;

  const warningSet = new Set(warnings);
  const risks = [
    ...(warningSet.has("CODEX_WEBSOCKET_NON_GPT_MODEL")
      ? [
          t("codexFeatures.saveWarnings.nonGptModel", {
            defaultValue: "WebSocket 传输仅支持 GPT 系列模型",
          }),
        ]
      : []),
    ...(warningSet.has("CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED")
      ? [
          t("codexFeatures.saveWarnings.proxyMayBeUnsupported", {
            defaultValue: "当前代理接管链路可能不支持 WebSocket",
          }),
        ]
      : []),
  ];
  if (!risks.length) return null;

  const status = t(`codexFeatures.saveWarnings.${operation}`, {
    defaultValue: operation === "added" ? "供应商已添加" : "供应商已保存",
  });
  const separator = t("codexFeatures.saveWarnings.separator", {
    defaultValue: "；",
  });
  return `${status}${separator}${risks.join(separator)}`;
};

export const useAddProviderMutation = (appId: AppId) => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (
      providerInput: Omit<Provider, "id"> & {
        providerKey?: string;
        addToLive?: boolean;
        ensureClaudeDesktopOfficialSeed?: boolean;
        ensureCodexOfficialSeed?: boolean;
        ensureGrokBuildOfficialSeed?: boolean;
      },
    ) => {
      const {
        providerKey: _providerKey,
        addToLive,
        ensureClaudeDesktopOfficialSeed,
        ensureCodexOfficialSeed,
        ensureGrokBuildOfficialSeed,
        ...rest
      } = providerInput;

      if (appId === "claude-desktop" && ensureClaudeDesktopOfficialSeed) {
        await providersApi.ensureClaudeDesktopOfficialProvider();
        const providers = await providersApi.getAll(appId);
        const officialProvider = providers["claude-desktop-official"];
        if (!officialProvider) {
          throw new Error("Claude Desktop official provider was not created");
        }
        return { value: officialProvider, liveConfigChanged: false };
      }

      if (appId === "codex" && ensureCodexOfficialSeed) {
        await providersApi.ensureCodexOfficialProvider();
        const providers = await providersApi.getAll(appId);
        const officialProvider = providers[CODEX_OFFICIAL_PROVIDER_ID];
        if (!officialProvider) {
          throw new Error("Codex official provider was not created");
        }
        // This seed operation only ensures a database row. It does not use a
        // live-config mutation command, so it cannot request a desktop restart.
        return { value: officialProvider, liveConfigChanged: false };
      }

      if (appId === "grokbuild" && ensureGrokBuildOfficialSeed) {
        await providersApi.ensureGrokBuildOfficialProvider();
        const providers = await providersApi.getAll(appId);
        const officialProvider = providers[GROKBUILD_OFFICIAL_PROVIDER_ID];
        if (!officialProvider) {
          throw new Error("Grok Build official provider was not created");
        }
        return { value: officialProvider, liveConfigChanged: false };
      }

      let id: string;

      if (appId === "opencode" || appId === "openclaw" || appId === "hermes") {
        if (
          providerInput.category === "omo" ||
          providerInput.category === "omo-slim"
        ) {
          const prefix = providerInput.category === "omo" ? "omo" : "omo-slim";
          id = `${prefix}-${generateUUID()}`;
        } else {
          if (!providerInput.providerKey) {
            throw new Error(`Provider key is required for ${appId}`);
          }
          id = providerInput.providerKey;
        }
      } else {
        id = generateUUID();
      }

      const newProvider: Provider = {
        ...rest,
        id,
        createdAt: Date.now(),
      };
      delete (newProvider as any).providerKey;

      if (appId === "codex") {
        const result = await providersApi.addWithResult(
          newProvider,
          appId,
          addToLive,
        );
        return {
          value: newProvider,
          liveConfigChanged: result.liveConfigChanged,
          ...(result.warningCodes?.length
            ? { warningCodes: result.warningCodes }
            : {}),
        };
      }

      await providersApi.add(newProvider, appId, addToLive);
      return { value: newProvider, liveConfigChanged: false };
    },
    onSuccess: async (outcome) => {
      await queryClient.invalidateQueries({ queryKey: ["providers", appId] });

      if (appId === "opencode") {
        await queryClient.invalidateQueries({
          queryKey: ["omo", "current-provider-id"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["omo", "provider-count"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["omo-slim", "current-provider-id"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["omo-slim", "provider-count"],
        });
      }

      if (appId === "openclaw") {
        await queryClient.invalidateQueries({
          queryKey: openclawKeys.health,
        });
      }

      if (appId === "hermes") {
        await invalidateHermesProviderCaches(queryClient);
      }

      try {
        await providersApi.updateTrayMenu();
      } catch (trayError) {
        console.error(
          "Failed to update tray menu after adding provider",
          trayError,
        );
      }

      const warningMessage = codexMutationWarningMessage(
        t,
        "added",
        outcome.warningCodes,
      );
      if (warningMessage) {
        toast.warning(warningMessage, { closeButton: true });
      } else {
        toast.success(
          t("notifications.providerAdded", {
            defaultValue: "供应商已添加",
          }),
          {
            closeButton: true,
          },
        );
      }
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(
        t("notifications.addFailed", {
          defaultValue: "添加供应商失败: {{error}}",
          error: detail,
        }),
      );
    },
  });
};

export const useUpdateProviderMutation = (appId: AppId) => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async ({
      provider,
      originalId,
    }: {
      provider: Provider;
      originalId?: string;
    }) => {
      if (appId === "codex") {
        const result = await providersApi.updateWithResult(
          provider,
          appId,
          originalId,
        );
        return {
          value: provider,
          liveConfigChanged: result.liveConfigChanged,
          ...(result.warningCodes?.length
            ? { warningCodes: result.warningCodes }
            : {}),
        };
      }

      await providersApi.update(provider, appId, originalId);
      return { value: provider, liveConfigChanged: false };
    },
    onSuccess: async (outcome, variables) => {
      const provider = outcome.value;
      await queryClient.invalidateQueries({ queryKey: ["providers", appId] });
      await queryClient.invalidateQueries({
        queryKey: usageKeys.script(provider.id, appId),
      });
      if (variables.originalId && variables.originalId !== provider.id) {
        await queryClient.invalidateQueries({
          queryKey: usageKeys.script(variables.originalId, appId),
        });
      }
      if (appId === "openclaw") {
        await queryClient.invalidateQueries({
          queryKey: openclawKeys.health,
        });
      }
      if (appId === "hermes") {
        await invalidateHermesProviderCaches(queryClient);
      }
      const warningMessage = codexMutationWarningMessage(
        t,
        "saved",
        outcome.warningCodes,
      );
      if (warningMessage) {
        toast.warning(warningMessage, { closeButton: true });
      } else {
        toast.success(
          t("notifications.updateSuccess", {
            defaultValue: "供应商更新成功",
          }),
          {
            closeButton: true,
          },
        );
      }
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(
        t("notifications.updateFailed", {
          defaultValue: "更新供应商失败: {{error}}",
          error: detail,
        }),
      );
    },
  });
};

export const useDeleteProviderMutation = (appId: AppId) => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (
      providerId: string,
    ): Promise<ProviderMutationOutcome<void>> => {
      if (appId === "codex") {
        const result = await providersApi.deleteWithResult(providerId, appId);
        return {
          value: undefined,
          liveConfigChanged: result.liveConfigChanged,
        };
      }

      await providersApi.delete(providerId, appId);
      return { value: undefined, liveConfigChanged: false };
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["providers", appId] });

      if (appId === "opencode") {
        await queryClient.invalidateQueries({
          queryKey: ["omo", "current-provider-id"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["omo", "provider-count"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["omo-slim", "current-provider-id"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["omo-slim", "provider-count"],
        });
      }

      if (appId === "openclaw") {
        await queryClient.invalidateQueries({
          queryKey: openclawKeys.health,
        });
      }

      if (appId === "hermes") {
        await invalidateHermesProviderCaches(queryClient);
      }

      try {
        await providersApi.updateTrayMenu();
      } catch (trayError) {
        console.error(
          "Failed to update tray menu after deleting provider",
          trayError,
        );
      }

      toast.success(
        t("notifications.deleteSuccess", {
          defaultValue: "供应商已删除",
        }),
        {
          closeButton: true,
        },
      );
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(
        t("notifications.deleteFailed", {
          defaultValue: "删除供应商失败: {{error}}",
          error: detail,
        }),
      );
    },
  });
};

export const useSwitchProviderMutation = (appId: AppId) => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (
      providerId: string,
    ): Promise<ProviderMutationOutcome<SwitchResult>> => {
      if (appId === "codex") {
        const result = await providersApi.switchWithResult(providerId, appId);
        return {
          value: result.value,
          liveConfigChanged: result.liveConfigChanged,
        };
      }

      return {
        value: await providersApi.switch(providerId, appId),
        liveConfigChanged: false,
      };
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["providers", appId] });
      if (appId === "claude-desktop") {
        await queryClient.invalidateQueries({ queryKey: proxyKeys.status });
        await queryClient.invalidateQueries({
          queryKey: ["claudeDesktopStatus"],
        });
      }

      // OpenCode/OpenClaw: also invalidate live provider IDs cache to update button state
      if (appId === "opencode") {
        await queryClient.invalidateQueries({
          queryKey: ["opencodeLiveProviderIds"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["opencode", "runtime-models"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["omo", "current-provider-id"],
        });
        await queryClient.invalidateQueries({
          queryKey: ["omo-slim", "current-provider-id"],
        });
      }
      if (appId === "openclaw") {
        await queryClient.invalidateQueries({
          queryKey: openclawKeys.liveProviderIds,
        });
        await queryClient.invalidateQueries({
          queryKey: openclawKeys.defaultModel,
        });
        await queryClient.invalidateQueries({
          queryKey: openclawKeys.health,
        });
      }
      if (appId === "hermes") {
        await invalidateHermesProviderCaches(queryClient);
      }

      try {
        await providersApi.updateTrayMenu();
      } catch (trayError) {
        console.error(
          "Failed to update tray menu after switching provider",
          trayError,
        );
      }
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");

      toast.error(
        t("notifications.switchFailedTitle", { defaultValue: "切换失败" }),
        {
          description: t("notifications.switchFailed", {
            defaultValue: "切换失败：{{error}}",
            error: detail,
          }),
          duration: 6000,
          action: {
            label: t("common.copy", { defaultValue: "复制" }),
            onClick: () => {
              navigator.clipboard?.writeText(detail).catch(() => undefined);
            },
          },
        },
      );
    },
  });
};

export const useDeleteSessionMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (input: DeleteSessionOptions) => {
      await sessionsApi.delete(input);
      return input;
    },
    onSuccess: async (input) => {
      queryClient.setQueryData<SessionMeta[]>(["sessions"], (current) =>
        (current ?? []).filter(
          (session) =>
            !(
              session.providerId === input.providerId &&
              session.sessionId === input.sessionId &&
              session.sourcePath === input.sourcePath
            ),
        ),
      );
      queryClient.removeQueries({
        queryKey: ["sessionMessages", input.providerId, input.sourcePath],
      });

      await queryClient.invalidateQueries({ queryKey: ["sessions"] });

      toast.success(
        t("sessionManager.sessionDeleted", {
          defaultValue: "会话已删除",
        }),
      );
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(
        t("sessionManager.deleteFailed", {
          defaultValue: "删除会话失败: {{error}}",
          error: detail,
        }),
      );
    },
  });
};

export const useSaveSettingsMutation = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (settings: Settings) => {
      await settingsApi.save(settings);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["settings"] });
      await queryClient.invalidateQueries({
        queryKey: ["opencode", "runtime-models"],
      });
    },
  });
};
