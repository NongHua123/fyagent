# V1 范围与非目标

## 1. 文档目的

本文件定义 FyAgent V1 的产品边界。Agent 在实现过程中不得因为“顺手优化”扩大范围；任何未列入 V1 的能力都进入 `15-FUTURE-ROADMAP.md`。

## 2. 产品目标

在 CC Switch `3.18.0` 架构基础上，将桌面产品的可见名称改为 **FyAgent**，并在 Codex Provider 页面顶部增加“一键安装 Codex”卡片，使内部团队用户可以：

1. 检测当前设备是否安装 Stable 渠道的官方桌面应用；
2. 通过中国大陆友好的 agentsmirror 获取当前平台、当前 CPU 架构自己的最新版元数据；
3. 下载 OpenAI 官方原始 MSIX 或 DMG；
4. 执行严格的哈希、身份、架构、签名/系统信任与安装后验证；
5. 首次安装、旧版本更新、同版本或本地更新版本启动；
6. 查看进度、取消下载、在可重试错误后重试；
7. 在镜像不可用时仍启动本地已安装应用。

## 3. 安装对象

用户界面名称：

```text
一键安装 Codex
Codex 桌面应用
```

实际安装对象：

```text
OpenAI 官方最新版 ChatGPT 桌面应用（包含 Codex）
```

当前系统外观可能变化，但 V1 管理的 Stable 身份固定为：

| 平台 | Stable 身份 | 必须排除 |
|---|---|---|
| Windows | `OpenAI.Codex` | `OpenAI.CodexBeta`、其他 OpenAI 包 |
| macOS | `com.openai.codex` | `com.openai.codex.beta`、`com.openai.chat`、未知 Bundle |

文件名、应用目录名、显示名、进程名和可执行文件名不得作为唯一身份依据。

## 4. 目标用户与发行范围

- 使用对象：内部团队；
- V1 不以公开发行标准为目标；
- 当前没有 Apple Developer ID、Notarization、Windows Authenticode 等 FyAgent 自身签名资源；
- 不处理商业授权、用户账号、许可证、硬件 Key 或后端鉴权；
- 不承诺企业 MDM、域环境、Store 许可或受控设备上的全部部署策略。

## 5. 平台范围

### 5.1 正式实现

- Windows x64；
- Windows ARM64；
- macOS Apple Silicon；
- Windows 普通 UI 只安装到当前用户；
- macOS 优先 `/Applications`，权限不足时回退 `~/Applications`。

### 5.2 实验实现

- Windows 所有用户预配：隐藏 CLI/headless 路径；
- 需要管理员权限；
- 必须重新校验包；
- 人工结果记录但不阻断 V1；
- 不出现在普通 UI、帮助、设置或用户操作路径中。

### 5.3 不支持

- macOS Intel：卡片显示“暂不支持”；
- Linux：隐藏卡片；
- Windows x64 包在 ARM64 缺包时的跨架构回退；
- Beta 渠道；
- ChatGPT Classic 管理。

## 6. 中国大陆友好运行时边界

### 6.1 必须满足

- 用户安装器运行时只访问内置 agentsmirror 最新版端点；
- 使用同一短链的中国大陆 S3 自动分流能力；
- 复用 FyAgent 现有全局 HTTP/SOCKS 代理；
- 不要求用户访问 GitHub Releases、OpenAI 官网或 Microsoft Store 网页；
- 远程检查失败不覆盖本地状态；
- 已安装用户仍可点击“启动 Codex”；
- 错误文案提示重试或检查 FyAgent 全局代理，而不是把境外站点作为唯一解决方案。

### 6.2 不在本项目控制范围

安装后的官方 ChatGPT 应用自身登录、模型访问、更新服务和网络可达性，不属于 FyAgent 安装器。FyAgent 不修改官方应用的网络配置、更新地址、证书、代理或服务协议。

### 6.3 开发阶段明确不受限

Agent 可自由使用公开互联网和官方开发资源；不要求开发机使用国内 npm/Cargo/GitHub 镜像；不把开发阶段网络优化混入产品代码。

## 7. V1 功能范围

### 7.1 本地检测

- Windows：精确查询 Package Identity `OpenAI.Codex`；
- macOS：只扫描 `/Applications` 和 `~/Applications` 的一级 `.app`，读取 Bundle ID；
- 返回本地平台版本、展示版本、实际路径/包信息、是否运行、是否存在歧义；
- 不扫描整个磁盘；
- 不按 `ChatGPT.exe`、`Codex.exe`、`ChatGPT.app`、`Codex.app` 名称推断身份。

### 7.2 远程检查

- 页面进入时获取元数据；
- 仅内存缓存五分钟；
- 用户刷新绕过缓存；
- 当前平台分支不可下载时明确显示不可用；
- 每个架构使用自己的 latest，不比较全局聚合标题。

### 7.3 安装与更新

- 未安装：一键安装；
- 本地版本较旧：更新；
- 相同版本：启动；
- 本地版本较新：启动，绝不降级；
- 下载期间显示进度和取消；
- 验证失败严格阻断；
- 安装阶段不可取消；
- 安装后重新检测，成功只显示 Toast。

### 7.4 启动

- Windows：通过已验证 manifest 的 Application ID 与 Package Family Name 构造 AUMID，由 Shell 激活；
- macOS：`open` 已验证的实际 Bundle 路径；
- 不直接运行 WindowsApps 中的 exe；
- 不使用 `open -a ChatGPT` 或进程名启动。

### 7.5 Codex CLI

保留：

- 本地版本；
- npm latest 或现有线上版本显示；
- 安装环境与多路径诊断；
- Provider、OAuth、代理、MCP、配置、会话和用量功能。

删除或禁用：

- 安装按钮；
- 更新按钮；
- 修复按钮；
- 批量安装/升级中的 Codex；
- 后端 `install/update` 对 `codex` 的执行能力。

## 8. V1 非目标

### 8.1 安装生命周期

- 卸载；
- 修复安装；
- 显式重新安装；
- 用户可选版本；
- 降级；
- 长期版本回滚；
- 历史包下载；
- 离线安装；
- 安装包缓存管理 UI。

### 8.2 下载器

- HTTP Range 断点续传；
- 应用重启后继续；
- 多线程分片；
- 下载速度/剩余时间预测；
- 多下载源测速；
- 官方源或 GitHub 回退；
- 用户自定义 URL；
- macOS Sparkle delta。

### 8.3 产品与发布

- FyAgent 自更新；
- 公开代码签名和 notarization；
- 国际化全语言翻译。

FyAgent Identifier、配置/数据库目录、deep-link、内部包名和应用图标属于 V1
clean break，不再是未来项。V1 不承诺对旧应用的并行安装、原位升级或数据兼容。

### 8.4 安全与企业能力

- TLS 证书固定；
- 多方哈希交叉验证；
- 自建透明日志；
- 企业 MDM/Store machine license 管理；
- 自动绕过组策略；
- 自动开启侧载；
- 长驻管理员服务；
- 后端授权、硬件 Key、设备证书或 Machine Key。

### 8.5 用户引导

- 多步新手向导；
- 安装完成弹窗；
- 自动登录；
- 自动创建/切换 Provider；
- 自动启用代理；
- 自动修改 `~/.codex`；
- 独立顶级“安装中心”。

## 9. 品牌范围

### V1 修改

- 当前产品、运行时、构建、安装、落盘和协议身份统一为
  `FyAgent` / `fyagent` / `fyagent_lib` / `com.fyagent.desktop` / `fyagent://`
  / `~/.fyagent` / `fyagent.db`；
- 不迁移、不读取、不接受旧数据目录、数据库、deep-link、自启动值或序列化标记；
- 移除未确立的旧网站与过时运营路由，但保留真实仓库、必要上游引用和合作方契约值；
- 禁用上游自动更新和更新 UI；
- 日志、发布产物与平台安装身份使用 FyAgent；
- 保留法律要求的 LICENSE、版权和第三方许可。

### V1 事实例外

- 真实 GitHub 仓库名与 URL `NongHua123/cc-switch`；
- 历史 changelog、release notes 与旧任务证据；
- LICENSE、版权、必要上游 issue/PR/源码引用；
- 外部合作方实际分配的 referral URL 参数、邀请码和优惠码；
- 证明生产路径已无旧身份的负向测试/审计模式。

## 10. 成功标准

V1 产品层面的成功必须同时满足：

1. 三个正式平台实现完成且 CI 通过；
2. 人工完成 Windows x64、Windows ARM64、macOS Apple Silicon 验收；
3. 普通用户无需访问境外下载页面即可完成安装；
4. 安装包保持官方字节与官方签名身份；
5. 镜像错误、包错配、哈希错误和身份错误均被严格阻断；
6. 不破坏 Codex Provider、OAuth、代理、MCP、配置和会话功能；
7. 不恢复 Codex CLI 安装能力；
8. 不把实验性所有用户路径暴露到 UI。
