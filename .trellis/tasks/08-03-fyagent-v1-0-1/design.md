# FyAgent v1-0.1 技术设计

## 架构边界

本任务由三个独立边界组成：应用版本元数据、Codex Provider/live 配置边界、WorkBuddy 独立配置边界。它们共享 Tauri IPC、前端查询/Mutation、四语言资源和安全错误处理，但不得通过把 WorkBuddy 接入 `AppType` 或通用 Provider 模型获取来节省代码。

## 版本设计

- `package.json`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 直接保持同一 `0.1.0` 值；Cargo 是 Rust 编译期版本传播源，Tauri 是 bundle/About 版本源，npm 是前端项目元数据。
- 新 Vitest 静态断言读取三份 manifest，以 JSON 解析和有限 TOML/JSON 字段检查验证值一致与 SemVer 合法性。它不调用网络、Tauri 或构建工具。
- 修改 Cargo manifest 后，以 Cargo 解析刷新根 package lock 记录；后续验证使用 `--locked --offline` 防止意外写 lock。

## Codex 能力与重启数据流

1. 前端 Provider 表单根据当前草稿与后端返回的 Codex TOML 派生能力状态；只有资格谓词通过时渲染开关。
2. 保存时，后端在结构化 Provider/TOML 边界执行无损 `toml_edit` 补丁并校验 WebSocket 与 Responses 格式。
3. Provider mutation 返回原结果与 `liveConfigChanged` 布尔摘要，不返回 TOML 原文、header 值或凭据。
4. 前端 mutation 成功后，协调器仅在 `liveConfigChanged` 和可信运行实例条件均成立时展示重启对话框。
5. 受限重启命令内部重新解析已验证的 Codex Desktop 安装身份；调用方无法传 PID、可执行路径或进程名。命令依次正常退出、等待、必要时在前端二次确认后强制退出、启动已验证目标、等待可信新实例。

### Codex IPC 契约

- `ProviderMutationResult<T>`：保留原 mutation payload，并附带 `liveConfigChanged: boolean`。只对会同步当前 Codex live 配置的 Provider 写入路径计算该字段；其他路径返回 `false`。
- 重启 API 使用能力范围固定的请求/结果 DTO：请求最多携带是否已获得强制退出确认，结果只提供可显示的状态/失败原因；不提供任意进程控制能力。
- `imageExtensionConfigured` 属于 Provider 私有元数据，只消除历史默认值歧义，不替代 TOML 读取。

## WorkBuddy 后端设计

### 公共 DTO

- `WorkBuddyStatus`：路径、是否存在、模型数量与 revision；无 URL、Key、完整 JSON 或 header。revision 是仅进程内密钥计算的完整文件 HMAC 摘要：它能使包括仅 Key 改动在内的外部变更触发冲突，但对 renderer 不构成可用候选 Key 的裸摘要；进程重启后的旧 revision 保守失效，页面重新读取状态后再保存。
- `FetchWorkBuddyModelsResult`：`models: string[]`、`truncated: boolean`。结果保留首次出现顺序，大小写敏感去重，不排序。
- `SaveWorkBuddyModelsRequest`：规范化 URL、瞬态 API Key、`allowNoApiKey`、自动选择 ID、手动 ID、可选清空已有 Key 标记、`expectedRevision`、`duplicatePolicy: "reject" | "updateAll"`。
- 结构化错误至少区分 URL/凭据/HTTP/重定向/响应结构/响应上限/文件无效/revision 冲突/重复模型；错误文本不回显秘密。

### HTTP 与 URL

- 以专用受限客户端请求唯一 `/models` endpoint；从现有全局代理策略中仅抽取代理配置，不复用其 URL 候选、空 Key 要求或排序语义。
- 仅原始或验证为同源的重定向请求发送 Authorization；跨源、HTTPS→HTTP 降级、超过三次、非 2xx、超时或超过 2 MiB 均失败。
- 仅解析对象 `data[]` 中的非空字符串 `id`；任何非法元素都使整次响应安全失败，而不是悄然跳过。第 1,001 个有效唯一 ID 出现时保留前 1,000 个并标记 `truncated=true`，但仍验证受限响应余量，避免截断掩盖后续畸形元素。

### 文件事务

1. 状态读取/保存均定位当前用户 `.workbuddy/models.json`，测试通过注入临时 HOME 或临时路径隔离。
2. 保存锁内重新读取、严格解析对象数组、以进程内 HMAC 计算不透明 revision、校验 `expectedRevision`，然后检查本次目标 ID 的重复项。HMAC 覆盖完整原始字节（含 Key）以保留并发检测，但密钥不持久化、不返回前端且不写日志。
3. `reject` 返回 `DuplicateConflict { duplicateIds: [{ id, count }] }`，不创建 backup、不写文件；`updateAll` 才继续。
4. 对已存在目标条目仅覆盖受管字段，保留原位置/未知字段；非目标条目保持字段和相对顺序；新目标按最终目标顺序追加。
5. 为合法既有文件生成单份同目录 backup；主文件与临时文件刷盘后调用平台安全替换。Windows 使用不需要先删除目标的替换策略，Unix 以 `0600` 创建/保留文件。

## WorkBuddy 前端设计

- 引入 `TopLevelAppId`，使 `AppSwitcher`、可见性设置和 App 根路由可表示 WorkBuddy，而 Provider 域继续只接受 `ProviderAppId` / `AppType`。
- WorkBuddy 页面独立持有 URL、Key、无 Key 开关、获取结果、选中模型、手动输入、pending 保存请求和错误状态；卸载时清空 Key。
- Fetch 成功时模型列表旁持续显示截断警告，并可额外显示有限成功 toast。手动模型仍可补充未出现在截断结果中的 ID。
- 发现重复模型时冻结 pending 请求，显示包含 ID、次数和“不自动去重”说明的专用确认对话框；确认只更改 policy 并重新提交，取消不写文件。

## 兼容性、安全与回滚

- 旧 `visibleApps` 缺少 WorkBuddy 时以默认 `true` 兼容；历史 Codex Provider 延迟迁移；旧产品身份和历史 `3.18.0` 资料不迁移、不改写。
- 所有密钥仅在请求构造时使用；日志和 Tauri 错误均为脱敏摘要。测试 fixture 使用假 Key。
- 配置写入的 rollback 边界是写前 backup 与失败时不替换主文件；Provider TOML 通过局部补丁避免格式回滚。代码问题按本任务 diff 回退，绝不删除用户配置。
