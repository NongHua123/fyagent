# Windows 安全与发布技术设计

## 构建与安装

`build.rs` 根据 release/test 构建选择独立 manifest：正式 `requireAdministrator`
且 `uiAccess=false`，测试 `asInvoker`；两者启用 Common Controls v6。WiX 迁移为
per-machine，默认 Program Files；自定义路径在安装器内针对固定本地磁盘、安全
ACL、非用户可写、非 UNC/移动介质和无 reparse escape 做失败关闭校验。旧
per-user 安装只检测/阻断，不跨安全上下文自动迁移。Release workflow 删除
Portable，签名/时间戳/验证成为发布必要步骤。

## 运行时边界

命名互斥与受限 activation pipe 在 Tauri builder、日志目录、数据库、托盘和
WebView 之前运行。第二进程只转发解析后的激活消息后退出。提升主进程调用
`process_launch` 业务服务；Windows 实现从 Explorer/受控交互用户 token 启动
浏览器、文件管理器、终端、编辑器和 Codex 等普通应用，缺少普通 token 失败
关闭。CLI 安装/升级通过显式 elevated 白名单调用。

## IPC 与深链接

以 `invoke_handler` 为唯一清单，定义 Q0–Q5 分类表。管理员/破坏性动作生成
后端保存的短期一次性 capability token，绑定命令与规范化请求摘要；路径、URL、
枚举、大小和状态均在后端复验。Capability 仅许可主窗口实际调用的插件/API，
CSP 的来源和资产范围最小化。深链接先严格解析、限长/限字段/限编码层、脱敏
日志，随后显示确认；它只能产生普通草稿，不能携带能力令牌或执行特权操作。

## 回滚与测试

提升 manifest、per-machine 安装、降权启动、早期单实例和 Portable 删除必须成组
回滚。静态 manifest/WiX/workflow/handler 测试和 fake token/process tests 可在本机
运行；UAC、ACL、Authenticode、安装/卸载、真实降权和真实 Codex 均为受控候选
环境人工验证。
