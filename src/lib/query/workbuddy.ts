import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import {
  workBuddyApi,
  type WorkBuddyModelIds,
  type WorkBuddyStatus,
} from "@/lib/api/workbuddy";

export const workBuddyKeys = {
  all: ["workbuddy"] as const,
  status: () => [...workBuddyKeys.all, "status"] as const,
  modelIds: () => [...workBuddyKeys.all, "model-ids"] as const,
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

/**
 * This query intentionally projects only safe, de-duplicated IDs. API keys,
 * URLs, vendors, occurrence counts, and the full configuration document never
 * enter the renderer cache.
 */
export const useWorkBuddyModelIdsQuery =
  (): UseQueryResult<WorkBuddyModelIds> =>
    useQuery({
      queryKey: workBuddyKeys.modelIds(),
      queryFn: () => workBuddyApi.getModelIds(),
    });
