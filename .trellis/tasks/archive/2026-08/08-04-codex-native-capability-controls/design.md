# Codex 原生能力放宽设计

## Architecture and Boundaries

- Rust `codex_config` 继续作为 TOML 分析与无损补丁的唯一权威；renderer 不直接解析或重写 TOML。
- capability eligibility 从“供应商资格”改为“配置是否可编辑”：合法 TOML 对所有 Codex 供应商可编辑，整份 TOML 无效时仅返回诊断。
- 添加/更新的结果型 IPC 在原 `ProviderMutationResult` 中返回结构化警告码；前端 mutation 层负责本地化并决定 success/warning toast。
- 代理服务只投影已保存能力，不新增 WebSocket 网络处理。

## Configuration Flow

1. 表单构建包含最新 TOML、meta 和模型目录的 Provider 草稿。
2. 分析 API 返回开关状态和字段诊断；控件仅在整份 TOML 无效或补丁进行中禁用。
3. 开关操作调用 Rust 补丁 API。补丁只修改目标字段，保留其他 TOML 格式和字段；非法目标字段按 PRD 的显式修复规则替换或删除。
4. 官方空配置在首次开启时创建 `model_provider = "custom"` 与 `[model_providers.custom]`，写入 `name = "OpenAI"`、`wire_api = "responses"`、`requires_openai_auth = true` 和启用的能力。私有 meta 仅标记该受管骨架的所有权。
5. 正常 provider add/update 才持久化草稿。保存后后端从最终 Provider 计算 WebSocket 风险警告，并随 mutation 结果返回。
6. 若当前 Codex 代理接管已启用，Live 投影保留显式能力；官方投影从选中供应商配置复制能力到代理 provider 表。

## Code Boundaries

- `src-tauri/src/codex_config.rs`：全文档分析、目标字段补丁、官方骨架所有权、模型风险分类和官方代理投影。
- `src-tauri/src/commands/provider.rs` 与 `src-tauri/src/provider.rs`：保存成功后读取最终 Provider，组装 `warningCodes` 且保持 mutation envelope 向后兼容。
- `src/components/providers/forms/**` 与 `src/hooks/useCodexProviderFeatures.ts`：高级区展示、表单内串行 TOML 草稿补丁、私有 meta 标记传递；不直接解析 TOML。
- `src/lib/query/mutations.ts` 与四语言资源：稳定警告码的本地化、去重与单 toast 展示。
- `src-tauri/src/services/proxy.rs`：代理接管、热切换和恢复不丢失供应商显式能力，不增加 WebSocket Upgrade 路由。

## Warning Contract

- 警告码使用稳定枚举字符串：`CODEX_WEBSOCKET_NON_GPT_MODEL`、`CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED`。
- 仅 WebSocket 显式为布尔值 `true` 时产生警告。
- 模型来源为 TOML 顶层 `model`、`review_model` 和 JSON `modelCatalog.models[].model`。忽略空值，取 `/` 后最后一段并用 ASCII 大小写不敏感的 `gpt-` 前缀判断。
- mutation 每次成功都重新计算并返回风险；不保存确认标记。旧 mutation 字段和旧命令保持兼容。
- 警告从保存成功后重读的最终 Provider 计算；保存失败时不会构造或展示风险提示。
- 稳定 DTO 形状为 `{ value, liveConfigChanged, app, warningCodes? }`；空警告列表在 Rust 序列化时省略。

## Compatibility and Safety

- 现有 `applicable`/`compatible` DTO 字段保留以避免不必要的接口破坏，但不再表达 Responses 或供应商身份硬门禁；UI 不再用它们隐藏控件或阻止保存。
- 单字段诊断不再作为全局保存错误。补丁操作只修复被操作字段；整份 TOML 解析失败仍是唯一 capability 写入硬错误。
- 生图请求头冲突和非法映射的覆盖行为由用户明确授权；只在主动开关时执行，普通保存不做隐式破坏性迁移。
- 受管官方骨架只有在所有预期静态字段匹配、能力字段均关闭且没有额外用户字段时才删除；否则只删除目标能力字段并保留用户配置。
- 回滚可通过恢复旧 capability 门禁、移除 mutation 警告字段和恢复官方代理默认关闭实现；Provider JSON 与 TOML 不需要数据库迁移。
