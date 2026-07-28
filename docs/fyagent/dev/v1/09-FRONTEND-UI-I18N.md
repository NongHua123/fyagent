# 前端、UI 与国际化实现规格

## 1. 目标

在 Codex Provider 页面顶部增加一个独立、可维护的安装卡片。前端只显示 Rust 权威状态，不实现安装状态机，不提供 source、URL、版本、安装范围或 all-users 选项。

## 2. 文件结构

```text
src/types/codexDesktop.ts
src/lib/api/codex-desktop.ts
src/lib/query/codex-desktop.ts
src/hooks/useCodexDesktopInstaller.ts
src/components/codex/CodexDesktopInstallerCard.tsx
```

测试与项目现有前端测试布局一致，不强制新目录风格。

## 3. App.tsx 改动边界

仅：

- import card；
- 在 `activeApp === "codex"` 且 `currentView === "providers"` 时，ProviderList 上方挂载；
- 保持现有滚动容器与布局。

示意：

```tsx
{activeApp === "codex" && currentView === "providers" && (
  <CodexDesktopInstallerCard />
)}
<ProviderList ... />
```

不得在 `App.tsx` 中加入：

- `invoke`；
- installer React state；
- event listener；
- 版本比较；
- platform 分支；
- Toast；
- 下载/取消；
- 错误处理。

## 4. API 层

`src/lib/api/codex-desktop.ts`：

```ts
export const codexDesktopApi = {
  getLocalStatus(): Promise<LocalInstallStatus>,
  checkLatest(force = false): Promise<RemoteReleaseStatus>,
  getJob(): Promise<JobSnapshot | null>,
  startInstall(expectedReleaseId: string): Promise<JobSnapshot>,
  cancelInstall(jobId: string): Promise<JobSnapshot>,
  launch(): Promise<void>,
  openLogDirectory(): Promise<void>,
};
```

只封装命令名和参数。错误保持后端结构，不在这里 Toast。

## 5. Query 层

建议 keys：

```ts
export const codexDesktopKeys = {
  all: ["codexDesktop"] as const,
  local: () => [...codexDesktopKeys.all, "local"] as const,
  remote: () => [...codexDesktopKeys.all, "remote"] as const,
  job: () => [...codexDesktopKeys.all, "job"] as const,
};
```

行为：

- 页面挂载时 local 和 remote 并行；
- remote staleTime 5 分钟，与后端 TTL 语义一致，但后端仍为最终缓存；
- refresh 调 `checkLatest(true)`；
- start 成功写入 job cache；
- job 终态使 local/remote invalidated；
- remote error 不清空 local；
- 不定时轮询 active job；使用事件，必要时窗口 focus 可调用 `getJob` 恢复。

## 6. Hook

`useCodexDesktopInstaller`：

输入无需参数，只有 Codex 页面挂载。

职责：

1. 调用 local/remote/job Query；
2. 订阅 `codex-desktop-installer://job-updated`；
3. 用 `jobId + sequence` 接受较新快照；
4. 推导 `InstallerViewState`；
5. 暴露安装、更新、启动、取消、重试、刷新、复制详情、打开日志；
6. Succeeded 首次出现时 Toast 一次；
7. 不在 remount 时重复成功 Toast；可记录最后 Toast 的 job ID 于组件生命周期或 Query metadata；
8. 处理 clipboard API 失败；
9. 不保存 authoritative job 到 localStorage。

输出建议：

```ts
interface CodexDesktopInstallerViewModel {
  state: InstallerViewState;
  localVersion?: string;
  remoteVersion?: string;
  progress?: { current: number; total: number; percent: number };
  primaryAction: "install" | "update" | "launch" | "retry" | null;
  primaryDisabled: boolean;
  canCancel: boolean;
  refresh(): Promise<void>;
  runPrimaryAction(): Promise<void>;
  cancel(): Promise<void>;
  copyErrorDetails(): Promise<void>;
  openLogs(): Promise<void>;
}
```

## 7. 卡片布局

位于 Provider 列表上方，随内容滚动。使用现有 Card/Button/Progress/Alert 组件，不引入 UI 依赖。

建议结构：

```text
┌────────────────────────────────────────────────────────┐
│ Codex 桌面应用                          [刷新图标]       │
│ 通过中国大陆友好镜像，下载并安装 OpenAI 官方最新版      │
│ ChatGPT 桌面应用（包含 Codex）。                        │
│                                                        │
│ OpenAI 官方安装包 · 中国大陆优化镜像                    │
│ 本地版本 26.x      最新版本 26.y                        │
│ [进度条 / 错误摘要]                                    │
│                                  [主按钮] [取消/辅助]    │
└────────────────────────────────────────────────────────┘
```

不显示：

- agentsmirror 是“发布者”；
- 下载 URL；
- 安装范围；
- Stable/Beta 选择；
- source 选择；
- 自定义 URL；
- hash 高级绕过；
- 重装/降级；
- 所有用户实验入口。

## 8. 状态与按钮表

| 条件 | 主文案 | 主按钮 | 辅助行为 |
|---|---|---|---|
| 本地/远程检查中 | 正在检查 | 禁用 | 刷新禁用 |
| 未安装 + release | 最新版 x | 一键安装 Codex | 刷新 |
| 旧版 + release | 可更新到 x | 更新 Codex | 启动可不单独显示，避免误启动旧版？可按产品选择保留辅助启动；V1 主按钮更新 |
| 同版 | 已是最新版 | 启动 Codex | 刷新 |
| 本地比远程新 | 已安装较新版本 | 启动 Codex | 不提供降级 |
| 本地安装 + remote fail | 暂时无法检查更新 | 启动 Codex | 重试 |
| 未安装 + remote fail | 无法获取最新版 | 重试 | 代理提示 |
| 下载中 | 已下载 x/y | 禁用主按钮 | 取消 |
| 验证中 | 正在验证官方安装包 | 禁用 | 若后端 cancellable 则取消 |
| 安装中 | 正在安装 | 禁用 | 不可取消 |
| 安装后验证 | 正在确认安装结果 | 禁用 | 不可取消 |
| 成功 | 已安装 | 启动 Codex | Toast 一次 |
| 失败 | 简短错误 | 重试 | 复制详情、打开日志 |
| Cancelled | 已取消 | 安装/更新 | 刷新 |
| Ambiguous | 检测到多个安装 | 禁用 | 复制详情 |
| Intel Mac | 暂不支持 Intel Mac | 禁用 | 无 |
| Linux | 卡片不渲染 | — | — |

## 9. 最终文案

### 9.1 简体中文

```text
标题：Codex 桌面应用
说明：通过中国大陆友好镜像，下载并安装 OpenAI 官方最新版 ChatGPT 桌面应用（包含 Codex）。
来源：OpenAI 官方安装包 · 中国大陆优化镜像

按钮：
一键安装 Codex
更新 Codex
启动 Codex
取消
重试
刷新
复制错误详情
打开日志目录

成功 Toast：
Codex 桌面应用安装成功。系统中可能显示为“ChatGPT”。
```

建议错误文案：

```text
暂时无法获取最新版本，请检查网络或 FyAgent 的全局代理设置后重试。
请保存工作并关闭 ChatGPT/Codex，然后重试更新。
下载的安装包未通过验证，已阻止安装。
检测到多个 Codex 桌面应用安装，请先人工处理。
```

### 9.2 English

```text
Title: Codex Desktop App
Description: Download and install the latest official OpenAI ChatGPT desktop app with Codex through a China-friendly mirror.
Source: Official OpenAI package · China-friendly mirror

Buttons:
Install Codex
Update Codex
Launch Codex
Cancel
Retry
Refresh
Copy error details
Open log folder

Success toast:
Codex Desktop App was installed successfully. It may appear as “ChatGPT” on your system.
```

## 10. i18n 策略

- `zh.json`、`en.json` 完整新增 installer keys；
- 其他语言可依赖 `fallbackLng: "en"`；
- 但 D25 的 FyAgent 品牌移除是全局可见要求：其他 locale 中现有 `CC Switch` 可见文本需替换或改为共享品牌 key，不能因“只维护中英文”而继续显示旧品牌；
- 不翻译错误原始系统码；稳定用户文案翻译。

建议 namespace：

```json
{
  "codexDesktop": {
    "title": "...",
    "description": "...",
    "source": "...",
    "actions": {},
    "states": {},
    "errors": {},
    "toast": {}
  }
}
```

## 11. Codex CLI 只读卡片

现有 About 工具区域：

- 显示名改为 `Codex CLI`；
- 保留本地版本、latest、环境/路径分布；
- 对 Codex 隐藏 install/update/repair action；
- 批量安装/更新目标排除 Codex；
- 手工命令文本移除 `npm i -g @openai/codex@latest`；
- 后端仍拒绝 direct IPC；
- 不把新桌面卡片和 CLI 版本混在同一个 DTO。

## 12. FyAgent 品牌与 updater UI

### 移除

- Header `CC Switch` 文案；
- `ccswitch.io` 链接；
- 上游 GitHub 可见链接；
- `UpdateBadge`；
- About 的 FyAgent 自更新检查/安装按钮；
- UpdateProvider 包裹（若完全不再使用）；
- upstream update error/toast。

### 保留

- LICENSE/版权；
- 内部 package/crate 名；
- identifier、deep-link、数据目录；
- 当前图标。

### 后端/配置配套

前端移除不够，`tauri.conf.json` updater endpoint、插件注册和相关命令也要禁用/移除，见 `11-REPOSITORY-CHANGE-MAP.md`。

## 13. 退出提示

卡片 Hook 的 job 状态应接入现有窗口关闭流程：

- Download/resolve/verify-download：提示退出将取消任务；
- Install/verify-install：提示正在安装，请等待；
- 不提供“强制退出并杀安装”；
- 确认下载退出后先请求 cancel，等待合理短时间进入 Cancelled/清理，再关闭。

若现有窗口 close guard 集中在其他模块，集成 Agent 统一添加，不在 Card 自己监听全局关闭两遍。

## 14. 可访问性与体验

- 进度条有 `aria-valuenow/min/max`；
- 不只用颜色表示失败/成功；
- 按钮 loading 时保留可读 label；
- 远程版本和本地版本可复制但不显示冗长内部 build 默认；
- 错误详情用 monospace 可选展开；
- 取消确认只在可能丢失已下载进度时出现；
- 安装成功不自动启动；
- 不自动修改 Provider。

## 15. 前端测试

至少覆盖：

- Linux 不渲染；
- Intel Mac unsupported；
- 未安装 → install；
- 旧版 → update；
- 同版/本地新 → launch；
- remote fail + local installed → launch 保留；
- remote fail + no local → retry；
- progress 与 cancel；
- install stage no cancel；
- success Toast exactly once；
- event subscription + initial snapshot race，用 sequence 取新；
- error copy redaction display；
- no scope/source/custom URL controls；
- Codex CLI read-only，无 install/update；
- batch targets exclude Codex；
- App.tsx 不直接 invoke（代码审查/测试结构）。
