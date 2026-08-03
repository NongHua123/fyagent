import { invoke } from "@tauri-apps/api/core";
import type {
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
};
