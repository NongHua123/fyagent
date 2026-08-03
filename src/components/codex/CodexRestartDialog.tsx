import { Loader2, RotateCw, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { CodexRestartDialogState } from "@/hooks/useCodexRestartCoordinator";

interface CodexRestartDialogProps {
  dialog: CodexRestartDialogState;
  isRestarting: boolean;
  onRestart: () => void;
  onConfirmForceRestart: () => void;
  onDefer: () => void;
}

/**
 * Presents only the renderer-safe restart choices. The process identity and
 * force token remain opaque and entirely backend-controlled.
 */
export function CodexRestartDialog({
  dialog,
  isRestarting,
  onRestart,
  onConfirmForceRestart,
  onDefer,
}: CodexRestartDialogProps) {
  const { t } = useTranslation();
  const isForceConfirmation = dialog?.kind === "force";

  return (
    <Dialog
      open={dialog !== null}
      onOpenChange={(open) => {
        if (!open && dialog !== null && !isRestarting) {
          onDefer();
        }
      }}
    >
      <DialogContent className="max-w-sm" zIndex="top">
        <DialogHeader className="space-y-3 border-b-0 bg-transparent pb-0">
          <DialogTitle className="flex items-center gap-2 text-lg font-semibold">
            {isForceConfirmation ? (
              <TriangleAlert className="h-5 w-5 text-amber-500" />
            ) : (
              <RotateCw className="h-5 w-5 text-blue-500" />
            )}
            {isForceConfirmation
              ? t("codexRestart.forceTitle", {
                  defaultValue: "Codex 未能正常退出",
                })
              : t("codexRestart.title", { defaultValue: "配置已更新" })}
          </DialogTitle>
          <DialogDescription className="whitespace-pre-line text-sm leading-relaxed">
            {isForceConfirmation
              ? t("codexRestart.forceDescription", {
                  defaultValue:
                    "配置已保存，但 Codex 没有在等待时间内正常退出。确认后才会强制关闭已验证的实例并重新启动；你也可以稍后手动重启。",
                })
              : t("codexRestart.description", {
                  defaultValue:
                    "当前 Codex 正在运行。重启后会加载刚刚保存的新配置。",
                })}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter className="flex gap-2 border-t-0 bg-transparent pt-2 sm:justify-end">
          <Button
            type="button"
            variant="outline"
            disabled={isRestarting}
            onClick={onDefer}
          >
            {t("codexRestart.laterAction", {
              defaultValue: "稍后手动重启",
            })}
          </Button>
          <Button
            type="button"
            variant={isForceConfirmation ? "destructive" : "default"}
            disabled={isRestarting}
            onClick={isForceConfirmation ? onConfirmForceRestart : onRestart}
          >
            {isRestarting ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : null}
            {isForceConfirmation
              ? t("codexRestart.forceAction", {
                  defaultValue: "强制退出并重启",
                })
              : t("codexRestart.restartAction", {
                  defaultValue: "重启 Codex",
                })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
