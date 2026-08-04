# 放宽 Codex 原生能力开关限制

## Goal

让所有 Codex 供应商都能在高级选项中查看和修改内置生图扩展与 WebSocket 能力。保存不再因供应商身份、API 格式或代理接管被拒绝；当模型或代理链路存在兼容性风险时，在添加或更新成功后给出可理解的文字警告。

## Background

- 当前能力分析把保留 provider ID、官方/托管身份、第三方 URL 和普通 API 凭据作为资格门禁，导致用户配置中的 `model_provider = "OpenAI"` 不显示开关。
- WebSocket 目前在 API 格式切换、前端保存准备和 Rust 保存校验三处被限制为 OpenAI Responses，编辑已有配置会报 `CODEX_FEATURE_INCOMPATIBLE_WEBSOCKET`。
- 当前本地代理仅实现 HTTP/SSE，不实现 WebSocket Upgrade；本任务允许保留 WebSocket 配置，但只承诺风险提示，不承诺代理或任意上游实际支持。

## Requirements

- **R1 全量展示**：所有 Codex 供应商在现有高级选项折叠区内显示两个能力开关，高级区初始保持折叠。官方供应商只新增这两个控件，其他仅面向非官方供应商的高级字段保持原有范围。供应商 ID、`base_url`、凭据类型、OAuth/托管身份、API 格式和代理接管都不是展示或编辑门禁。
- **R2 默认状态**：固定官方供应商（仅 `category = "official"` 或 ID `codex-official`）缺少显式配置时两个能力默认关闭且不注入 provider 表；不根据名称 `OpenAI`、`requires_openai_auth` 或 URL 推断官方身份。其他 Codex 供应商生图默认开启、WebSocket 默认关闭；已有显式 TOML 配置优先。
- **R3 编辑边界**：整份 TOML 无效时开关可见但禁用并显示错误。单个 `http_headers` 或 `supports_websockets` 字段无效时允许普通保存并保留诊断，只有用户操作对应开关时才修复。
- **R4 WebSocket 补丁**：开启无条件写入布尔值 `supports_websockets = true`，关闭删除字段。不得因 `wire_api`、`meta.apiFormat`、供应商身份或代理接管自动关闭或拒绝保存。
- **R5 生图请求头补丁**：开启时大小写不敏感地移除全部同名键，并写入唯一规范键 `x-openai-actor-authorization = "local-image-extension"`；关闭时移除全部同名键。合法映射保留其他请求头，非法映射在主动开启时整体替换、主动关闭时整体删除。
- **R6 官方延迟生成**：固定官方供应商首次实际开启能力时生成 `model_provider = "custom"` 及只含必要字段的 provider 表：`name = "OpenAI"`、`requires_openai_auth = true`、`wire_api = "responses"` 与用户开启的能力，以保留 ChatGPT 登录语义。私有 meta 标记受管骨架；两个能力都关闭时，只在骨架仍是受管形状且没有用户扩展时删除，否则只删能力字段；显式表优先于统一会话历史注入。
- **R7 保存警告**：仅在最终配置显式启用 WebSocket 时计算。检查顶层 `model`、顶层 `review_model` 和 `settingsConfig.modelCatalog.models[].model`；仅忽略空字符串，取模型标识最后一个 `/` 后片段并不区分大小写地判断 `gpt-` 前缀。任一非空模型不是 GPT 即产生非 GPT 警告，完全无可识别模型时不产生该警告。Codex 本地代理接管开启时另产生代理风险警告，但仍允许保存。
- **R8 提示行为**：警告只用于添加和更新成功结果，每次风险保存都提示，不持久化确认状态，不用于供应商切换。多项风险按“非 GPT、代理链路”顺序合并成一条 warning toast 并替换普通成功 toast；添加使用“供应商已添加”前缀，编辑使用“供应商已保存”前缀。无风险保留原成功提示，失败只显示错误。
- **R9 代理投影**：普通和官方 Codex 代理投影都保留显式 WebSocket 与受管生图请求头。官方投影不再固定关闭 WebSocket；不为本地代理新增 WebSocket 协议实现。
- **R10 兼容与文档**：结果型 provider mutation 增加可选结构化警告码，旧的 `value`、`liveConfigChanged`、`app` 字段和旧非结果命令保持兼容；同步四种界面语言、FyAgent 规范和开发文档，不新增依赖或数据库表。开关只修改表单草稿，取消编辑不得写入数据库或 Live 配置；Codex Live 变更与桌面端重启协调行为保持不变。

## Acceptance Criteria

- [x] **AC1 / R1-R3**：官方、保留名 `OpenAI`、Codex OAuth、xAI OAuth 和普通第三方供应商都显示可用开关；仅整份 TOML 无效时禁用。
- [x] **AC2 / R2-R6**：官方空配置保持无注入；首次开启生成正确 `custom` 表；全部关闭只安全清理由本功能拥有的空骨架。非官方生图默认开启且显式关闭可持久化。
- [x] **AC3 / R4**：Responses、Chat 和 Anthropic 格式均可保存 `supports_websockets = true`，API 格式变化不删除字段，也不再出现旧兼容性错误。
- [x] **AC4 / R3-R5**：重复/冲突请求头、错误值、非字符串映射和无效 WebSocket 类型按主动开关规则修复；未操作的异常字段可原样保存。
- [x] **AC5 / R7-R8**：GPT、非 GPT、混合目录、空目录、代理接管及双风险场景均返回正确警告；添加/更新每次风险保存只显示一条合并提示，切换供应商不提示。
- [x] **AC6 / R9**：普通及官方代理投影与恢复均保留能力字段；真实代理 WebSocket 连接不作为验收条件。
- [x] **AC7 / R10**：前后端 DTO、四语言资源、规范和文档一致；类型检查、格式检查、单元测试、renderer build、Rust fmt/clippy/test 与 `git diff --check` 通过，无法执行的 Windows 人工验证明确记录。

## Out of Scope

- 不实现本地代理 WebSocket Upgrade 或协议转发。
- 不联网探测模型或第三方上游的真实 WebSocket 能力。
- 不改变 Codex CLI 或 OpenAI API 自身约束。
- 不在供应商切换时提示，不记录警告确认状态。
- 不引入新依赖、数据库表或无关重构。
