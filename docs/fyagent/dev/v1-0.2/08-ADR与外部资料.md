# 08 — ADR 与外部资料

## 1. 文档目的

本文记录本次已确认的重要架构决策、被否决方案、后果和已接受风险，并列出用于事实核查的官方资料。ADR 只解释“为什么这样设计”，需求本身以 [01](./01-需求规格与决策基线.md) 为准。

## 2. ADR-001：Codex 歧义状态改为一次强制重启确认

**状态：已接受**

### 背景

当前只有唯一可信运行实例时才进入重启流程；多安装、多实例和身份绑定不唯一时只显示手动重启 Toast。需求方明确要求这些情况进入询问框并提供强制重启。

### 决策

- 精确应用身份成立时，非唯一状态均可进入一次强制重启确认；
- 不先正常退出，不进行第二次强制确认；
- 强制关闭全部精确匹配实例，只启动一次；
- 任何关闭失败均不启动。

### 被否决方案

1. 继续手动 Toast：不满足用户决策需求；
2. 随机或枚举第一个实例：平台顺序不是稳定业务规则；
3. 只重启一个旧实例：其他实例可能继续使用旧配置；
4. 两阶段正常退出/强制：需求方选择一次确认。

### 后果

- 平台层必须返回候选集合；
- 原 ambiguous 不操作测试需要重写；
- 强制关闭可能导致未保存工作丢失，必须明确提示；
- 用户 UI 保持简单，技术详情只进入诊断。

## 3. ADR-002：Codex 运行候选最低身份边界

**状态：已接受，含风险**

### 决策

- Windows：精确 Package Family Name；
- macOS：精确 Bundle Identifier；
- 不额外要求发布者/代码签名作为本次运行候选最低门槛；
- 不允许进程名、窗口标题、显示名模糊匹配。

### 已接受风险

macOS Bundle Identifier 由应用声明，单独使用时弱于 designated requirement 或签名身份。需求方选择便利性和可执行性优先。现有安装验证仍可保留更强签名校验，但运行时强制关闭资格按本决策执行。

### 硬停止

若连精确 PFN/Bundle ID 都无法确认，内部状态必须为 `untrusted_target`，不得执行强制终止。

## 4. ADR-003：多安装自动选择而非用户选择

**状态：已接受**

### 决策

固定比较器：

```text
当前运行 > 版本较新 > 系统级 > 用户级 > 稳定标识
```

用户对话框不展示安装选择器或内部目标。

### 原因

- 用户要求界面简单；
- 平台枚举顺序不稳定；
- 前端选择器会暴露路径、版本归属和竞态复杂性；
- 固定比较器可测试、可重复。

### 后果

候选集合变化时按最新集合重新计算并继续；安装目标的详细信息只在脱敏诊断中记录。

## 5. ADR-004：WorkBuddy 双根结构、保持原格式写回

**状态：已接受**

### 背景

项目旧实现只支持根数组，而当前公开示例使用对象根 `{ "models": [...] }`。

### 决策

- 读取数组和对象两种根；
- 原数组写回数组；原对象写回对象；
- 新文件使用对象根；
- 保留未知顶层字段、模型字段和顺序。

### 被否决方案

- 强制把旧数组迁移为对象：可能破坏旧 WorkBuddy 版本或用户工具；
- 继续只支持数组：拒绝当前公开格式；
- 每次重建标准对象：会丢未知字段。

## 6. ADR-005：已有模型采用覆盖确认，历史重复不整理

**状态：已接受，含兼容风险**

### 决策

- 只要目标 ID 已存在就汇总确认；
- 已有 ID 仍可选择，远程模型全部默认选中；
- 覆盖所有同 ID 条目，但不合并或删除；
- UI 静默去重，不显示重复数量；
- 覆盖只改 URL 和 API Key。

### 原因

用户需要知道哪些 ID 已存在，并允许用当前端点批量同步；但本次不扩展为完整模型编辑器，也不应擅自决定历史重复条目保留哪一项。

### 风险

公开资料说明不同配置层级中相同 ID 采用覆盖语义，但没有明确保证同一数组内部重复项的处理顺序。FyAgent 只保证自身不丢数据，不保证重复配置在 WorkBuddy 中的最终解释。

## 7. ADR-006：API Key 保留在 WorkBuddy 页面内存

**状态：已接受**

### 决策

保存、获取、错误和覆盖确认后不清空；离开页面或关闭窗口时清空；不持久化，不从文件回填。

### 被否决方案

- 保存后立即清空：连续配置体验差；
- 跨页面全局保留：扩大内存生命周期；
- 系统凭据库持久化：超出本次范围。

### 后果

JavaScript 无法保证字符串立即物理擦除，只能保证从应用可达状态中移除；文档不得宣称“安全内存清零”。

## 8. ADR-007：`availableModels` 保守增量维护

**状态：已接受**

### 决策

- 缺失保持缺失；
- 空数组保持空；
- 非空数组追加本次目标；
- 不删除、不重排、不自动创建。

### 原因

非空 `availableModels` 会限制模型选择器显示范围；完全不修改可能导致保存成功但模型不可见，全部重建又会破坏用户主动过滤。

## 9. ADR-008：顶部栏使用固定最右主操作槽

**状态：已接受**

### 决策

```text
品牌 → P1 → P2/More → AppSwitcher → 固定最右槽
```

Provider 槽内为 `+`；WorkBuddy 为同宽、不可交互、aria-hidden 空槽。

### 原因

- 保留用户对最右 `+` 的操作习惯；
- AppSwitcher 右边缘稳定；
- 不需要为全部上下文操作预留大块空白；
- 可以同时使用动态收纳。

### 被否决方案

- 将 `+` 放在 AppSwitcher 左侧：违背用户习惯；
- 只增加 z-index：不能解决裁剪；
- WorkBuddy 显示禁用 `+`：产生误导；
- 仅扩大窗口：无法覆盖语言、DPI、未来应用增长。

## 10. ADR-009：目标最小宽度由自动容量测量产生

**状态：已接受**

### 决策

- 正常工作区直接显示九个应用和 P0/P1；
- P2 可以收纳；
- 最大组合测试测量所需宽度，加安全余量并版本化提交；
- 工作区不足时进入受限模式，不将窗口强行挤出屏幕。

### 后果

产品最小宽度会高于当前 900，但不预先写死 1200/1280；最终值由稳定夹具产生。旧窗口状态必须受控恢复。

## 11. ADR-010：版本显示使用显式字段状态

**状态：已接受**

### 决策

本地和远程版本使用判别联合，区分首次加载、后台刷新、刷新失败有数据、首次失败、平台无版本和元数据异常。

### 原因

`undefined` 同时代表太多状态，导致“正在获取”错误显示为“暂不可用”。TanStack Query 已提供足够原始状态，无需自建请求状态机。

## 12. ADR-011：Windows 正式版主程序全程管理员

**状态：已接受，偏离最小权限常规建议**

### 决策

- 正式 EXE `requireAdministrator`；
- 无启动器；
- 原生 UAC；
- 用户拒绝则不启动；
- 只正式支持当前登录管理员账户。

### 原因

目标用户环境中，普通权限启动时 Codex 一键安装无法稳定工作；需求方要求双击即请求管理员权限。

### 事实边界

Windows `PackageManager.AddPackageByUriAsync` 官方描述为“为当前用户添加包”，并未说明 API 本身必然要求管理员。因此文档把全程提升描述为 FyAgent 的兼容性产品决策，而不是 Windows API 的硬性前提。

### 后果

必须同时实施：

- Authenticode；
- Program Files/per-machine；
- 普通外部应用降权；
- 早期单实例；
- IPC 收敛；
- Windows 自启禁用。

## 13. ADR-012：per-machine + 安全自定义安装目录

**状态：已接受**

### 决策

默认 Program Files，允许其他本地固定盘安全目录；拒绝用户可写、网络、移动盘和重解析点逃逸；测试阶段旧 per-user 版本要求手动卸载。

### 被否决方案

- 继续 LocalAppData：普通用户可替换未来以管理员运行的文件；
- 允许任意目录后只警告：不能形成安全边界；
- 自动迁移旧版：当前仍在测试阶段，不值得扩大安装器复杂度。

## 14. ADR-013：停止 Windows Portable

**状态：已接受**

正式 EXE 必须管理员运行，而 Portable 无法控制解压目录和 ACL。只保留正式安装包，避免用户从下载目录提权运行可被普通进程替换的二进制。

## 15. ADR-014：管理员主进程的外部应用默认降权

**状态：已接受**

### 决策

- 浏览器、终端、编辑器、文件管理器、WorkBuddy、Codex 桌面应用使用普通交互用户权限；
- 无普通用户令牌时失败，不以管理员兜底；
- CLI 安装/升级是明确的管理员白名单例外。

### 官方事实

`CreateProcessW` 默认让新进程使用调用进程安全上下文；Microsoft 提供从 Explorer 调用 ShellExecute 的示例，用于从提升进程启动非提升进程。

### 后果

项目中分散的 `Command::new` 需要逐步迁移为业务级启动服务，不能只改 WorkBuddy 下载按钮。

## 16. ADR-015：不增加启动器，允许瞬时激活进程

**状态：已接受**

### 决策

- 操作系统层面允许 UAC 后出现瞬时第二进程；
- 业务层面只有一个实例；
- 第二进程只转发激活/深链接并退出；
- 重复启动和深链接仍可能出现 UAC。

### 原因

用户不希望引入第二个长期发布的启动器 EXE。

### 后果

必须在 Tauri 业务初始化前增加命名互斥和安全激活通道；现有 single-instance 插件回调时机不足以单独满足要求。

## 17. ADR-016：Windows 禁用开机自启，但保留默认托盘

**状态：已接受，含持续提升风险**

- Windows 清理并禁用开机自启；
- 关闭窗口仍默认最小化到托盘；
- 设置说明进程会继续以管理员权限后台运行；
- 用户可关闭托盘行为。

这是一项明确产品取舍：减少同一登录会话中的重复 UAC，但增加管理员进程驻留时间。

## 18. ADR-017：保留深链接明文 API Key

**状态：已接受风险**

需求方选择保留现有兼容性。补偿措施：

- 长度/schema/动作限制；
- 应用内二次确认；
- 默认遮罩；
- 日志剥离查询参数；
- 不允许深链接直接调用管理员或破坏性动作。

仍无法消除浏览器历史、第三方生成器、聊天记录或启动参数链路中的泄漏风险。

## 19. ADR-018：保留局域网代理且不新增访问令牌

**状态：已接受风险**

需求方选择保持现有非回环监听能力和简单界面，不增加令牌生成、轮换、撤销和设备授权。局域网其他设备可能访问代理并消耗上游额度的风险继续存在。

本次至少要求：

- 保持现有绑定地址校验；
- 日志不泄露上游 Key；
- 管理员权限不成为自动放宽网络边界的理由；
- 安全文档明确记录，不将其表述为已认证代理。

## 20. ADR-019：IPC 全量分类整改

**状态：已接受**

主 WebView 所能调用的命令面在管理员进程中风险显著上升。本次不只审核新命令，而是以 `invoke_handler` 为清单，对全部命令分类、复验、最小化 Capability/CSP，并为管理员/破坏性命令使用一次性能力令牌。

## 21. ADR-020：桌面 E2E、关键区域视觉基线、手动 CI

**状态：已接受**

- 新增有限 WebdriverIO/Tauri 桌面 E2E；
- 几何与关键区域视觉回归阻塞；
- 平台基线独立；
- PNG 使用 Git LFS；
- 自动化开发代理用于本地语义检查和辅助修复，不进入普通 CI 依赖；
- CI 保持 `workflow_dispatch` 手动触发。

## 22. 已接受风险总表

| 风险 ID | 风险 | 补偿措施 | 用户界面是否展示技术细节 |
|---|---|---|---|
| R-01 | macOS 运行候选仅以 Bundle ID 为最低门槛 | 禁止模糊名称；无精确 ID 时硬停止 | 否 |
| R-02 | 强制关闭导致未保存工作丢失 | 一次明确风险确认；默认焦点手动处理 | 只展示风险，不展示技术细节 |
| R-03 | 历史重复模型继续存在 | 覆盖全部；不删除未知数据 | 否 |
| R-04 | API Key 在页面内存中保留 | 页面卸载清空；不持久化/缓存/日志 | 否 |
| R-05 | Windows 管理员进程默认驻留托盘 | 可关闭设置；禁用开机自启；IPC 收敛 | 简短说明 |
| R-06 | 深链接明文 API Key | 二次确认、遮罩、日志脱敏、长度限制 | 不展示安全术语 |
| R-07 | 局域网代理无访问令牌 | 保持现有校验和日志脱敏 | 否 |
| R-08 | CLI 安装/升级管理员运行 | 明确白名单；其他应用降权 | 否 |
| R-09 | 重复启动/深链接可能再次 UAC | 单一业务实例、托盘恢复 | 必要时帮助文档说明 |
| R-10 | 全程管理员偏离最小权限 | 签名、Program Files、降权启动、IPC 整改 | 设置页显示管理员状态 |

## 23. 外部资料索引

访问日期均为 **2026-08-06**。优先使用官方产品、框架和平台资料。

### 23.1 OpenAI 产品身份

1. **ChatGPT is now a partner for your most ambitious work**<br>
   https://openai.com/index/chatgpt-for-your-most-ambitious-work/<br>
   关键事实：2026-07-09；Codex app merging with new ChatGPT desktop app；原 Codex 更新后成为新 ChatGPT；原 ChatGPT 改名 ChatGPT Classic。

### 23.2 WorkBuddy 配置

2. **CloudBase — WorkBuddy**<br>
   https://docs.cloudbase.net/en/ai/ai-tools/workbuddy<br>
   关键事实：用户级 `models.json` 路径；2026-06-09 示例使用对象根 `models` 数组。

3. **models.json Configuration Guide**<br>
   https://www.workbuddy.ai/docs/ide/Features/models<br>
   关键事实：`models`、`availableModels`；SmartMerge 中相同 ID 覆盖、不同 ID 追加；过滤在合并后执行。

4. **WorkBuddy 官网**<br>
   https://www.workbuddy.cn/<br>
   用途：页面固定下载入口。

### 23.3 TanStack Query

5. **useQuery Reference**<br>
   https://tanstack.com/query/latest/docs/framework/react/reference/useQuery<br>
   关键事实：`isLoading`、`isLoadingError`、`isRefetching`、`isRefetchError`、`data` 等状态可用于区分首次加载和后台刷新。

### 23.4 Tauri

6. **Windows Installer**<br>
   https://v2.tauri.app/distribute/windows-installer/<br>
   关键事实：Windows 安装器、模板定制、per-user/per-machine 安装模式。

7. **Windows Code Signing**<br>
   https://v2.tauri.app/distribute/sign/windows/<br>
   用途：Tauri Windows 签名配置与发布参考。

8. **WebDriver**<br>
   https://v2.tauri.app/develop/tests/webdriver/<br>
   关键事实：Tauri 桌面应用可使用 WebdriverIO 服务进行 Windows、Linux、macOS E2E。

9. **Deep Linking**<br>
   https://v2.tauri.app/plugin/deep-linking/<br>
   用途：桌面深链接注册、激活和单实例组合参考。

10. **Capabilities**<br>
    https://v2.tauri.app/security/capabilities/<br>
    关键事实：Capability 约束指定窗口/WebView 的权限；多个 Capability 会合并权限边界。

11. **Window API**<br>
    https://v2.tauri.app/reference/javascript/api/namespacewindow/<br>
    关键事实：`workArea` 为排除任务栏/Dock 的物理像素区域，需结合 `scaleFactor` 转为逻辑像素。

### 23.5 Microsoft Windows

12. **Application manifests**<br>
    https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests<br>
    关键事实：`requireAdministrator` 的执行语义和凭据提示。

13. **PackageManager.AddPackageByUriAsync**<br>
    https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.addpackagebyuriasync?view=winrt-28000<br>
    关键事实：API 描述为为当前用户添加包；不支持把全程管理员决定表述为该 API 的官方硬性要求。

14. **Guidelines for Authoring Secure Installations**<br>
    https://learn.microsoft.com/en-us/windows/win32/msi/guidelines-for-authoring-secure-installations<br>
    用途：提升安装、源文件和目录权限安全原则。

15. **Execute In Explorer Sample**<br>
    https://learn.microsoft.com/en-us/windows/win32/shell/samples-execinexplorer<br>
    关键事实：从提升进程通过 Explorer 调用 ShellExecute，以启动非提升进程。

16. **CreateProcessW**<br>
    https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw<br>
    关键事实：新进程默认运行在调用进程安全上下文中。

17. **How User Account Control works**<br>
    https://learn.microsoft.com/en-us/windows/security/application-security/application-control/user-account-control/how-it-works<br>
    用途：UAC、管理员审批、凭据和已验证发布者行为。

18. **Windows code signing options**<br>
    https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options<br>
    用途：Windows 应用签名方案概览。

19. **Get-AuthenticodeSignature**<br>
    https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature<br>
    用途：发布后签名验证门禁。

### 23.6 视觉测试与基线

20. **WebdriverIO Visual Testing**<br>
    https://webdriver.io/docs/visual-testing/<br>
    关键事实：屏幕、元素和页面截图比较及基线目录配置。

21. **WebdriverIO Visual Testing Considerations**<br>
    https://webdriver.io/docs/visual-testing/considerations/<br>
    关键事实：应在相同平台环境内比较截图，不跨操作系统复用基线。

22. **Git Large File Storage**<br>
    https://git-lfs.com/<br>
    关键事实：以指针文件管理 Git 中的大型二进制内容，适合版本化 PNG 基线。

## 24. 资料使用规则

- 官方资料只支持其明确说明的事实；
- 未找到公开规范的行为，例如同一 WorkBuddy 数组内部重复 ID 的最终生效顺序，必须标记为未知；
- 源码行为优先以本次上传快照为准；
- 产品决策可以偏离通用最佳实践，但必须记录后果和补偿措施；
- 第三方页面后续变化时，应更新访问日期和证据摘要，不静默沿用过期结论。
