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
  codexNativeCapabilitiesGeneratedProvider?: boolean;
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
  codexNativeCapabilitiesGeneratedProvider?: boolean;
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
    ...(override.codexNativeCapabilitiesGeneratedProvider !== undefined
      ? {
          codexNativeCapabilitiesGeneratedProvider:
            override.codexNativeCapabilitiesGeneratedProvider,
        }
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
  const getDraftRef = useRef(getDraft);
  const isNewRef = useRef(isNew);
  const configTextRef = useRef(configText);
  const onTomlPatchedRef = useRef(onTomlPatched);
  const mountedRef = useRef(true);
  const analysisSequenceRef = useRef(0);
  const pendingMarkerRef = useRef(false);
  const pendingGeneratedProviderRef = useRef<boolean | undefined>(undefined);
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
      codexNativeCapabilitiesGeneratedProvider:
        pendingGeneratedProviderRef.current ??
        override?.codexNativeCapabilitiesGeneratedProvider,
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
      options: { showBusy?: boolean } = {},
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
          if (result.codexNativeCapabilitiesGeneratedProvider !== undefined) {
            pendingGeneratedProviderRef.current =
              result.codexNativeCapabilitiesGeneratedProvider;
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

  /** Keep queued feature analysis aligned with an API-format form change.
   * Format selection no longer rewrites or disables WebSocket capability.
   */
  const handleApiFormatChange = useCallback(
    (nextApiFormat: CodexApiFormat, nextTomlText: string): Promise<void> =>
      enqueuePatch(async () => {
        configTextRef.current = nextTomlText;
        if (!enabled) return;

        const draft = buildDraft({
          apiFormat: nextApiFormat,
          tomlText: nextTomlText,
        });
        const featureState = await analyze(draft, {
          commit: false,
          isNew: isNewRef.current,
        });
        if (mountedRef.current) {
          setState(featureState);
        }
      }),
    [analyze, buildDraft, enabled, enqueuePatch],
  );

  /** Complete queued UI patches before saving and carry private, form-local
   * marker decisions into the normal provider mutation.
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
        // A form-local migration marker is meaningful only while the backend
        // can parse the whole TOML document. Keep it out of a malformed draft
        // until the user has corrected the document.
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
          ...(pendingGeneratedProviderRef.current !== undefined
            ? {
                codexNativeCapabilitiesGeneratedProvider:
                  pendingGeneratedProviderRef.current,
              }
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
    patchFeatures,
    handleApiFormatChange,
    prepareForSave,
  };
}
