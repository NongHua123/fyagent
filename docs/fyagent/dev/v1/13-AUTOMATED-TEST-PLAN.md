# 自动化测试计划

## 1. 范围

自动化测试验证逻辑、协议、状态和平台封装，不执行真实生产安装。测试目标是快速、确定、可在现有 Ubuntu/Windows/macOS CI 运行。

## 2. 强制质量命令

```bash
pnpm install --frozen-lockfile
pnpm typecheck
pnpm format:check
pnpm test:unit

cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

现有 `.github/workflows/ci.yml` 已覆盖：

- frontend: Ubuntu；
- backend: Ubuntu、Windows、macOS。

V1 不新增专用 workflow，除非现有 workflow因 target feature无法覆盖且集成 Agent提供证据。优先扩展现有测试。

## 3. 禁止的自动化行为

- 下载完整生产 MSIX/DMG；
- 安装/更新/卸载 OpenAI 应用；
- 写 `/Applications`；
- 调用真实 PackageManager部署；
- 触发 UAC；
- 杀死 ChatGPT/Codex；
- 修改真实 `~/.codex`；
- 依赖公网实时内容决定测试成败；
- 依赖特定当前版本号。

## 4. 测试分层

```text
大量纯单元
  +
适量 service/fake 集成
  +
前端组件/Hook
  +
三平台编译
  +
人工真实安装
```

不设置数字覆盖率门槛，但新解析器、状态机、版本比较、错误映射必须覆盖核心分支。

## 5. Core 领域测试

### 5.1 版本

Windows：

- `1.2.3.4`；
- 大小比较每一段；
- 非数字、负数、段数错误、溢出；
- display version不参与。

macOS：

- `1`, `1.2`, `1.2.0`, `1.10`；
- 前导零策略；
- 非数字夹具；
- 不可比较失败。

### 5.2 Release ID

- 字段顺序稳定；
- hash大小写规范；
- 任一字段变化导致 ID变化；
- unknown remote字段不影响；
- download redirect URL不参与，避免短期 URL使 ID漂移。

### 5.3 Job

- 单任务互斥；
- 合法转换；
- 非法转换；
- cancellable计算；
- cancel before install；
- cancel/install竞态；
- terminal不可更新；
- stale job update拒绝；
- event sink失败不终止任务。
- settings restart 与 start_install 对同一 job 槽的互斥；默认 capability 不得包含
  `process:allow-restart`，使渲染层不能绕过 `restart_app` 的 restart claim；已有显式
  退出路径保留最窄的 `process:allow-exit`，并继续走应用 exit guard。

## 6. Source 测试

使用脱敏 fixture：

- 当前平台完整；
- unknown fields；
- 其他平台缺失但当前平台完整；
- 当前平台必需字段缺失；
- ARM64 catalog-only；
- 每架构不同版本；
- invalid size；
- invalid version；
- duplicate platform entries；
- checksum exact filename；
- `  filename` 与 ` *filename`；
- invalid hash；
- duplicate conflicting hash；
- missing checksum。

缓存：

- hit <5min；
- expired；
- refresh bypass；
- platform/arch key隔离；
- failure不污染成功；
- install revalidation detects change。

## 7. HTTP/Downloader 测试

使用本地 mock HTTP server：

### Redirect

- 0–5 HTTPS模拟（在测试可抽象 scheme policy）；
- 第6次拒绝；
- HTTPS→HTTP拒绝；
- userinfo URL拒绝；
- relative Location；
- query不写日志。
- 固定 manifest/checksum 的合规 HTTPS redirect（当前 R2 跳转）可达；不安全、缺失
  `Location` 或第 6 跳在请求前被拒绝，且最终 URL 不进入 DTO/日志 query。
- metadata 的 redirect-policy 拒绝必须映射为不可重试的 `REDIRECT_REJECTED` 和
  `OpenLogs`，不能降级为可重试的 `SOURCE_UNAVAILABLE`。

如果本地 TLS fixture成本过高，将 redirect policy抽成纯函数测试，并对 Client builder做构造测试；不降低生产策略。

### Retry

- connection error；
- 408；
- 429 + bounded Retry-After；
- 500/503；
- 404不重试；
- checksum不重试；
- 总尝试3。

### Stream

- 正常长度；
- chunked；
- body truncated；
- 超 expected size；
- cancellation；
- `.part` cleanup；
- progress throttling；
- second attempt从0开始。

## 8. Verification 测试

- SHA match/mismatch；
- expected hash格式；
- size mismatch；
- 包解析/平台验证后、实际 Windows deploy 或 macOS mount 前替换同长度 artifact：重新
  校验必须失败，且 fake deploy/attach 不得被调用；
- `size * 3` checked arithmetic；
- temp/target同卷去重；
- 两卷任一不足；
- release drift；
- invalid file type；
- redaction。

文件系统和 free-space查询必须可注入 fake，避免依赖 CI机器真实空间。

## 9. Windows 测试

### 9.1 Manifest fixtures

- Stable x64；
- Stable ARM64；
- Beta；
- wrong publisher；
- wrong name；
- wrong arch；
- multiple namespaces；
- missing Application Id；
- multiple Applications（确定选择规则）；
- min OS；
- malformed XML；
- XML external entity拒绝。

### 9.2 Deployment facade

Fake scenarios：

- current-user success；
- progress；
- package in use；
- policy blocked；
- dependency missing；
- unknown HRESULT；
- post-query missing；
- post-query higher version；
- launch success/fail。

### 9.3 Elevation

- UAC cancelled；
- nonce mismatch；
- path outside root；
- symlink/reparse replacement；
- hash changed；
- identity changed；
- stage fail；
- provision unsupported；
- elevated child不写入 user-temp 结果文件；
- fixed drive / reparse source handle / protected staging 的 Windows HIL 验证；
- ordinary IPC cannot invoke all-users。

Windows runner只运行 fake，不改变系统 package inventory。

## 10. macOS 测试

### 10.1 Bundle

- Stable；
- Classic；
- Beta；
- wrong Team；
- wrong arch；
- min OS；
- build version；
- unreadable plist。
- malformed unrelated `.app` 不阻断 valid Stable，且单独存在时为 NotInstalled；
- probe 已识别为 Stable 后，严格字段缺失/损坏仍为 fail closed；
- 记录的 ARM64 identity/provenance fixture 与 exact Bundle ID、Team allowlist、版本、
  DMG hash 和 launcher 架构一致；fixture 必须明确 native `codesign`/`spctl` 尚未在
  Windows 上取证，且不得成为运行时信任输入。

### 10.2 Scan matrix

- none；
- `/Applications` one Stable；
- `~/Applications` one Stable；
- both Stable；
- Classic only；
- Classic path collision + user path free；
- both paths conflict；
- Beta + Stable。

### 10.3 Command runner

- hdiutil plist success/fail；
- multiple mount entities；
- no app/multiple app；
- codesign fail；
- Team mismatch；
- spctl fail；
- ditto fail；
- detach fail warning；
- timeout；
- args with spaces/no shell injection。

### 10.4 Safe replace

在 temp root模拟：

- new install；
- update；
- rename failure before swap；
- failure after old backup；
- restore success/failure；
- symlink target拒绝；
- cleanup boundaries。

## 11. Service 集成测试

以 FakeSource/FakeDownloader/FakePlatform：

1. install happy path；
2. update happy path；
3. local same/newer direct `start_install`：服务端重新检测并只启动可信 Stable 应用；断言零下载、零 preflight、零安装、零临时目录，绝不重装或降级；
4. metadata changed；
5. disk fail；
6. download retry then success；
7. cancel；
8. checksum fail；
9. platform signature fail；
10. install fail；
11. post verify fail；
12. event sequence；
13. temp cleanup；
14. remote fail local launch；
15. job already running；
16. source/platform flow panic：发布 `Failed(INTERNAL_ERROR)`、清理已创建的受控临时目录，且旧 job 不再阻塞新的 `start_install` 或 `claim_restart()`。

断言每条路径的 terminal state和稳定错误码。

## 12. IPC 测试

- DTO camelCase；
- start request只有 expectedReleaseId；
- URL/scope不可传；
- structured error serialization；
- `get_job` Option；
- command调用同一 AppState service；
- all-user命令不在 `generate_handler!`。

## 13. 前端测试

### 13.1 Hook

- initial local/latest/job；
- event after initial query；
- event before query response（新快照获胜）；
- unmount不cancel；
- success Toast一次；
- terminal invalidation；
- retained `Succeeded` + force refresh 的不同 release ID：重新派生 `ready_update`、以新的 expected release ID start，且不误 launch 旧安装；
- copy error；
- refresh true；
- remote failure local preserved。

### 13.2 Card

使用表驱动覆盖 `09` 状态表。验证：

- 按钮 label/disabled；
- cancel visibility；
- progress；
- errors；
- support；
- accessibility。

### 13.3 About/CLI

- `Codex CLI` label；
- 版本显示；
- 无 install/update/repair；
- bulk commands无 Codex；
- 其他工具动作仍存在。

### 13.4 Branding/updater

- 无 UpdateProvider自动调用；
- 无上游 update banner/button；
- FyAgent标题；
- `assets/fyagent.png` 与获批输入的 SHA-256、1024×1024、RGBA 和透明度一致；
- Tauri CLI 标准图标 inventory 完整且 `64x64.png` 存在，所有既有应用品牌路径均已变更；
- About 图标与生成的 32×32 PNG 字节一致；
- macOS template 为 24×24、48×48、72×72 黑色 RGBA，非透明边界等比位于 18pt 内容框，
  alpha 包含透明、实心和抗锯齿值；
- `dmg-background.png`、provider/partner 图标、截图等排除资产无 diff；
- release workflow 无 Tauri updater signing key、updater artifact、`latest.json` 或旧品牌/旧官网发布面；仅发布 FyAgent 手动安装资产；
- 不测试许可证文本被删除。

## 14. 静态审计测试

可加入小型单元/脚本但不要过度：

- 新安装器 source只含 agentsmirror；
- 无用户 URL input；
- all-user不在 IPC；
- no production package fixtures；
- no `PowerShell Add-AppxPackage` fallback；
- no `open -a ChatGPT`；
- no `pgrep -x`/killall。
- 图标检查必须解析 PNG/ICO/ICNS 元数据和 alpha，而不能只比较文件名或非零大小；自动化
  只能证明文件级生成结果，不能替代 Windows shell/安装器或 macOS Dock/menu bar 视觉验收。

若不加入 CI脚本，集成报告必须人工运行 `rg` 审计。

## 15. 测试结果记录

每个命令记录：

```text
command
OS
exit code
summary
failures
```

不得只写“tests pass”而不列命令。

## 16. CI 失败处理

- 平台特有 compile失败由对应 Owner修复；
- 不用全局 `allow(dead_code)`掩盖结构问题；
- Linux不得编译 Windows/macOS系统 import；
- Windows/macOS runner不得执行真实安装；
- flaky公网测试必须改为 fixture/mock，不允许简单重试 CI掩盖。
