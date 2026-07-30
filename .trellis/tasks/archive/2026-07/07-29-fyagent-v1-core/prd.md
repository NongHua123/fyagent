# FyAgent V1 核心领域与安装服务

## Goal

建立 FyAgent V1 安装器的可测试核心：受限领域 DTO、错误与诊断、镜像 source、下载与
通用校验、单内存 Job、`CodexDesktopService`、普通 IPC 契约和 Linux-safe unsupported
行为。该任务不实现真实 Windows/macOS 系统调用，也不修改共享注册文件。

## Confirmed Facts

- 当前仓库没有 `codex_desktop`、agentsmirror、桌面安装 Job、安装器 IPC 或事件实现。
- 可复用 `reqwest`、`sha2`、`tempfile`、`url`、`bytes`、`zip`、`tokio`、`serde`、
  `uuid`；全局 HTTP client 的代理继承和 URL 脱敏可参考，但其自动重定向不能用于
  安装器。
- 当前 mirror 的 manifest 为 schema v5，且 raw manifest SHA-256 已与 checksums
  条目核验一致。source 只能消费 V1 所需的受限字段和内置 endpoint 枚举。

## Requirements

- 定义 Rust/TS 一一对应的 platform/architecture/version、local/remote 状态、
  `JobSnapshot`、稳定错误、`release_id` 与 `StartInstallRequest`；普通请求只能含
  `expected_release_id`。
- 实现完整 Job 状态机、单任务互斥、sequence 单调、取消边界、完整快照事件、五分钟
  内存缓存与应用重启不恢复语义。
- 实现 agentsmirror checksums-first、raw manifest hash-before-parse、schema v5
  fail-closed 解析、每架构独立 release、固定 endpoint 选择与开始安装时强制重验证。
- 实现手工 HTTPS redirect（最多 5 跳）、三次网络尝试、取消/进度、固定 `.part` 临时
  路径、大小/hash/三倍磁盘/安全清理校验和脱敏诊断。
- 定义只接收 `VerifiedPackage` 的平台 trait、fake/unsupported adapter、service 编排与
  七个普通 IPC 函数的薄壳；共享注册由 integration 子任务完成。

## Acceptance Criteria

- [ ] source、checksum、release ID、版本、状态转换、取消竞态、并发、旧事件、清理和
  redaction 都有不依赖公网/大包的单元或 service fake 测试。
- [ ] DTO JSON fixture 被 Rust 序列化测试和 TypeScript 消费测试共同覆盖，字段 casing
  明确，不靠前端猜测。
- [ ] `start_install` 在 metadata 变化、非允许字段、降级/重装路径或 active Job 时安全
  拒绝；remote 失败不妨碍 local launch。
- [ ] Linux build 可产生明确 unsupported 状态，且通用模块不导入 Windows/macOS API。
- [ ] 日志/复制详情不包含 token、cookie、userinfo、URL query/fragment、完整 home 路径
  或不受限命令输出。

## Boundaries

- Owned paths：`src-tauri/src/codex_desktop/**`（除平台子目录）、
  `src-tauri/src/services/codex_desktop/**`、`src-tauri/src/commands/codex_desktop.rs` 与
  对应 fixture/test。
- 不修改 `lib.rs`、`store.rs`、`commands/mod.rs`、`services/mod.rs`、Cargo、全局 HTTP
  builder、前端或 platform/windows/macos。任何所需共享 patch 交给 integration。
- 不下载生产 MSIX/DMG、不真实安装、不持久化 Job/metadata、不增加下载源或 URL 输入。

## Dependencies and Risks

- 该任务先于 Windows、macOS、UI 开始，并冻结 DTO/trait 后供它们使用。
- Windows Publisher fixture 不是本任务可自行猜测的常量；缺失时以受控注入/fixture
  contract 保持验证接口，不能放宽 allowlist。
- 当前 schema 或 identity 发生动态变化时需重新取证、更新 fixture，不接受远程 URL
  或未知 schema 来“兼容”。
