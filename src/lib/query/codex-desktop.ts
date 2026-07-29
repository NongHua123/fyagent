import { useQuery } from "@tanstack/react-query";
import { codexDesktopApi } from "@/lib/api/codex-desktop";

export const CODEX_DESKTOP_REMOTE_STALE_TIME_MS = 5 * 60 * 1000;

export const codexDesktopKeys = {
  all: ["codexDesktop"] as const,
  local: () => [...codexDesktopKeys.all, "local"] as const,
  remote: () => [...codexDesktopKeys.all, "remote"] as const,
  job: () => [...codexDesktopKeys.all, "job"] as const,
};

export function useCodexDesktopLocalStatus() {
  return useQuery({
    queryKey: codexDesktopKeys.local(),
    queryFn: () => codexDesktopApi.getLocalStatus(),
  });
}

export function useCodexDesktopLatestRelease() {
  return useQuery({
    queryKey: codexDesktopKeys.remote(),
    queryFn: () => codexDesktopApi.checkLatest(false),
    staleTime: CODEX_DESKTOP_REMOTE_STALE_TIME_MS,
  });
}

/**
 * The hook owns the listener-first recovery sequence, so this query does not
 * issue an automatic request before a Tauri event subscription exists.
 */
export function useCodexDesktopJob() {
  return useQuery({
    queryKey: codexDesktopKeys.job(),
    queryFn: () => codexDesktopApi.getJob(),
    enabled: false,
    staleTime: Infinity,
  });
}
