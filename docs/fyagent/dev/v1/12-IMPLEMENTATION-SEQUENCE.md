# 实施顺序与原子交付

## 1. 总原则

- Core 契约先于平台实现；
- 每一步都可编译、可测试、可回滚；
- 不以一次“大提交”完成全部功能；
- 不在尚未冻结 DTO 时并行编写互不兼容的前后端；
- 真实安装只在全部自动化质量门槛通过后人工执行。

## 2. 里程碑总览

```text
M0 基线与夹具
M1 领域契约和 Job
M2 镜像 Source + Download + Validation
M3 Windows Adapter
M4 macOS Adapter
M5 IPC + AppState 集成
M6 Frontend Card
M7 Codex CLI 只读
M8 FyAgent 品牌和禁用自更新
M9 退出保护与诊断
M10 全量质量门槛
M11 人工验收
```

## 3. M0 — 基线、外部契约与测试夹具

### 目标

建立可复现开发上下文，不写生产安装逻辑。

### 动作

1. 记录 baseline SHA和工具链；
2. 校验 CC Switch `3.18.0` 路径；
3. 在开发环境读取当前 agentsmirror manifest/checksums；
4. 从当前官方镜像包提取最小 MSIX AppxManifest、macOS Info.plist/签名元数据夹具；
5. 脱敏 URL/query/个人路径；
6. 记录 fixture 来源日期；
7. 验证现有 CI命令在未修改基线上的状态。

### 禁止

- 提交完整 MSIX/DMG；
- 从旧 VibeKey复制生产代码；
- 在 fixture中保留预签名 URL；
- 将开发网络限制误写为产品限制。

### 完成定义

- 夹具可被单元测试读取；
- 外部字段有证据；
- 基线报告完成。

## 4. M1 — 领域契约和 Job

### 建议 commit

```text
feat(codex-desktop): add domain models and job state machine
```

### 实现

- `types.rs`；
- `error.rs` 基础；
- `PlatformInstaller`/`InstallerSource`；
- `JobStore`；
- 合法转换；
- 取消边界；
- 完整快照；
- event sink抽象；
- Unsupported platform adapter；
- Rust serde casing测试。

### 测试

- 所有合法/非法转换；
- terminal不可变；
- job互斥；
- 旧 job不覆盖；
- cancel竞态；
- DTO snapshot。

### 完成定义

- 三平台 cargo test可编译；
- 平台 adapter可以是 fake/stub；
- 契约 commit SHA发布给 Worker。

## 5. M2 — Source、Downloader、Validation

建议拆分：

```text
feat(codex-desktop): parse agentsmirror release metadata
feat(codex-desktop): add cancellable installer downloader
feat(codex-desktop): add common validation and disk preflight
```

### Source

- Raw mirror DTO；
- current-platform validation；
- checksum parser；
- per-arch endpoint；
- release_id；
- 5 分钟 cache；
- refresh；
- metadata drift。

### Downloader

- scoped HTTP client；
- proxy inheritance；
- redirect policy；
- retries；
- progress；
- cancellation；
- `.part`；
- cleanup。

### Validation

- size；
- 3× disk；
- SHA-256；
- file type basic checks；
- redaction。

### 完成定义

- 所有网络测试使用 mock server或自定义 transport；
- 无真实大文件；
- 生产主动 URL仅 source常量。

## 6. M3 — Windows Adapter

建议 commits：

```text
feat(codex-desktop): parse and validate Windows MSIX metadata
feat(codex-desktop): add Windows current-user deployment adapter
feat(codex-desktop): add experimental all-user provisioning command
```

### 顺序

1. AppxManifest parser；
2. Package identity/version/arch/min OS；
3. PackageManager query；
4. signature verifier抽象；
5. current-user deployment；
6. AUMID launch；
7. HRESULT mapping；
8. internal all-user descriptor/elevation/stage/provision。

### 特别要求

- all-user提交独立，可在实验不稳定时不影响普通路径；
- 普通 IPC不暴露 scope；
- 不使用 PowerShell fallback；
- 不真实安装。

## 7. M4 — macOS Adapter

建议 commits：

```text
feat(codex-desktop): add macOS bundle detection and validation
feat(codex-desktop): add safe DMG installation flow
```

### 顺序

1. 标准目录扫描；
2. Info.plist/版本/身份；
3. running detector抽象；
4. hdiutil plist parser；
5. codesign/spctl命令抽象；
6. target path policy；
7. ditto临时复制；
8. 安全替换和补偿；
9. detach guard；
10. `open <path>`。

### 特别要求

- 保留原始 app basename；
- Classic不覆盖；
- 不写真实 Applications；
- Intel明确不支持。

## 8. M5 — Service、IPC 与 AppState 集成

建议 commit：

```text
feat(codex-desktop): wire installer service and Tauri commands
```

### 实现

- `CodexDesktopService` orchestration；
- `AppState` 唯一实例；
- `AppHandle` event；
- 7 个 commands；
- `generate_handler!` 注册；
- 启动 temp cleanup；
- terminal rescan；
- open log directory；
- Linux unsupported behavior。

### 测试

- fake source/platform end-to-end service；
- install success；
- source fail；
- download cancel；
- verify fail；
- platform fail；
- post-check fail；
- event快照序列。

## 9. M6 — Frontend

建议 commits：

```text
feat(ui): add Codex desktop installer API and query layer
feat(ui): add Codex desktop installer card
```

### 顺序

1. TS DTO；
2. API；
3. Query keys/mutations；
4. Hook + Tauri event；
5. Card；
6. App单点挂载；
7. zh/en；
8. 前端测试。

### 验收

- Linux hidden；
- Intel unsupported；
- remote fail local launch；
- success Toast一次；
- Installing无取消；
- 无 URL/scope/source控件。

## 10. M7 — Codex CLI 只读

建议 commit：

```text
refactor(tools): make Codex CLI lifecycle read-only
```

### 实现

- label `Codex CLI`；
- 保留探测和版本；
- 去安装/更新/修复按钮；
- 批量操作排除；
- 复制命令排除；
- 后端 lifecycle拒绝；
- 更新测试确保直接 IPC失败。

### 回归

其他工具生命周期必须保持。

## 11. M8 — FyAgent 品牌与宿主自更新关闭

建议 commit：

```text
chore(fyagent): update visible branding and disable self-update
```

### 实现

- productName/window title；
- About可见名称；
- 移除 ccswitch.io/GitHub可见链接；
- logger启动标题；
- 移除 UpdateProvider；
- 移除 updater plugin/endpoint/artifacts；
- 移除 update commands/UI；
- 将用户提供的 1024×1024 RGBA PNG 原始字节保存为 `assets/fyagent.png`；
- 运行 `pnpm tauri icon assets/fyagent.png --output src-tauri/icons` 生成桌面、Windows
  Store、Android、iOS 图标并保留 `64x64.png`；
- 从生成的 32×32 输出替换 About 图标；从源图 alpha 轮廓生成 24/48/72 像素 macOS
  template，按 24pt 画布中的 18pt 内容框等比居中，RGB 全黑并保留抗锯齿 alpha；
- 保留 LICENSE、identifier、deep-link、data dir、内部包名和 DMG 背景；不修改
  provider/partner 图标或截图。

### 审计

运行 `rg`，逐项判断残留。不得全局替换内部 `cc-switch`。

## 12. M9 — 退出保护、诊断和精修

建议 commit：

```text
feat(codex-desktop): add exit protection and installer diagnostics
```

- close behavior；
- copy error；
- open logs；
- redaction；
- warning处理；
- temp stale cleanup；
- UI loading/empty/accessibility；
- progress节流。

## 13. M10 — 集成质量门槛

按 `13` 运行完整命令。处理顺序：

1. 格式；
2. TypeScript；
3. 前端测试；
4. Rust fmt；
5. Clippy三平台；
6. Rust tests；
7. endpoint/static审计；
8. 图标源 hash、输出 inventory/尺寸/mode/alpha、About 一致性和无关资产 diff 审计；
9. 无真实包审计；
10. 生成集成报告。

禁止通过：

- 跳过测试；
- `#[allow]` 隐藏新 warning；
- 删除失败用例；
- 将平台代码改为未实现占位但声称完成。

## 14. M11 — 人工验收

- 使用内部测试设备；
- 不由 Agent自动卸载；
- 按 `14`；
- 在真实 Windows/macOS 构建上检查安装器/快捷方式/任务栏或 Dock/About/menu bar 图标；
- 记录应用版本、镜像 release、OS、结果；
- 测试失败回到对应模块，不在验收机手工修改生产文件掩盖。

## 15. 依赖图

```text
M0 → M1 → M2 → M5 → M6
         ↘ M3 ↗
         ↘ M4 ↗
M6 → M7 → M8 → M9 → M10 → M11
```

M3/M4可在 M1 后与 M2并行，但最终需和 M2/M5集成。

## 16. 回滚策略

每个 commit原子。若某平台不可合并：

- 回滚该平台 commit；
- 保留 Core/其他平台；
- 不用 `cfg` 假装支持；
- UI按后端 support显示不可用；
- 但 Windows x64/ARM64/macOS ARM64均为V1目标，最终必须补齐后再签收。

## 17. Agent 不得自行增加的任务

- 通用下载管理器；
- 插件化安装源；
- 数据库任务历史；
- 自动遥测；
- 自建后端；
- Apple helper；
- Windows service；
- 安装中心新页面；
- 自动登录；
- Provider自动配置；
- 全仓库品牌重命名；
- 修复/卸载/rollback UI。
