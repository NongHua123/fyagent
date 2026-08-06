# Codex v1.0.2 运行时实施计划

1. 将现有“ambiguous 不操作”和两阶段强制确认测试改为描述 v1.0.2 目标的 fake
   失败场景；保留 identity hard-stop 测试。
2. 在 `services/codex_desktop` 增加私有 restart plan/capability 边界，演进已有
   runtime instance 向量；Windows/macOS adapter 仅提供精确身份集合。
3. 更新 Tauri command/typed facade/DTO，并把协调器和对话框重构为
   confirm/progress/incomplete；同步四语言与可访问性焦点规则。
4. 在 installer hook/card 增加 version-state 联合和测试，保持现有 Query 重试
   策略而不是重建请求状态机。
5. 运行 fake/fixture Rust、Vitest、typecheck、format；审查无真实 process/
   PackageManager/Codex config 调用。

主要文件：`src-tauri/src/services/codex_desktop/*`、
`src-tauri/src/codex_desktop/platform/*`、`src-tauri/src/commands/codex_desktop.rs`、
`src/types/codexDesktop.ts`、`src/lib/api/codex-desktop.ts`、
`src/hooks/useCodexRestartCoordinator.ts`、`src/hooks/useCodexDesktopInstaller.ts`、
`src/components/codex/*` 及对应测试。
