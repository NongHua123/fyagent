import { useCallback, useEffect, useRef, useState } from "react";
import {
  providersApi,
  type CodexProviderFeatureIntent,
  type CodexProviderFeaturePatchResult,
  type CodexProviderFeatureState,
} from "@/lib/api";
import type { CodexApiFormat, Provider } from "@/types";

type FeatureError = "analysis" | "patch" | null;

interface DraftOverride {
  apiFormat?: CodexApiFormat;
  tomlText?: string;
  /**
   * A capability toggle has already made the migration decision in this form,
   * but the normal provider save has not persisted its private marker yet.
   */
  imageExtensionConfigured?: true;
}

interface UseCodexProviderFeaturesOptions {
  enabled: boolean;
  isNew: boolean;
  /** Changes whenever a manual TOML or feature-relevant form field changes. */
  analysisKey: string;
  configText: string;
  getDraft: () => Provider;
  onTomlPatched: (tomlText: string) => void;
}

export interface CodexFeatureSavePreparation {
  tomlText: string;
  imageExtensionConfigured?: true;
}

const providerWithOverride = (
  provider: Provider,
  override: DraftOverride = {},
): Provider => {
  const settingsConfig = {
    ...(provider.settingsConfig ?? {}),
    ...(override.tomlText !== undefined ? { config: override.tomlText } : {}),
  };
  const meta = {
    ...(provider.meta ?? {}),
    ...(override.apiFormat !== undefined
      ? { apiFormat: override.apiFormat }
      : {}),
    ...(override.imageExtensionConfigured === true
      ? { imageExtensionConfigured: true }
      : {}),
  };

  return {
    ...provider,
    settingsConfig,
    meta,
  };
};

/**
 * Owns only form-local Codex native capability state. It never writes a
 * provider row or the live Codex configuration: all patches operate on a
 * renderer draft and the normal save path persists the returned TOML later.
 */
export function useCodexProviderFeatures({
  enabled,
  isNew,
  analysisKey,
  configText,
  getDraft,
  onTomlPatched,
}: UseCodexProviderFeaturesOptions) {
  const [state, setState] = useState<CodexProviderFeatureState | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [isPatching, setIsPatching] = useState(false);
  const [error, setError] = useState<FeatureError>(null);
  const [websocketAutoDisabled, setWebsocketAutoDisabled] = useState(false);
  const getDraftRef = useRef(getDraft);
  const isNewRef = useRef(isNew);
  const configTextRef = useRef(configText);
  const onTomlPatchedRef = useRef(onTomlPatched);
  const mountedRef = useRef(true);
  const analysisSequenceRef = useRef(0);
  const pendingMarkerRef = useRef(false);
  const patchQueueRef = useRef<Promise<void>>(Promise.resolve());

  getDraftRef.current = getDraft;
  isNewRef.current = isNew;
  // Keep save preparation synchronous with the latest rendered draft. Using an
  // effect here leaves a short click-save window after a manual TOML edit.
  configTextRef.current = configText;
  onTomlPatchedRef.current = onTomlPatched;

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
      analysisSequenceRef.current += 1;
    };
  }, []);

  const buildDraft = useCallback((override?: DraftOverride): Provider => {
    const draft = getDraftRef.current();
    return providerWithOverride(draft, {
      tomlText: override?.tomlText ?? configTextRef.current,
      apiFormat: override?.apiFormat,
      // Keep an image-toggle migration decision visible to later analyses in
      // this form. The marker is still persisted only by the normal submit.
      imageExtensionConfigured: pendingMarkerRef.current
        ? true
        : override?.imageExtensionConfigured,
    });
  }, []);

  const analyze = useCallback(
    async (
      draft: Provider,
      options: { commit?: boolean; isNew?: boolean } = {},
    ): Promise<CodexProviderFeatureState> => {
      const sequence = analysisSequenceRef.current + 1;
      analysisSequenceRef.current = sequence;
      const commit = options.commit ?? true;
      if (commit && mountedRef.current) {
        setIsAnalyzing(true);
      }

      try {
        const next = await providersApi.analyzeCodexProviderFeatures(
          draft,
          options.isNew ?? isNewRef.current,
        );
        if (
          commit &&
          mountedRef.current &&
          sequence === analysisSequenceRef.current
        ) {
          setState(next);
          setError(null);
        }
        return next;
      } catch {
        if (
          commit &&
          mountedRef.current &&
          sequence === analysisSequenceRef.current
        ) {
          setError("analysis");
        }
        throw new Error("CODEX_FEATURE_ANALYSIS_FAILED");
      } finally {
        if (
          commit &&
          mountedRef.current &&
          sequence === analysisSequenceRef.current
        ) {
          setIsAnalyzing(false);
        }
      }
    },
    [],
  );

  useEffect(() => {
    analysisSequenceRef.current += 1;
    if (!enabled) {
      setState(null);
      setError(null);
      setIsAnalyzing(false);
      return;
    }

    const timeout = window.setTimeout(() => {
      const draft = buildDraft();
      void analyze(draft).catch(() => undefined);
    }, 250);

    return () => {
      window.clearTimeout(timeout);
      analysisSequenceRef.current += 1;
    };
  }, [analysisKey, analyze, buildDraft, enabled]);

  const enqueuePatch = useCallback(
    <T>(operation: () => Promise<T>): Promise<T> => {
      const next = patchQueueRef.current.then(operation, operation);
      patchQueueRef.current = next.then(
        () => undefined,
        () => undefined,
      );
      return next;
    },
    [],
  );

  const applyPatch = useCallback(
    async (
      draft: Provider,
      intent: CodexProviderFeatureIntent,
      options: { showBusy?: boolean; autoDisabled?: boolean } = {},
    ): Promise<CodexProviderFeaturePatchResult> => {
      if (options.showBusy !== false && mountedRef.current) {
        setIsPatching(true);
      }
      analysisSequenceRef.current += 1;

      try {
        const result = await providersApi.patchCodexProviderFeatures(
          draft,
          intent,
          isNewRef.current,
        );
        configTextRef.current = result.tomlText;
        if (mountedRef.current) {
          onTomlPatchedRef.current(result.tomlText);
          setState(result.state);
          setError(null);
          if (result.imageExtensionConfigured === true) {
            pendingMarkerRef.current = true;
          }
          if (options.autoDisabled) {
            setWebsocketAutoDisabled(true);
          }
        }
        return result;
      } catch {
        if (mountedRef.current) {
          setError("patch");
        }
        throw new Error("CODEX_FEATURE_PATCH_FAILED");
      } finally {
        if (options.showBusy !== false && mountedRef.current) {
          setIsPatching(false);
        }
      }
    },
    [],
  );

  const patchFeatures = useCallback(
    (intent: CodexProviderFeatureIntent): Promise<void> =>
      enqueuePatch(async () => {
        if (!enabled) return;
        await applyPatch(buildDraft(), intent);
      }),
    [applyPatch, buildDraft, enabled, enqueuePatch],
  );

  /**
   * Responses is the only compatible WebSocket transport. Switching away must
   * actively remove an existing TOML field rather than merely disabling UI.
   */
  const handleApiFormatChange = useCallback(
    (nextApiFormat: CodexApiFormat, nextTomlText: string): Promise<void> =>
      enqueuePatch(async () => {
        configTextRef.current = nextTomlText;
        if (!enabled || nextApiFormat === "openai_responses") return;

        const draft = buildDraft({
          apiFormat: nextApiFormat,
          tomlText: nextTomlText,
        });
        const featureState = await analyze(draft, {
          commit: false,
          isNew: isNewRef.current,
        });
        if (featureState.applicable && featureState.websockets.enabled) {
          await applyPatch(
            draft,
            { websockets: false },
            { autoDisabled: true },
          );
        } else if (mountedRef.current) {
          setState(featureState);
        }
      }),
    [analyze, applyPatch, buildDraft, enabled, enqueuePatch],
  );

  /**
   * Complete queued UI patches before saving, then enforce the same invariant
   * the backend will enforce. The format-change handler above owns automatic
   * removal; a user who manually writes an incompatible WebSocket field after
   * that must see a blocked save rather than have their TOML silently changed.
   */
  const prepareForSave = useCallback(
    (): Promise<CodexFeatureSavePreparation> =>
      enqueuePatch(async () => {
        if (!enabled) {
          return { tomlText: configTextRef.current };
        }

        const draft = buildDraft();
        const featureState = await analyze(draft, {
          commit: false,
          isNew: isNewRef.current,
        });
        if (
          featureState.applicable &&
          !featureState.websockets.compatible &&
          featureState.websockets.enabled
        ) {
          throw new Error("CODEX_FEATURE_INCOMPATIBLE_WEBSOCKET");
        }

        // A form-local migration marker is meaningful only for a provider
        // that the backend can currently analyze. Do not smuggle it into a
        // newly official/managed/incomplete row through the normal save path.
        // Keep it local so correcting the same draft before saving does not
        // discard the user's explicit image-toggle decision.
        if (!featureState.applicable) {
          return { tomlText: configTextRef.current };
        }

        const unresolvedImageState =
          featureState.imageExtension.kind === "conflict" ||
          featureState.imageExtension.kind === "invalid";
        if (unresolvedImageState) {
          pendingMarkerRef.current = false;
        }

        return {
          tomlText: configTextRef.current,
          ...(pendingMarkerRef.current && !unresolvedImageState
            ? { imageExtensionConfigured: true as const }
            : {}),
        };
      }),
    [analyze, applyPatch, buildDraft, enabled, enqueuePatch],
  );

  return {
    state,
    isAnalyzing,
    isPatching,
    error,
    websocketAutoDisabled,
    patchFeatures,
    handleApiFormatChange,
    prepareForSave,
  };
}
