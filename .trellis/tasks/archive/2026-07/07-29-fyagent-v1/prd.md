# 完成 FyAgent V1 开发

## Goal

以 `docs/fyagent/dev/v1/` 的冻结基线为权威，在 CC Switch `3.18.0`
上完成 FyAgent V1：为内部团队提供一个安全、可诊断、仅使用大陆友好镜像的
“一键安装 Codex”能力，同时保持既有 Codex Provider、OAuth、代理、MCP、配置、
会话和用量功能可用。

FyAgent 所安装的是 OpenAI 官方原始的 ChatGPT 桌面应用（其中包含 Codex），
而不是 FyAgent 自行修改、重包、重签或维护的分发版本。

## Authority and Evidence

- 需求权威：`docs/fyagent/dev/v1/00-AGENT-START-HERE.md` 至
  `18-REFERENCES.md`；其中 P0、`02` 的最终决策与 `16` 的 Canonical ADR
  高于现有代码惯例。
- 当前基线：`main` / `2400031a85f6b45b4db7aec89394b997a88826a8`，任务
  创建前工作树干净，`package.json` 与 `src-tauri/Cargo.toml` 均为 3.18.0。
- 开发工具链：Node v22.12.0、pnpm 10.12.3、rustc 1.95.0、cargo 1.95.0。
- 当前镜像元数据于 2026-07-29 核验：`/latest/manifest` 为 schema v5；其原始
  字节 SHA-256 `937e5293…f98471df` 与 `/latest/checksums` 的
  `release-manifest.json` 条目一致。该 schema 含上游 URL、delta 与非目标平台
  字段，生产代码只能读取 V1 所需的受限字段，并从内置端点选择下载短链。
- 初步代码检索未发现既有 `codex_desktop`、`CodexDesktop`、`agentsmirror` 或
  `FyAgent` 实现；完整缺口与现有惯例将在各子任务的需求工件中持续核对。

## In Scope

### Installer Product Behavior

- 在 Codex Provider 页面顶部显示独立的“Codex 桌面应用”卡片：Linux 隐藏，
  macOS Intel 显示暂不支持。
- 检测 OpenAI Stable 身份：Windows `OpenAI.Codex`；macOS
  `com.openai.codex`，并排除 Beta、Classic 和不明应用。
- 使用固定 `agentsmirror` manifest、checksums 和平台短链取得每个架构自己的
  latest；镜像失败不覆盖本地状态，已安装应用仍可安全启动。
- 支持首次安装、旧版更新、同版/本地更高版本启动；下载有进度、取消及总计三次
  网络尝试；安装与安装后校验不可取消。
- 当本地版本低于可用 remote release 时，卡片只显示“更新 Codex”主行动，不提供
  次级“启动 Codex”按钮；用户仍可从系统自行启动已有版本。
- 实施严格的元数据、重定向、大小、SHA-256、身份、架构、签名/系统信任、OS、
  磁盘空间和安装后状态验证。没有绕过入口。

### Architecture and Platforms

- 实现 Rust 领域模型、稳定错误、单内存 Job 状态机、完整快照事件、服务编排和七个
  普通 Tauri IPC 命令；普通 `start_install` 只接收 `expected_release_id`。
- 实现 Windows x64/ARM64 当前用户 PackageManager 安装与 AUMID 启动；隐藏的
  all-users headless 预配保持实验性，不能经普通 IPC/UI 到达。
- 实现 macOS Apple Silicon 的标准目录检测、DMG 挂载、签名/Gatekeeper 校验、
  事务式复制替换、权限回退和按已验证实际路径启动。
- 通过 `cfg` 使 Linux 能编译后端并安全返回不支持；不得假装支持未实现平台。

### Product Integration

- 完成前端 API、Query、Hook、Card、简中和英文文案、事件恢复与可访问性测试；
  `App.tsx` 只作最小挂载。
- 使 Codex CLI 只读：保留版本、latest、路径/环境诊断，移除并在后端拒绝
  install/update/repair，批量操作也不得包含 Codex。
- 将用户可见品牌改为 FyAgent，移除可见 CC Switch/上游链接，关闭宿主上游自动
  更新的配置、插件、命令和 UI；保留许可证、内部 identifier、数据目录、deep-link、
  图标与兼容性。
- 清理 updater 在 Tauri capability、发布 workflow 和数据库版本过新恢复界面中的
  运行时耦合；数据库安全阻断仍须保留，但不得借该分支请求、安装或链接上游更新。
- 准备退出保护、结构化脱敏诊断、日志入口、静态审计和人工验收记录模板。

## Explicitly Out of Scope

- 修改、重包、重新签名、内置或缓存完整官方安装包；自定义 URL、官方/GitHub
  fallback、多源测速、用户选择版本、Beta、Intel Mac、Linux 桌面安装。
- 断点续传、跨重启 Job、数据库或 settings 持久化、下载管理器、修复/卸载/降级/
  rollback、Sparkle delta、安装向导、Provider 自动配置、`~/.codex` 修改。
- 新 FyAgent identifier、数据迁移、图标、公开代码签名/公证、后端授权、硬件 Key、
  管理员 helper、常驻服务或企业 MDM 支持。
- 自动化下载完整生产 MSIX/DMG、真实安装、卸载、覆盖 `/Applications`、触发 UAC、
  杀目标应用，或声称人工真机验收已经通过。

## Delivery Tree

| Child task | Owns | Depends on | Independently verifiable outcome |
| --- | --- | --- | --- |
| `07-29-fyagent-v1-core` | M0–M2、M5 的领域契约、source/downloader/validator、Job、service、普通 IPC | 基线和真实元数据 schema | Linux-safe Rust unit/service tests，冻结 Rust/TS wire contract |
| `07-29-fyagent-v1-windows` | M3 Windows x64/ARM64、当前用户适配与隐藏 all-users 实验 | Core DTO/platform trait | fixture/fake tests；Windows runner 编译，无普通 IPC scope |
| `07-29-fyagent-v1-macos` | M4 macOS Apple Silicon 的检测、DMG、事务式安装与启动 | Core DTO/platform trait | fake command/fixture tests；macOS runner 编译 |
| `07-29-fyagent-v1-ui` | M6 前端 DTO、API、Query、Hook、Card、i18n | Core IPC/event DTO | Vitest 覆盖状态矩阵与事件竞态 |
| `07-29-fyagent-v1-integration` | M5、M7–M11 共享注册、CLI 只读、品牌/updater、退出保护、全量审计和验收准备 | 前四项可合并提交 | 全量质量门槛、静态审计、人工验收包 |

父任务只维护源需求、依赖图、跨任务验收和最终集成审查；不在子任务未完成时把
部分平台能力宣称为 V1 完成。

## Acceptance Criteria

- [ ] **FR-001–FR-011:** 卡片、检测、镜像元数据、安装/更新/启动、进度/取消/重试、
  严格验证、后验检测、单 Job、镜像失败保留本地启动均有实现与对应自动化证据。
- [ ] **FR-012:** Codex CLI 的生命周期写操作从普通 UI、批量路径与直接后端 IPC
  同时移除/拒绝；其他 CLI 工具行为不回归。
- [ ] **FR-013:** FyAgent 宿主 updater 的运行时注册、endpoint、artifact、UI 和命令
  均已禁用/移除；目标 OpenAI 应用的更新机制不被修改。
- [ ] **FR-014–FR-015:** 错误码稳定、可复制且脱敏；日志目录可打开；不得泄露 token、
  cookie、预签名 query、完整 home 路径或对话内容。
- [ ] **FR-016:** all-users 只能通过受限 headless/CLI 测试路径触达，并再次验证输入；
  不阻断普通 V1，但不进入 UI 或普通 IPC。
- [ ] Windows x64、Windows ARM64、macOS Apple Silicon 都有实现与各自的
  fixture/mock/编译证据；Windows ARM64 还必须取得明确的目标编译或相应 runner
  证据，不能以 Windows x64 代替；macOS Intel 与 Linux 行为符合范围说明。
- [ ] 运行 `pnpm install --frozen-lockfile`、`pnpm typecheck`、
  `pnpm format:check`、`pnpm test:unit`、`cargo fmt --check --manifest-path
  src-tauri/Cargo.toml`、`cargo clippy --manifest-path src-tauri/Cargo.toml --
  -D warnings` 和 `cargo test --manifest-path src-tauri/Cargo.toml`，并记录每条
  命令的 OS、退出码与结果。
- [ ] 静态审计确认新安装器只有 agentsmirror 生产下载入口、无自定义 URL/
  all-users IPC/PowerShell 或 winget 主路径/`open -a ChatGPT`/按进程名判定；
  可见 CC Switch 链接和 updater 残留已按范围处理。
- [ ] `14-MANUAL-ACCEPTANCE.md` 的记录模板、待测矩阵和人工签收前置已准备；
  自动化结果只标记为 implementation complete，人工平台签收保持 pending，直到
  人工明确执行。

## Risks and Evidence Gates

- Windows Publisher allowlist 必须来自已签名官方包的匿名 manifest fixture 与系统
  信任验证；当前元数据只证明 Package Identity/PFN，不能替代 Publisher 证据。该
  fixture 未到位时不得放宽 allowlist 或宣称 Windows 安装验证完成。
- macOS Bundle ID `com.openai.codex`、Team ID `2DC432GLL2`、官方包结构和
  Windows mirror schema 都可能漂移；合并前重新核验，一旦变化 fail closed 并更新
  fixture/ADR，而非接受任意 OpenAI 字符串。
- 真正的 Windows/macOS 编译、签名与安装行为必须在相应 CI runner 和人工测试机
  验证；本机 Windows 不能替代 macOS 真机验收。
- 宿主 updater 当前还被数据库版本过新恢复界面和发布 workflow 使用。移除它时必须
  保留不修改数据库的安全阻断，并提供不访问上游更新渠道的本地/受控分发提示；不能
  仅删除 plugin/command 后留下死调用或无出口界面。
- UI、平台和共享注册文件的依赖顺序由 Core 契约控制。任一平台无法完成时不得用
  stub 宣称支持，必须保持 UI/后端的明确不支持状态并继续补齐 V1。

## Key Decisions

- **KD-001 — 旧版本可更新时不显示辅助启动。** 用户于 2026-07-29 确认：当本地
  Stable 版本低于可用 remote release 时，仅提供“更新 Codex”，不保留次级“启动
  Codex”按钮。这保持更新状态的单一明确行动，但不限制用户从操作系统自行启动已有
  应用。
- **KD-002 — 数据库版本过新保留安全阻断而不保留 updater。** 移除上游更新渠道后，
  现有恢复界面改为本地化、无网络的受控分发/支持提示，不修改数据库也不留下 dead
  command。
- **KD-003 — 动态身份和镜像事实 fail closed。** Windows Publisher fixture、macOS
  Bundle/Team、mirror schema 与签名常量均需在合并前重新取证；这类技术证据门槛不改变
  已冻结的产品范围或验收语义。

所有用户拥有的产品、范围、UX、兼容性和风险决策均已收敛。规划可进入最终审阅，但在
用户对最新摘要作出后续明确批准前，仍不得启动子任务、创建实现分支或修改产品代码。
