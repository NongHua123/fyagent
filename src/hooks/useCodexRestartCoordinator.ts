import { useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { codexDesktopApi } from "@/lib/api";
import type {
  CodexDesktopRestartOutcome,
  CodexDesktopRestartPhase,
} from "@/types/codexDesktop";

export type CodexRestartDialogState =
  | { kind: "restart" }
  | { kind: "force"; token: string }
  | null;

const phaseMessageKey: Record<CodexDesktopRestartPhase, string> = {
  detect: "codexRestart.phase.detect",
  quit: "codexRestart.phase.quit",
  force_quit: "codexRestart.phase.forceQuit",
  launch: "codexRestart.phase.launch",
  verify: "codexRestart.phase.verify",
};

/**
 * Coordinates a trusted Codex Desktop restart after a *successful* provider
 * write. It deliberately receives no provider or process identity: the
 * backend's `liveConfigChanged` result and trusted runtime commands are the
 * only authority for deciding whether to offer a restart.
 */
export function useCodexRestartCoordinator() {
  const { t } = useTranslation();
  const [dialog, setDialog] = useState<CodexRestartDialogState>(null);
  const [isRestarting, setIsRestarting] = useState(false);
  const dialogOpenRef = useRef(false);
  const runtimeCheckInFlightRef = useRef(false);

  const closeDialog = useCallback(() => {
    dialogOpenRef.current = false;
    setDialog(null);
  }, []);

  const showManualRestartNotice = useCallback(
    (reason?: "notInstalled" | "ambiguous" | "unsupported" | "unavailable") => {
      const key =
        reason === "notInstalled"
          ? "codexRestart.notInstalled"
          : reason === "ambiguous"
            ? "codexRestart.ambiguous"
            : reason === "unsupported"
              ? "codexRestart.unsupported"
              : "codexRestart.manualRequired";
      const defaultValue =
        reason === "notInstalled"
          ? "配置已保存；未检测到可信 Codex Desktop，无法自动重启。"
          : reason === "ambiguous"
            ? "配置已保存；无法唯一确认正在运行的 Codex Desktop，请手动重启。"
            : reason === "unsupported"
              ? "配置已保存；当前平台不支持自动重启，请手动重启 Codex。"
              : "配置已保存；请手动重启 Codex 以加载新配置。";
      toast.info(t(key, { defaultValue }));
    },
    [t],
  );

  const handleRestartOutcome = useCallback(
    (outcome: CodexDesktopRestartOutcome) => {
      switch (outcome.state) {
        case "restarted":
          closeDialog();
          toast.success(
            t("codexRestart.restarted", {
              defaultValue: "Codex 已重启，新配置已生效。",
            }),
          );
          return;
        case "force_confirmation_required":
          // Keep the backend-issued token opaque. It is only held in local
          // component state and echoed to the continuation IPC if confirmed.
          setDialog({ kind: "force", token: outcome.token });
          return;
        case "unavailable":
          closeDialog();
          showManualRestartNotice(
            outcome.reason === "not_installed"
              ? "notInstalled"
              : outcome.reason === "unsupported"
                ? "unsupported"
                : "ambiguous",
          );
          return;
        case "cancelled":
          closeDialog();
          showManualRestartNotice("unavailable");
          return;
        case "failed":
          closeDialog();
          toast.error(
            t("codexRestart.failed", {
              defaultValue:
                "配置已保存，但 Codex 重启失败。请手动重启后再继续。",
            }),
            {
              description: t(phaseMessageKey[outcome.phase], {
                defaultValue: "自动重启未能完成。",
              }),
            },
          );
          return;
      }
    },
    [closeDialog, showManualRestartNotice, t],
  );

  /**
   * Called only for a true backend `liveConfigChanged` result. Normal provider
   * success notifications are emitted by their own mutations; a not-running
   * desktop therefore adds no restart UI and is never launched automatically.
   */
  const notifyLiveConfigChanged = useCallback(async () => {
    if (dialogOpenRef.current || runtimeCheckInFlightRef.current) return;

    runtimeCheckInFlightRef.current = true;
    try {
      const runtime = await codexDesktopApi.getRuntimeStatus();
      if (runtime.state === "running") {
        dialogOpenRef.current = true;
        setDialog({ kind: "restart" });
        return;
      }
      if (runtime.state === "not_running") {
        return;
      }
      if (runtime.state === "not_installed") {
        showManualRestartNotice("notInstalled");
        return;
      }
      showManualRestartNotice(
        runtime.state === "unsupported" ? "unsupported" : "ambiguous",
      );
    } catch {
      // Runtime detection is deliberately advisory. The saved configuration is
      // still valid, so failure yields a manual-restart notice rather than a
      // failed provider action or an arbitrary process operation.
      showManualRestartNotice("unavailable");
    } finally {
      runtimeCheckInFlightRef.current = false;
    }
  }, [showManualRestartNotice]);

  const requestRestart = useCallback(async () => {
    if (dialog?.kind !== "restart" || isRestarting) return;
    setIsRestarting(true);
    try {
      handleRestartOutcome(await codexDesktopApi.requestRestart());
    } catch {
      closeDialog();
      toast.error(
        t("codexRestart.failed", {
          defaultValue: "配置已保存，但 Codex 重启失败。请手动重启后再继续。",
        }),
      );
    } finally {
      setIsRestarting(false);
    }
  }, [closeDialog, dialog?.kind, handleRestartOutcome, isRestarting, t]);

  const confirmForceRestart = useCallback(async () => {
    if (dialog?.kind !== "force" || isRestarting) return;
    setIsRestarting(true);
    try {
      handleRestartOutcome(
        await codexDesktopApi.continueRestartWithForce(dialog.token),
      );
    } catch {
      closeDialog();
      toast.error(
        t("codexRestart.failed", {
          defaultValue: "配置已保存，但 Codex 重启失败。请手动重启后再继续。",
        }),
      );
    } finally {
      setIsRestarting(false);
    }
  }, [closeDialog, dialog, handleRestartOutcome, isRestarting, t]);

  const deferRestart = useCallback(async () => {
    if (isRestarting) return;
    const forceToken = dialog?.kind === "force" ? dialog.token : null;

    if (forceToken) {
      // Keep the force-confirmation dialog authoritative until its opaque
      // capability has been discarded. Otherwise another live-config change
      // could open a retry dialog while the old token still blocks the backend
      // restart slot.
      setIsRestarting(true);
      try {
        // A force-confirmation token would otherwise reserve the restart slot
        // until its TTL expires. Send it back unchanged so the backend can
        // discard only this pending continuation before a later retry.
        await codexDesktopApi.cancelRestartWithForce(forceToken);
        closeDialog();
      } catch {
        toast.error(
          t("codexRestart.failed", {
            defaultValue: "配置已保存，但 Codex 重启失败。请手动重启后再继续。",
          }),
        );
        return;
      } finally {
        setIsRestarting(false);
      }
    } else {
      closeDialog();
    }

    toast.info(
      t("codexRestart.later", {
        defaultValue: "配置已保存；请稍后手动重启 Codex。",
      }),
    );
  }, [closeDialog, dialog, isRestarting, t]);

  return {
    dialog,
    isRestarting,
    notifyLiveConfigChanged,
    requestRestart,
    confirmForceRestart,
    deferRestart,
  };
}
