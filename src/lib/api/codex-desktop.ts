import { invoke } from "@tauri-apps/api/core";
import type {
  CodexDesktopRestartOutcome,
  CodexDesktopRuntimeStatus,
  JobSnapshot,
  LocalInstallStatus,
  RemoteReleaseStatus,
} from "@/types/codexDesktop";

/**
 * Thin IPC wrapper for the fixed Codex desktop installer command surface.
 *
 * No URL, package path, hash, source, scope, or validation override can cross
 * this boundary from the renderer.
 */
export const codexDesktopApi = {
  async getLocalStatus(): Promise<LocalInstallStatus> {
    return await invoke("codex_desktop_get_local_status");
  },

  async checkLatest(force = false): Promise<RemoteReleaseStatus> {
    return await invoke("codex_desktop_check_latest", { force });
  },

  async getJob(): Promise<JobSnapshot | null> {
    return await invoke("codex_desktop_get_job");
  },

  async startInstall(expectedReleaseId: string): Promise<JobSnapshot> {
    return await invoke("codex_desktop_start_install", {
      request: { expectedReleaseId },
    });
  },

  async cancelInstall(jobId: string): Promise<JobSnapshot> {
    return await invoke("codex_desktop_cancel_install", { jobId });
  },

  async launch(): Promise<void> {
    await invoke("codex_desktop_launch");
  },

  async openLogDirectory(): Promise<void> {
    await invoke("codex_desktop_open_log_directory");
  },

  async getRuntimeStatus(): Promise<CodexDesktopRuntimeStatus> {
    return await invoke("get_codex_desktop_runtime_status");
  },

  async requestRestart(): Promise<CodexDesktopRestartOutcome> {
    return await invoke("request_codex_desktop_restart");
  },

  /**
   * The backend-issued token is opaque, single-use, and short-lived. The
   * renderer never derives, alters, or combines it with process information.
   */
  async continueRestartWithForce(
    token: string,
  ): Promise<CodexDesktopRestartOutcome> {
    return await invoke("continue_codex_desktop_restart_with_force", {
      token,
    });
  },

  /**
   * Cancels only the pending backend continuation identified by this opaque
   * token. It is intentionally a no-result capability: the renderer cannot
   * learn whether a process or token still exists.
   */
  async cancelRestartWithForce(token: string): Promise<void> {
    await invoke("cancel_codex_desktop_restart_with_force", { token });
  },
};
