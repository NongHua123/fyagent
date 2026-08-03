# 实现 FyAgent v1-0.1

## 目标

在现有 FyAgent 桌面应用中交付 v1-0.1 文档包定义的 Codex 与 WorkBuddy 能力，并把产品应用版本从历史 CC Switch 基线独立为 `0.1.0`。用户可在不丢失既有配置、注释、未知 JSON/TOML 字段或密钥的前提下，配置 Codex 第三方 Provider 的原生能力，安全地管理 WorkBuddy 模型配置，并在真实 live Codex 配置改变后得到受可信安装身份约束的重启选择。

## 已确认事实与输入

- 功能与验收基线仅来自 `docs/fyagent/dev/v1-0.1/README.md` 及其 `01` 至 `04` 文档；其中 `3.18.0` 是审阅源码快照，不是本次产品版本，应保留为历史证据。
- 当前分支为 `feature/fyagent-v1`。需求文档目录是用户已有的未跟踪输入，必须保留且不改写。
- 当前版本同时存在于 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`，根 Cargo lock 记录与 About 测试也使用旧值。Tauri、Cargo 与 Windows MSI 要求合法 SemVer。
- 当前项目已具备 `toml_edit`、可信 Codex Desktop 安装/运行检测边界、Tauri IPC、四语言本地化和临时 HOME 的 Rust 测试隔离机制；它们应被复用而不是替换。

## 范围与需求

### R1：FyAgent 独立版本链路

- 将 npm、Cargo、Tauri 应用与打包元数据统一为 `0.1.0`；通过 Cargo 正常刷新根包的 lock 记录。
- 保持现有 Tauri/Cargo 传播链，让 About、日志、崩溃信息、现有客户端标识和 MSI 自动获得同一版本，不为每个消费者新增常量。
- 增加本地版本一致性测试，验证三份应用 manifest 一致且是合法 SemVer；更新 About 的版本夹具。
- 不修改 release workflow、updater、tag、GitHub Release、历史 CHANGELOG、历史 release note 或历史文档基线。

### R2：Codex Provider 原生能力

- 在既有新增/编辑表单的高级区内增加两个供应商级开关：内置生图扩展与 WebSocket 传输；不得新增独立设置页。
- 只有适用的第三方 Codex Provider 可显示/启用开关：必须可编辑 `model_providers.<id>`、具有第三方 base URL 与普通 API 凭据，且不是官方内置或托管 OAuth/账号 Provider。资格判断不得依赖单一 Provider 类型字符串。
- 生图扩展的新建适用 Provider 默认开启。开启时只写受管 header `x-openai-actor-authorization = "local-image-extension"`；关闭时只删除大小写不敏感匹配且值精确相等的受管 header。冲突值、其他 header、未知字段、注释、空行和表顺序必须保留。
- WebSocket 默认关闭，仅限 Responses 格式。开启写 `supports_websockets = true`，关闭删除该字段；切换离开 Responses 时草稿自动移除，绕过 UI 的 TOML 保存也必须被拒绝。
- 以实际 Codex TOML 双向推导开关状态。历史 Provider 不批量改写；仅通过私有 `imageExtensionConfigured` 元数据区分历史待迁移与用户明确关闭，TOML 仍是生效真相。

### R3：live 配置变化与 Codex Desktop 重启

- Provider 写入结果必须向前端暴露非敏感 `liveConfigChanged`，它只表示本次成功操作是否实际改变当前用户 `~/.codex/config.toml` 的最终字节内容。
- 每次用户操作最多提示一次。只有 `liveConfigChanged` 为真且发现唯一可信、正在运行的 Codex Desktop 实例时才显示重启模态框；未运行、身份歧义、平台不支持或仅数据库变化都不得弹框或启动进程。
- 重启只能复用已验证安装身份，不能根据模糊进程名匹配，也不能暴露任意 PID 结束接口。正常退出等待最多 8 秒；超时才提供二次确认的强制退出；旧实例退出后才启动，并最多等待 15 秒确认可信新实例。
- 重启失败绝不回滚已成功保存的配置；“稍后手动重启”不执行进程操作。

### R4：WorkBuddy 独立顶层配置页

- WorkBuddy 是独立的顶层导航 ID，不是 `AppType`、Provider、MCP、Skills、Prompt、Profile、Session、用量或本地代理域的一部分。它位于 Codex 后、Gemini 前；旧 `visibleApps` 配置缺少该项时默认显示。
- 页面只显示路径、文件存在状态、模型数量、URL、瞬态 API Key、允许无 Key、模型获取/选择/手动输入与保存。不得从已有 `models.json` 回填 URL、Key 或模型；Key 只存在于组件内存，离开页面即清除。
- 新增独立 Tauri 命令：`get_workbuddy_status`、`fetch_workbuddy_models` 与 `save_workbuddy_models`。禁止给通用 Provider API 增加 WorkBuddy `app` 分支。
- URL 只接受绝对 HTTP(S)，拒绝 userinfo、query、fragment、无 host、非 HTTP(S)；移除标准尾端点后补齐 `/v1`。非 loopback HTTP 必须警告。
- 获取仅访问规范化 base URL 的 `/models`：15 秒总超时、2 MiB 上限、最多 3 次同源重定向、禁止 HTTPS 降级，仅安全同源请求携带 Authorization。只接收 `data[].id`，保持上游顺序、大小写敏感去重。超过 1,000 个唯一 ID 时返回前 1,000 个与 `truncated=true`，页面持续警告而不是把有限成功伪装为全量数据。
- 保存只读写当前用户的 `~/.workbuddy/models.json`。合法文件必须是对象数组且每项有字符串 `id`；损坏/非法结构安全失败，不静默修复。目标模型按文档强制设置受管字段，保留未知字段、`reasoning` 未知字段、非目标条目及原顺序。
- 保存前锁内重新读取并校验 revision。目标 ID 重复时首次默认拒绝、返回 ID/次数、不写主文件或 backup；用户确认后同一冻结请求带 `duplicatePolicy=updateAll` 重提，后端再次校验 revision 后更新所有重复条目但不自动去重。
- 成功写入前对已有合法文件保留固定单份 backup，并进行同目录刷盘与平台安全替换。Windows 不可使用先删除目标再 rename 的弱替换路径；Unix 保持 `0600`，Windows 不扩大 profile ACL。

### R5：体验、资源与安全

- 使用文档包提供的本地 WorkBuddy 图标，不运行时下载且不改动原始文档资产。
- 补齐简体中文、繁体中文、英语、日语文案；选择状态、截断警告、重复覆盖确认、API Key 掩码、键盘操作和屏幕阅读器语义均可访问。
- API Key、Authorization、完整 WorkBuddy JSON、带凭据 URL、Codex 配置正文与敏感 header 不得出现在日志、错误对象、Tauri 返回值或测试 fixture。

## 验收标准

- 文档 `04-验收标准与测试场景.md` 的 P0 Codex、重启、WorkBuddy、安全、回归和平台代码路径要求都有相应实现与自动化测试；所有不适于当前 Windows 环境的运行时项目明确标为人工验收。
- Codex TOML 补丁对受管 header、大小写冲突、注释/顺序保留、历史迁移、WebSocket 格式约束和损坏 TOML 具有确定测试。
- `liveConfigChanged` 只基于实际 live 文件最终字节差异，且不存在按名称或任意 PID 终止进程的实现。
- WorkBuddy 页面不加载 Provider 域数据；旧可见性配置兼容；四语言无缺键；模型截断、无 Key、URL、重定向、响应大小、保存顺序、重复确认、revision 冲突、backup 与失败回滚均有测试。
- `package.json`、`Cargo.toml` 与 `tauri.conf.json` 均为 `0.1.0`，Cargo lock 已同步，版本一致性测试和 About 测试通过。
- 优先离线恢复依赖后，本地执行 TypeScript、格式、Vitest、renderer build、Rust format check、clippy、Rust tests 与 `git diff --check`；若缓存不足，用户已授权使用冻结锁文件和忽略生命周期脚本的正常联网恢复，实际路径必须如实记录。

## 非范围与验证边界

- 不推送、不运行 CI、不创建 release、不打 tag、不修改 release workflow、不新增 updater，也不做真实第三方 API 调用。
- 不进行真实端到端测试；Windows 原生窗口与进程行为、真实 WorkBuddy 文件 ACL/替换、真实外部 API、macOS 与 Linux 运行时行为均交由人工验收。
- 不迁移、探测、删除或兼容旧 CC Switch 的本地状态；不把 WorkBuddy 纳入既有 Provider 域。
