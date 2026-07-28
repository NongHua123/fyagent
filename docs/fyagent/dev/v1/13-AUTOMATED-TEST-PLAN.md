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
- result file ACL/path validation；
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
3. local newer no start allowed? `start_install`应验证 action并拒绝降级；
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
15. job already running。

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
