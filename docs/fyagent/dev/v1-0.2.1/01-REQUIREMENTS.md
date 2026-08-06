# 01 — 需求规格与决策基线

## 1. 文档目的

本文定义 FyAgent `0.2.1` 的需求边界和验收口径。编号需求是后续设计、代码、测试和发布证据的追踪主键。

本版本包含两条相互关联但范围不同的工作流：

- **Windows 安装器工作流**：用户可选择安装目录，且管理员权限应用不能安装到可被普通用户篡改的位置；
- **全局版本治理工作流**：FyAgent 在 Windows、macOS、Linux 和发布系统中使用同一个应用版本，由脚本统一更新。

## 2. 背景和已确认决策

### 2.1 背景

当前 MSI 已包含 WiX 目录选择 UI，但在 UI 到达目录页之前运行内联 VBScript。脚本错误地调用 WMI `GetSecurityDescriptor`，其失败通过 `Err.Raise` 传播为 Windows Installer Error 1720。

当前 FyAgent 应用版本 `0.2.0` 又分别写在 `package.json`、`tauri.conf.json`、`Cargo.toml`、本地 `Cargo.lock` 包块、测试常量和多个构建脚本的读取逻辑中。一次版本更新需要搜索和人工判别，容易漏改或误改依赖/历史文本。

### 2.2 已确认决策

1. 本次直接发布一个补丁版本 `0.2.1`，不拆分过渡版。
2. 用户必须可以选择 Windows 安装目录。
3. 保持 `perMachine + elevated`，不切换为 CC Switch 的 per-user 权限模型。
4. 删除目标机 VBScript 依赖，使用原生 DLL 校验目录。
5. 版本号是 FyAgent 全局应用版本，不是 Windows 专用版本。
6. 版本更新必须有一键脚本；Codex 不再通过全仓库搜索替换完成版本迭代。

## 3. 范围

### 3.1 本次范围

- Windows x64/ARM64 MSI 目录选择、目录安全校验、升级路径保留和生命周期测试；
- FyAgent 全局应用版本的单一真源；
- 一键设置、递增和校验版本的脚本；
- Windows、macOS、Linux 构建脚本和发布工作流的版本契约；
- 下载清单、资产文件名和运行时版本来源；
- 相关单元测试、静态验证和发布证据。

### 3.2 非目标

- 不把 FyAgent 改成 per-user 应用；
- 不允许安装到任意用户可写目录；
- 不在 `0.2.1` 中拆分 GUI 和 Windows 服务/提权 broker；
- 不引入新的自动更新协议；
- 不自动修改历史 CHANGELOG、旧发布说明或归档文档中的版本；
- 不把依赖版本、Rust/Node/pnpm/WiX 工具链版本、数据库 schema 版本、配置格式版本或协议版本纳入 FyAgent 应用版本脚本；
- 不让版本脚本自动提交、打标签、推送或创建 GitHub Release；
- 不以 Wine/Linux 的 MSI 结构检查替代原生 Windows 安装生命周期验证。

## 4. 产品与交互需求

### INS-001 — 保持按机器安装

Windows MSI 必须继续使用：

```xml
InstallScope="perMachine"
InstallPrivileges="elevated"
```

应用继续注册为所有用户可见的机器级产品，安装数据写入 HKLM/ProgramData 等当前设计位置。

### INS-002 — 默认安装目录

首次安装默认目录必须为当前目标架构对应的 Program Files 下的专用 `FyAgent` 子目录。

典型 x64 展示值：

```text
C:\Program Files\FyAgent
```

默认值必须无需用户修改即可通过安全校验。

### INS-003 — 用户可选择安装目录

完整 UI 安装时必须展示目录选择页，并支持：

- 直接编辑路径；
- 点击“浏览”；
- 返回后重新选择；
- 校验失败后留在当前页修正；
- 选择其他本机固定磁盘上的安全 FyAgent 专用目录。

### INS-004 — 目录选择采用 WiX 标准体系

继续使用 `WixUI_InstallDir`、`WIXUI_INSTALLDIR=INSTALLDIR` 和 `ConfigurableDirectory="INSTALLDIR"`。允许复制并定制其 UI Fragment/Publish 事件，但不自研独立 bootstrapper UI。

### INS-005 — 允许目录的产品定义

用户选择的目录必须同时满足：

1. 是规范化后的绝对本地 DOS 路径；
2. 位于本机固定磁盘；
3. 文件系统支持持久 Windows ACL；
4. 不是卷根、Windows 系统目录、用户配置文件或临时目录；
5. 目标路径及既有祖先不包含 junction、symbolic link 或其他重解析点；
6. 最近存在祖先由可信主体控制；
7. 非可信主体不能抢先创建目标的首个缺失组件；
8. 非可信主体不能写入、删除、重命名既有路径组件，也不能修改其 DACL 或所有者；
9. 目标是 FyAgent 专用目录，而不是已有共享软件目录；
10. 任何关键安全信息无法可靠读取时按不安全处理。

### INS-006 — 明确拒绝的目录

至少拒绝：

- UNC、网络共享、映射网络盘；
- 可移动磁盘、光盘、RAM disk；
- 不支持持久 ACL 的文件系统；
- `%USERPROFILE%`、Desktop、Downloads、Documents、AppData、Temp 下的路径；
- Windows/System32/SysWOW64 等系统目录及其不当子路径；
- 磁盘根目录；
- 含重解析点的路径；
- 普通用户或未知主体可写、可删除或可改安全描述符的父目录；
- 非空且不能识别为既有 FyAgent 安装的目录；
- 设备命名空间、驱动器相对路径和语义不明确的路径。

### INS-007 — UI 即时校验

用户点击安装目录页“下一步”时必须按以下顺序执行：

1. 把编辑框值提交到 `INSTALLDIR`；
2. 执行 WiX 标准路径有效性检查；
3. 执行 FyAgent 原生目录安全检查；
4. 通过时进入安装确认页；
5. 未通过时弹出简单提示，关闭后仍停留在目录页。

可预期的策略拒绝不得让 Custom Action 返回失败，也不得抛异常制造 1720/1723。

### INS-008 — 执行序列权威校验

`InstallExecuteSequence` 必须在 `CostFinalize` 后、`InstallValidate` 前调用同一核心校验，覆盖：

- `/qn`；
- `/passive`；
- `msiexec INSTALLDIR=...`；
- SCCM、Intune、组策略等企业部署；
- UI 校验后路径/ACL 被修改的场景。

执行序列校验失败必须使用明确的 MSI 错误消息终止，不得继续复制提权程序文件。

### INS-009 — 同一策略实现

UI 与 Execute 入口必须调用同一个 Rust 核心函数、同一个错误枚举和同一个策略配置。不得维护两套路径判定逻辑。

### INS-010 — 用户错误文案

用户可见文案只表达下一步动作。建议固定归并为少量类别：

| 类别 | 用户文案 |
|---|---|
| 非本机磁盘 | `无法安装到该位置。请选择本机固定磁盘上的文件夹。` |
| 用户/临时目录 | `无法安装到该位置。请选择 Program Files 或由管理员管理的应用文件夹。` |
| 链接/重定向路径 | `该路径包含链接或重定向。请选择其他文件夹。` |
| 权限不安全 | `该文件夹可能被普通用户修改。请选择受保护的应用文件夹。` |
| 已有无关内容 | `请选择一个新的 FyAgent 文件夹，或先清空所选文件夹。` |
| 无法确认 | `无法确认该文件夹是否安全。请选择其他文件夹。` |

不得向普通用户展示 ACL、SID、ACE、WMI、Custom Action、返回码或内部检查数量。

### INS-011 — 技术诊断日志

MSI verbose 日志必须记录：

- 稳定错误代码；
- 校验阶段（UI/Execute）；
- 规范化路径的安全表示；
- 失败检查项；
- Win32/MSI 错误码；
- 架构和 DLL 版本；
- 短关联 ID。

不得记录访问令牌、证书私钥、环境秘密或与诊断无关的用户数据。

## 5. 升级、修复和卸载需求

### UPG-001 — 从 0.2.0 升级

升级必须优先恢复当前机器 HKLM 中保存的既有 `InstallDir`，并继续在原目录升级。`0.2.1` 不提供升级时搬迁安装目录的功能。

### UPG-002 — 既有目录重新校验

升级执行前必须对既有目录运行新策略：

- 通过：正常升级；
- 不通过：停止升级并提示用户卸载后重新安装到安全目录；
- 不得在升级中静默修改任意父目录 ACL 或自动搬迁程序。

### UPG-003 — 修复

修复使用 Windows Installer 既有目录，不展示目录选择。若目录安全边界已被外部破坏，修复失败并要求重新安装。

### UPG-004 — 卸载可用性

卸载不运行目录准入校验。即使路径部分缺失或 ACL 被改变，也应尽最大可能完成注册信息、服务/任务、快捷方式和已安装文件的清理。

### UPG-005 — 安装目录持久化

成功安装后必须继续把最终规范化 `INSTALLDIR` 写入受保护的 HKLM 产品键。升级读取失败时不得退回用户可控 HKCU 值。

## 6. 原生动作与构建需求

### NAT-001 — 删除脚本 Custom Action

必须从 WiX 模板中移除：

- 内联 VBScript；
- `ValidateInstallDirectory` 的 UI/Execute 调度；
- WMI `Win32_LogicalFileSecuritySetting`；
- `Scripting.FileSystemObject`；
- 通过 `Err.Raise` 表达目录策略失败的逻辑。

### NAT-002 — 独立安装器动作 crate

新增 `src-tauri/installer-actions` Rust crate，输出 Windows `cdylib`。不得把主应用 crate 改为 `cdylib` 以复用，避免 Tauri bundler 把无关 DLL 当应用资源打包。

### NAT-003 — 架构匹配

- x64 MSI 只能嵌入 x64 动作 DLL；
- ARM64 MSI 只能嵌入 ARM64 动作 DLL；
- 构建和测试必须验证 PE Machine 字段与 MSI Template Summary 一致。

### NAT-004 — 无目标机脚本依赖

目录安全校验不得依赖 VBScript、JScript、PowerShell、WMI 或外部可执行文件。只使用随 MSI 内嵌的 DLL 和 Windows API。

### NAT-005 — 失败封闭

对路径、卷、重解析点、所有者、DACL 或 ACE 的关键读取失败时，校验必须拒绝。不得把“无法判断”当作安全。

### NAT-006 — 校验只读

校验阶段不得创建测试文件、创建目录、接管所有权或修改 ACL。目录创建和最终 ACL 应由 Windows Installer 标准事务完成。

## 7. FyAgent 全局版本需求

### VER-001 — 全局应用版本

`0.2.1` 表示同一提交构建出的 FyAgent 应用版本，覆盖：

- Windows x64/ARM64；
- macOS；
- Linux x86_64/ARM64；
- Tauri 运行时 `getVersion()`；
- MSI ProductVersion；
- macOS bundle 版本；
- Linux 包元数据；
- 发布标签、资产名和下载 manifest。

不得让非 Windows 平台保持 `0.2.0`。

### VER-002 — 单一真源

仓库中 FyAgent 当前版本只能有一个人工维护的版本字面量，位置固定为：

```toml
# src-tauri/Cargo.toml
[workspace.package]
version = "0.2.1"
```

其他平台与脚本必须继承或读取该值。

### VER-003 — Tauri 继承

`src-tauri/tauri.conf.json` 必须删除 `version` 字段，由 Tauri 使用 Cargo 包版本。不得在平台覆盖配置中重新写入应用版本。

### VER-004 — npm 元数据去重

项目不发布为 npm 包，因此 `package.json` 必须：

- 删除应用 `version` 字段；
- 增加 `"private": true`；
- 增加统一版本命令。

### VER-005 — Workspace 继承

FyAgent 主 crate 和 `fyagent-installer-actions` 必须使用：

```toml
version.workspace = true
```

不得各自保存版本字面量。

### VER-006 — 一键命令

必须提供：

```bash
pnpm run version:get
pnpm run version:check
pnpm run version:set -- X.Y.Z
pnpm run version:bump -- patch|minor|major
```

### VER-007 — 受控修改范围

`version:set` 只能修改：

1. `[workspace.package].version`；
2. `Cargo.lock` 中 FyAgent 本地 workspace 包的版本条目。

不得修改：

- 依赖版本；
- Node/Rust/pnpm/WiX 等工具链版本；
- 数据库 schema、配置、协议或 API 版本；
- 外部 Claude/Codex/Gemini 版本；
- 历史文档和 CHANGELOG；
- 文件中仅作为示例或测试数据出现的相同数字。

### VER-008 — 稳定 SemVer 与 MSI 兼容

当前正式发布仅接受无前导 `v`、无 prerelease、无 build metadata 的 `X.Y.Z`。同时满足 Windows Installer ProductVersion 范围：

```text
major <= 255
minor <= 255
patch <= 65535
```

需要预发布版本时另立设计，不在本次脚本中隐式支持。

### VER-009 — 原子性和失败回滚

版本脚本必须：

- 更新前验证仓库结构；
- 精确定位目标字段；
- 任何写入失败时恢复已修改文件；
- 写入后重新运行契约检查；
- 支持 `--dry-run`；
- 对未知结构直接失败，不尝试“智能修复”。

### VER-010 — Git 与发布解耦

版本脚本不得自动提交、打标签或推送。正式标签必须由发布流程单独创建，并严格为：

```text
v${APP_VERSION}
```

## 8. 发布与 CI 需求

### REL-001 — 提前版本门禁

所有平台构建前运行 `version:check`。标签构建还必须验证 `GITHUB_REF_NAME == v${APP_VERSION}`。不匹配立即失败，不消耗签名和公证资源。

### REL-002 — 同一提交

Windows、macOS、Linux 正式产物必须由同一 Git tag 和同一 commit SHA 构建。发布清单记录 SHA。

### REL-003 — 资产命名

资产名使用纯应用版本，不含标签前导 `v`：

```text
FyAgent-0.2.1-Windows.msi
FyAgent-0.2.1-Windows-arm64.msi
FyAgent-0.2.1-macOS.dmg
FyAgent-0.2.1-macOS.zip
FyAgent-0.2.1-Linux-x86_64.AppImage
FyAgent-0.2.1-Linux-x86_64.deb
FyAgent-0.2.1-Linux-x86_64.rpm
FyAgent-0.2.1-Linux-arm64.*
```

### REL-004 — 内嵌版本验证

发布前至少验证：

- MSI Property 表 `ProductVersion`；
- Windows EXE 文件/产品版本；
- macOS `CFBundleShortVersionString`；
- DEB/RPM 包版本；
- 下载 manifest `version` 和 `tag`；
- 文件名版本。

所有值必须由同一 `APP_VERSION` 派生。

### REL-005 — Windows 原生发布门禁

公开 MSI 必须在原生 Windows runner/VM 上完成：

- 安装；
- 启动；
- 自定义目录；
- 不安全目录拒绝；
- 静默安装；
- `0.2.0 -> 0.2.1` 升级；
- 修复；
- 卸载；
- EXE/MSI 签名验证。

Wine 构建可继续生成候选产物，但不能单独批准发布。

## 9. 质量属性

| 属性 | 要求 |
|---|---|
| 安全 | 提权二进制不得落在普通用户可替换的位置；无法判断时拒绝 |
| 可用性 | 目录错误可恢复，不关闭安装器，不暴露技术术语 |
| 一致性 | UI、静默、升级调用同一目录策略；所有平台使用同一应用版本 |
| 可维护性 | 版本更新不做搜索替换；策略、错误码和脚本有单元测试 |
| 可审计性 | 需求编号、日志代码、CI 证据和产物哈希可追踪 |
| 可回滚性 | MSI 安装事务、版本脚本写入和发布步骤均有明确失败边界 |
| 可移植性 | 版本脚本仅依赖 Node 标准库；Windows 动作分别构建 x64/ARM64 |

## 10. 发布阻断条件

出现任一情况不得发布 `0.2.1`：

- 旧 VBScript/WMI 字符串仍在 MSI；
- 目录 UI 可选，但 `/qn` 可绕过；
- 默认 Program Files 被误拒绝；
- 可安装到用户 Desktop/AppData/Temp；
- 动作 DLL 架构错误或未签名策略未验证；
- 任一正式平台内嵌版本不是 `0.2.1`；
- 标签与单一真源不一致；
- 版本脚本误改依赖或历史文本；
- 原生 Windows 生命周期测试未完成；
- 发布资产无法关联到同一 commit SHA 和哈希清单。
