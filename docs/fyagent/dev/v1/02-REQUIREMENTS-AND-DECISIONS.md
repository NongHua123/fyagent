# 需求与决策基线

## 1. 需求优先级

| 级别 | 含义 |
|---|---|
| P0 | 缺失即不能合并或不能宣称实现完成 |
| P1 | V1 必须有确定行为；局部体验可在不破坏语义的前提下调整 |
| EXP | 实验能力，不进入普通用户路径，不阻断 V1 |
| FUTURE | 明确延期 |

## 2. 功能需求

| ID | 级别 | 需求 | 验收摘要 |
|---|---|---|---|
| FR-001 | P0 | 在 Codex Provider 页顶部显示桌面安装卡片 | Linux 隐藏；Intel Mac 禁用说明 |
| FR-002 | P0 | 检测 Stable 官方桌面应用 | 精确 Package Identity/Bundle ID |
| FR-003 | P0 | 获取当前架构自己的最新版 | 只消费 agentsmirror manifest/checksums/short link |
| FR-004 | P0 | 首次安装 | Win 当前用户、mac 标准 Applications 路径 |
| FR-005 | P0 | 旧版本更新 | 不杀应用；运行中明确阻断 |
| FR-006 | P0 | 同版/本地更高版本启动 | 永不自动降级 |
| FR-007 | P0 | 下载进度、取消与最多两次自动重试 | 总尝试最多三次；安装阶段不可取消 |
| FR-008 | P0 | 严格下载与包验证 | 无绕过入口 |
| FR-009 | P0 | 安装后重新检测 | 版本和身份满足期望才成功 |
| FR-010 | P0 | 页面切换后任务继续 | 单内存任务；完整快照事件 |
| FR-011 | P0 | 镜像失败保留本地启动 | 远程错误不覆盖本地状态 |
| FR-012 | P0 | 删除 Codex CLI 生命周期操作 | 前后端同时禁止；只读展示保留 |
| FR-013 | P0 | 禁用 FyAgent 上游自动更新 | 移除 updater UI、端点和运行时注册 |
| FR-014 | P1 | 结构化错误与复制详情 | 脱敏，不含 token、完整 URL query 或会话内容 |
| FR-015 | P1 | 打开日志目录 | 复用现有本地日志，不打包上传 |
| FR-016 | EXP | Windows 所有用户预配 | 隐藏 CLI/headless + UAC；不在 IPC/UI |

## 3. 非功能需求

### NFR-001 — 架构一致性

新增代码遵循 CC Switch 的“领域模块 + service 编排 + 薄 commands”结构；前端遵循 API + Query + Hook + Component。不得把安装器做成单个巨型文件。

### NFR-002 — 最小侵入

不得借此重构 ProviderService、ProxyService、数据库、MCP、Skill、Prompt、Usage 或其他无关核心。

### NFR-003 — 运行时中国大陆友好

用户安装链路不依赖境外网站；只使用内置 agentsmirror。开发阶段网络不受限制。

### NFR-004 — 官方包不变性

不得修改、重包、重签官方应用。任何会改变签名内容的动作均为缺陷。

### NFR-005 — 可维护性

平台细节通过 adapter/trait 边界隔离；解析器、命令执行器、部署器可替换为测试 double；共享状态由 service 持有，不散落在 React。

### NFR-006 — 并发安全

全局同时最多一个安装任务。事件载荷是完整快照；任务 ID 防止旧事件覆盖新任务。

### NFR-007 — 可诊断性

每个失败映射稳定错误码，原始平台码放入诊断；日志中的下载 URL 去除 userinfo、query 和 fragment。

### NFR-008 — 跨平台编译

Windows/macOS 模块必须使用 `cfg` 隔离；Linux CI 仍可编译和运行通用测试。

### NFR-009 — 人工验收边界

Agent 只完成自动化单元、fixture、mock 和编译验证；真实安装由人工完成。

## 4. 决策登记 D01–D96

以下是访谈后的最终解释。若编号在原访谈中被后续决策覆盖，以本表为准。

| ID | 最终决策 |
|---|---|
| D01 | 用户称“一键安装 Codex”；实际安装最新官方 ChatGPT 桌面应用（包含 Codex） |
| D02 | 移除 Codex CLI 安装体系；后续由 D85 精确为只读展示保留 |
| D03 | V1 完全纯桌面，无后端、硬件 Key 或联网授权 |
| D04 | 参考 VibeKey 行为，从头按 CC Switch 架构重写，不复制旧生产代码 |
| D05 | V1 可见产品名改为 FyAgent |
| D06 | 内部团队使用 |
| D07 | Win x64、Win ARM64、mac ARM64 正式实现；Intel 延后 |
| D08 | 最低系统要求跟随 FyAgent 与目标包实际要求 |
| D09 | 检测、首次安装、更新、同版启动 |
| D10 | 被 D29 覆盖；不开发引导流程 |
| D11 | 自动选择平台短链；运行时从大陆友好角度设计 |
| D12 | agentsmirror 为 V1 唯一镜像，不计划自建 |
| D13 | 验证失败严格阻断，无绕过 |
| D14 | 不允许用户自定义下载 URL |
| D15 | 禁用 FyAgent 自更新，后续再设计 |
| D16 | 暂无产品签名资源，V1 不按公开发行标准验收 |
| D17 | 本地日志与用户主动诊断 |
| D18 | 最终只交付 Markdown |
| D19 | 文档基于上传的 CC Switch 3.18.0；开发记录实际基线 SHA |
| D20 | 实施者可使用 worktree 并行；允许联网与安装开发环境 |
| D21 | 最小侵入，不顺带重构核心 |
| D22 | 多文档，但不拆独立任务卡文件 |
| D23 | 删除 CLI 生命周期入口，保留 Provider/OAuth/代理/MCP/配置/会话等 |
| D24 | FyAgent 采用 clean break：identifier、deep-link、数据目录/数据库、自启动、序列化标记和内部包名全部切换，不迁移或兼容读取旧身份 |
| D26 | 安装卡片位于 Codex Provider 页顶部 |
| D27 | 进入页面检查本地与远程；其他页面不后台检查 |
| D28 | 安装/更新/启动/取消/重试按状态映射；不降级 |
| D29 | 安装成功只 Toast，不弹引导框 |
| D30 | V1 唯一 source provider 为 agentsmirror |
| D31 | 实现最小但强制的多层校验 |
| D32 | 有进度、取消、两次自动重试；无断点续传 |
| D33 | 复用现有全局代理设置 |
| D34 | 单内存任务；页面离开继续；重启不恢复；无 DB |
| D35 | 下载时退出提示取消；安装时提示等待，不强制终止 |
| D36 | 日志、复制详情、打开日志目录；无诊断 ZIP |
| D37 | macOS Intel V1 完全延后 |
| D38 | Agent 不做真实 E2E，人工验收 |
| D39 | `main` 基线、`feature/fyagent-v1` 集成分支、记录工具链 |
| D40 | Core/Windows/macOS/UI worktree，原子 commit，集成 cherry-pick |
| D41 | 由 D24 取代：V1 更换 FyAgent 图标并重命名 Rust/npm/bin 身份 |
| D42 | Stable only；排除 Beta 和 Classic |
| D43 | 使用 manifest、checksums、平台短链 |
| D44 | 初始 URL 固定 HTTPS；最多五次 HTTPS 重定向；最终严格校验 |
| D45 | Windows 使用 WinRT PackageManager |
| D46 | 原需求允许当前/所有用户；被 D61、D63、D83 收敛为 UI 当前用户、隐藏实验 all-users |
| D47 | 不自动或强制关闭官方应用 |
| D48 | 不绕过 Windows 设备策略 |
| D49 | macOS 优先 `/Applications`，权限不足回退 `~/Applications` |
| D50 | 保持 DMG 内原始 `.app` 名称 |
| D51 | macOS 使用 `hdiutil/plutil/codesign/spctl/ditto/open` |
| D52 | Bundle ID + 实际路径检测运行与多安装；歧义阻断 |
| D53 | Windows/macOS 使用各自平台版本模型 |
| D54 | 后端状态机与明确取消边界 |
| D55 | 系统临时目录；成功/取消/过期清理；不进 DB |
| D56 | Linux 隐藏；Intel Mac 显示暂不支持 |
| D57 | 远程失败时本地安装状态与启动仍可用 |
| D58 | 新功能完整维护简中和英文；其他语言回退英文 |
| D59 | 复用现有 CI，不新增真实安装工作流 |
| D60 | 代码可先合并；三个正式平台人工验收后才签收 V1 |
| D61 | 当前用户正式；所有用户实验，不阻断 V1 |
| D62 | 同一 FyAgent 可执行文件的受限 headless 提权子进程 |
| D63 | 安装范围 UI 不显示 |
| D64 | all-users 是设备预配语义，不修改其他用户数据 |
| D65 | 原名安装；路径被非目标占用时先尝试用户目录，再阻断 |
| D66 | UI 解释系统中可能显示为 ChatGPT |
| D67 | 完整任务状态机在 Rust |
| D68 | 分离式 Tauri IPC |
| D69 | 完整快照事件 + 主动查询 |
| D70 | 已验证远程 descriptor 内存缓存五分钟 |
| D71 | 总请求最多三次，只重试临时网络/服务错误 |
| D72 | Windows 依赖缺失明确报错，不自动抓取未知依赖 |
| D73 | 每个平台架构独立 latest；ARM64 不回退 x64 |
| D74 | 下载前检查 OS、CPU、包最低系统要求 |
| D75 | Windows AUMID；macOS 已验证实际路径启动 |
| D76 | 稳定错误码全集 |
| D77 | 复制结构化、脱敏诊断 |
| D78 | all-users 人工结果记录但不阻断 |
| D79 | Agent/验收脚本不自动卸载用户应用 |
| D80 | 延后能力进入未来路线图 |
| D81 | ZIP 根目录名为 `v1` |
| D82 | 文档给结构/接口/约束草案，不复制整套生产实现 |
| D83 | 普通 UI 固定当前用户；all-users 仅内部隐藏实验入口 |
| D84 | 不修改或禁用官方应用自身更新机制 |
| D85 | Codex CLI 版本与分布只读展示；无安装/更新/修复 |
| D86 | 最小依赖；可新增 `windows`、`quick-xml`；前端无新生产依赖 |
| D87 | 后端目录遵循 CC Switch 领域 + service + commands 结构 |
| D88 | 前端目录遵循 API + Query + Hook + Component，App 只挂载 |
| D89 | worktree 文件所有权固定，集成 Agent 注册共享文件 |
| D90 | 采用确认的中英文卡片和 Toast 文案 |
| D91 | 规范化字段计算 `release_id`，开始安装前重新验证 |
| D92 | Windows 精确包查询；macOS 只扫两个标准目录 |
| D93 | 每个相关卷的可用空间至少为安装包预期大小 3 倍 |
| D94 | 运行完整现有前后端质量门槛，无真实安装 |
| D95 | 采用完整三平台人工验收矩阵 |
| D96 | 交付 `v1.zip/v1/*.md`，含集中 References 文档 |

## 5. 后续澄清形成的横切决策

### CR-001 — 镜像不是独立发行版

agentsmirror 只承担传输和元数据；目标包的版本、身份、发布者与签名均来自 OpenAI/Microsoft 官方包。FyAgent 不维护另一个 Codex 版本。

### CR-002 — 网络约束的适用对象

中国大陆友好只约束最终产品用户运行时。开发环境、Agent 搜索和依赖安装不受限制。

### CR-003 — macOS 原名与身份分离

新安装保留 DMG 中 `.app` 原名；更新已存在 Stable 时更新其现有路径。无论目录叫什么，只有 `com.openai.codex` + 允许 Team ID 才可管理。

### CR-004 — Windows 所有用户不出现在 UI

普通 Tauri IPC 的 `start_install` 不接受 `InstallScope`。实验入口只存在于命令行/headless 分发，防止前端误用。

## 6. 需求可追踪性

| 需求域 | 设计文档 | 自动化 | 人工验收 |
|---|---|---|---|
| 架构与状态 | 04、05 | 13 | 14 通用检查 |
| 下载与镜像 | 06 | 13 source/downloader | 14 大陆网络、失败恢复 |
| Windows | 07 | 13 Windows fixture/mock | 14 Windows x64/ARM64 |
| macOS | 08 | 13 mac fixture/mock | 14 mac ARM64 |
| UI/i18n | 09 | 13 前端测试 | 14 操作与文案 |
| 错误诊断 | 10 | 13 映射/脱敏测试 | 14 失败场景 |
| 仓库改动 | 11 | CI | 集成审查 |
| 实施顺序 | 12 | 每阶段门槛 | 不适用 |
