import { renderHook, act } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { useSettingsMetadata } from "@/hooks/useSettingsMetadata";

const isPortableMock = vi.hoisted(() => vi.fn());
const getRuntimePrivilegeStatusMock = vi.hoisted(() => vi.fn());

vi.mock("@/lib/api", () => ({
  settingsApi: {
    isPortable: (...args: unknown[]) => isPortableMock(...args),
    getRuntimePrivilegeStatus: (...args: unknown[]) =>
      getRuntimePrivilegeStatusMock(...args),
  },
}));

describe("useSettingsMetadata", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getRuntimePrivilegeStatusMock.mockResolvedValue({
      platform: "other",
      supported: false,
      elevated: false,
      localAdministrator: false,
      interactiveUserMatch: "unavailable",
    });
  });

  it("loads portable flag and handles success path", async () => {
    isPortableMock.mockResolvedValue(true);

    const { result } = renderHook(() => useSettingsMetadata());

    expect(result.current.isLoading).toBe(true);
    expect(result.current.isPortable).toBe(false);

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.isPortable).toBe(true);
    expect(result.current.runtimePrivilege).toEqual({
      platform: "other",
      supported: false,
      elevated: false,
      localAdministrator: false,
      interactiveUserMatch: "unavailable",
    });
    expect(result.current.isLoading).toBe(false);
  });

  it("handles errors from settingsApi and proceeds", async () => {
    isPortableMock.mockRejectedValue(new Error("network failure"));

    const { result } = renderHook(() => useSettingsMetadata());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.isPortable).toBe(false);
    expect(result.current.runtimePrivilege).toEqual({
      platform: "other",
      supported: false,
      elevated: false,
      localAdministrator: false,
      interactiveUserMatch: "unavailable",
    });
    expect(result.current.isLoading).toBe(false);
  });

  it("allows updating restart flag via setters", async () => {
    isPortableMock.mockResolvedValue(false);

    const { result } = renderHook(() => useSettingsMetadata());

    await act(async () => {
      await Promise.resolve();
    });

    await act(async () => {
      result.current.setRequiresRestart(true);
      await Promise.resolve();
    });

    expect(result.current.requiresRestart).toBe(true);

    await act(async () => {
      result.current.acknowledgeRestart();
      await Promise.resolve();
    });

    expect(result.current.requiresRestart).toBe(false);
  });

  it("keeps other metadata available if the privilege read fails", async () => {
    isPortableMock.mockResolvedValue(true);
    getRuntimePrivilegeStatusMock.mockRejectedValue(
      new Error("host metadata unavailable"),
    );

    const { result } = renderHook(() => useSettingsMetadata());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.isPortable).toBe(true);
    expect(result.current.runtimePrivilege).toBeNull();
  });
});
