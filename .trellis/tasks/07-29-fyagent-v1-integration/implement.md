# 集成实施计划

1. 审查每个 child 的 contract、owned files、tests 和 dependency request；按冻结顺序合并。
2. 以最小 patch 接入 modules/services/AppState/commands/handler/AppHandle/temp cleanup 和
   scoped proxy builder，补 service/IPC integration tests。
3. 在现有 close/exit coordinator 增加 Job-aware gate，测试 download cancel 与 install wait。
4. 使 Codex CLI backend/UI/bulk/manual command 全部只读，补其他工具回归。
5. 移除 updater 的 config/capability/plugin/commands/frontend/DatabaseUpgrade/release workflow
   产物，补 no-network recovery test。
6. 完成 FyAgent 可见品牌、locale、Header/About/README/发布文本的分类清理；保存精确
   FyAgent 图标源，使用 Tauri CLI 生成应用品牌图标，单独生成 macOS tray template，并
   搜索核验兼容性路径及无关图像未被机械替换。
7. 运行 frontend/Rust 全量质量门、Windows ARM64 target evidence、静态审计与 review；
   记录每一项结果并准备人工验收表，不执行真实安装。

## Required Static Audit

```powershell
rg -n "CC Switch|ccswitch.io|farion1231/cc-switch" src src-tauri README.md --glob '!**/LICENSE*' --glob '!**/*.lock'
rg -n "@openai/codex@latest|npm i -g @openai/codex|volta install @openai/codex" src src-tauri
rg -n "UpdateProvider|useUpdate|tauri_plugin_updater|plugins.*updater|latest.json" src src-tauri .github
rg -n "agentsmirror|github.com|oaistatic|apps.microsoft.com" src-tauri/src/codex_desktop src/components/codex src/lib/api/codex-desktop.ts
```

图标替换还必须核对源文件 hash/尺寸/mode/alpha、全部已跟踪应用品牌路径的变更、各输出
尺寸与透明度、About 32×32 字节一致性、macOS template 黑色轮廓和 18pt 内容框，以及
`dmg-background.png`、provider/partner 图标、截图等排除项未变。原生 Windows shell/安装器、
macOS Dock/菜单栏的视觉正确性保留给人工验收。

## 2026-07-30 应用品牌图标替换记录

以下命令均在 Windows 开发主机执行；构建了未签名 MSI，但没有安装或启动原生应用，也没有
进行 Windows shell、macOS Dock/菜单栏视觉验收。

| 类别 | 命令 / 检查 | 结果 |
| ---- | ----------- | ---- |
| 源图 | `Get-FileHash C:\Users\Administrator\Desktop\fyagent.png -Algorithm SHA256`；Pillow 读取外部输入与 `assets/fyagent.png` 的 PNG metadata/alpha | 退出码 0；两者 SHA-256 均为 `17236EBB0DD38D8A9FE5C4BA8D1621E4048909B86F1BD8C88BA55E8DBA63C9BF`；1024×1024、RGBA、alpha 0..255，含抗锯齿值 |
| 标准集合 | `pnpm tauri icon assets/fyagent.png --output src-tauri/icons` | 退出码 0；生成桌面、Windows Store、Android、iOS 共 50 个既有路径，包含 `64x64.png`；未新增额外生成路径 |
| About / tray | `Copy-Item src-tauri\icons\32x32.png src\assets\icons\app-icon.png -Force`；Pillow 按源 alpha 非透明边界生成三个 macOS template | About 与 32×32 输出 SHA-256 同为 `A0E4AC31157CAA5B9DD893A38A558B3BD506A6DDD37A61174BFB05EE12B54C19`；template 为 24/48/72 RGBA、RGB 全黑、alpha 0..255 且含部分透明值，非透明 bbox 分别为 `(4,3,20,21)`、`(8,6,41,42)`、`(12,9,61,62)` |
| 文件级验证 | 内联 Python/Pillow + `git ls-files`/`git diff --name-only` 校验 source、inventory、PNG/ICO/ICNS、About、template、排除资产与 ZIP | 退出码 0；53/53 个既有应用品牌路径均变化；51/51 PNG 可解码且为 RGBA；ICO 含 16/24/32/48/64/256；ICNS 最大 1024×1024；DMG 背景、partner、截图和其他 renderer icons 无 diff；`v1.zip` 19/19 Markdown 逐字节一致 |
| Diff | `git diff --check` | 退出码 0 |
| 前端 / Rust | `pnpm format:check`；`pnpm typecheck`；`pnpm build:renderer`；`cargo check --manifest-path src-tauri/Cargo.toml` | 全部退出码 0；renderer 构建 3305 modules / 13.36s，仅既有 warnings；cargo check 13.40s |
| 聚焦回归 | `pnpm exec vitest run tests/releaseWorkflow.test.ts tests/components/AboutSection.test.tsx` | 退出码 0；2 files / 6 tests 通过 |
| Windows MSI | `pnpm tauri build --bundles msi` | 退出码 0；424.3s；`FyAgent_3.18.0_x64_en-US.msi` 为 12,791,808 bytes，SHA-256 `31C180695E45575A06D624CD0D05425D9657FAFB9E4A1DBA4102069D1D2ED3DF`；WiX `candle`/`light` 成功，Authenticode `NotSigned`，未安装。构建仅输出已知的 `__TAURI_BUNDLE_TYPE`/updater warning；V1 已禁用 updater |

长期维护约束已写入 `.trellis/spec/backend/application-brand-assets.md`，并从 frontend/backend
规范索引共同引用；ICNS 容器重生成的原始字节不稳定时必须比较可解码尺寸/像素，而不是把
容器 byte equality 当作门槛。

上述仅证明源文件与生成资产的静态一致性。Windows 安装器/快捷方式/任务栏和 macOS
Finder/Dock/应用切换器/menu bar 的实际观感仍为 `Pending human`。

## 2026-07-29 收尾复验记录

分支为 `feature/fyagent-v1`，基线提交为
`2400031a85f6b45b4db7aec89394b997a88826a8`。以下自动化均在 Windows
开发主机执行；没有执行真实下载、安装、UAC、PackageManager 部署、DMG 挂载、卸载或
`/Applications` 写入。

### 审查修正

- 保留 JobStore 的 terminal snapshot 作为审计和一次性成功 Toast 证据；但 `Succeeded`
  job 的 release ID 与成功刷新的 remote release ID 不同时，前端立即回到 local + remote
  的版本派生，因此可再次显示 `ready_update` 并使用新的 expected release ID。回归测试
  同时断言不会误 launch 旧版。
- `run_install_flow` 在保留 `JobTempDir` 的边界内捕获 Rust unwinding panic。panic 先走
  fail-closed cleanup，再发布 `Failed(INTERNAL_ERROR)`，释放后续安装和受控 restart 的
  job 槽；不把 panic payload 写入日志或 DTO。`panic=abort`、进程强制终止、OOM/SEH 和
  该边界外的 runtime 崩溃仍不能由进程内代码结算。
- Windows Run 注册表 value name 仍使用 `CC Switch`，确保既有自启记录仍可由
  FyAgent 控制；它不是可见品牌。其余 V1 要求的用户可见品牌、官网和 updater 面已按
  分类清理，兼容标识、历史数据和注释另行保留。

### 当前本地证据

| 类别             | 命令 / 检查                                                                                                                                                                                                                                                                                                         | 结果                                                                                                                                                                                                            |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 前端依赖与质量   | `pnpm install --frozen-lockfile`；`pnpm typecheck`；`pnpm format:check`；`pnpm test:unit`；`pnpm build:renderer`                                                                                                                                                                                                    | 全部退出码 0                                                                                                                                                                                                    |
| Rust 全量质量    | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`；`cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`；`cargo test --manifest-path src-tauri/Cargo.toml`                                                                                                                                  | 全部退出码 0；完整 Rust suite 输出 2346 tests，所有 integration test binaries 完成                                                                                                                              |
| Windows x64      | `cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc`；`cargo test --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --lib codex_desktop::`；`cargo test --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --lib services::codex_desktop::tests` | 全部退出码 0；156/156 installer-domain tests、11/11 service tests 通过                                                                                                                                          |
| 前端关键回归     | `pnpm exec vitest run tests/hooks/useCodexDesktopInstaller.test.tsx`                                                                                                                                                                                                                                                | 0；18 tests 通过，覆盖成功后 refresh 新 release 的再次更新                                                                                                                                                      |
| Rust panic 回归  | `cargo test --manifest-path src-tauri/Cargo.toml --lib services::codex_desktop::tests -- --nocapture`                                                                                                                                                                                                               | 0；11 tests 通过，覆盖 source/platform fake panic、临时目录 cleanup、restart/start slot release                                                                                                                 |
| Windows MSI 打包 | 在全新的 `src-tauri/target/v1-msi` 下运行 `pnpm tauri build --bundles msi`                                                                                                                                                                                                                                          | 0；Tauri `candle`/`light` 成功，产物 `FyAgent_3.18.0_x64_en-US.msi`（12,718,080 bytes，SHA-256 `49214C116A9DFE0D1E7FF1CE2A8EA1665FEF3F0961F1A613B6F842D046063948`）；生成 WiX 不含 `cc_switch_lib.dll` resource |
| 文档包           | `docs/fyagent/dev/v1.zip` 对 `docs/fyagent/dev/v1/*.md` 的逐文件 SHA-256 对比                                                                                                                                                                                                                                       | 19 个源 Markdown、19 个 archive 文件，全部位于 `v1/`，无 hash mismatch                                                                                                                                          |
| 静态审计         | Required Static Audit 与 capability/旧品牌精确 `rg`                                                                                                                                                                                                                                                                 | 旧 Codex CLI 安装命令、updater 运行链均无命中；仅允许 AgentsMirror 端点；`createUpdaterArtifacts: false`；生产 capability 仅有 `process:allow-exit`。`CC Switch` 字面量仅剩兼容 Run value name 与负向断言       |

完整 Vitest 和 Rust 测试仍会输出已有的 browsers data、`punycode`、mock 预期错误、测试
dead-code 等警告；上述命令均成功退出，且 `cargo clippy -D warnings` 已通过。

### P1/P2 修复后的独立复审

对 retained `Succeeded` 不遮蔽新 release、以及 installer worker panic 收敛的调用链和
回归测试进行了独立静态复审，未发现可证实的 P0/P1/P2 问题。前者只在远端
`releaseId` 改变时让 UI 按当前远端信息重新派生状态并使用新 ID；后者在
`run_install_flow` 的可展开 panic 路径中清理临时目录并结算 `INTERNAL_ERROR`，且不会让
已替换 job 的旧 worker 覆盖新 job。该结论不替代真实 Tauri 多窗口、进程 abort/原生崩溃、
OOM 或异常文件系统对象的运行时证据。

### 规范收束

- `.trellis/spec/frontend/state-management.md` 已移除把 `UpdateProvider` 和已忽略宿主更新
  版本当作现役状态的旧描述，并记录 V1 的无 updater/no-network `DatabaseUpgrade` 恢复边界。
  其余 IPC、状态机、错误、重启和平台验证契约已经由
  `.trellis/spec/backend/codex-desktop-installer.md` 与 V1 文档的对应章节覆盖，未重复抄写。

### Windows MSI 收束

- V1 是 desktop-only 范围，`Cargo.toml` 不产生未被桌面宿主消费的 `cdylib`。这是为了避免
  Tauri WiX bundler 把 release DLL 自动作为 per-user resource，并以 file KeyPath 触发
  ICE38；不是通过关闭 ICE 校验绕过问题。
- 自定义 per-user WiX template 对主程序和未来显式 bundled binary 使用 `File KeyPath="no"`
  与 HKCU registry KeyPath。新建的忽略 target 已完成 `candle`/`light` 链接，且生成的
  `main.wxs` 不含 `cc_switch_lib.dll`。本地 MSI 的 Authenticode 状态为 `NotSigned`，不应
  视为发布签名或真实安装成功。

### Windows x64 生产包 ZIP64 兼容修复

- 2026-07-29 的 Windows 10 22H2 x64 人工尝试在 `verifying_download` 阶段返回
  `PACKAGE_PARSE_FAILED`，诊断为 `multi-disk or ZIP64 MSIX archives are unsupported`。
  该失败发生在 FyAgent 自有预检器，尚未调用 Windows PackageManager；设备唯一标识不属于
  故障条件，也不写入仓库记录。
- 对固定 `win-x64` 端点只执行 HEAD 与精确 Range 取证，没有下载完整 744,080,244-byte
  MSIX。包是单磁盘 ZIP64：classic EOCD 使用 sentinel；ZIP64 EOCD 为固定 56 bytes，
  `record size=44`、`entries=9548`、`cdSize=1295228`、`cdOffset=742784918`、
  `recordOffset=744080146`，locator 声明 disk 0 / total disks 1。中央目录 9,548 条完整扫描
  恰好消费 1,295,228 bytes，未发现非零 disk-start、ZIP64 local-header offset、加密条目、
  不安全或重复名称。
- Range 读取的根 manifest 标识为 `OpenAI.Codex` x64 `26.721.4979.0`，
  `TargetDeviceFamily MinVersion=10.0.19041.0`；报告设备 build 19045 高于此门槛。中央目录
  声明压缩总量 741,413,378 bytes、解压总量 1,948,467,324 bytes，因此旧 512 MiB 聚合
  声明上限会在 ZIP64 修复后形成第二个串联阻断。
- 解析器现接受严格有界的单磁盘 ZIP64，同时继续拒绝多磁盘、缺失/错位/可扩展 ZIP64
  记录、classic/ZIP64 双元数据冲突、目录间隙/越界/超限与条目级非零 disk-start。聚合声明
  上限调整为与现有 MSIX 文件上限一致的 4 GiB；实际仍只解压最多 512 KiB 的根 manifest，
  哈希、签名、身份、架构、最低系统版本和 PackageManager 信任边界均未绕过。
- 独立复审后的证据：`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、
  `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`、manifest tests 7/7、
  Windows platform tests 23/23 与 `git diff --check` 均退出码 0。尚未用修复后的 FyAgent
  完整下载生产包或执行 PackageManager 部署，所以 Windows x64 人工矩阵仍是 Pending human。

### 未闭合的人工/外部证据

- `aarch64-pc-windows-msvc` target 已安装，但本机 ARM64 check 在 `aws-lc-sys` C 阶段因
  缺少 ARM64 MSVC `cl.exe` 退出；本机 VS2019 Build Tools 仅有 x86/x64 工具，未安装
  `Microsoft.VisualStudio.Component.VC.Tools.ARM64`。这不是 ARM64 编译通过的证据，也
  不应归因为 V1 Rust 源码。
- 仍需人工执行 Windows x64、Windows ARM64、Apple Silicon macOS 和中国大陆网络的
  `14-MANUAL-ACCEPTANCE.md` 矩阵；all-users 实验还需真实 UAC、ProgramData ACL、
  reparse/中间目录替换、UNC/映射盘和签名 MSIX HIL。
- 真实 macOS `codesign`、`spctl`、Gatekeeper、DMG 挂载/替换，以及 GitHub Actions 的
  签名、公证、打包产物尚无成功外部证据。任务保持 `in_progress`，自动化不等同于平台
  验收完成。
