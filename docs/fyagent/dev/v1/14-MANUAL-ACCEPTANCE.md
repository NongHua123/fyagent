# 人工验收计划

## 1. 原则

- 真实安装只由人工在明确授权的测试设备执行；
- Agent 不自动卸载、清理或覆盖现有生产环境；
- 验收者自行准备未安装、旧版、最新版、冲突等状态；
- 每个平台必须保留证据和实际结果；
- 测试失败不通过手工绕过签名/系统策略来“完成”；
- Windows 所有用户是实验项，不阻断 V1。

## 2. 验收前置

### 通用

- FyAgent测试构建 SHA；
- 目标设备 OS版本/架构；
- 当前本地应用状态；
- 代理配置状态；
- 足够磁盘空间；
- 可访问 agentsmirror 的中国大陆常见网络；
- 日志目录可写；
- 确认设备可用于安装测试。

### 安全

- 备份重要 Codex项目/会话（目标应用通常保留，但验收不应依赖假设）；
- 不在唯一生产设备做破坏性冲突场景；
- Classic共存场景使用专门测试设备或可恢复快照；
- 验收脚本不自动执行卸载。

## 3. 验收记录模板

```markdown
# Platform acceptance record

- Date:
- Tester:
- FyAgent commit/build:
- OS edition/version/build:
- CPU architecture:
- Network location/provider:
- FyAgent global proxy:
- Local app before:
- Mirror release ID/version:
- Test case:
- Steps:
- Expected:
- Actual:
- Logs/error code:
- Pass/Fail/Blocked:
- Attachments:
```

## 4. 中国大陆运行时网络专项

在至少一个真实中国大陆网络环境执行：

1. 不开启开发 VPN；
2. 进入 Codex页面；
3. 确认 manifest/checksum可获取；
4. 确认安装包开始并完成下载；
5. 日志确认初始请求只有 agentsmirror；
6. 允许镜像重定向到 S3/R2，但日志不泄露 query；
7. 确认不请求 GitHub/OpenAI/Microsoft作为 fallback；
8. 切换 FyAgent全局代理后重复一次 metadata检查，确认代理语义生效；
9. 模拟镜像不可达，确认错误清晰且已安装应用仍可启动。

此专项只验证 FyAgent下载安装链路。不要把目标应用登录或模型服务连接结果作为该项失败依据。

## 5. Windows x64 阻断性矩阵

### W64-01 未安装 → 当前用户安装

前置：当前用户无 Stable `OpenAI.Codex`。

步骤：

1. 打开 Codex Provider页面；
2. 确认显示未安装和远程版本；
3. 点击一键安装；
4. 观察下载进度；
5. 等待安装和后验验证；
6. 确认成功 Toast；
7. 点击启动。

期望：

- 不出现安装范围选择/UAC；
- 安装官方 Stable；
- PackageManager查询身份正确；
- 启动系统中可能显示 ChatGPT；
- 临时包清理。

### W64-02 旧版 → 更新

- 准备低于镜像版本的 Stable；
- 目标应用关闭；
- 点击更新；
- 检查数据/登录不由 FyAgent主动删除；
- 版本变为 remote或更高；
- 可启动。

### W64-03 最新版 → 启动

- 相同版本；
- 主按钮为启动；
- 不重新下载；
- AUMID激活成功。

### W64-04 本地较新

- 若可构造；
- 显示本地较新；
- 不降级；
- 可启动。

### W64-05 下载取消

- 下载中取消；
- 迅速进入取消；
- `.part`删除；
- 可重新开始。

### W64-06 网络中断/重试

- 中途断网；
- 自动最多2次重试；
- 恢复网络后可成功或明确 terminal；
- 不断点续传是预期。

### W64-07 校验阻断

使用测试构建/fake源或受控 fixture触发 checksum错误，不篡改真实生产包：

- 严格阻断；
- 无忽略按钮；
- 未调用 PackageManager；
- 错误详情无敏感 URL。

### W64-08 应用运行中更新

- 打开目标应用；
- 尝试更新；
- 不杀进程；
- 返回 `WINDOWS_PACKAGE_IN_USE` 或平台允许无损更新的实际行为；
- 若平台允许更新，记录证据；规格禁止的是强制关闭，不强求必然失败。

### W64-09 设备策略阻断

在受策略测试设备：

- 显示 `WINDOWS_DEPLOYMENT_BLOCKED`；
- 无注册表/PowerShell绕过；
- 提示管理员；
- 不自动 Store fallback。

### W64-10 远程失败但本地启动

- 阻断镜像；
- 本地已安装；
- 显示无法检查更新；
- 启动仍工作。

### W64-11 退出保护

- 下载中退出提示取消；
- 安装中退出提示等待；
- 不强制杀部署。

## 6. Windows ARM64 阻断性矩阵

### WA-01 架构 source

- 确认请求 `/latest/win-arm64`；
- 不请求 x64；
- remote版本是 ARM64自己的 latest。

### WA-02 首次安装

同 W64-01，额外验证：

- MSIX ProcessorArchitecture ARM64；
- 系统实际安装包架构；
- 启动成功。

### WA-03 已是最新版

- 不下载；
- 启动。

### WA-04 ARM64暂不可用

若镜像条目 `catalog-only`/暂无包：

- `RELEASE_NOT_AVAILABLE`；
- 不回退 x64；
- 可稍后刷新。

### WA-05 更新

- 有旧版时更新；
- 版本比较使用 ARM64 MSIX四段版本；
- 人工记录实际性能/启动，但性能不是 V1功能门槛。

## 7. Windows 所有用户实验矩阵（非阻断）

仅内部 CLI：

### WALL-01 UAC取消

- 启动实验命令；
- 取消 UAC；
- 返回 `WINDOWS_UAC_CANCELLED`；
- 不自动当前用户安装。

### WALL-02 成功路径

- 验证下载/校验；
- elevated child重新校验；
- Stage；
- Provision；
- 查询结果；
- 记录是否受 license/策略限制。

### WALL-03 文件替换防护

受控开发测试：descriptor创建后替换文件，确保 elevated child拒绝。

### WALL-04 不支持

若官方包无法预配：

- 记录 `WINDOWS_ALL_USERS_UNSUPPORTED`/原始错误；
- 普通 UI不受影响；
- 不阻断 V1。

## 8. macOS Apple Silicon 阻断性矩阵

### M-01 未安装 → `/Applications`

- 两个标准目录无 Stable；
- `/Applications`可写；
- DMG原始 app名保持；
- Bundle ID/Team/codesign/spctl通过；
- 成功启动。

### M-02 权限不足 → `~/Applications`

使用无 `/Applications`写权限的测试账号：

- 自动回退用户目录；
- 不弹管理员密码；
- 不调用 osascript提权；
- 安装和启动成功。

### M-03 已安装 → 原路径更新

- Stable在任一标准路径；
- 更新保持现有路径，即使 DMG basename变化；
- 不删除用户数据；
- 后验验证成功。

### M-04 Classic路径冲突

- `/Applications/ChatGPT.app` 为 `com.openai.chat`；
- 新Stable DMG原名也为 `ChatGPT.app`；
- 不覆盖 Classic；
- 若 `~/Applications/ChatGPT.app`空闲，安装至用户目录；
- 两个应用可区分。

### M-05 两处冲突

- 两候选路径均为非Stable；
- `MAC_TARGET_PATH_CONFLICT`；
- 不删除、改名或覆盖。

### M-06 多个 Stable

- 两标准目录各一个 `com.openai.codex`；
- `MAC_MULTIPLE_INSTALLATIONS`；
- 不自动选择。

### M-07 应用运行中更新

- 启动 Stable；
- 更新阻断；
- 不 kill/quit；
- 关闭后重试成功。

### M-08 签名/Gatekeeper

通过受控 fixture/测试包验证失败路径，不篡改并安装真实目标：

- codesign失败阻断；
- Team mismatch阻断；
- spctl拒绝阻断；
- 无绕过按钮。

### M-09 原始名称

- 安装目标 basename与 DMG内完全一致；
- Bundle内部未修改；
- 签名复制后仍有效。

### M-10 启动

- 使用实际路径；
- 不依赖 `open -a ChatGPT`；
- Classic共存时启动的是 Stable。

### M-11 detach/cleanup

- 挂载卷成功推出；
- 临时文件清理；
- 若 detach warning，安装结果和 warning记录合理。

## 9. macOS Intel

- 打开 Codex页面；
- 显示“macOS Intel 暂不支持”；
- 不发 manifest/checksum/package请求；
- 按钮禁用；
- 不崩溃。

## 10. Linux

- Codex Provider页面不显示卡片；
- 无安装器网络请求；
- 其他 Codex Provider功能正常。

## 11. Codex CLI 回归

各平台：

- About显示 `Codex CLI`版本/来源；
- 无安装、更新、修复按钮；
- 批量命令不含 Codex；
- 直接调用旧 lifecycle IPC对 Codex写操作失败；
- 已有 CLI仍可在终端使用；
- Provider/OAuth/代理/MCP无回归。

## 12. FyAgent 品牌和更新回归

- 窗口/About显示 FyAgent；
- 无 ccswitch.io/上游 GitHub可见跳转；
- 无启动自动检查 FyAgent更新；
- 网络抓取无 CC Switch GitHub latest请求；
- 当前应用版本仍显示；
- LICENSE仍存在；
- identifier/data dir仍保持。

## 13. 签收规则

### Agent implementation complete

自动化全通过即可标记，人工平台为 pending。

### Platform accepted

单个平台全部阻断用例通过或有维护者明确接受的已知限制。

### V1 accepted

- Windows x64 accepted；
- Windows ARM64 accepted；
- macOS Apple Silicon accepted；
- 中国大陆运行时网络专项通过；
- 所有用户实验结果已记录但不要求成功。
