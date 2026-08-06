import { describe, expect, it } from "vitest";
import {
  hasWorkBuddyRemoteHttpWarning,
  isWorkBuddyLoopbackHostname,
} from "@/components/workbuddy/urlSafety";

describe("WorkBuddy HTTP transport warning", () => {
  it.each([
    ["localhost", true],
    ["localhost.", true],
    ["127.0.0.1", true],
    ["127.255.255.255", true],
    ["[::1]", true],
    ["[::ffff:7f00:1]", true],
    ["192.168.1.20", false],
    ["10.0.0.10", false],
    ["api.example.test", false],
  ])("classifies %s loopback status", (hostname, expected) => {
    expect(isWorkBuddyLoopbackHostname(hostname)).toBe(expected);
  });

  it("warns only for remote HTTP URLs", () => {
    expect(hasWorkBuddyRemoteHttpWarning("http://localhost:8080/v1")).toBe(
      false,
    );
    expect(hasWorkBuddyRemoteHttpWarning("http://127.0.0.42/v1")).toBe(false);
    expect(hasWorkBuddyRemoteHttpWarning("http://[::1]:8080/v1")).toBe(false);
    expect(
      hasWorkBuddyRemoteHttpWarning("http://[::ffff:127.0.0.1]:8080/v1"),
    ).toBe(false);
    expect(hasWorkBuddyRemoteHttpWarning("http://192.168.1.20:8080/v1")).toBe(
      true,
    );
    expect(hasWorkBuddyRemoteHttpWarning("http://api.example.test/v1")).toBe(
      true,
    );
    expect(hasWorkBuddyRemoteHttpWarning("https://api.example.test/v1")).toBe(
      false,
    );
  });
});
