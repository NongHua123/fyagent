# Codex 原生能力放宽执行计划

## Implementation Order

1. 加载任务工件、backend/frontend/unit-test/docs 规范和共享跨层指南，复核工作树及现有 Codex 能力测试。
2. 调整 Rust capability 分析、目标字段修复、官方延迟 provider 表生成/清理以及保存校验；补齐直接单元测试。
3. 扩展 provider mutation 结果和警告分类，接入 Codex 代理接管状态；让普通及官方代理投影保留显式能力。
4. 调整前端 hook 与高级选项布局，取消 API 格式自动关闭和保存硬拦截；接入添加/更新合并警告 toast 及四语言资源。
5. 更新前后端测试、FyAgent 配置域规范和开发文档，检查 DTO、错误传播、Live 配置变化与重启协调链路。
6. 运行局部测试后执行完整质量门禁，修复回归，记录 Windows 桌面端人工验收和未验证风险。

## Validation Commands

```powershell
pnpm typecheck
pnpm format:check
pnpm test:unit
pnpm run build:renderer

cargo fmt --manifest-path .\src-tauri\Cargo.toml --check
cargo clippy --manifest-path .\src-tauri\Cargo.toml --locked --offline -- -D warnings
cargo test --manifest-path .\src-tauri\Cargo.toml --locked --offline
git diff --check
```

优先先运行 Codex capability、provider mutation、代理投影和相关组件的定向测试。不得运行会改写源码的 `pnpm format`，不得变更 lockfile，最终 Rust 验证保持 `--locked --offline`。

## Risk and Rollback Points

- 生图非法映射修复会主动丢弃无法安全保留的字段，只能由用户操作开关触发；测试必须证明普通保存不改写。
- 官方配置生成会改变 `model_provider`；私有 ownership 标记与严格形状检查必须共同防止删除用户表。
- mutation warning DTO 横跨 Rust、Tauri、TypeScript 和 toast；测试必须覆盖字段缺省的向后兼容。
- 本地代理没有 WebSocket Upgrade，允许配置可能导致运行时连接失败；产品边界仅为每次风险保存提示。
- 不触碰无关模块，不执行真实 API 调用、真实用户 Codex 配置写入、push、tag 或 release。

## Implementation Evidence

| Requirement | Code and test evidence |
| --- | --- |
| R1-R3 | `CodexFormFields.tsx` 与 `CodexNativeCapabilities.tsx` 无条件展示能力分组；`analyze_codex_provider_features` 仅将整份 TOML 解析失败标为不可编辑。组件测试覆盖官方、保留 ID、普通分类、xAI OAuth 与两类诊断。 |
| R4-R6 | `patch_codex_provider_features` 与受管补丁 helper 实现无格式门禁的 WebSocket、大小写归一生图头、非法字段显式修复和官方延迟骨架；Rust 单元测试覆盖安全清理及用户扩展保留。 |
| R7-R8 | `codex_provider_save_warning_codes`、provider mutation commands 与 `codexMutationWarningMessage` 实现最终 Provider 分类、代理风险及单条本地化 toast；Rust 模型矩阵与 hook 测试覆盖无风险、双风险、重复保存和失败。 |
| R9 | `apply_codex_official_proxy_route` 复制显式能力，普通投影继续只改路由字段；代理热切换测试验证官方、第三方与恢复链路。 |
| R10 | `ProviderMutationResult`、TypeScript API 类型、四语言 JSON、FyAgent 配置域规范及 v1.0.1 开发文档已同步；未新增依赖、lockfile 或数据库迁移。 |

## Validation Regression Hardening

全量 Rust 测试首次失败并非 Codex 能力回归，而是 `export_sql_returns_error_for_invalid_path` 把固定绝对路径 `/nonexistent/directory` 假定为不可写。root、提升权限的 Windows 账户或容器挂载可以创建该目录；测试随后在持有共享 fixture 锁时 panic，锁中毒又把首个失败放大为 22 个级联失败。

- 根因类别：隐式环境/权限假设；级联类别：共享锁中毒掩盖首个错误。
- 失败路径改为隔离测试根内的“普通文件充当目标父路径”，以目录形状而不是权限保证写入失败；同时移除两个会话扫描测试中的固定 `/nonexistent/path`。
- `RecoveringTestMutex` 在每个持锁测试都会立即重置 fixture 的既有前提下恢复 poisoned guard，并由专门的 panic 回归测试锁定行为。
- `.trellis/spec/backend/development-environment.md` 已记录规则：文件系统失败测试不得依赖固定绝对路径或权限位，必须同时适用于普通与提升权限环境。
- 仓库不存在 `src/templates/markdown/spec/` 或其他 Trellis spec 模板目录，因此没有可同步的生成模板；未为流程目的虚构目录。

## Validation Record

- 已通过：前端 Codex 定向测试 5 个文件 / 24 项。
- 已通过：Rust `codex_config::tests` 83 项，包含官方用户自建 `custom` 表所有权回归。
- 已通过：`codex_takeover_hot_switches_between_builtin_official_and_third_party` 代理热切换与恢复测试。
- 已通过：`import_export_sync` 27 项、共享锁 poison 恢复测试及两个隔离 missing-path 会话扫描测试。
- 已通过：`pnpm typecheck`、`pnpm format:check`、`pnpm test:unit`（99 个文件 / 642 项）与 `pnpm run build:renderer`。
- 已通过：最终树上的 Rust fmt、严格 Clippy（`-D warnings`）及 `cargo test --locked --offline`；全量库测试 2402 项并继续通过全部集成套件。
- 已通过：四语言 JSON 语法检查、`git diff --check`、Trellis context validate 与跨层 check 清单审查。
- 待 Windows 人工验收：真实桌面端控件、Chat/Anthropic 保存、每次风险 toast 及代理接管提示。
- 剩余产品风险：本地代理仍只实现 HTTP/SSE，不实现 WebSocket Upgrade；配置可保存不代表模型、上游或代理链路真实可用。
