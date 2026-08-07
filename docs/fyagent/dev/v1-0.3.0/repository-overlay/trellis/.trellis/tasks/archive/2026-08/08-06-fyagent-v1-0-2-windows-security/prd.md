# FyAgent v1.0.2 Windows 安全与发布边界

## 目标

在正式版全程管理员运行的已确认产品取舍下，建立可信安装、最小权限子进程、
单业务实例、受限 IPC 和可验证发布边界，而不把管理员权限扩散到用户应用。

## 范围

- WIN-001–012：正式/测试 manifest 分离、per-machine WiX、固定安全目录、旧
  per-user 阻断、无 Portable、Authenticode/时间戳发布门禁。
- WIN-013–020：Windows 禁用自启/清理旧项、默认托盘、早期单业务实例、交互
  用户普通权限启动和管理员 CLI 白名单。
- WIN-021–029：深链接限制/二次确认/脱敏、Q0–Q5 IPC 分类、能力令牌、参数
  复验、Capability/CSP 收敛和日志 redaction。

## 约束

- 不新增非提权启动器；用户拒绝 UAC 时正式 app 不启动。测试构建保持普通权限。
- 不在本机运行真实 UAC、安装器、签名、PackageManager、真实 Codex/ChatGPT
  或提升/降权子进程测试；这些只在受控候选环境人工验收。
- 不实施全用户 Codex、深链接直达管理员/破坏性动作、任意命令/路径/PID IPC，
  或因权限缺失静默回退到管理员启动普通应用。

## 验收标准

- [ ] 构建/静态测试能区分正式 `requireAdministrator` 与测试 `asInvoker`，保留
  Common Controls v6；发布 workflow 不再产出 Portable，签名失败阻止发布。
- [ ] WiX/安装路径/ACL/旧版本逻辑满足 WIN-006–010，且不能由用户可写目录、
  UNC、移动盘或重解析点逃逸。
- [ ] 单业务实例与激活转发早于用户数据/业务初始化；autolaunch/托盘行为符合
  WIN-013–016。
- [ ] 所有调用面分类、Capability/CSP 和令牌测试证明无未授权管理员/破坏性操作；
  普通外部启动不继承管理员权限。
- [ ] 本机只运行 fake/static/unit 验证；WIN-T 正式产物项目明确列为人工候选
  环境验收，绝不声称本机完成。
