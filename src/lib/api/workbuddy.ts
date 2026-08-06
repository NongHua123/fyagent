import { invoke } from "@tauri-apps/api/core";

export interface WorkBuddyStatus {
  path: string;
  exists: boolean;
  modelCount: number;
  revision: string | null;
  backupExists: boolean;
  format: "legacyArray" | "objectRoot" | "missing";
}

export interface WorkBuddyModelIds {
  /**
   * First-occurrence, case-preserving IDs only. Configuration details and
   * credentials deliberately stay in the native service.
   */
  ids: string[];
  revision: string | null;
}

export interface WorkBuddyFetchModelsRequest {
  baseUrl: string;
  apiKey: string;
  allowNoApiKey: boolean;
}

export interface WorkBuddyFetchModelsResult {
  models: string[];
  truncated: boolean;
}

export interface WorkBuddyDuplicateId {
  id: string;
  count: number;
}

export interface WorkBuddySaveModelsRequest
  extends WorkBuddyFetchModelsRequest {
  selectedModelIds: string[];
  manualModelIds: string[];
  clearExistingApiKeys: boolean;
  expectedRevision: string | null;
  /** Opaque, short-lived confirmation capability issued by the backend. */
  overwriteToken?: string;
}

export interface WorkBuddySaveModelsSavedResult {
  state: "saved";
  revision: string;
  modelCount: number;
  createdEntries: number;
  updatedEntries: number;
}

export interface WorkBuddyOverwriteConfirmationRequiredResult {
  state: "overwrite_confirmation_required";
  /** Opaque capability; never persist or put this value in a query key. */
  token: string;
  /** Unique existing model IDs, without occurrence counts or details. */
  existingIds: string[];
}

export interface WorkBuddyConcurrentModificationResult {
  state: "concurrent_modification";
}

export type WorkBuddySaveModelsResult =
  | WorkBuddySaveModelsSavedResult
  | WorkBuddyOverwriteConfirmationRequiredResult
  | WorkBuddyConcurrentModificationResult;

export type WorkBuddyErrorCode =
  | "WORKBUDDY_INVALID_URL"
  | "WORKBUDDY_API_KEY_REQUIRED"
  | "WORKBUDDY_FETCH_HTTP_ERROR"
  | "WORKBUDDY_FETCH_TIMEOUT"
  | "WORKBUDDY_FETCH_REDIRECT_REJECTED"
  | "WORKBUDDY_FETCH_RESPONSE_TOO_LARGE"
  | "WORKBUDDY_FETCH_INVALID_SCHEMA"
  | "WORKBUDDY_CONFIG_READ_FAILED"
  | "WORKBUDDY_CONFIG_INVALID_JSON"
  | "WORKBUDDY_CONFIG_ROOT_NOT_ARRAY"
  | "WORKBUDDY_CONFIG_ROOT_UNSUPPORTED"
  | "WORKBUDDY_CONFIG_MODELS_NOT_ARRAY"
  | "WORKBUDDY_CONFIG_INVALID_ENTRY"
  | "WORKBUDDY_CONFIG_NO_TARGET_MODELS"
  | "WORKBUDDY_CONFIG_CONCURRENT_MODIFICATION"
  | "WORKBUDDY_CONFIG_BACKUP_FAILED"
  | "WORKBUDDY_CONFIG_WRITE_FAILED"
  | "WORKBUDDY_INTERNAL_ERROR"
  | "WORKBUDDY_CONFIG_DUPLICATE_TARGET"
  | "WORKBUDDY_CONFIG_EXISTING_TARGET"
  | "WORKBUDDY_OVERWRITE_TOKEN_INVALID"
  | "WORKBUDDY_OVERWRITE_TOKEN_EXPIRED";

export interface WorkBuddyErrorDetails {
  httpStatus?: number;
  redactedSummary?: string;
}

export interface WorkBuddyError {
  code: WorkBuddyErrorCode | string;
  messageKey: string;
  details: WorkBuddyErrorDetails;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === "object";

const tryParseJson = (value: unknown): unknown => {
  if (typeof value !== "string") return value;

  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
};

const toWorkBuddyError = (value: unknown): WorkBuddyError | null => {
  const parsed = tryParseJson(value);
  if (!isRecord(parsed) || typeof parsed.code !== "string") return null;
  if (!parsed.code.startsWith("WORKBUDDY_")) return null;

  const details = isRecord(parsed.details) ? parsed.details : {};
  return {
    code: parsed.code,
    messageKey:
      typeof parsed.messageKey === "string"
        ? parsed.messageKey
        : "workbuddy.error.internal",
    details: {
      httpStatus:
        typeof details.httpStatus === "number" ? details.httpStatus : undefined,
      redactedSummary:
        typeof details.redactedSummary === "string"
          ? details.redactedSummary
          : undefined,
    },
  };
};

/**
 * Tauri can reject an invoke with a structured payload directly, nested under
 * `payload`, or serialized in an Error message. Normalize those transport
 * shapes without ever using arbitrary backend text as UI output.
 */
export const getWorkBuddyError = (error: unknown): WorkBuddyError | null => {
  const direct = toWorkBuddyError(error);
  if (direct) return direct;
  if (!isRecord(error)) return null;

  return (
    toWorkBuddyError(error.payload) ??
    toWorkBuddyError(error.message) ??
    toWorkBuddyError(error.error)
  );
};

export const workBuddyApi = {
  async getStatus(): Promise<WorkBuddyStatus> {
    return await invoke("get_workbuddy_status");
  },

  async getModelIds(): Promise<WorkBuddyModelIds> {
    return await invoke("get_workbuddy_model_ids");
  },

  async fetchModels(
    request: WorkBuddyFetchModelsRequest,
  ): Promise<WorkBuddyFetchModelsResult> {
    return await invoke("fetch_workbuddy_models", { request });
  },

  async saveModels(
    request: WorkBuddySaveModelsRequest,
  ): Promise<WorkBuddySaveModelsResult> {
    return await invoke("save_workbuddy_models", { request });
  },
};
