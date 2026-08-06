import { describe, expect, it } from "vitest";

import { claudeDesktopProviderPresets } from "@/config/claudeDesktopProviderPresets";
import { providerPresets as claudeProviderPresets } from "@/config/claudeProviderPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { geminiProviderPresets } from "@/config/geminiProviderPresets";
import {
  grokBuildOfficialPreset,
  grokBuildProviderPresets,
} from "@/config/grokBuildProviderPresets";
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import { openclawProviderPresets } from "@/config/openclawProviderPresets";
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";

const presetGroups: Array<[string, readonly unknown[]]> = [
  ["claudeDesktop", claudeDesktopProviderPresets],
  ["claude", claudeProviderPresets],
  ["codex", codexProviderPresets],
  ["gemini", geminiProviderPresets],
  ["grokBuild", [grokBuildOfficialPreset, ...grokBuildProviderPresets]],
  ["hermes", hermesProviderPresets],
  ["openclaw", openclawProviderPresets],
  ["opencode", opencodeProviderPresets],
];

const partnerMetadataKeys = [
  "isPartner",
  "primePartner",
  "partnerPromotionKey",
] as const;
const trackingQueryKeys = new Set(["affiliate", "ic", "ref", "referral"]);

describe("provider preset promotion boundary", () => {
  it("keeps partner metadata and tracking parameters out of every preset", () => {
    for (const [app, presets] of presetGroups) {
      for (const preset of presets) {
        const record = preset as Record<string, unknown>;
        const label = `${app}:${String(record.id ?? record.name ?? "unknown")}`;

        for (const key of partnerMetadataKeys) {
          expect(record, `${label} contains ${key}`).not.toHaveProperty(key);
        }

        for (const field of ["websiteUrl", "apiKeyUrl"] as const) {
          const value = record[field];
          if (typeof value !== "string" || value.trim() === "") continue;

          const url = new URL(value);
          const trackingKeys = [...url.searchParams.keys()].filter((key) => {
            const normalized = key.toLowerCase();
            return (
              trackingQueryKeys.has(normalized) || normalized.startsWith("utm_")
            );
          });
          expect(
            trackingKeys,
            `${label}.${field} contains tracking query data`,
          ).toEqual([]);
        }
      }
    }
  });
});
