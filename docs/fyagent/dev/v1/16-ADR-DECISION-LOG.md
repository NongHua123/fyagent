---
status: final
version: v1
note: interview identifiers D61-D65 were reused; canonical ADRs remove ambiguity
---

# ADR 与访谈决策记录

## 1. 说明

访谈过程中 `D61`–`D65` 曾被复用于两个主题。本文用 Canonical ADR 作为最终权威，并在最后提供 legacy D 映射。后续 Agent 不应只引用含歧义的 D 编号。

## 2. Canonical ADR

### ADR-001：安装对象是官方 ChatGPT 桌面应用（Accepted）

用户功能名为“一键安装 Codex”，实际安装 OpenAI 最新官方桌面应用，其中包含 Codex。系统可显示 ChatGPT。

### ADR-002：FyAgent 不是目标应用分叉（Accepted）

只下载官方原始 MSIX/DMG，不修改、不重打包、不重新签名、不维护独立版本。

### ADR-003：只用 agentsmirror 作为 V1 运行时源（Accepted）

固定大陆友好短链，不做 fallback、多源、测速、自建或用户 URL。

### ADR-004：中国大陆友好只约束最终用户运行时（Accepted）

开发 Agent/CI 可自由访问互联网和依赖仓库。

### ADR-005：纯桌面端（Accepted）

无后端、硬件密钥、设备证书、Machine Key、联网授权。

### ADR-006：重写而非搬运 VibeKey（Accepted）

旧代码只作为行为与测试参考，按 CC Switch 架构重写。

### ADR-007：最小侵入 CC Switch（Accepted）

不重构核心 Provider/Proxy/DB/同步等模块，不新增安装 DB 表。

### ADR-008：V1 内部团队发行（Accepted）

不按公开签名/公证标准阻断。

### ADR-009：可见品牌 FyAgent，内部身份暂保留（Accepted）

改 product/窗口/链接，保留 identifier、数据目录、deep link、内部包名和图标。

### ADR-010：禁用 FyAgent 自更新（Accepted）

移除上游 updater config/UI；目标官方应用自身更新不受影响。

### ADR-011：平台范围（Accepted）

Windows x64、Windows ARM64、macOS Apple Silicon正式；macOS Intel延后；Linux不支持。

### ADR-012：Codex 页面卡片（Accepted）

不新增顶级页面；进入页面时检查；成功仅 Toast。

### ADR-013：Rust 后端状态机（Accepted）

后端拥有 Job，前端只显示快照；事件 + 主动查询恢复。

### ADR-014：单内存 Job（Accepted）

页面切换继续、重启不恢复、不写 DB。

### ADR-015：下载能力最小集（Accepted）

进度、取消、初始+2重试；无断点续传、分片、ETA、delta。

### ADR-016：严格验证（Accepted）

SHA、身份、架构、发布者/Team、系统签名、后置检测，失败不可绕过。

### ADR-017：磁盘门槛三倍包大小（Accepted）

对相关卷检查 `expected_size × 3`。

### ADR-018：各架构独立 latest（Accepted）

ARM64不等 x64，不回退下载其他架构。

### ADR-019：Windows WinRT current-user为正式路径（Accepted）

使用 PackageManager，普通 UI不显示范围、不触发 UAC。

### ADR-020：Windows all-users隐藏实验（Accepted）

同一官方 MSIX；复用 FyAgent受限 headless + runas；非阻断。

### ADR-021：不自动关闭目标应用或绕过策略（Accepted）

Windows/macOS更新占用时提示重试。

### ADR-022：macOS保持 DMG原始 app名（Accepted）

Bundle ID驱动身份；Classic/未知路径冲突不覆盖；系统目录优先、用户目录回退。

### ADR-023：macOS系统工具安装（Accepted）

hdiutil、codesign、spctl、ditto、open；无管理员 Helper。

### ADR-024：Codex CLI只读（Accepted）

保留版本/latest/分布/诊断，删除安装/更新/修复并在后端拒绝。

### ADR-025：中英文完整，其他回退英文（Accepted）

不扩大翻译工作。

### ADR-026：现有 CI + mock测试（Accepted）

不新增 workflow，不执行真实安装。

### ADR-027：人工验收最终签收（Accepted）

代码可先合并，三个正式平台和大陆运行时链路需人工通过。

### ADR-028：Codex worktree并行（Accepted）

Core先冻结契约，Windows/macOS/UI并行，集成 Agent cherry-pick。

### ADR-029：文档多 Markdown，无独立任务卡（Accepted）

最终 `v1.zip/v1/`，含复制提示词和参考资料。

### ADR-030：结构遵循 CC Switch而非机械预设（Accepted）

Commands/Services/Domain/Platform、API/Query/Hook/Component 分层，允许按实际代码量合理合并。

## 3. Legacy D01–D60 映射

| D | 最终结果 |
|---|---|
| D01 | 用户称“一键安装 Codex”，实际官方最新桌面应用 |
| D02 | 原 Codex CLI安装能力删除；后由 D85细化为只读展示 |
| D03 | 无后端/硬件密钥 |
| D04 | 参考旧代码重写 |
| D05 | 显示名称 FyAgent |
| D06 | 内部团队使用 |
| D07 | Win x64/ARM64、Mac ARM64；Intel延后 |
| D08 | OS要求跟随宿主和目标包实际要求 |
| D09 | 检测、安装、更新、启动 |
| D10 | 早期引导提议，最终被 D29覆盖为 Toast |
| D11 | 中国大陆友好；最终单 agentsmirror |
| D12 | 接受 agentsmirror，不自建 |
| D13 | 校验失败严格阻断 |
| D14 | 不允许自定义 URL |
| D15 | 禁用 FyAgent自动更新 |
| D16 | 无签名资源，V1不考虑公开发行标准 |
| D17 | 本地日志/诊断 |
| D18 | 只要 Markdown |
| D19 | 上传 CC Switch 3.18.0为稳定基线 |
| D20 | Codex worktree并行，允许联网与安装环境 |
| D21 | 最小侵入 |
| D22 | 多文档，不拆大量任务卡 |
| D23 | 删除 CLI生命周期写操作，保留核心能力 |
| D24 | V1不要求与 CC Switch共存，身份迁移以后 |
| D25 | 移除可见 CC Switch品牌 |
| D26 | Codex页顶部卡片 |
| D27 | 进入页面检查 |
| D28 | 安装/更新/启动状态映射 |
| D29 | 成功 Toast only |
| D30 | agentsmirror必须覆盖 Win/Mac；已核验 |
| D31 | 最小强制验证集合 |
| D32 | 进度/取消/2重试，其他延后 |
| D33 | 复用全局代理客户端 |
| D34 | 单内存任务、页面切换继续 |
| D35 | 退出时下载提示取消、安装提示等待 |
| D36 | 日志、复制详情、打开日志目录 |
| D37 | macOS Intel完全延后 |
| D38 | Agent不做真实 E2E，人工验收 |
| D39 | main基线、feature/fyagent-v1 |
| D40 | Core/Win/Mac/UI worktrees，cherry-pick |
| D41 | 表层品牌，保留图标/内部身份 |
| D42 | Stable only |
| D43 | manifest + checksums + endpoint |
| D44 | 5次 HTTPS重定向 |
| D45 | WinRT PackageManager |
| D46 | 早期选择当前/所有用户，最终被 D63/D83覆盖 |
| D47 | 不结束目标应用 |
| D48 | 不绕过策略 |
| D49 | mac系统目录优先、用户目录回退 |
| D50 | 保持 DMG原名 |
| D51 | mac系统工具 |
| D52 | Bundle ID/实际路径/多安装 |
| D53 | 平台独立版本模型 |
| D54 | 状态机与取消边界 |
| D55 | 临时目录与24h清理 |
| D56 | Linux隐藏、Intel显示不支持 |
| D57 | remote失败但local可启动 |
| D58 | zh/en完整 |
| D59 | 复用现有 CI |
| D60 | 人工验收阻断最终签收但不阻断合并 |

## 4. 重用编号 D61a–D65a：分发纠偏

| ID | 最终结果 |
|---|---|
| D61a | FyAgent是官方应用镜像辅助安装器，不是分叉 |
| D62a | 镜像只承担传输与元数据 |
| D63a | Windows current/all-users使用同一官方 MSIX |
| D64a | macOS保持官方 app 名与内容 |
| D65a | 按 OS身份与签名识别，不按显示名 |

## 5. 后续 D61b–D96 映射

| D | 最终结果 |
|---|---|
| D61b | current-user正式，all-users实验 |
| D62b | 同一 FyAgent受限 headless提权，不增加服务/helper |
| D63b | 安装范围 UI不显示 |
| D64b | all-users是设备预配语义 |
| D65b | mac路径冲突先 user目录再阻断 |
| D66 | UI解释系统可能显示 ChatGPT |
| D67 | Rust后端完整状态机 |
| D68 | 分离式 IPC commands |
| D69 | 完整快照事件 + 查询 |
| D70 | 5分钟内存缓存 |
| D71 | 总3次网络尝试 |
| D72 | Windows缺依赖不从未知源下载 |
| D73 | 各架构消费自己 latest |
| D74 | 下载前系统/架构/最低版本检查 |
| D75 | Windows AUMID，mac实际路径启动 |
| D76 | 稳定错误码 |
| D77 | 结构化脱敏复制详情 |
| D78 | all-users人工结果非阻断 |
| D79 | Agent/脚本不自动卸载清理 |
| D80 | 接受后续路线图 |
| D81 | ZIP根目录 `v1/` |
| D82 | 文档精确到接口/文件/测试，但不复制全套生产代码 |
| D83 | UI固定current-user；all-users隐藏CLI实验 |
| D84 | 不禁用目标官方应用自更新 |
| D85 | Codex CLI只读展示 |
| D86 | 最小依赖，可新增 windows/quick-xml，前端无新依赖 |
| D87 | 后端结构按 CC Switch优秀工程实践规划 |
| D88 | 前端结构按现有分层规划 |
| D89 | worktree文件所有权固定 |
| D90 | 接受最终中英文卡片文案 |
| D91 | canonical release ID防漂移 |
| D92 | Windows身份查询；mac标准目录扫描 |
| D93 | 三倍安装包大小磁盘门槛 |
| D94 | 完整现有质量门槛 |
| D95 | 完整人工矩阵 |
| D96 | `v1.zip`含19份 Markdown与参考资料 |

## 6. 被明确拒绝的主要方案

- 继续旧 VibeKey后端/硬件授权；
- 整体复制旧安装器；
- 用户自定义 URL；
- 多源测速/官方 fallback；
- 校验失败允许继续；
- Windows PowerShell/winget主路径；
- mac重命名为 Codex.app；
- 自动强杀应用；
- V1 mac Intel；
- 真实 E2E由 Agent自动执行；
- 大范围核心重构；
- 文档只做传统 PRD；
- 开发阶段也强制中国大陆镜像。
