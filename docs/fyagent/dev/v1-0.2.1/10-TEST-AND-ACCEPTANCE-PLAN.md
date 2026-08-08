# 10 — 测试与验收计划

## 1. 目的

本文定义 FyAgent `0.2.1` 的测试层级、环境、用例和发布证据。测试必须同时证明：

1. Windows 安装目录选择可用且安全；
2. 旧 Error 1720 根因已移除；
3. UI、静默、升级和修复使用同一目录策略；
4. FyAgent 全局版本在 Windows、macOS、Linux、标签和资产中一致；
5. 一键版本脚本不会误改依赖或历史文本。

## 2. 测试层级

| 层级 | 目标 | 是否可替代下一层 |
|---|---|---|
| Node 单元测试 | 版本脚本解析、更新、回滚 | 否 |
| Rust 单元测试 | 路径规则、错误映射、ACL判定纯逻辑 | 否 |
| Windows Rust 集成测试 | 真实 Win32 路径/ACL/reparse 行为 | 否 |
| WiX 源码静态测试 | 结构和旧脚本回归 | 否 |
| MSI 表验证 | 生成结果的 Binary/CA/UI/Sequence | 否 |
| Windows 生命周期测试 | 真实 msiexec 安装/升级/修复/卸载 | 否 |
| 跨平台包元数据验证 | 各平台内嵌版本 | 否 |
| 发布契约测试 | 标签、SHA、资产名、manifest | 最终门禁 |

任何单一层级都不能独立批准发布。尤其是源码字符串测试和 Wine 构建不能替代真实 Windows Installer 生命周期。

## 3. 测试环境矩阵

### 3.1 Windows

最低覆盖：

| 环境 | 架构 | 权限/文件系统 | 用途 |
|---|---|---|---|
| Windows 11 当前稳定版 | x64 | NTFS，标准用户 + UAC 管理员 | 主功能/安全/生命周期 |
| Windows 11 ARM64 | ARM64 | NTFS | DLL/MSI 架构和安装 |
| Windows Server 或企业版 VM | x64 | 静默部署场景 | `/qn`、服务账户/企业分发 |
| 可选兼容最低版本 | x64 | 项目声明的最低 Windows | 兼容性回归 |

每个 VM 在测试前使用快照恢复，避免旧 ACL、注册表或缓存污染。

### 3.2 macOS/Linux

使用现有支持矩阵，至少覆盖正式发布架构。目标是版本一致性和包可读性，不因本次 Windows 安装器改造改变其他平台行为。

### 3.3 工具

- Node/pnpm：仓库锁定版本；
- Rust：仓库锁定 toolchain/MSRV；
- WiX/Tauri：仓库现有正式版本；
- Windows Installer：目标系统自带；
- MSI 表检查：Windows Installer SDK/WiX 工具或等价只读检查器；
- ACL 构造：PowerShell/`icacls` 仅用于测试 fixture，不用于产品校验。

## 4. 版本脚本单元测试

### 4.1 正常路径

| ID | 操作 | 期望 |
|---|---|---|
| VER-T001 | `get` on `0.2.1` | stdout 仅 `0.2.1` |
| VER-T002 | `check` 合法结构 | 退出 0 |
| VER-T003 | `set 0.2.2` | 仅 canonical + 本地 lock 包改变 |
| VER-T004 | `set` 当前版本 | 幂等，文件字节不变 |
| VER-T005 | `bump patch` | `0.2.1 -> 0.2.2` |
| VER-T006 | `bump minor` | `0.2.1 -> 0.3.0` |
| VER-T007 | `bump major` | `0.2.1 -> 1.0.0` |
| VER-T008 | `--dry-run` | 输出计划但不写入 |
| VER-T009 | `check --tag v0.2.1` | 成功 |

### 4.2 精确范围

fixture 中同时放置：

```text
dependency-a version = 0.2.1
historical CHANGELOG says 0.2.1
schemaVersion = 0.2.1
packageManager = pnpm@10.12.3
```

执行 `set 0.2.2` 后，这些内容必须逐字不变。

### 4.3 拒绝路径

| ID | 输入/状态 | 期望 |
|---|---|---|
| VER-T010 | `v0.2.2` | 拒绝 |
| VER-T011 | `0.2.2-beta.1` | 拒绝 |
| VER-T012 | `0.2.2+build` | 拒绝 |
| VER-T013 | `00.2.2` | 拒绝 |
| VER-T014 | `256.0.0` | MSI 上限拒绝 |
| VER-T015 | `0.256.0` | MSI 上限拒绝 |
| VER-T016 | `0.0.65536` | MSI 上限拒绝 |
| VER-T017 | 缺失 `[workspace.package]` | 拒绝，不写文件 |
| VER-T018 | 两个 version 字面量 | 拒绝 |
| VER-T019 | 主 package 未继承 workspace | 拒绝 |
| VER-T020 | `package.json.version` 被加入 | `check` 失败 |
| VER-T021 | `tauri.conf.json.version` 被加入 | `check` 失败 |
| VER-T022 | helper manifest 存在但 lock 条目缺失 | 失败 |
| VER-T023 | 标签为 `v0.2.2`，source 为 `0.2.1` | 失败 |
| VER-T024 | 模拟写第二文件失败 | 第一文件恢复 |
| VER-T025 | 后验校验失败 | 全部恢复 |

### 4.4 项目级测试

在实际仓库执行：

```bash
node --test tests/version.test.mjs
pnpm run version:check
pnpm run version:set -- 0.2.1 --dry-run
```

然后用 `git diff --exit-code` 验证 dry-run 无改动。

## 5. Rust 策略单元测试

### 5.1 纯路径/规则

| ID | 输入 | 期望错误/结果 |
|---|---|---|
| DIR-U001 | `C:\Program Files\FyAgent` | 允许（在受控 fixture/模拟层） |
| DIR-U002 | 相对路径 | `InvalidPath` |
| DIR-U003 | `C:relative` | `InvalidPath` |
| DIR-U004 | UNC | `NetworkPath` |
| DIR-U005 | `\\?\UNC\...` | `NetworkPath` |
| DIR-U006 | 卷根 `D:\` | `RootDirectory` |
| DIR-U007 | 用户 profile 子目录 | `UserControlledLocation` |
| DIR-U008 | Windows/System32 | `SystemDirectory` |
| DIR-U009 | Temp | `TemporaryLocation` |
| DIR-U010 | 尾随点/空格/设备名歧义 | 规范化失败或明确拒绝 |
| DIR-U011 | FyAgent 专用空目录 | 允许 |
| DIR-U012 | 非空未知目录 | `DirectoryNotDedicated` |
| DIR-U013 | 可识别旧 FyAgent 安装 | 按升级上下文允许 |

### 5.2 错误映射

每个 `PolicyError` 必须有：

- 稳定日志 code；
- UI 文案类别；
- 是否可重试；
- 不泄漏 SID/ACL/系统原文的用户消息。

测试确保新增枚举未映射时编译失败或测试失败。

### 5.3 FFI/MSI wrapper

使用抽象 session 接口或测试替身验证：

- 正确读取 `INSTALLDIR`；
- 调用前清空旧属性；
- 成功设置 `VALID=1`；
- 策略拒绝 UI 入口仍返回 MSI success；
- Execute 入口为 Type 19 准备固定消息；
- 属性写失败、panic、无效 handle fail closed；
- panic 不跨越 FFI 边界。

## 6. Windows 路径与 ACL 集成测试

这些测试必须在 Windows 上创建真实目录树。

### 6.1 卷类型

| ID | 场景 | 期望 |
|---|---|---|
| DIR-W001 | 本地 NTFS 固定磁盘 | 进入后续检查 |
| DIR-W002 | 映射网络盘 | 拒绝 |
| DIR-W003 | UNC share | 拒绝 |
| DIR-W004 | USB/可移动盘 | 拒绝 |
| DIR-W005 | 不支持持久 ACL 的文件系统 | 拒绝 |

### 6.2 重解析点

创建：

- 目标目录本身 junction；
- 中间祖先 junction；
- symlink；
- mount point；
- 校验后快速替换的竞争场景。

全部必须拒绝或在执行序列重验时捕获。至少验证使用 `OPEN_REPARSE_POINT` 不会无意跟随目标。

### 6.3 ACL

构造目录：

| ID | ACL | 期望 |
|---|---|---|
| ACL-W001 | SYSTEM/Admin full，Users read/execute | 允许 |
| ACL-W002 | Users write | 拒绝 |
| ACL-W003 | Authenticated Users modify | 拒绝 |
| ACL-W004 | Everyone full | 拒绝 |
| ACL-W005 | 普通用户 `DELETE_CHILD` on parent | 拒绝 |
| ACL-W006 | 普通用户 WRITE_DAC | 拒绝 |
| ACL-W007 | 普通用户 WRITE_OWNER | 拒绝 |
| ACL-W008 | unknown/unresolvable owner | 拒绝 |
| ACL-W009 | owner 为管理员/SYSTEM/TrustedInstaller | 进入后续判定 |
| ACL-W010 | deny/allow 顺序复杂 | Authz 有效权限为准 |
| ACL-W011 | NULL DACL | 拒绝 |
| ACL-W012 | 无法读取 security descriptor | 拒绝 |

不要只检查 ACE 是否“看起来安全”；应测试最终有效权限。

### 6.4 路径缺失组件

测试最近存在祖先：

```text
D:\Applications exists, FyAgent absent
D:\ exists, Applications/FyAgent absent
```

只有当普通用户无法在竞争窗口抢先创建/替换首个缺失组件时才允许。对磁盘根的默认宽松权限要按实际有效权限处理，不能仅凭路径名称允许。

## 7. WiX 源码与 MSI 表测试

### 7.1 源码静态测试

断言不存在：

```text
Script="vbscript"
Win32_LogicalFileSecuritySetting
Scripting.FileSystemObject
GetSecurityDescriptor
FyAgentInstallDirectoryPolicy
```

断言存在：

```text
WIXUI_INSTALLDIR
ConfigurableDirectory="INSTALLDIR"
FyAgentInstallerActions
ValidateFyAgentInstallDirUi
ValidateFyAgentInstallDirExecute
FyAgentUnsafeInstallDirDlg
```

### 7.2 MSI 表

导出并保存为 CI artifact：

```text
Property.idt
Binary.idt
CustomAction.idt
InstallUISequence.idt
InstallExecuteSequence.idt
Dialog.idt
Control.idt
ControlEvent.idt
Directory.idt
Component.idt
Feature.idt
Registry.idt
Upgrade.idt
```

验证：

- Type 1 actions 指向同一个 Binary；
- 无 FyAgent script CA type；
- UI event order 与设计一致；
- Execute action 在 `InstallValidate` 前；
- uninstall 条件排除；
- `INSTALLDIR` 仍可配置；
- HKLM InstallDir 保存；
- PermissionEx/MsiLockPermissionsEx 存在；
- ProductVersion 为 `0.2.1`；
- MSI 平台与 DLL PE Machine 一致。

## 8. Windows MSI 用户流程测试

### 8.1 首次安装

| ID | 场景 | 期望 |
|---|---|---|
| MSI-I001 | 打开 MSI | 不出现 1720，进入欢迎页 |
| MSI-I002 | 默认 Program Files | 可继续并安装成功 |
| MSI-I003 | 浏览到安全 `D:\Applications\FyAgent` | 安装成功 |
| MSI-I004 | 编辑路径后返回/重进 | 使用最新值重新校验 |
| MSI-I005 | 失败后换安全目录 | 无需重启安装器即可成功 |
| MSI-I006 | 用户目录 | 简单错误，停留目录页 |
| MSI-I007 | Temp | 简单错误，停留目录页 |
| MSI-I008 | UNC | 简单错误，停留目录页 |
| MSI-I009 | junction 路径 | 简单错误，停留目录页 |
| MSI-I010 | 非空未知目录 | 简单错误，停留目录页 |
| MSI-I011 | 取消安装 | 无残留产品注册/文件 |

用户界面不得显示 ACL、SID、WMI、脚本、Custom Action 或内部统计数量。

### 8.2 静默安装

```cmd
msiexec /i FyAgent-0.2.1-Windows.msi /qn /L*V log.txt INSTALLDIR="..."
```

| ID | 目录 | 期望 |
|---|---|---|
| MSI-Q001 | 默认/安全目录 | 退出 0，安装成功 |
| MSI-Q002 | 用户 Desktop | 非零，未安装 |
| MSI-Q003 | UNC | 非零 |
| MSI-Q004 | 可写父目录 | 非零 |
| MSI-Q005 | reparse | 非零 |
| MSI-Q006 | 空值/格式错误 | 非零 |

日志必须包含稳定错误码和 `Return value 3` 附近的明确原因，但不能只显示通用 1720。

### 8.3 竞态重验

自动化流程：

1. UI 选择安全目录并通过；
2. 在执行开始前改变父目录 ACL 或替换为 junction；
3. Execute Sequence 必须拒绝；
4. 不复制主 EXE；
5. MSI 回滚无残留。

该测试证明 UI 校验不是安全边界。

## 9. 升级、修复、卸载

### 9.1 `0.2.0 -> 0.2.1`

| ID | 场景 | 期望 |
|---|---|---|
| MSI-U001 | 0.2.0 在默认目录 | 原地升级，版本 0.2.1 |
| MSI-U002 | 0.2.0 在安全自定义目录 | 恢复原目录，不搬迁 |
| MSI-U003 | 外部传另一个 INSTALLDIR | 不改变既有安装位置 |
| MSI-U004 | 旧目录已变不安全 | 阻断并给重新安装建议 |
| MSI-U005 | 用户数据/配置 | 按现有产品策略保留 |

### 9.2 修复

- 不显示目录选择；
- 使用已注册目录；
- 安全目录修复成功；
- 目录安全边界破坏时失败，不静默接管父目录；
- 修复后版本和签名不变。

### 9.3 卸载

- 不运行目录准入阻断；
- 正常和部分损坏路径都可尽力卸载；
- 产品注册、快捷方式、服务/任务和安装文件按当前策略清理；
- 用户数据是否保留按既有需求，不在本次擅自改变。

## 10. 架构测试

| ID | MSI | Helper DLL | 期望 |
|---|---|---|---|
| ARCH-001 | x64 | x64 | 成功 |
| ARCH-002 | ARM64 | ARM64 | 成功 |
| ARCH-003 | x64 | ARM64 | 构建/静态门禁失败 |
| ARCH-004 | ARM64 | x64 | 构建/静态门禁失败 |

检测 PE Machine 字段，不依赖文件名判断。

## 11. 跨平台版本验收

### 11.1 构建前

```bash
pnpm run version:check -- --tag v0.2.1
```

### 11.2 产物

| 对象 | 期望 |
|---|---|
| Windows MSI ProductVersion | `0.2.1` |
| Windows EXE version | `0.2.1` 数值投影 |
| macOS Info.plist | `0.2.1` |
| DEB metadata | `0.2.1`（允许明确 revision） |
| RPM metadata | `0.2.1` |
| AppImage diagnostic | `0.2.1` |
| 运行时 `getVersion()` | `0.2.1` |
| 资产名 | `FyAgent-0.2.1-*` |
| Git tag | `v0.2.1` |
| manifest.version | `0.2.1` |
| manifest.tag | `v0.2.1` |
| source SHA | 所有产物相同 |

### 11.3 负向发布测试

- Cargo `0.2.1` + tag `v0.2.2`：version-contract 失败；
- 某平台文件名含 `v0.2.1`：命名检查失败；
- macOS 内嵌 `0.2.0`：发布失败；
- manifest 从另一个 run 混入资产：SHA/provenance 失败；
- 任何 build job试图修改 version：工作树检查失败。

## 12. 回归测试

除新测试外，必须执行现有：

```bash
pnpm run typecheck
pnpm run format:check
pnpm run test:unit
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo clippy --workspace --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --workspace --manifest-path src-tauri/Cargo.toml
```

以及仓库已有 desktop acceptance、macOS cross、release workflow 测试。具体命令以最终 `package.json` 和 CI 为准。

## 13. 安全测试与人工审查

人工安全审查至少检查：

- 权限检查是否使用有效权限而非简单 ACE 包含判断；
- 父目录 `DELETE_CHILD` 与抢先创建风险；
- 所有 reparse point 处理；
- 长路径、路径规范化和设备命名空间；
- Rust unsafe/FFI buffer 生命周期；
- MSI immediate CA 权限和属性边界；
- UI 属性是否可由命令行伪造，但 Execute仍重验；
- 日志是否泄漏用户敏感信息；
- 签名发生在最终字节完成后。

建议至少一名未参与实现的人审查 `04` 中的安全不变量和 Windows API 调用。

## 14. 测试证据

每个正式 release run保存：

```text
version-contract.txt
source-sha.txt
node-version-tests.tap
rust-tests-*.txt
msi-table-dumps/
msi-install-default.log
msi-install-custom.log
msi-reject-*.log
msi-upgrade.log
msi-repair.log
msi-uninstall.log
embedded-version-report.json
signature-verification.txt
checksums.txt
download-manifest.json
```

敏感信息清理后上传为受限 CI artifact，并设置合理保留期。

## 15. 进入与退出条件

### 15.1 进入候选发布

- 源码完成 review；
- version-contract 通过；
- Node/Rust/WiX 静态测试通过；
- x64/ARM64 候选 MSI 成功生成；
- 无无关依赖版本 diff。

### 15.2 退出并批准发布

- 全部阻断用例通过；
- Windows 原生生命周期通过；
- 不安全目录负向测试全部拒绝；
- 所有正式平台内嵌版本一致；
- 签名、公证、哈希验证通过；
- 发布 manifest 只包含已验收产物；
- 残余风险已记录并接受。

## 16. 不可降级的发布门禁

以下理由不能用于跳过测试：

- “跨平台脚本已经成功生成 MSI”；
- “源码中看起来没有 VBScript”；
- “默认目录能安装”；
- “UI 已经校验过”；
- “文件名是 0.2.1”；
- “Cargo 和 tag 看起来一致”；
- “只改了小版本”。

必须以生成产物和真实生命周期证据为准。
