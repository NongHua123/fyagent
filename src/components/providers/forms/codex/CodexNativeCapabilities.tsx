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
  websocketAutoDisabled: boolean;
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
  websocketAutoDisabled,
  onImageExtensionChange,
  onWebsocketsChange,
}: CodexNativeCapabilitiesProps) {
  const { t } = useTranslation();

  const diagnostics = state?.diagnostics ?? [];
  const imageState = state?.imageExtension;
  const hasImageConflict = imageState?.kind === "conflict";
  const hasImageInvalid = imageState?.kind === "invalid";
  const imageChecked =
    imageState?.kind === "on" || imageState?.kind === "legacyPendingOn";

  if (!state?.applicable && diagnostics.length === 0 && !error) {
    return isAnalyzing ? (
      <p className="text-xs text-muted-foreground">
        {t("codexFeatures.analyzing", {
          defaultValue: "正在检查 Codex 原生能力…",
        })}
      </p>
    ) : null;
  }

  return (
    <section className="space-y-3 border-t border-border-default pt-3">
      {state?.applicable ? (
        <>
          <div className="space-y-1">
            <FormLabel>
              {t("codexFeatures.title", { defaultValue: "Codex 原生能力" })}
            </FormLabel>
            <p className="text-xs leading-relaxed text-muted-foreground">
              {t("codexFeatures.description", {
                defaultValue:
                  "这些选项只修改当前供应商的 TOML 草稿；保存后才会写入 Codex 配置。",
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
                    "向当前第三方 API 供应商写入受管请求头，以启用 Codex 的内置生图扩展。",
                })}
              </p>
            </div>
            <Switch
              checked={imageChecked}
              disabled={isPatching || hasImageConflict || hasImageInvalid}
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
                    "当前 TOML 含有同名但非受管的请求头。FyAgent 不会覆盖、删除或新建大小写变体，请先手工处理。",
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
                    "请求头结构不是 Codex 所需的字符串映射。请在 TOML 中修复后再修改此开关。",
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
                    "仅 OpenAI Responses 格式可用。关闭时会删除 supports_websockets，而不是写入 false。",
                })}
              </p>
            </div>
            <Switch
              checked={state.websockets.enabled}
              disabled={isPatching || !state.websockets.compatible}
              onCheckedChange={onWebsocketsChange}
              aria-label={t("codexFeatures.websockets.label", {
                defaultValue: "启用 WebSocket 传输",
              })}
            />
          </div>

          {!state.websockets.compatible ? (
            <p className="text-xs leading-relaxed text-muted-foreground">
              {t("codexFeatures.websockets.incompatible", {
                defaultValue:
                  "当前上游格式不是 OpenAI Responses，因此 WebSocket 传输不可用。",
              })}
            </p>
          ) : null}

          {websocketAutoDisabled ? (
            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                {t("codexFeatures.websockets.autoDisabled", {
                  defaultValue:
                    "已因上游格式改为非 Responses 自动关闭 WebSocket，并从当前 TOML 草稿移除该字段。",
                })}
              </AlertDescription>
            </Alert>
          ) : null}
        </>
      ) : null}

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
