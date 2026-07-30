# UI 设计

类型层与 Rust DTO 保持显式 casing。API 只调用 command，Query 保存 local/remote/job cache，
Hook 订阅 `codex-desktop-installer://job-updated` 并以 `jobId + sequence` 拒绝 stale event。
后端是 Job 和版本比较唯一权威；ViewModel 只推导按钮、label、progress、错误操作。

Card 使用现有 Card/Button/Progress/Alert primitives。remote error 与 local status 并列而非
互相覆盖，因此已安装时始终可启动。错误详情显示稳定码与脱敏文本，可复制/开日志，但
不显示 URL query、安装路径或原始系统控制输出。成功不自动启动、不自动改 Provider。
