# Codex v1.0.2 运行时技术设计

## 重启计划与能力令牌

平台适配器返回仅后端可见的可信安装和运行实例集合。服务层用固定比较器选择：
有运行实例、版本、新旧安装范围、稳定键；平台枚举顺序永不成为业务规则。请求
确认时创建 `RestartPlan` 及一次性 token。token 绑定 exact app identity、操作
类型和计划摘要，不绑定执行时允许重新计算的瞬时安装选择。

消费 token 后：重新枚举 -> 以固定比较器重建计划 -> 验证选中安装可启动 ->
按进程创建时间去重的所有精确实例强制关闭 -> 等待全部退出 -> 仅启动一次 ->
等待新的精确实例。关闭或身份/安装验证任一步失败时返回 incomplete/manual，
launch count 必须为零。重试使用独立短期一次性 retry capability，且不重新展示
风险确认；不能安全签发时只允许手动处理。

`untrusted_target`、unsupported、not-running、confirming、progress、incomplete
和 restarted 是明确状态；renderer 只能根据脱敏 DTO 显示固定本地化文案。

## 版本状态

从 TanStack Query 的 `data`、首次 loading/error、refetching/refetch error 和
平台/元数据错误派生 `LocalVersionState`、`RemoteVersionState` 联合类型。安装、
更新、启动可用性由该联合集中计算；卡片只 switch state，避免 `undefined`/`null`
承载多个语义。

## 安全与测试

Windows 当前精确 PFN 证据与 allowlist 一致，但代码不依赖显示名称；macOS 候选
身份未在本机验证时 fail closed。所有服务测试通过 fake runtime/installer、
mock clock、fixture PID/creation-time 运行；没有真实 OS 进程、用户配置、安装或
桌面 app 调用。Rust/TS DTO 升级与协调器/对话框必须成组回滚。
