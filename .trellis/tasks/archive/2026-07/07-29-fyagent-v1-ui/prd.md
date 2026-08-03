# FyAgent V1 前端安装体验

## Goal

在 Codex Provider 页面顶部实现独立的 Codex Desktop App 卡片及其 TypeScript DTO、
Tauri API、TanStack Query、事件恢复 Hook、简中/英文文案和前端测试。前端只呈现 Rust
权威快照，不管理安装状态机或信任安装参数。

## Requirements

- 新增 `src/types/codexDesktop.ts`、`src/lib/api/codex-desktop.ts`、
  `src/lib/query/codex-desktop.ts`、`src/hooks/useCodexDesktopInstaller.ts`、
  `src/components/codex/CodexDesktopInstallerCard.tsx` 与测试；`App.tsx` 只作一处
  ProviderList 上方的条件挂载。
- API 只能封装七个固定 command；Hook 在 listener 与 `get_job` 间以 `jobId + sequence`
  合并完整 snapshots，页面卸载不取消 Job，成功 Toast 每 Job 一次。
- 页面进入同时查询 local/remote；remote 失败不清空 local。Linux 不渲染，macOS Intel
  显示不支持，其他状态严格按文档的 install/update/launch/progress/cancel/retry 表渲染。
- 本地版本较旧且可更新时，只显示“更新 Codex”主按钮，不显示次级“启动 Codex”；用户
  可继续通过操作系统自行启动旧版。
- 完整维护 installer zh/en；其他 locale 使用项目既有英文 fallback，但全局 FyAgent
  品牌审计由 integration 完成。不得添加前端生产依赖。
- 不提供 URL/source/scope/path/hash/Stable-Beta selector、all-users、重装、降级、
  自定义校验绕过、安装向导或 Provider 自动修改。

## Resolved UX Decision

用户已确认：本地版本较旧且存在可用更新时，不保留次级“启动 Codex”按钮。更新态保持
单一“更新 Codex”行动，以避免误导用户跳过可用更新；此决定不限制用户从系统自行启动
已有应用。

## Acceptance Criteria

- [ ] Hook 覆盖 initial local/latest/job、event-before-query/event-after-query、旧 sequence
  丢弃、terminal invalidation、unmount 不取消、success toast exactly once、copy error 和
  refresh force。
- [ ] Card 覆盖 Linux/Intel、未安装、旧版、同版、本地较新、remote fail + local、remote
  fail + no local、download/cancel/verify/install/success/fail/ambiguous，以及 aria/keyboard。
- [ ] `App.tsx` 没有 installer invoke、监听、版本比较、下载 state 或平台分支；无禁止的
  用户控件；结构和快照字段与 Rust fixture 兼容。
- [ ] UI 不再把 Codex CLI lifecycle 与新的桌面安装 DTO 混合；CLI 清理由 integration
  完成但卡片不得依赖它。

## Ownership

拥有新增前端 installer 文件、installer 测试、zh/en installer key 及最小 App 挂载 patch。
不修改 About、updater、全局品牌/其他 locale、`src/main.tsx` 或 shared Rust registration。
