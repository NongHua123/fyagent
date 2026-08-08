# FyAgent v1.0.2 父任务技术设计

## 架构边界

本任务以六个可独立验收的子任务交付。父任务不承载产品代码，负责来源优先级、
跨层契约、依赖顺序和最终集成审查。各子任务不得以“共享需求”为理由绕过自身
的 PRD、设计、验证和回滚记录。

```text
历史文档迁移 ──────────────┐
WorkBuddy 文档模型/IPC ─ UI ├─> Desktop 验收与证据
Codex 版本状态 ────────────┤
顶部栏/窗口状态 ─ Windows ─┤
Codex 重启协议 ─────────────┘
```

Windows 提权与发布收敛只在 WorkBuddy、顶部栏和 Codex 运行时的普通权限
fake/单测稳定后推进。E2E/视觉任务最后集成各个已冻结接口，不得用截图取代
核心 Rust/React 行为测试。

## 文档基线与迁移

- `docs/fyagent/dev/v1-0.2/` 是当前任务的权威输入；`v1-0.0` 是最初版
  历史设计；`v1-0.1` 是历史配置域输入，仅在 v1.0.2 未覆盖的契约中参考。
- 文档迁移使用 Git 感知重命名，保留 19 个 Markdown 的正文与相对链接。
  只更新仍面向未来开发的规范和当前任务工件；归档任务文本和 Git 对象保留。
- 禁止全局替换 `v1`：深链接协议、API 路径、版本号、历史分支、归档 zip 和
  release 标签不属于目录迁移。

## 跨层契约

### WorkBuddy

Rust 固定当前用户 WorkBuddy 路径和安全事务边界；前端只持有页面生命周期内的
URL、API Key、选择状态和冻结保存请求。状态、已有 ID 和保存结果使用独立 DTO，
禁止将完整模型对象、路径、Key 或原始 JSON 返回 renderer。覆盖 token 在后端
保存完整规范化请求的不可逆摘要，消费时重新检查 revision；首次冲突不写入。

### Codex

后端拥有应用身份、候选集合、安装比较、能力令牌和实际流程判定；前端只消费
脱敏状态并回传 opaque token。令牌绑定精确身份和命令，不绑定允许在确认后
重新计算的瞬时安装选择。无精确身份、令牌失效或任何关闭失败均失败关闭。
版本呈现通过 TypeScript 判别联合从 Query 原始状态派生，不能再依赖 nullable
版本字段推断。

### 顶部栏与窗口

从 `App.tsx` 拆出顶部栏组合层、固定 trailing slot 和基于 ResizeObserver 的
容量逻辑。P0 由 DOM 结构而非 z-index/overflow 偶然可见性保证。窗口服务使用
版本化布局常量、工作区逻辑像素、保存状态钳制与受限模式事件，不将动态测量值
直接变成每次启动的产品最小宽度。

### Windows 安全边界

正式 manifest、WiX、发布 workflow 与 Rust 运行服务是同一个权限契约：主进程
提升不意味着子进程继承提升。所有外部启动必须经过业务级普通用户启动服务或
显式管理员白名单；IPC 按 Q0–Q5 分类，Capability/CSP 仅列出实际需要项。
单业务实例必须早于 WebView、数据库、托盘和用户数据初始化。

## 兼容与安全

- WorkBuddy 原根形状、未知字段、顺序与历史重复数据保持；不把兼容问题变成
  自动格式迁移。
- API Key、capability token、完整 URL、PID、私有全路径和原始配置不得进入
  renderer state、普通日志、错误 DTO、fixture 或截图。
- 正常保存未检测到运行 Codex 时不启动应用；只在已显示确认后的重新枚举路径中
  允许“实例自行退出后仍启动一次”。
- 真实 Codex/ChatGPT 安装、配置、进程、UAC、签名和安装器均排除本机测试。

## 外部环境门禁与回滚

- Windows PFN 已有只读元数据证据；macOS Bundle ID 及真实运行行为须在受控
  候选环境确认。若未确认，macOS 返回 `untrusted_target`，不执行进程操作。
- Authenticode/时间戳证书不可用时，发布验证失败关闭；不以 unsigned 产物标记
  为正式完成。
- 可独立回滚：文档迁移、WorkBuddy 页面、版本状态、顶部栏/窗口、E2E 基础。
  必须成组回滚：WorkBuddy DTO/事务、Codex token/状态机、Windows 提权/安装/
  降权启动、Portable 删除/签名门禁。
