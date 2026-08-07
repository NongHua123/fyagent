/**
 * Grok Build (Grok CLI) 预设供应商配置模板
 *
 * 独立维护，与 codexProviderPresets.ts 无数据联动（Jason 2026-07-21 定）。
 * 初始条目取自当时的 Codex 预设快照，此后两边各自演进：
 * Provider 链接 / 图标 / endpoint 变更需要在本文件单独修改。
 *
 * 收录规则：
 * - 不含官方 / 托管 OAuth 预设：Grok CLI 自带 xAI 订阅登录，官方态走
 *   独立的 "Grok Official" 条目（对应 providers_seed.rs 的 seed，
 *   空 config = 不写自定义模型表）。
 * - 不含国产模型官方直连（cn_official）与纯开源模型托管站
 *   （SiliconFlow / ModelScope / Novita / Nvidia / AtlasCloud / OpenCode Go）：
 *   这些上游没有 Grok 模型，无法在 Grok CLI 中使用。
 * - 只收聚合站与第三方中转站，默认模型统一为 grok-4.5；
 *   OpenRouter 系命名空间的路由站用 "x-ai/grok-4.5"。
 *
 * config 字段沿用 Codex 风格 TOML 作为载体：Grok 表单只从中提取
 * base_url / model / wire_api 三个字段（extractCodex* 工具），再重建
 * Grok CLI 自己的 config.toml。
 */
import type { ProviderCategory } from "../types";
import type { CodexApiFormat } from "../types";
import { GROK_BUILD_DEFAULT_MODEL } from "../utils/grokBuildConfig";

export interface GrokBuildProviderPreset {
  name: string;
  nameKey?: string; // i18n key for localized display name
  websiteUrl: string;
  apiKeyUrl?: string;
  auth: Record<string, any>;
  config: string; // Codex 风格 TOML 载体（只消费 base_url / model / wire_api）
  isOfficial?: boolean;
  category?: ProviderCategory;
  endpointCandidates?: string[];
  icon?: string;
  iconColor?: string;
  apiFormat?: CodexApiFormat;
}

// 官方条目与后端 seed（providers_seed.rs 的 "Grok Official"）对应：
// 空 config = 不写自定义模型表，Grok CLI 回落到自带的 xAI OAuth 登录。
// 预设 id 复用固定 provider id，AddProviderDialog 据此走 ensure seed 流程。
export const grokBuildOfficialPreset: GrokBuildProviderPreset = {
  name: "Grok Official",
  websiteUrl: "https://x.ai/grok",
  isOfficial: true,
  category: "official",
  auth: {},
  config: "",
  icon: "grok",
  iconColor: "currentColor",
};

/** OpenRouter 系命名空间路由站的 Grok 模型 id */
const OPENROUTER_STYLE_GROK_MODEL = "x-ai/grok-4.5";

const grokAuth = (): Record<string, any> => ({ OPENAI_API_KEY: "" });

function grokPresetConfig(
  providerName: string,
  baseUrl: string,
  model = GROK_BUILD_DEFAULT_MODEL,
): string {
  const tomlString = (value: string) => JSON.stringify(value);

  return `model_provider = "custom"
model = ${tomlString(model)}

[model_providers.custom]
name = ${tomlString(providerName)}
base_url = ${tomlString(baseUrl)}
wire_api = "responses"
requires_openai_auth = true`;
}

export const grokBuildProviderPresets: GrokBuildProviderPreset[] = [
  {
    name: "PackyCode",
    websiteUrl: "https://www.packyapi.ai",
    apiKeyUrl: "https://www.packyapi.ai/register",
    auth: grokAuth(),
    config: grokPresetConfig("PackyCode", "https://www.packyapi.ai/v1"),
    endpointCandidates: [
      "https://www.packyapi.ai/v1",
      "https://cf.api.fan/v1",
      "https://slb-v1.api.fan/v1",
      "https://www.packyapi.com/v1",
    ],
    category: "third_party",
    icon: "packycode",
  },
  {
    name: "ZetaAPI",
    websiteUrl: "https://zetaapi.ai",
    apiKeyUrl: "https://zetaapi.ai/",
    auth: grokAuth(),
    config: grokPresetConfig("ZetaAPI", "https://api.zetaapi.ai/v1"),
    endpointCandidates: ["https://api.zetaapi.ai/v1"],
    category: "aggregator",
    icon: "zetaapi",
  },
  {
    name: "APINebula",
    websiteUrl: "https://apinebula.ai",
    apiKeyUrl: "https://apinebula.ai/VjM74M",
    auth: grokAuth(),
    config: grokPresetConfig("APINebula", "https://apinebula.ai/v1"),
    endpointCandidates: ["https://apinebula.ai/v1"],
    apiFormat: "openai_responses",
    category: "third_party",
    icon: "apinebula",
  },
  {
    name: "AICodeMirror",
    websiteUrl: "https://www.aicodemirror.ai",
    apiKeyUrl: "https://www.aicodemirror.ai/register",
    auth: grokAuth(),
    config: grokPresetConfig(
      "AICodeMirror",
      "https://api.aicodemirror.ai/api/codex/backend-api/codex",
    ),
    endpointCandidates: [
      "https://api.aicodemirror.ai/api/codex/backend-api/codex",
    ],
    icon: "aicodemirror",
    iconColor: "#000000",
  },
  {
    name: "PatewayAI",
    websiteUrl: "https://pateway.ai",
    apiKeyUrl: "https://pateway.ai/",
    auth: grokAuth(),
    config: grokPresetConfig("PatewayAI", "https://api.pateway.ai/v1"),
    endpointCandidates: ["https://api.pateway.ai/v1"],
    category: "third_party",
    icon: "pateway",
  },
  {
    name: "FennoAI",
    websiteUrl: "https://api.fenno.ai",
    apiKeyUrl: "https://api.fenno.ai/register",
    auth: grokAuth(),
    config: grokPresetConfig("FennoAI", "https://api.fenno.ai"),
    endpointCandidates: ["https://api.fenno.ai"],
    category: "aggregator",
    icon: "fenno",
  },
  {
    name: "RunAPI",
    websiteUrl: "https://runapi.co",
    apiKeyUrl: "https://runapi.co/register",
    auth: grokAuth(),
    config: grokPresetConfig("RunAPI", "https://runapi.co/v1"),
    category: "aggregator",
    icon: "runapi",
  },
  {
    name: "Unity2.ai",
    websiteUrl: "https://unity2.ai",
    apiKeyUrl: "https://unity2.ai/register",
    auth: grokAuth(),
    config: grokPresetConfig("Unity2.ai", "https://api.unity2.ai"),
    endpointCandidates: ["https://api.unity2.ai"],
    category: "aggregator",
    icon: "unity2",
  },
  {
    name: "Shengsuanyun",
    nameKey: "providerForm.presets.shengsuanyun",
    websiteUrl: "https://www.shengsuanyun.com/",
    apiKeyUrl: "https://www.shengsuanyun.com/",
    auth: grokAuth(),
    config: grokPresetConfig(
      "Shengsuanyun",
      "https://router.shengsuanyun.com/api/v1",
      OPENROUTER_STYLE_GROK_MODEL,
    ),
    category: "aggregator",
    icon: "shengsuanyun",
  },
  {
    name: "AIGoCode",
    websiteUrl: "https://aigocode.app",
    apiKeyUrl: "https://aigocode.app",
    auth: grokAuth(),
    config: grokPresetConfig("AIGoCode", "https://api.aigocode.app"),
    endpointCandidates: ["https://api.aigocode.app"],
    category: "third_party",
    icon: "aigocode",
    iconColor: "#5B7FFF",
  },
  {
    name: "Qiniu",
    nameKey: "providerForm.presets.qiniu",
    websiteUrl: "https://s.qiniu.com/nMvAvy",
    apiKeyUrl: "https://s.qiniu.com/nMvAvy",
    auth: grokAuth(),
    config: grokPresetConfig(
      "Qiniu",
      "https://api.qnaigc.com/bypass/openai/v1",
    ),
    endpointCandidates: [
      "https://api.qnaigc.com/bypass/openai/v1",
      "https://api.modelink.ai/bypass/openai/v1",
    ],
    category: "aggregator",
    icon: "qiniu",
  },
  {
    name: "SubRouter",
    websiteUrl: "https://subrouter.ai",
    apiKeyUrl: "https://subrouter.ai/register",
    auth: grokAuth(),
    config: grokPresetConfig("SubRouter", "https://subrouter.ai/v1"),
    endpointCandidates: ["https://subrouter.ai/v1"],
    category: "aggregator",
    icon: "subrouter",
  },
  {
    name: "APIKEY.FUN",
    websiteUrl: "https://apikey.fun",
    apiKeyUrl: "https://apikey.fun/register",
    auth: grokAuth(),
    config: grokPresetConfig("APIKEY.FUN", "https://api.apikey.fun/v1"),
    endpointCandidates: [
      "https://api.apikey.fun/v1",
      "https://slb.apikey.fun/v1",
    ],
    apiFormat: "openai_responses",
    category: "third_party",
    icon: "apikeyfun",
  },
  {
    name: "Code0",
    websiteUrl: "https://code0.ai",
    apiKeyUrl: "https://code0.ai/",
    auth: grokAuth(),
    config: grokPresetConfig("Code0", "https://code0.ai/v1"),
    endpointCandidates: ["https://code0.ai/v1"],
    category: "aggregator",
    icon: "code0",
  },
  {
    name: "TeamoRouter",
    websiteUrl: "https://teamorouter.com",
    apiKeyUrl: "https://teamorouter.com/",
    auth: grokAuth(),
    config: grokPresetConfig("TeamoRouter", "https://api.teamorouter.com/v1"),
    endpointCandidates: ["https://api.teamorouter.com/v1"],
    category: "aggregator",
    icon: "teamorouter",
  },
  {
    name: "ClaudeCN",
    websiteUrl: "https://claudecn.top",
    apiKeyUrl: "https://claudecn.ai/register",
    auth: grokAuth(),
    config: grokPresetConfig("ClaudeCN", "https://claudecn.top/v1"),
    category: "third_party",
    icon: "claudecn",
  },
  {
    name: "NekoCode",
    websiteUrl: "https://nekocode.ai",
    apiKeyUrl: "https://nekocode.ai/",
    auth: grokAuth(),
    config: grokPresetConfig("NekoCode", "https://nekocode.ai/v1"),
    endpointCandidates: ["https://nekocode.ai/v1"],
    category: "aggregator",
    icon: "nekocode",
  },
  {
    name: "A6API",
    websiteUrl: "https://www.a6api.com",
    apiKeyUrl: "https://a6api.com/register",
    auth: grokAuth(),
    config: grokPresetConfig("A6API", "https://api.a6api.com/v1"),
    endpointCandidates: ["https://api.a6api.com/v1"],
    category: "aggregator",
    icon: "a6api",
  },
  {
    name: "Compshare",
    nameKey: "providerForm.presets.ucloud",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl: "https://www.compshare.cn/coding-plan",
    auth: grokAuth(),
    config: grokPresetConfig("Compshare", "https://api.modelverse.cn/v1"),
    endpointCandidates: ["https://api.modelverse.cn/v1"],
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "Compshare Coding Plan",
    nameKey: "providerForm.presets.ucloudCoding",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl: "https://www.compshare.cn/coding-plan",
    auth: grokAuth(),
    config: grokPresetConfig(
      "Compshare Coding Plan",
      "https://cp.compshare.cn/v1",
    ),
    endpointCandidates: ["https://cp.compshare.cn/v1"],
    category: "aggregator",
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "CCSub",
    websiteUrl: "https://www.ccsub.net",
    apiKeyUrl: "https://www.ccsub.net/register",
    auth: grokAuth(),
    config: grokPresetConfig("CCSub", "https://www.ccsub.net/v1"),
    endpointCandidates: ["https://www.ccsub.net/v1"],
    category: "aggregator",
    icon: "ccsub",
  },
  {
    name: "SSSAiCode",
    websiteUrl: "https://sssaicodeapi.com",
    apiKeyUrl: "https://sssaicodeapi.com/register",
    auth: grokAuth(),
    config: grokPresetConfig(
      "SSSAiCode",
      "https://node-hk.sssaicodeapi.com/api/v1",
    ),
    endpointCandidates: [
      "https://node-hk.sssaicodeapi.com/api/v1",
      "https://node-hk.sssaiapi.com/api/v1",
      "https://node-cf.sssaicodeapi.com/api/v1",
    ],
    category: "third_party",
    icon: "sssaicode",
    iconColor: "#000000",
  },
  {
    name: "Micu",
    websiteUrl: "https://www.micuapi.ai",
    apiKeyUrl: "https://www.micuapi.ai/register",
    auth: grokAuth(),
    config: grokPresetConfig("Micu", "https://www.micuapi.ai/v1"),
    endpointCandidates: ["https://www.micuapi.ai/v1"],
    category: "third_party",
    icon: "micu",
    iconColor: "#000000",
  },
  {
    name: "RightCode",
    websiteUrl: "https://www.rightapi.ai",
    apiKeyUrl: "https://www.rightapi.ai/register",
    auth: grokAuth(),
    config: grokPresetConfig("RightCode", "https://www.rightapi.ai/codex/v1"),
    category: "third_party",
    icon: "rc",
    iconColor: "#E96B2C",
  },
  {
    name: "ETok.ai",
    websiteUrl: "https://etok.ai",
    apiKeyUrl: "https://etok.ai",
    auth: grokAuth(),
    config: grokPresetConfig("ETok.ai", "https://api.etok.ai/v1"),
    endpointCandidates: ["https://api.etok.ai/v1"],
    category: "third_party",
    icon: "etok",
    iconColor: "#000000",
  },
  {
    name: "Cubence",
    websiteUrl: "https://cubence.com",
    apiKeyUrl: "https://cubence.com/signup",
    auth: grokAuth(),
    config: grokPresetConfig("Cubence", "https://api.cubence.com/v1"),
    endpointCandidates: [
      "https://api.cubence.com/v1",
      "https://api-cf.cubence.com/v1",
      "https://api-dmit.cubence.com/v1",
      "https://api-bwg.cubence.com/v1",
    ],
    category: "third_party",
    icon: "cubence",
    iconColor: "#000000",
  },
  {
    name: "CrazyRouter",
    websiteUrl: "https://www.crazyrouter.com",
    apiKeyUrl: "https://www.crazyrouter.com/register",
    auth: grokAuth(),
    config: grokPresetConfig("CrazyRouter", "https://cn.crazyrouter.com/v1"),
    endpointCandidates: ["https://cn.crazyrouter.com/v1"],
    icon: "crazyrouter",
    iconColor: "#000000",
  },
  {
    name: "DMXAPI",
    websiteUrl: "https://www.dmxapi.cn",
    auth: grokAuth(),
    config: grokPresetConfig("DMXAPI", "https://www.dmxapi.cn/v1"),
    endpointCandidates: ["https://www.dmxapi.cn/v1"],
    category: "aggregator",
  },
  {
    name: "SudoCode.chat",
    websiteUrl: "https://sudocode.chat",
    apiKeyUrl: "https://sudocode.chat/sign-up",
    auth: grokAuth(),
    config: grokPresetConfig("SudoCode.chat", "https://api.sudocode.chat/v1"),
    endpointCandidates: ["https://api.sudocode.chat/v1"],
    apiFormat: "openai_responses",
    category: "third_party",
    icon: "sudocode",
  },
  {
    name: "SudoCode.us",
    websiteUrl: "https://sudocode.us",
    apiKeyUrl: "https://sudocode.us",
    auth: grokAuth(),
    config: grokPresetConfig("SudoCode.us", "https://sudocode.us/v1"),
    endpointCandidates: ["https://sudocode.us/v1", "https://sudocode.run/v1"],
    apiFormat: "openai_responses",
    category: "third_party",
    icon: "sudocode-us",
  },
  {
    name: "xAI (Grok)",
    websiteUrl: "https://x.ai/api",
    apiKeyUrl: "https://console.x.ai",
    auth: grokAuth(),
    config: grokPresetConfig("xAI (Grok)", "https://api.x.ai/v1"),
    endpointCandidates: ["https://api.x.ai/v1"],
    apiFormat: "openai_responses",
    category: "third_party",
    icon: "xai",
    iconColor: "#000000",
  },
  {
    name: "Amux",
    websiteUrl: "https://amux.ai",
    apiKeyUrl: "https://amux.ai",
    auth: grokAuth(),
    config: grokPresetConfig("Amux", "https://api.amux.ai/v1"),
    endpointCandidates: ["https://api.amux.ai/v1"],
    category: "aggregator",
    icon: "amux",
  },
  {
    name: "AiHubMix",
    websiteUrl: "https://aihubmix.com",
    auth: grokAuth(),
    config: grokPresetConfig("AiHubMix", "https://aihubmix.com/v1"),
    endpointCandidates: [
      "https://aihubmix.com/v1",
      "https://api.aihubmix.com/v1",
    ],
    category: "aggregator",
    icon: "aihubmix",
    iconColor: "#006FFB",
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    auth: grokAuth(),
    config: grokPresetConfig(
      "CherryIN",
      "https://open.cherryin.net/v1",
      OPENROUTER_STYLE_GROK_MODEL,
    ),
    endpointCandidates: ["https://open.cherryin.net/v1"],
    category: "aggregator",
    icon: "cherryin",
  },
  {
    name: "RelaxyCode",
    websiteUrl: "https://www.relaxycode.com",
    apiKeyUrl: "https://www.relaxycode.com/register",
    auth: grokAuth(),
    config: grokPresetConfig("RelaxyCode", "https://www.relaxycode.com/v1"),
    category: "third_party",
    icon: "relaxcode",
  },
  {
    name: "E-FlowCode",
    websiteUrl: "https://e-flowcode.cc",
    apiKeyUrl: "https://e-flowcode.cc",
    auth: grokAuth(),
    config: grokPresetConfig("E-FlowCode", "https://e-flowcode.cc/v1"),
    endpointCandidates: ["https://e-flowcode.cc/v1"],
    category: "third_party",
    icon: "eflowcode",
    iconColor: "#000000",
  },
  {
    name: "PIPELLM",
    websiteUrl: "https://code.pipellm.ai",
    apiKeyUrl: "https://code.pipellm.ai/login",
    auth: grokAuth(),
    config: grokPresetConfig("PIPELLM", "https://cc-api.pipellm.ai/v1"),
    endpointCandidates: ["https://cc-api.pipellm.ai/v1"],
    category: "aggregator",
    icon: "pipellm",
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    auth: grokAuth(),
    config: grokPresetConfig(
      "OpenRouter",
      "https://openrouter.ai/api/v1",
      OPENROUTER_STYLE_GROK_MODEL,
    ),
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6566F1",
  },
  {
    name: "TheRouter",
    websiteUrl: "https://therouter.ai",
    apiKeyUrl: "https://dashboard.therouter.ai",
    auth: grokAuth(),
    config: grokPresetConfig(
      "TheRouter",
      "https://api.therouter.ai/v1",
      OPENROUTER_STYLE_GROK_MODEL,
    ),
    endpointCandidates: ["https://api.therouter.ai/v1"],
    category: "aggregator",
  },
];
