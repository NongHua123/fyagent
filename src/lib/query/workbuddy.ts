import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { workBuddyApi, type WorkBuddyStatus } from "@/lib/api/workbuddy";

export const workBuddyKeys = {
  all: ["workbuddy"] as const,
  status: () => [...workBuddyKeys.all, "status"] as const,
};

/**
 * WorkBuddy status intentionally contains only path/existence/count/revision
 * metadata. It never reads a saved URL, API key, or model list into the UI.
 */
export const useWorkBuddyStatusQuery = (): UseQueryResult<WorkBuddyStatus> =>
  useQuery({
    queryKey: workBuddyKeys.status(),
    queryFn: () => workBuddyApi.getStatus(),
  });
