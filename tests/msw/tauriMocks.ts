import { vi } from "vitest";
import { server } from "./server";

const TAURI_ENDPOINT = "http://tauri.local";
const invokedCommands: string[] = [];
let tauriRequestHeaders: HeadersInit | undefined;

export const clearTauriInvocations = () => {
  invokedCommands.length = 0;
  tauriRequestHeaders = undefined;
};

export const getTauriInvocations = (): readonly string[] => invokedCommands;

export const setTauriRequestHeaders = (headers: HeadersInit | undefined) => {
  tauriRequestHeaders = headers;
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: async (command: string, payload: Record<string, unknown> = {}) => {
    invokedCommands.push(command);
    const response = await fetch(`${TAURI_ENDPOINT}/${command}`, {
      method: "POST",
      headers: tauriRequestHeaders ?? { "Content-Type": "application/json" },
      body: JSON.stringify(payload ?? {}),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(text || `Invoke failed for ${command}`);
    }

    const text = await response.text();
    if (!text) return undefined;
    try {
      return JSON.parse(text);
    } catch {
      return text;
    }
  },
}));

const listeners = new Map<string, Set<(event: { payload: unknown }) => void>>();

const ensureListenerSet = (event: string) => {
  if (!listeners.has(event)) {
    listeners.set(event, new Set());
  }
  return listeners.get(event)!;
};

export const emitTauriEvent = (event: string, payload: unknown) => {
  const handlers = listeners.get(event);
  handlers?.forEach((handler) => handler({ payload }));
};

vi.mock("@tauri-apps/api/event", () => ({
  listen: async (
    event: string,
    handler: (event: { payload: unknown }) => void,
  ) => {
    const set = ensureListenerSet(event);
    set.add(handler);
    return () => {
      set.delete(handler);
    };
  },
}));

// Ensure the MSW server is referenced so tree shaking doesn't remove imports
void server;

vi.mock("@tauri-apps/api/path", () => ({
  homeDir: async () => "/home/mock",
  join: async (...segments: string[]) => segments.join("/"),
}));

const mockCurrentWindow = {
  close: async () => undefined,
  isMaximized: async () => false,
  minimize: async () => undefined,
  onResized: async (_handler: () => void) => () => undefined,
  setDecorations: async (_decorations: boolean) => undefined,
  toggleMaximize: async () => undefined,
};

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => mockCurrentWindow,
}));
