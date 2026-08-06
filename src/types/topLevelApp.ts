import type { AppId } from "@/lib/api/types";

/**
 * A selectable application in the top-level navigation.
 *
 * WorkBuddy owns an independent configuration document and never participates
 * in the provider-domain `AppId` IPC contract.
 */
export type TopLevelAppId = AppId | "workbuddy";

export const isProviderAppId = (appId: TopLevelAppId): appId is AppId =>
  appId !== "workbuddy";
