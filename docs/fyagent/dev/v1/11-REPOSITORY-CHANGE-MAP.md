# 仓库代码变更地图

> 基线：上传的 CC Switch `3.18.0` 快照。行号会漂移，以下以路径、符号和职责定位。实际 Agent 开始时必须用 `rg` 重新确认。

> 2026-07-30 clean-break 决策：下文中“保留旧 identifier、协议、数据目录或内部包名”的早期边界已被取代。当前身份统一为 `com.fyagent.desktop`、`fyagent://`、`~/.fyagent`、`fyagent.db`、`fyagent`/`fyagent_lib`，不迁移或兼容读取旧身份；历史基线与执行记录仍按当时事实保留。

## 1. 新增后端文件

| 路径                                             | Owner   | 作用                                    |
| ------------------------------------------------ | ------- | --------------------------------------- |
| `src-tauri/src/codex_desktop/mod.rs`             | Core    | 领域模块导出、平台构造                  |
| `src-tauri/src/codex_desktop/types.rs`           | Core    | 平台、版本、release、本地状态、结果 DTO |
| `src-tauri/src/codex_desktop/error.rs`           | Core    | 稳定错误码、诊断、脱敏                  |
| `src-tauri/src/codex_desktop/source.rs`          | Core    | agentsmirror parser、cache、release_id  |
| `src-tauri/src/codex_desktop/download.rs`        | Core    | 专用 HTTP、重试、进度、临时文件         |
| `src-tauri/src/codex_desktop/verify.rs`          | Core    | hash、size×3、release漂移、通用验证     |
| `src-tauri/src/codex_desktop/platform/mod.rs`    | Core    | trait、cfg、unsupported adapter         |
| `src-tauri/src/codex_desktop/platform/windows/*` | Windows | MSIX、WinRT、启动、提权实验             |
| `src-tauri/src/codex_desktop/platform/macos/*`   | macOS   | Bundle、DMG、签名、复制、启动           |
| `src-tauri/src/services/codex_desktop/mod.rs`    | Core    | 用例编排、缓存、AppHandle               |
| `src-tauri/src/services/codex_desktop/job.rs`    | Core    | 状态机、互斥、取消、事件                |
| `src-tauri/src/commands/codex_desktop.rs`        | Core    | 薄 Tauri commands                       |

## 2. 修改后端共享文件

### 2.1 `src-tauri/src/lib.rs`

现有定位：

- Tauri setup；
- logger；
- updater plugin；
- `AppState::new(db)`；
- `app.manage(app_state)`；
- `tauri::generate_handler![...]`。

修改：

- `pub mod codex_desktop;`（若 root modules 在文件上部）；
- AppState 创建后给新 service 设置 `AppHandle`；
- 注册 7 个 commands；
- 启动时清理 >24h installer temp；
- 删除/禁用 `tauri_plugin_updater` 初始化；
- 日志启动标题从 CC Switch改为 FyAgent；
- 需要时将退出/窗口关闭保护接到 job状态。

禁止：重排整个 200+ command列表或顺便重构 setup。

### 2.2 `src-tauri/src/store.rs`

当前：

```rust
pub struct AppState {
    pub db: Arc<Database>,
    pub proxy_service: ProxyService,
    pub usage_cache: Arc<UsageCache>,
}
```

修改：

```rust
pub codex_desktop_service: Arc<CodexDesktopService>
```

在 `new` 中创建唯一实例。若构造需要可注入平台/source，生产构造放 service factory，测试直接构造。

### 2.3 `src-tauri/src/commands/mod.rs`

当前采用 `mod x; pub use x::*;`。

增加：

```rust
mod codex_desktop;
pub use codex_desktop::*;
```

### 2.4 `src-tauri/src/services/mod.rs`

增加：

```rust
pub mod codex_desktop;
pub use codex_desktop::CodexDesktopService;
```

### 2.5 `src-tauri/src/proxy/http_client.rs`

当前：

- `GLOBAL_CLIENT`；
- `CURRENT_PROXY_URL`；
- `get()`；
- 私有 `build_client`；
- 支持 http/https/socks5/socks5h；
- 系统代理自环保护。

最小修改：

- 新增安装器专用 scoped client builder；或
- 暴露安全的只读代理配置 + 共用构造逻辑。

不得改变现有 `get()` 的 timeout、解压、redirect或代理行为。

### 2.6 `src-tauri/Cargo.toml`

已有可复用依赖应先确认 feature：

```text
reqwest, sha2, zip, tempfile, url, tokio, serde, serde_json, thiserror
```

允许新增：

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "...", features = [所需 WinRT/Win32 API] }

[dependencies]
quick-xml = "..." # 若现有 XML 能力不可复用
```

只启用最小 feature。由集成 Agent统一修改 lockfile。

### 2.7 `src-tauri/src/main.rs`

只为内部 Windows all-users实验能力增加 Tauri初始化前的严格 headless子命令分流。非 Windows不启用。

不要增加通用 CLI shell、任意命令、URL或路径执行能力。

## 3. Codex CLI 修改

### 3.1 `src-tauri/src/commands/misc.rs`

关键符号：

```text
ToolVersion
get_tool_versions
run_tool_lifecycle_action
npm_install_command_for
install_command_for
probe_tool_installations
```

目标：

- `get_tool_versions` 继续返回 Codex read-only信息；
- latest版本探测可保留；
- `run_tool_lifecycle_action` 对 `tool == "codex"` 的 install/update/repair立即返回稳定拒绝；
- `npm_install_command_for("codex")` 不再作为生产执行路径；
- 批量计划排除 Codex；
- 删除/更新与 Codex安装命令有关的测试；
- 不破坏其他工具 Claude/Gemini/OpenCode/Grok/OpenClaw/Hermes。

建议引入工具 capability：

```rust
struct ToolCapabilities {
    can_install: bool,
    can_update: bool,
    can_repair: bool,
}
```

若改动过大，可用集中 helper `is_lifecycle_writable(tool)` 最小实现；禁止分散多个字符串特判。

### 3.2 后端错误

CLI 写操作拒绝可继续使用该模块既有错误格式，但文案明确：

```text
Codex CLI lifecycle management is disabled in FyAgent V1; version detection remains read-only.
```

## 4. FyAgent updater/品牌后端

### 4.1 `src-tauri/tauri.conf.json`

修改：

- `productName`: `FyAgent`；
- `bundle.createUpdaterArtifacts`: `false`；
- 删除 `plugins.updater` endpoint/pubkey；
- identifier 改为 `com.fyagent.desktop`，deep-link 仅注册 `fyagent`；minimum system version 保持不变；图标按下节替换。

### 4.2 应用品牌图标

- 将用户提供的 1024×1024 RGBA PNG 以精确字节保存为 `assets/fyagent.png`，作为唯一可审计
  视觉源；不得 AI 重绘、改色或调整构图；
- 执行 `mise exec -- pnpm tauri icon assets/fyagent.png --output src-tauri/icons`，覆盖现有桌面、
  Windows Store、Android 和 iOS 应用品牌路径，并保留生成的 `64x64.png`；
- `src/assets/icons/app-icon.png` 必须与生成的 `src-tauri/icons/32x32.png` 字节一致；
- `src-tauri/icons/tray/macos/` 的 `statusTemplate.png`、`statusTemplate@2x.png` 和
  `statusbar_template_3x.png` 分别为 24×24、48×48、72×72；从源图 alpha 的非透明边界
  等比缩放到 18pt 内容框并居中，RGB 全黑、alpha 保留抗锯齿，不使用彩色 bitmap；
  LICENSE。identifier、deep-link、数据目录、crate/npm 名由本次 clean-break 身份切换统一修改，
  不把这些改动伪装成图标生成结果。

### 4.3 `src-tauri/src/lib.rs`

删除 updater plugin初始化。

### 4.4 `src-tauri/src/commands/misc.rs`

定位：

```text
check_for_updates
check_app_update_available
install_update_and_restart
```

V1 应停止前端调用并从 invoke registration删除 updater命令。是否删除函数实现取决于无其他依赖；优先删除不可达上游运行时检查，避免未来误调用。

注意不要删除与“Codex桌面应用镜像版本检查”同名但新模块独立的能力。

### 4.5 权限/Capabilities

检查 `src-tauri/capabilities/*` 与 updater plugin权限。删除 updater插件后清理无用权限，确保构建通过。

## 5. 新增前端文件

| 路径                                                 | Owner | 作用              |
| ---------------------------------------------------- | ----- | ----------------- |
| `src/types/codexDesktop.ts`                          | UI    | DTO               |
| `src/lib/api/codex-desktop.ts`                       | UI    | invoke            |
| `src/lib/query/codex-desktop.ts`                     | UI    | query/mutations   |
| `src/hooks/useCodexDesktopInstaller.ts`              | UI    | 事件、组合、Toast |
| `src/components/codex/CodexDesktopInstallerCard.tsx` | UI    | 卡片              |

测试文件按项目惯例放在同级 `__tests__` 或现有 test目录。

## 6. 修改前端共享文件

### 6.1 `src/App.tsx`

现有 `ProviderList` 渲染在 providers view。只增加安装卡片挂载，不新增业务 state。

预期结构：

```tsx
<>
  {activeApp === "codex" && <CodexDesktopInstallerCard />}
  <ProviderList ... />
</>
```

Linux隐藏逻辑可由 Card/Hook根据后端 support返回 null；避免 App自己做重复平台判断。

### 6.2 `src/lib/api/index.ts`

导出新 API。

### 6.3 `src/lib/query/index.ts`

若现有 query barrel统一导出，则导出新 hooks/keys；若项目不统一导出，遵循现有惯例。

### 6.4 `src/i18n/locales/zh.json`、`en.json`

增加 `codexDesktopInstaller` namespace和 CLI只读文案。

`ja.json`、`zh-TW.json` 不修改或只删除失效直接引用；缺失新键回退英文。

### 6.5 `src/components/settings/AboutSection.tsx`

关键区域：

- `TOOL_NAMES`；
- 一键安装命令常量；
- 工具卡片；
- update/install handlers；
- 批量工具动作；
- About标题/链接；
- updater UI。

修改：

- Codex label为 `Codex CLI`；
- Codex卡片只读；
- 排除所有写 action；
- 一键命令移除 Codex；
- 可见品牌改 FyAgent；
- About 图标使用由同一 FyAgent 源生成的 `src/assets/icons/app-icon.png`；
- 移除 ccswitch.io、过时上游运营路由与 updater 区域；真实仓库 URL 与必要上游引用按事实保留；
- 保留版本信息与其他工具管理。

不要把新桌面安装器塞入 AboutSection。

### 6.6 `src/main.tsx`

当前用 `UpdateProvider` 包裹 `<App />`。移除 wrapper/import。保留 Query、Theme、Toaster。

### 6.7 `src/contexts/UpdateContext.tsx`、`src/lib/updater.*`

若无其他使用，删除文件和 import；若为了后续保留代码，也必须确保不被 bundle/调用且无 endpoint。建议彻底删除 dead runtime updater，减少误触发。

### 6.8 Update banner/components

使用 `rg "useUpdate|UpdateProvider|checkForUpdate|installUpdate"` 清理所有消费者。

### 6.9 窗口/标题

- `src-tauri/tauri.windows.conf.json` title → FyAgent；
- 搜索 UI 旧品牌文案；

## 7. 可能涉及退出行为的文件

源码中查找：

```bash
rg "CloseRequested|onCloseRequested|RunEvent::ExitRequested|prevent_close|tray" src src-tauri/src
```

只在现有退出协调点查询 `codex_desktop_service.get_job()`。不要新建第二套窗口关闭框架。

## 8. 测试/fixture 新增

建议：

```text
src-tauri/tests/fixtures/codex_desktop/*
src/components/codex/__tests__/CodexDesktopInstallerCard.test.tsx
src/hooks/__tests__/useCodexDesktopInstaller.test.tsx
```

如仓库测试惯例是模块内 `#[cfg(test)]`，按现有方式即可。

## 9. 不应修改的区域

除编译所需 import外，不修改：

```text
src-tauri/src/services/provider/*
src-tauri/src/services/proxy.rs
src-tauri/src/proxy/protocol transforms
src-tauri/src/database/schema.rs
src-tauri/src/database/migration.rs
MCP/Skill/Prompt/Usage domain
Provider form/data model
```

身份清理的窄例外：允许在上述既有文件中把应用自有的旧品牌、命名空间、协议标记和结构化
错误码按冻结映射改为 FyAgent，并补针对性测试；不得借此修改 provider form 的状态、数据

## 10. 搜索审计清单

集成完成后：

```bash
rg -n "CC Switch|ccswitch.io|farion1231/cc-switch" src src-tauri \
  --glob '!**/LICENSE*' --glob '!**/*.lock'

rg -n "@openai/codex@latest|npm i -g @openai/codex|volta install @openai/codex" \
  src src-tauri

rg -n "UpdateProvider|useUpdate|tauri_plugin_updater|plugins.*updater|latest.json" \
  src src-tauri

rg -n "agentsmirror|github.com|oaistatic|apps.microsoft.com" \
  src-tauri/src/codex_desktop src/components/codex src/lib/api/codex-desktop.ts
```

解释：

- Codex install命令只允许出现在历史注释/测试迁移说明时，最好清理；
- agentsmirror只应在 source模块常量；
- GitHub/OpenAI/Microsoft地址不得出现在新安装器生产代码；-许可证文本中的 CC Switch不删除。

## 11. 变更规模控制

- 新领域模块可多文件；
- 共享文件每个只做必要最小 patch；
- 不重命名整个仓库；
- 新安装只使用 `~/.fyagent/fyagent.db`，不迁移或读取旧配置目录；
- 不修改数据库 schema；仅切换数据库文件身份；
- 不重新设计 About页面；
- 不把旧 VibeKey后端/硬件模块复制进来。
