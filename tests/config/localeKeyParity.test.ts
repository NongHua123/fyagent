import { describe, expect, it } from "vitest";
import en from "@/i18n/locales/en.json";
import ja from "@/i18n/locales/ja.json";
import zhTW from "@/i18n/locales/zh-TW.json";
import zh from "@/i18n/locales/zh.json";

type TranslationTree = Record<string, unknown>;

function leafKeyPaths(tree: TranslationTree, prefix = ""): string[] {
  return Object.entries(tree).flatMap(([key, value]) => {
    const path = prefix ? `${prefix}.${key}` : key;

    if (typeof value === "string") {
      return [path];
    }

    if (value && typeof value === "object" && !Array.isArray(value)) {
      return leafKeyPaths(value as TranslationTree, path);
    }

    throw new Error(`Unsupported translation value at ${path}`);
  });
}

const baselineKeys = new Set(leafKeyPaths(zh));
const locales = [
  ["en", en],
  ["ja", ja],
  ["zh-TW", zhTW],
] as const;

describe("locale key parity", () => {
  it.each(locales)("has the same translation keys in %s", (_locale, locale) => {
    const localeKeys = new Set(leafKeyPaths(locale));
    const missing = [...baselineKeys]
      .filter((key) => !localeKeys.has(key))
      .sort();
    const unexpected = [...localeKeys]
      .filter((key) => !baselineKeys.has(key))
      .sort();

    expect(missing).toEqual([]);
    expect(unexpected).toEqual([]);
  });
});
