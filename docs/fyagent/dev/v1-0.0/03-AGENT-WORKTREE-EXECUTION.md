# Worktree 并行执行规范

## 1. 目标

通过 Git worktree 让 Core、Windows、macOS 和 UI 并行开发，同时避免多个 Agent 在 `App.tsx`、`lib.rs`、模块注册、依赖文件和 updater 清理上产生高冲突。

开发阶段网络不受中国大陆友好约束。Agent 可以访问 GitHub、OpenAI/Microsoft/Apple 官方文档、npm、crates.io，也可以安装缺失构建环境；不得把个人开发镜像配置提交到仓库。

## 2. 分支拓扑

```text
main
└── feature/fyagent-v1                  集成分支
    ├── worktree/fyagent-core           领域与公共契约
    ├── worktree/fyagent-windows        Windows adapter
    ├── worktree/fyagent-macos          macOS adapter
    └── worktree/fyagent-ui             前端实现
```

建议创建命令：

```bash
git switch main
git pull --ff-only              # 当前仓库有远端更新时；无远端更新则跳过
git switch -c feature/fyagent-v1

mkdir -p ../fyagent-worktrees
git worktree add ../fyagent-worktrees/core -b worktree/fyagent-core feature/fyagent-v1
git worktree add ../fyagent-worktrees/windows -b worktree/fyagent-windows feature/fyagent-v1
git worktree add ../fyagent-worktrees/macos -b worktree/fyagent-macos feature/fyagent-v1
git worktree add ../fyagent-worktrees/ui -b worktree/fyagent-ui feature/fyagent-v1
```

## 3. 并行前置：Core 契约先行

Windows、macOS 和 UI 不得在领域 DTO 尚未稳定时各自定义一套类型。

Core 第一提交必须只包含：

- `codex_desktop` 公共领域类型；
- 稳定错误结构；
- platform trait/边界；
- JobSnapshot 与事件名；
- commands 函数签名草案或编译可用薄壳；
- 测试 double 接口；
- source descriptor 契约；
- 不包含平台真实实现。

推荐 commit：

```text
feat(codex-desktop): establish installer domain and IPC contracts
```

集成负责人将该提交 cherry-pick 到 `feature/fyagent-v1`，再让三个并行工作分支 rebase/cherry-pick 此契约后继续。

## 4. 文件所有权

### 4.1 Core 独占

```text
src-tauri/src/codex_desktop/mod.rs
src-tauri/src/codex_desktop/types.rs
src-tauri/src/codex_desktop/error.rs
src-tauri/src/codex_desktop/source.rs
src-tauri/src/codex_desktop/download.rs
src-tauri/src/codex_desktop/verify.rs
src-tauri/src/codex_desktop/platform/mod.rs
src-tauri/src/services/codex_desktop/mod.rs
src-tauri/src/services/codex_desktop/job.rs
src-tauri/src/commands/codex_desktop.rs
```

Core 负责通用测试和 fake platform，但不写 Windows/macOS 具体系统调用。

### 4.2 Windows 独占

```text
src-tauri/src/codex_desktop/platform/windows/mod.rs
src-tauri/src/codex_desktop/platform/windows/manifest.rs
src-tauri/src/codex_desktop/platform/windows/deployment.rs
src-tauri/src/codex_desktop/platform/windows/elevation.rs
对应 tests/fixtures
```

Windows worker 不修改 `commands/mod.rs`、`services/mod.rs`、`lib.rs` 或 Cargo 依赖；用临时分支假设依赖存在，最后由集成 Agent统一加入。

### 4.3 macOS 独占

```text
src-tauri/src/codex_desktop/platform/macos/mod.rs
src-tauri/src/codex_desktop/platform/macos/bundle.rs
src-tauri/src/codex_desktop/platform/macos/dmg.rs
对应 tests/fixtures
```

### 4.4 UI 独占

```text
src/types/codexDesktop.ts
src/lib/api/codex-desktop.ts
src/lib/query/codex-desktop.ts
src/hooks/useCodexDesktopInstaller.ts
src/components/codex/CodexDesktopInstallerCard.tsx
对应前端测试
src/i18n/locales/zh.json 中 installer 新 key
src/i18n/locales/en.json 中 installer 新 key
```

`App.tsx` 属于 UI worker，但其改动必须限制为 import 和条件挂载，不加入业务状态。

### 4.5 仅集成 Agent 修改

```text
src-tauri/src/lib.rs
src-tauri/src/store.rs
src-tauri/src/commands/mod.rs
src-tauri/src/services/mod.rs
src-tauri/src/main.rs
src-tauri/Cargo.toml
src-tauri/Cargo.lock
src-tauri/tauri.conf.json
package.json
pnpm-lock.yaml
src/main.tsx
src/components/settings/AboutSection.tsx
src/components/UpdateBadge.tsx
src/contexts/UpdateContext.tsx
src/lib/updater.ts
其他语言中的品牌文案
```

## 5. Worker 提交规则

每个 worktree：

- 至少一个原子 commit；
- 不保留未提交工作给集成 Agent；
- 不自行合并集成分支；
- 不修改不属于自己的文件；
- 不为了编译而提交临时 stub 到共享文件；
- commit message 说明平台、主要行为和测试；
- 新增依赖需求写在 commit body，不直接改 Cargo 共享区；
- 所有 TODO 必须对应明确的后续 commit，不允许模糊“稍后完成”。

推荐提交拆分：

```text
Core
1. feat(codex-desktop): add domain contracts and job state
2. feat(codex-desktop): add mirror source and validated downloader
3. test(codex-desktop): cover source, retry, cancellation and drift

Windows
1. feat(codex-desktop): add Windows package inspection
2. feat(codex-desktop): add current-user deployment and launch
3. feat(codex-desktop): add experimental all-user elevation path
4. test(codex-desktop): cover Windows adapters with fixtures

macOS
1. feat(codex-desktop): add macOS bundle inspection
2. feat(codex-desktop): add DMG transactional installation
3. test(codex-desktop): cover macOS command adapters and conflicts

UI
1. feat(codex-desktop): add installer query and hook
2. feat(codex-desktop): add Codex installer card
3. test(codex-desktop): cover card states and event recovery
```

## 6. 集成顺序

```text
1. Core contract
2. Core source/downloader/service
3. Windows
4. macOS
5. UI
6. 模块注册与依赖
7. Codex CLI 生命周期清理
8. FyAgent 品牌与 updater 清理
9. 跨平台测试和格式化
10. 人工验收文档准备
```

每次 cherry-pick 后先运行受影响子集，不要把所有冲突堆到最后。

## 7. 共享契约变更流程

Worker 发现公共契约不满足平台需求时：

1. 不在自己的平台文件里定义重复 DTO；
2. 提交一份小型契约变更说明，包括必要字段、使用位置、兼容影响；
3. 由 Core Agent 在 Core worktree 修改；
4. 集成分支先 cherry-pick 新契约；
5. 其他 worker 同步后继续。

禁止：

- Windows 和 macOS 各自定义不同的 `ReleaseDescriptor`；
- TypeScript 手写与 Rust 不一致的字段语义；
- 为平台特例把系统句柄、HRESULT 或命令输出暴露进通用 DTO；
- 为隐藏 all-users 实验能力给普通 UI DTO 增加 `scope`。

## 8. 合并冲突原则

### `App.tsx`

只保留：

```tsx
{activeApp === "codex" && currentView === "providers" && (
  <CodexDesktopInstallerCard />
)}
```

实际插入位置是 Provider 列表上方。不得把 hook 展开到 `App.tsx`。

### `lib.rs`

集成 Agent统一完成：

- `mod codex_desktop;`
- service re-export；
- `AppState` 创建；
- command 注册；
- startup temp cleanup；
- headless 参数在 Tauri 初始化前分流；
- updater plugin 移除。

### `Cargo.toml`

只加入经过证明必要的依赖/feature：

- `quick-xml`；
- Windows target 的 `windows` crate 与最小 WinRT feature；
- 不新增前端生产依赖。

## 9. Agent 进度回报格式

每个 worker 完成时输出：

```text
Worktree:
Base SHA:
Commits:
Owned files changed:
Public contract assumptions:
Dependencies requested:
Tests run:
Tests not run and reason:
Known risks:
Manual verification required:
```

不得使用“应该能工作”“大概通过”替代测试结果。

## 10. 环境与联网

开发 Agent 可以：

- 搜索官方与开源资料；
- 安装 Rust target、Node、pnpm、系统 SDK；
- 获取小型公开 fixture 或生成匿名 fixture；
- 使用正常 npm/crates.io/GitHub。

开发 Agent 不得：

- 把个人代理、registry、npm mirror、Cargo source replacement 写入项目；
- 提交真实用户路径、token、Cookie、预签名下载 URL；
- 把完整生产 MSIX/DMG 提交到 Git；
- 在 CI 中请求真实生产大包。

## 11. 回滚与恢复

由于每个 worktree 是原子 commit，集成失败时：

- 使用 `git revert` 或重新建立集成分支；
- 不在共享分支强制重写其他 worker 历史；
- 不删除 worker 分支，直到完整 CI 通过；
- 平台实现无法按时完成时，不用 stub 假装支持；明确编译隔离并保持 UI 不暴露。
