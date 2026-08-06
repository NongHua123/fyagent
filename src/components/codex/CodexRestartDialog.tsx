import { Loader2, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { CodexDesktopRestartPromptReason } from "@/types/codexDesktop";
import type { CodexRestartDialogState } from "@/hooks/useCodexRestartCoordinator";

interface CodexRestartDialogProps {
  dialog: CodexRestartDialogState;
  onConfirm: () => void;
  onRetry: () => void;
  onDefer: () => void;
}

function confirmationReasonText(
  reason: CodexDesktopRestartPromptReason,
  t: (key: string, options?: { defaultValue?: string }) => string,
): string {
  switch (reason) {
    case "unique_runtime":
      return t("codexRestart.confirm.uniqueRuntime", {
        defaultValue: "Codex 正在运行。",
      });
    case "multiple_instances":
      return t("codexRestart.confirm.multipleInstances", {
        defaultValue: "检测到多个正在运行的 Codex。",
      });
    case "multiple_installations":
      return t("codexRestart.confirm.multipleInstallations", {
        defaultValue: "检测到多个可用的 Codex 安装。",
      });
    case "identity_binding_ambiguous":
      return t("codexRestart.confirm.identityAmbiguous", {
        defaultValue: "当前无法确认唯一的 Codex 实例。",
      });
  }
}

/**
 * Renders only the renderer-safe restart choices. The backend retains all
 * runtime identity and force-close details; this dialog receives opaque tokens
 * through its state but never displays them.
 */
export function CodexRestartDialog({
  dialog,
  onConfirm,
  onRetry,
  onDefer,
}: CodexRestartDialogProps) {
  const { t } = useTranslation();
  const isProgress = dialog?.kind === "progress";
  const confirmationReason =
    dialog?.kind === "confirm"
      ? confirmationReasonText(dialog.reason, t)
      : null;

  return (
    <Dialog
      open={dialog !== null}
      onOpenChange={(open) => {
        if (!open && dialog !== null && !isProgress) {
          onDefer();
        }
      }}
    >
      <DialogContent
        className="max-w-sm"
        zIndex="top"
        onEscapeKeyDown={(event) => {
          if (isProgress) event.preventDefault();
        }}
      >
        {dialog?.kind === "confirm" && (
          <>
            <DialogHeader className="space-y-3 border-b-0 bg-transparent pb-0">
              <DialogTitle className="flex items-center gap-2 text-lg font-semibold">
                <TriangleAlert className="h-5 w-5 text-amber-500" />
                {t("codexRestart.confirm.title", {
                  defaultValue: "需要重启 Codex",
                })}
              </DialogTitle>
              <DialogDescription className="space-y-2 text-sm leading-relaxed">
                <span className="block">
                  {t("codexRestart.confirm.saved", {
                    defaultValue: "配置已保存。",
                  })}
                </span>
                <span className="block">{confirmationReason}</span>
                <span className="block">
                  {t("codexRestart.confirm.risk", {
                    defaultValue:
                      "继续后将强制关闭所有匹配的 Codex。未保存的工作可能丢失，随后只会启动一个实例。",
                  })}
                </span>
              </DialogDescription>
            </DialogHeader>
            <DialogFooter className="flex gap-2 border-t-0 bg-transparent pt-2 sm:justify-end">
              <Button
                autoFocus
                type="button"
                variant="outline"
                onClick={onDefer}
              >
                {t("codexRestart.confirm.manual", {
                  defaultValue: "稍后手动重启",
                })}
              </Button>
              <Button type="button" variant="destructive" onClick={onConfirm}>
                {t("codexRestart.confirm.force", {
                  defaultValue: "强制关闭并重启",
                })}
              </Button>
            </DialogFooter>
          </>
        )}

        {isProgress && (
          <DialogHeader className="space-y-3 border-b-0 bg-transparent pb-0">
            <DialogTitle className="flex items-center gap-2 text-lg font-semibold">
              <Loader2
                aria-hidden="true"
                className="h-5 w-5 animate-spin text-blue-500"
              />
              {t("codexRestart.progress", {
                defaultValue: "正在重启 Codex…",
              })}
            </DialogTitle>
            <DialogDescription className="sr-only">
              {t("codexRestart.progressDescription", {
                defaultValue: "请稍候。",
              })}
            </DialogDescription>
          </DialogHeader>
        )}

        {dialog?.kind === "incomplete" && (
          <>
            <DialogHeader className="space-y-3 border-b-0 bg-transparent pb-0">
              <DialogTitle className="flex items-center gap-2 text-lg font-semibold">
                <TriangleAlert className="h-5 w-5 text-amber-500" />
                {t("codexRestart.incomplete.title", {
                  defaultValue: "Codex 重启未完成",
                })}
              </DialogTitle>
              <DialogDescription className="text-sm leading-relaxed">
                {t("codexRestart.incomplete.body", {
                  defaultValue: "配置已保存，但 Codex 重启未完成。",
                })}
              </DialogDescription>
            </DialogHeader>
            <DialogFooter className="flex gap-2 border-t-0 bg-transparent pt-2 sm:justify-end">
              <Button
                autoFocus
                type="button"
                variant="outline"
                onClick={onDefer}
              >
                {t("codexRestart.incomplete.manual", {
                  defaultValue: "我将手动重启",
                })}
              </Button>
              <Button
                type="button"
                disabled={!dialog.retryToken}
                onClick={onRetry}
              >
                {t("codexRestart.incomplete.retry", {
                  defaultValue: "再次尝试重启",
                })}
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
