import { AlertCircle, Image, Radio } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { FormLabel } from "@/components/ui/form";
import { Switch } from "@/components/ui/switch";
import type { CodexProviderFeatureState } from "@/lib/api";

interface CodexNativeCapabilitiesProps {
  state: CodexProviderFeatureState | null;
  isAnalyzing: boolean;
  isPatching: boolean;
  error: "analysis" | "patch" | null;
  onImageExtensionChange: (enabled: boolean) => void;
  onWebsocketsChange: (enabled: boolean) => void;
}

const diagnosticKey = (code: string): string => {
  switch (code) {
    case "CODEX_FEATURE_INVALID_TOML":
      return "codexFeatures.diagnostic.invalidToml";
    case "CODEX_FEATURE_INVALID_HEADER":
      return "codexFeatures.diagnostic.invalidHeaders";
    case "CODEX_FEATURE_INVALID_WEBSOCKET":
      return "codexFeatures.diagnostic.invalidWebsockets";
    default:
      return "codexFeatures.diagnostic.generic";
  }
};

/**
 * UI for the two provider-scoped Codex capabilities. The renderer receives
 * only non-sensitive state; it never parses/re-writes a TOML document itself.
 */
export function CodexNativeCapabilities({
  state,
  isAnalyzing,
  isPatching,
  error,
  onImageExtensionChange,
  onWebsocketsChange,
}: CodexNativeCapabilitiesProps) {
  const { t } = useTranslation();

  const diagnostics = state?.diagnostics ?? [];
  const imageState = state?.imageExtension;
  const hasImageConflict = imageState?.kind === "conflict";
  const hasImageInvalid =
    imageState?.kind === "invalid" &&
    imageState.code === "CODEX_FEATURE_INVALID_HEADER";
  const imageChecked =
    imageState?.kind === "on" || imageState?.kind === "legacyPendingOn";
  const hasInvalidToml = diagnostics.some(
    (diagnostic) => diagnostic.code === "CODEX_FEATURE_INVALID_TOML",
  );
  const controlsDisabled =
    isPatching ||
    isAnalyzing ||
    !state ||
    hasInvalidToml ||
    error === "analysis";

  return (
    <section className="space-y-3 border-t border-border-default pt-3">
      <div className="space-y-1">
        <FormLabel>
          {t("codexFeatures.title", { defaultValue: "Codex 原生能力" })}
        </FormLabel>
        <p className="text-xs leading-relaxed text-muted-foreground">
          {t("codexFeatures.description", {
            defaultValue:
              "这些选项可用于所有 Codex 供应商，只修改当前 TOML 草稿；保存后才会写入配置。",
          })}
        </p>
      </div>

      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0 space-y-1">
          <FormLabel className="flex items-center gap-2">
            <Image className="h-4 w-4" />
            {t("codexFeatures.imageExtension.label", {
              defaultValue: "启用内置生图扩展",
            })}
          </FormLabel>
          <p className="text-xs leading-relaxed text-muted-foreground">
            {t("codexFeatures.imageExtension.description", {
              defaultValue:
                "向当前供应商写入受管请求头，以启用 Codex 的内置生图扩展。",
            })}
          </p>
        </div>
        <Switch
          checked={imageChecked}
          disabled={controlsDisabled}
          onCheckedChange={onImageExtensionChange}
          aria-label={t("codexFeatures.imageExtension.label", {
            defaultValue: "启用内置生图扩展",
          })}
        />
      </div>

      {imageState?.kind === "legacyPendingOn" ? (
        <Alert>
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>
            {t("codexFeatures.imageExtension.legacyTitle", {
              defaultValue: "保存时迁移",
            })}
          </AlertTitle>
          <AlertDescription>
            {t("codexFeatures.imageExtension.legacyDescription", {
              defaultValue:
                "这是历史供应商的默认开启迁移。取消不会修改配置；保存后才会写入受管请求头。",
            })}
          </AlertDescription>
        </Alert>
      ) : null}

      {hasImageConflict ? (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>
            {t("codexFeatures.imageExtension.conflictTitle", {
              defaultValue: "生图扩展请求头冲突",
            })}
          </AlertTitle>
          <AlertDescription>
            {t("codexFeatures.imageExtension.conflictDescription", {
              defaultValue:
                "当前 TOML 含有同名或大小写不同的请求头；操作开关会覆盖这些同名值。",
              key: imageState.key,
            })}
          </AlertDescription>
        </Alert>
      ) : null}

      {hasImageInvalid ? (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>
            {t("codexFeatures.imageExtension.invalidTitle", {
              defaultValue: "无法读取生图扩展设置",
            })}
          </AlertTitle>
          <AlertDescription>
            {t("codexFeatures.imageExtension.invalidDescription", {
              defaultValue:
                "请求头不是字符串映射；操作开关会用受管请求头替换或删除该字段。",
            })}
          </AlertDescription>
        </Alert>
      ) : null}

      <div className="flex items-start justify-between gap-4 border-t border-border-default pt-3">
        <div className="min-w-0 space-y-1">
          <FormLabel className="flex items-center gap-2">
            <Radio className="h-4 w-4" />
            {t("codexFeatures.websockets.label", {
              defaultValue: "启用 WebSocket 传输",
            })}
          </FormLabel>
          <p className="text-xs leading-relaxed text-muted-foreground">
            {t("codexFeatures.websockets.description", {
              defaultValue:
                "写入 supports_websockets = true；上游模型或代理链路可能不支持，保存后会提示已识别的风险。",
            })}
          </p>
        </div>
        <Switch
          checked={state?.websockets.enabled ?? false}
          disabled={controlsDisabled}
          onCheckedChange={onWebsocketsChange}
          aria-label={t("codexFeatures.websockets.label", {
            defaultValue: "启用 WebSocket 传输",
          })}
        />
      </div>

      {diagnostics.map((diagnostic) => (
        <Alert
          key={`${diagnostic.code}-${diagnostic.field ?? ""}`}
          variant="destructive"
        >
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>
            {t(diagnosticKey(diagnostic.code), {
              defaultValue:
                "Codex 原生能力配置无效；FyAgent 不会自动重写该 TOML 草稿。",
            })}
          </AlertDescription>
        </Alert>
      ))}

      {error ? (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>
            {t(
              error === "patch"
                ? "codexFeatures.patchFailed"
                : "codexFeatures.analysisFailed",
              {
                defaultValue:
                  "无法更新 Codex 原生能力草稿，请检查 TOML 后重试。",
              },
            )}
          </AlertDescription>
        </Alert>
      ) : null}
    </section>
  );
}
