import { invoke } from "@tauri-apps/api/core";
// @ts-expect-error jsdom is a test dependency without bundled declarations.
import { JSDOM } from "jsdom";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import { server } from "./server";
import {
  clearTauriInvocations,
  getTauriInvocations,
  setTauriRequestHeaders,
} from "./tauriMocks";

const TAURI_ENDPOINT = "http://tauri.local";

describe("native Fetch through the MSW Tauri mock", () => {
  it("returns JSON and records the invoked command", async () => {
    server.use(
      http.post(`${TAURI_ENDPOINT}/native_fetch_success`, async ({ request }) =>
        HttpResponse.json({ payload: await request.json(), transport: "msw" }),
      ),
    );

    await expect(
      invoke("native_fetch_success", { provider: "codex" }),
    ).resolves.toEqual({
      payload: { provider: "codex" },
      transport: "msw",
    });
    expect(getTauriInvocations()).toEqual(["native_fetch_success"]);
  });

  it("surfaces the text body from a non-2xx response", async () => {
    server.use(
      http.post(`${TAURI_ENDPOINT}/native_fetch_error`, () =>
        HttpResponse.text("native fetch rejected", { status: 422 }),
      ),
    );

    await expect(invoke("native_fetch_error")).rejects.toThrow(
      "native fetch rejected",
    );
    expect(getTauriInvocations()).toEqual(["native_fetch_error"]);
  });

  it("maps a 204 response to undefined", async () => {
    server.use(
      http.post(
        `${TAURI_ENDPOINT}/native_fetch_empty`,
        () => new HttpResponse(null, { status: 204 }),
      ),
    );

    await expect(invoke("native_fetch_empty")).resolves.toBeUndefined();
    expect(getTauriInvocations()).toEqual(["native_fetch_empty"]);
  });

  it("accepts jsdom-realm Headers through native Fetch and MSW", async () => {
    const realm = new JSDOM("");
    try {
      const headers = new realm.window.Headers({
        "Content-Type": "application/json",
        "X-FyAgent-Realm": "jsdom",
      });
      expect(headers).not.toBeInstanceOf(globalThis.Headers);
      setTauriRequestHeaders(headers);
      server.use(
        http.post(`${TAURI_ENDPOINT}/native_fetch_cross_realm`, ({ request }) =>
          HttpResponse.json({
            realm: request.headers.get("X-FyAgent-Realm"),
          }),
        ),
      );

      await expect(invoke("native_fetch_cross_realm")).resolves.toEqual({
        realm: "jsdom",
      });
      expect(getTauriInvocations()).toEqual(["native_fetch_cross_realm"]);
    } finally {
      realm.window.close();
    }

    clearTauriInvocations();
    server.use(
      http.post(`${TAURI_ENDPOINT}/native_fetch_after_reset`, ({ request }) =>
        HttpResponse.json({
          contentType: request.headers.get("Content-Type"),
          realm: request.headers.get("X-FyAgent-Realm"),
        }),
      ),
    );
    await expect(invoke("native_fetch_after_reset")).resolves.toEqual({
      contentType: "application/json",
      realm: null,
    });
    expect(getTauriInvocations()).toEqual(["native_fetch_after_reset"]);
  });
});
