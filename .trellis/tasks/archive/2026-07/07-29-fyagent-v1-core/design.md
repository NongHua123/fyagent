# Core 设计

`codex_desktop` 分为 `types`、`error`、`source`、`download`、`verify`、`platform`；
service 分为编排与 Job controller。领域层不依赖 Tauri，事件通过窄 `EventSink` 注入，
测试可用 fake source/platform/filesystem/clock/transport。

Source 的信任顺序固定为 checksums → raw manifest hash → 当前平台分支解析 → release
descriptor。descriptor 不保存远程下发 URL；download endpoint 只能由代码内枚举决定。开始
安装用 `expected_release_id` 重新解析得到的 descriptor 比较，变化即 `METADATA_CHANGED`。

Job controller 在短临界区内原子占用唯一槽、更新快照与取消边界；网络/磁盘/平台 await
不持有 mutex。每个变更发布完整 `JobSnapshot`，同 Job sequence 严格递增，新 Job 不接受
旧 Job 更新。`VerifiedPackage` 构造受限，平台 adapter 不能接受裸下载路径。

安装器专用 HTTP client 从现有代理配置继承合法 HTTP/SOCKS 代理，但显式
`reqwest::redirect::Policy::none()`，以纯函数和传输 fake 检查每一跳 HTTPS、userinfo、
相对 Location、最多五跳和日志脱敏。它不改变既有全局 client 的 timeout/redirect 语义。
