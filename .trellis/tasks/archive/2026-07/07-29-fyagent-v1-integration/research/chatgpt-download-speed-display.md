# Research: ChatGPT 客户端下载速度展示

- Query: 定位 ChatGPT/Codex Desktop 客户端下载安装期间显示下载速度的最小实现路径；确认现有事件/DTO 是否已有累计字节与时间信息、采样和重置边界、聚焦测试，以及当前 `Cargo.lock` 排序差异是否相关。
- Scope: internal
- Date: 2026-07-30

## Findings

### 结论

现有 renderer 收到的完整 `JobSnapshot` 已足够在前端计算下载速度，不需要修改 Rust 下载器、Tauri 事件名、IPC DTO、TypeScript wire type 或 DTO contract fixture：

- 下载器按本次 attempt 从 `0` 开始累计 `completed_bytes`，每次 chunk 后递增，并在完成、累计增加至少 1 MiB、或距上次发送至少 100 ms 时发送进度（`src-tauri/src/codex_desktop/download.rs:581-627`）。
- core 进度更新本身含 `completed_bytes` / `total_bytes`，不含时间戳（`src-tauri/src/codex_desktop/download.rs:268-275`）。
- service 将该累计字节映射为 `JobProgress`，并在写入 `JobStore` 时用 clock 生成新的 RFC 3339 时间（`src-tauri/src/services/codex_desktop/mod.rs:724-765`）。
- `JobStore` 每次接受进度都会递增 `sequence` 并替换 `updated_at`（`src-tauri/src/codex_desktop/jobs.rs:100-106,308-335`）；完整事件 DTO 同时含 `jobId`、`sequence`、`stage`、`updatedAt` 和进度（`src-tauri/src/codex_desktop/types.rs:615-658`）。
- Tauri glue 原样发送完整 snapshot（`src-tauri/src/lib.rs:654-668`），前端 wire type 已声明 `updatedAt` 与累计字节（`src/types/codexDesktop.ts:169-189`）。

因此准确回答是：**累计字节已有，时间信息也已有，但时间在外层 `JobSnapshot.updatedAt`，不在内部 `DownloadProgressUpdate` / `JobProgress`。** 现有 `sequence` 负责顺序，`updatedAt` 可作为相邻采样的时间轴。

### 最小文件集

推荐只改以下三个生产文件/文件组和两个现有测试文件：

1. `src/hooks/useCodexDesktopInstaller.ts`
   - 当前 hook 已按同一 job 的递增 `sequence` 接受完整 snapshot（`src/hooks/useCodexDesktopInstaller.ts:52-78`），监听、恢复查询和 cache merge 集中在 `src/hooks/useCodexDesktopInstaller.ts:233-276`。
   - 当前 view model 只把 `completedBytes`、`totalBytes`、`percent` 投影成 `CodexDesktopProgress`，丢掉了可用于采样的 `updatedAt`（`src/hooks/useCodexDesktopInstaller.ts:29-50,444-451`）。
   - 在 hook 内用 ref 保存上一个已接受下载样本 `{ jobId, completedBytes, updatedAtMs }`，为 `CodexDesktopProgress` 增加 renderer-only 的 `bytesPerSecond: number | null`（或语义等价字段）。这不是 wire DTO 字段。

2. `src/components/codex/CodexDesktopInstallerCard.tsx`
   - 现有 `formatBytes` 和 byte pair 均在该组件内（`src/components/codex/CodexDesktopInstallerCard.tsx:63-79,112-120`）。复用它把速度格式化为 `B/s`、`KB/s`、`MB/s`、`GB/s`，并在当前百分比/累计字节行末显示，例如 `50% · 512 MB / 744 MB · 8.4 MB/s`（现有渲染点：`src/components/codex/CodexDesktopInstallerCard.tsx:221-230`）。
   - 必须继续复用 `state === "job_downloading"` 门控；安装阶段复用相同数值字段表示 Windows `0..100/100` 或 macOS `0..3/3`，不能被标成网络速度。该边界也由 `.trellis/spec/backend/codex-desktop-installer.md:91-100,320-332` 明确规定。

3. locale
   - 推荐的紧凑呈现只有数值与标准单位（如 `8.4 MB/s`），与组件现有未翻译的 `B/KB/MB/GB` 单位模式一致，因此**无需新增 locale key**。
   - 若产品希望显式显示“下载速度”标签，则 key 应放在 `codexDesktop.details.downloadSpeed`：现有详情键位于 `src/i18n/locales/en.json:3014-3018` 与 `src/i18n/locales/zh.json:3014-3018`。四个注册语言来自 `src/i18n/index.ts:4-7`，缺失键回退英文（`src/i18n/index.ts:79-82`）。当前 `ja.json`、`zh-TW.json` 中没有任何 `codexDesktop` 节点；若新增文本，按 frontend 规范应在四个 locale 都补 key，而不是继续扩大该缺口。

不建议修改：

- `src-tauri/src/codex_desktop/download.rs`
- `src-tauri/src/codex_desktop/types.rs`
- `src-tauri/src/services/codex_desktop/mod.rs`
- `src/types/codexDesktop.ts`
- `tests/fixtures/codexDesktopDtoContract.v1.json`
- `tests/codexDesktopDtoContract.test.ts`

这些层已经提供计算所需输入；增加 wire-level `bytesPerSecond` 会把一个纯展示派生值扩散到 Rust DTO、fixture 与跨层契约，超出“小需求”的最小范围。

### 速度采样与重置边界

推荐计算相邻已接受下载 snapshot 的区间平均速度：

`bytesPerSecond = (currentCompleted - previousCompleted) * 1000 / (currentUpdatedAtMs - previousUpdatedAtMs)`

只在以下条件全部满足时产出数值：

- 当前 `job.stage === "downloading"`；
- 当前 `job.progress?.phase === "download"`；
- `completedBytes` 为有限、非负数；
- 当前样本与上一样本 `jobId` 相同；
- byte delta 大于 `0`；
- `Date.parse(updatedAt)` 有效且 time delta 大于 `0`；
- 结果有限且非负。

首个合法下载样本只建立 baseline，速度显示为空；现有 100 ms / 1 MiB 进度节流意味着正常下载很快会得到第二个样本。以下任一情况必须清空速度并重建/丢弃 baseline：

- jobId 改变；
- 离开 `downloading`，或 progress phase 不再是 `download`；
- progress 为空；
- `completedBytes` 小于或等于上一样本（下载 retry 会把累计值重置为 `0`；attempt 虽未进入 renderer DTO，但 service 的 attempt-change 会强制发布该进度，见 `src-tauri/src/services/codex_desktop/mod.rs:849-875`）；
- 时间无效或不递增。

不要用 `startedAt` 计算从 job 启动至今的累计平均，因为它包含 checking/preflight，不是下载起点（字段定义见 `src-tauri/src/codex_desktop/types.rs:647-654`）。也不要用安装阶段的 counter。

现有下载事件不是 heartbeat：网络完全停滞且没有新 chunk 时不会产生新 snapshot，所以最小实现显示的是“最近两个已接收样本间速度”，停滞期间会保留最后值。若必须在停滞后自动变成 `0 B/s`，需另加前端 staleness timer；这不是本次最小闭环的必要条件，应避免顺带引入定时器与平滑算法。

### 聚焦测试

1. `tests/hooks/useCodexDesktopInstaller.test.tsx`
   - 现有 fixture 已含 `updatedAt`（`tests/hooks/useCodexDesktopInstaller.test.tsx:130-147`），事件注入模式位于 `tests/hooks/useCodexDesktopInstaller.test.tsx:188-199,316-341`。
   - 新增测试：同 job 先发送 `0 bytes @ T0`，再发送 `1_048_576 bytes @ T0+1000ms`，断言 hook 暴露 `1_048_576 B/s`。
   - 同一测试或独立表格测试覆盖：第一样本为 null；离开 downloading 后清空；新 job 清空；同 job byte 回退到 `0`（retry）清空；相同/倒退/无效时间不产生 Infinity、负值或 NaN。

2. `tests/components/CodexDesktopInstallerCard.test.tsx`
   - 扩展现有下载进度用例（`tests/components/CodexDesktopInstallerCard.test.tsx:65-88`），给 view model 提供已知 rate，断言下载行含精确格式化速度，例如 `1 MB/s`。
   - 扩展现有安装阶段保护用例（`tests/components/CodexDesktopInstallerCard.test.tsx:90-104`），即使测试输入误带 rate，也断言安装 UI 不出现 `/s`，从而锁定“只有下载阶段展示网络速度”。

聚焦验证命令：

```powershell
pnpm exec vitest run tests/hooks/useCodexDesktopInstaller.test.tsx tests/components/CodexDesktopInstallerCard.test.tsx
pnpm typecheck
pnpm format:check
```

由于推荐方案不改 backend/wire contract，无需为本需求新增 Cargo/DTO fixture 测试；最终 Trellis full-scope check 可按任务既有质量门运行。

### `Cargo.lock` 差异

当前工作树在研究开始时只有 `src-tauri/Cargo.lock` 为 dirty。`git diff --numstat -- src-tauri/Cargo.lock` 是 `76` additions / `76` deletions；diff 仅把完全相同的 `fyagent 3.18.0` root package block 从旧的 `cc`/`cesu8` 位置移动到按包名排序的 `fxhash`/`gdk` 位置。当前 block 从 `src-tauri/Cargo.lock:1778` 开始，名称/版本见 `src-tauri/Cargo.lock:1779-1781`，依赖列表没有增删。

该排序变化与下载速度展示**无关**，也不应被解释为本需求所需的依赖或 Rust 行为变更。它更像 package rename 后 Cargo 重写 lockfile 时产生的规范排序。提交时应把它当作预先存在/独立的 task residue 单独识别；不要为了前端速度改动声称它是必要文件。

### Files found

- `src-tauri/src/codex_desktop/download.rs` — 累计下载字节与 core 进度发送节流。
- `src-tauri/src/services/codex_desktop/mod.rs` — core progress 到 JobSnapshot 的桥接、attempt/phase 节流与 clock 注入。
- `src-tauri/src/codex_desktop/jobs.rs` — snapshot sequence 和 `updatedAt` 更新规则。
- `src-tauri/src/codex_desktop/types.rs` — Rust `JobProgress` / `JobSnapshot` wire DTO。
- `src-tauri/src/lib.rs` — 完整 snapshot 的 Tauri event glue。
- `src/types/codexDesktop.ts` — TypeScript wire contract，已有累计字节和 `updatedAt`。
- `src/hooks/useCodexDesktopInstaller.ts` — event merge、view-state 派生与 renderer progress view model；推荐速度采样归属。
- `src/components/codex/CodexDesktopInstallerCard.tsx` — 现有字节格式化和进度展示；推荐速度渲染归属。
- `src/i18n/index.ts`、`src/i18n/locales/{en,ja,zh,zh-TW}.json` — 四语言注册/回退及可选显式速度标签位置。
- `tests/hooks/useCodexDesktopInstaller.test.tsx` — event/hook 聚焦测试族。
- `tests/components/CodexDesktopInstallerCard.test.tsx` — 下载与安装进度的组件保护测试族。
- `src-tauri/Cargo.lock` — 与本需求无关的现有 root package block 排序 diff。

### External references

无。该需求可由仓库当前事件契约、规范和测试模式完整回答，不需要外部资料。

### Related specs

- `.trellis/spec/backend/codex-desktop-installer.md:91-100,183-186,246-250,320-332` — 下载字节只属于 `job_downloading`，安装 counter 不得标成 bytes。
- `.trellis/spec/frontend/index.md:8-19` — frontend 变更前定位 API/hook/type/schema/test 与四 locale。
- `.trellis/spec/frontend/hook-guidelines.md:17-31` — effect/ref/timer 清理和 named hook return shape。
- `.trellis/spec/frontend/component-guidelines.md:44-57` — feature component 与用户可见文本模式。
- `.trellis/spec/frontend/quality-guidelines.md:1-34` — Vitest/Testing Library 与四 locale 质量约束。

## Caveats / Not Found

- 研究角色隔离协议禁止读取 `implement.jsonl` / `check.jsonl`；本结论依据任务 `prd.md`、`design.md`、`implement.md`、其明确引用的 installer/frontend 规范及当前代码。
- `DownloadProgressUpdate` 没有时间字段；可用时间来自最终 event 的 `JobSnapshot.updatedAt`。因此建议保持 renderer-only 派生，不要误写成 downloader 原生测速值。
- 当前 `ja.json` 与 `zh-TW.json` 没有 `codexDesktop` 根节点，依赖英文 fallback；这不是下载速度需求引入的问题。本次若不用显式标签，不应顺带搬运整个 locale 区块。
- 未运行测试或构建；这是只读实现研究，验证命令供 implement/check 阶段执行。
