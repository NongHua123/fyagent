# FyAgent V1 集成、品牌与质量门槛

## Goal

将 Core/Windows/macOS/UI 的已验证交付物接入应用，并完成 V1 的共享文件改动：AppState/
IPC 注册、安装器 scoped proxy、退出/临时清理、Codex CLI 只读、FyAgent 可见品牌、宿主
updater 全层禁用、应用品牌图标替换、自动化质量门与人工验收资料。

## Requirements

- 只在本任务修改共享注册和产品级路径：`lib.rs`、`store.rs`、`commands/mod.rs`、
  `services/mod.rs`、`main.rs`、Cargo/lock、Tauri config/capabilities、`src/main.tsx`、
  About/updater/DatabaseUpgrade、可见品牌 locale、release workflow 和必要的 App mount
  冲突解决，以及 `assets/fyagent.png`、`src-tauri/icons/**`（不含
  `dmg-background.png`）和 `src/assets/icons/app-icon.png` 的应用品牌资产。
- 在 `AppState` 中持有唯一 `Arc<CodexDesktopService>`，attach AppHandle 后注册七个
  commands、startup stale-temp cleanup 和安全的 existing close coordination。不得建立第二
  套 React close listener 或数据库表。
- 对现有 Codex CLI lifecycle 写操作进行集中后端拒绝，同时保留 read-only version/latest/
  installation diagnostics；前端禁止 Codex install/update/repair、bulk 和复制命令，其他
  工具生命周期不回归。
- 关闭宿主 upstream updater：移除 config endpoint/artifact、capability、Rust plugin、
  commands、UpdateProvider/API/Badge/About UI 和 release workflow updater 产物。数据库版本
  过新保护须保留，但恢复界面改为无网络、受控分发/支持提示，不能留下死调用或修改 DB。
- 将用户可见名称、窗口、Header、About、README/发布文本和所有 locale 的可见 CC Switch
  /官网/上游 GitHub 替换为 FyAgent 或删除；不得改 identifier、deep-link、数据目录、
  内部 crate/npm 名和 LICENSE。应用品牌图标以用户提供的 1024×1024 RGBA PNG 原始字节
  为唯一视觉源：Tauri 标准桌面/Windows Store/Android/iOS 集合由 CLI 生成，About 使用
  生成的 32×32 图标；macOS tray 仅以源图 alpha 轮廓生成 24/48/72 像素黑色 template。
  不重绘、改色或改构图，也不修改 provider/partner 图标、截图或 DMG 背景。
- 汇总所有质量命令、Windows ARM64 target evidence、静态安全审计、fixture/mock 证据和
  `14` 人工验收记录模板；不执行真实安装或声称人工验收完成。

## Acceptance Criteria

- [ ] 七个 commands、AppState、service、event、scoped proxy 与 temp cleanup 接入但不让
  commands/App 变成业务层；既有 unit/support 可构造 AppState，不需要真实 Tauri runtime。
- [ ] 下载/校验态关闭按既有 exit coordinator 提示取消；install/后验态提示等待，不强杀
  平台操作；无 active Job 正常退出。
- [ ] 直接调用 Codex lifecycle IPC 被拒绝，UI/批量/复制安装命令均排除 Codex，其他工具
  的行为有回归测试。
- [ ] updater 在 config、capability、Cargo/npm、plugin、handler、frontend、DatabaseUpgrade
  和 release workflow 中无生产残留；database-too-new 分支保持安全、无网络。
- [ ] 可见 CC Switch/ccswitch.io/上游 GitHub 已按范围审计并处理，legal/internal identity
  保留；installer 生产路径只含 agentsmirror 常量。
- [ ] 所有既有应用品牌图标路径均由同一 FyAgent 源图重新生成，`64x64.png` 保留，About
  与生成的 32×32 图标一致；macOS tray template 尺寸、黑色 RGB、透明度和 18pt 内容框
  满足约束，DMG 背景及 provider/partner 等非应用品牌资产未变。
- [ ] 全量前后端/Rust质量门与静态审计均记录 command、OS、exit code、summary；Windows
  ARM64 有独立 build/target evidence；Windows/macOS 图标视觉验收明确 Pending human。

## Dependencies and Boundaries

在 Core contract 与至少一个适配实现可编译后接入；按 Core → Windows → macOS → UI 顺序
合并。若 platform 失败，不能用 stub 伪装支持；保持明确 unsupported，继续修复。所有
不属于 V1 的大重构、数据迁移、identifier 更改或新 update channel 均被拒绝。
