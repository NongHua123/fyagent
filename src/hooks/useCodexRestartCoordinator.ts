import { useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { codexDesktopApi } from "@/lib/api";
import type {
  CodexDesktopRestartOutcome,
  CodexDesktopRestartPromptReason,
} from "@/types/codexDesktop";

export type CodexRestartDialogState =
  | {
      kind: "confirm";
      token: string;
      reason: CodexDesktopRestartPromptReason;
    }
  | { kind: "progress" }
  | { kind: "incomplete"; retryToken?: string }
  | null;

/**
 * Coordinates the backend-owned Codex Desktop restart capability after a
 * successful live configuration write. The renderer holds only opaque,
 * short-lived capabilities and never receives process or installation data.
 */
export function useCodexRestartCoordinator() {
  const { t } = useTranslation();
  const [dialog, setDialog] = useState<CodexRestartDialogState>(null);
  const dialogOpenRef = useRef(false);
  const restartRequestInFlightRef = useRef(false);
  const restartExecutionInFlightRef = useRef(false);

  const closeDialog = useCallback(() => {
    dialogOpenRef.current = false;
    setDialog(null);
  }, []);

  const showManualRestartNotice = useCallback(() => {
    toast.info(
      t("codexRestart.manualRequired", {
        defaultValue: "配置已保存；请手动重启 Codex 以加载新配置。",
      }),
    );
  }, [t]);

  const showIncomplete = useCallback((retryToken?: string) => {
    dialogOpenRef.current = true;
    setDialog(
      retryToken ? { kind: "incomplete", retryToken } : { kind: "incomplete" },
    );
  }, []);

  const handleRestartOutcome = useCallback(
    (
      outcome: CodexDesktopRestartOutcome,
      { duringExecution = false }: { duringExecution?: boolean } = {},
    ) => {
      switch (outcome.state) {
        case "restarted":
          closeDialog();
          toast.success(
            t("codexRestart.restarted", {
              defaultValue: "Codex 已重启，新配置已生效。",
            }),
          );
          return;
        case "confirmation_required":
          dialogOpenRef.current = true;
          setDialog({
            kind: "confirm",
            token: outcome.token,
            reason: outcome.reason,
          });
          return;
        case "not_running":
          // Saving remains successful, and a normally saved configuration
          // must never launch a desktop application that is not running.
          if (duringExecution) {
            showIncomplete();
          } else {
            closeDialog();
          }
          return;
        case "manual_restart_required":
          if (duringExecution) {
            // Once the user has confirmed the destructive action, a
            // fail-closed capability result must stay visible rather than
            // disappearing into a manual-restart toast.
            showIncomplete();
          } else {
            closeDialog();
            showManualRestartNotice();
          }
          return;
        case "incomplete":
          showIncomplete(outcome.retryToken);
          return;
      }
    },
    [closeDialog, showIncomplete, showManualRestartNotice, t],
  );

  const continueRestart = useCallback(
    async (token: string) => {
      if (restartExecutionInFlightRef.current) return;

      restartExecutionInFlightRef.current = true;
      dialogOpenRef.current = true;
      setDialog({ kind: "progress" });
      try {
        handleRestartOutcome(
          await codexDesktopApi.continueRestartWithForce(token),
          { duringExecution: true },
        );
      } catch {
        // A transport failure gives the renderer no trustworthy detail and no
        // reusable capability. Preserve the saved-config result and offer the
        // manual path instead of a transient error toast.
        showIncomplete();
      } finally {
        restartExecutionInFlightRef.current = false;
      }
    },
    [handleRestartOutcome, showIncomplete],
  );

  /**
   * Called only for a true backend `liveConfigChanged` result. The prepare
   * command is authoritative for the not-running/manual/confirmation result;
   * the renderer does not make a second runtime inspection first.
   */
  const notifyLiveConfigChanged = useCallback(async () => {
    if (dialogOpenRef.current || restartRequestInFlightRef.current) return;

    restartRequestInFlightRef.current = true;
    try {
      handleRestartOutcome(await codexDesktopApi.requestRestart());
    } catch {
      // The configuration is already saved. A failed capability request must
      // still have a durable, actionable UI rather than a transient toast.
      showIncomplete();
    } finally {
      restartRequestInFlightRef.current = false;
    }
  }, [handleRestartOutcome, showIncomplete]);

  const requestRestart = useCallback(async () => {
    if (dialog?.kind !== "confirm") return;
    await continueRestart(dialog.token);
  }, [continueRestart, dialog]);

  const retryRestart = useCallback(async () => {
    if (dialog?.kind !== "incomplete" || !dialog.retryToken) return;
    await continueRestart(dialog.retryToken);
  }, [continueRestart, dialog]);

  const deferRestart = useCallback(() => {
    if (dialog?.kind === "progress") return;

    const token =
      dialog?.kind === "confirm"
        ? dialog.token
        : dialog?.kind === "incomplete"
          ? dialog.retryToken
          : undefined;

    // This is an intentionally quiet dismissal: the configuration has already
    // been saved, and no extra toast should compete with that success result.
    closeDialog();

    if (token) {
      void codexDesktopApi.cancelRestartWithForce(token).catch(() => {
        // Discarding an opaque server capability is best-effort; failure must
        // not reopen a dialog or expose token-validation internals.
      });
    }
  }, [closeDialog, dialog]);

  return {
    dialog,
    isRestarting: dialog?.kind === "progress",
    notifyLiveConfigChanged,
    requestRestart,
    retryRestart,
    deferRestart,
  };
}
