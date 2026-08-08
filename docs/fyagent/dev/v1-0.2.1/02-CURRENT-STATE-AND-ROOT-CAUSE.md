# 02 — 当前状态、故障根因与流程缺口

## 1. 核查对象

| 对象 | 基线 |
|---|---|
| FyAgent 源码 | `cc-switch-feature-fyagent-v1(1)(2).zip` |
| 问题 MSI | `FyAgent-0.2.0-Windows(1).msi` |
| 问题现象 | Windows Installer Error 1720 截图 |
| CC Switch 对照 | 上游 `src-tauri/wix/per-user-main.wxs` 及本次留存快照 |
| 官方资料 | Tauri v2、Cargo、npm、SemVer、WiX/FireGiant、Windows Installer 与 Win32 API |

输入哈希见当前目录 `PACKAGE-METADATA.md`。

## 2. 当前 MSI 已经具备目录选择能力

`src-tauri/wix/per-machine-main.wxs` 中已经存在：

```xml
<Property Id="WIXUI_INSTALLDIR" Value="INSTALLDIR" />
<UIRef Id="WixUI_InstallDir" />
```

目录树默认落在：

```xml
<Directory Id="$(var.PlatformProgramFilesFolder)">
  <Directory Id="INSTALLDIR" Name="{{product_name}}"/>
</Directory>
```

主 Feature 又使用：

```xml
ConfigurableDirectory="INSTALLDIR"
```

对应当前模板位置约为：

| 能力 | 行号 |
|---|---:|
| `WIXUI_INSTALLDIR` | 335 |
| `WixUI_InstallDir` | 352 |
| 默认 `INSTALLDIR` | 354–357 |
| `ConfigurableDirectory` | 515 |
| HKLM 保存 `InstallDir` | 401 |

因此，Error 1720 不是“MSI 不支持选择目录”，也不是“跨平台编译自动删除了目录页”。目录页存在，只是在到达前被自定义脚本阻断。

## 3. CC Switch 的可参考部分与不可照搬部分

CC Switch 的 WiX 模板使用标准 `WixUI_InstallDir`，将 `WIXUI_INSTALLDIR` 指向 `INSTALLDIR`，并从 HKCU 读取此前选择的目录。它的安装模型是：

```text
perUser + limited + LocalAppData\Programs
```

FyAgent 可以复用其：

- 标准目录选择对话框；
- `INSTALLDIR` 作为 Feature 可配置目录；
- 保存并恢复上次安装目录的用户体验；
- 通过 WiX Publish 事件定制 Next/Back 流程。

但不能照搬其权限模型，因为 FyAgent 当前是：

```text
perMachine + elevated + requireAdministrator
```

把管理员权限应用安装到普通用户可修改的 AppData/Desktop/Downloads 会改变安全边界。因此本方案参考 CC Switch 的**目录选择机制**，不复制它的**per-user 安装范围**。

## 4. Error 1720 的确定性故障链

### 4.1 脚本被安排在目录 UI 之前运行

模板第 47–54 行：

```xml
<InstallExecuteSequence>
  <Custom Action="ValidateInstallDirectory"
          After="CostFinalize">NOT Installed</Custom>
</InstallExecuteSequence>

<InstallUISequence>
  <Custom Action="ValidateInstallDirectory"
          After="CostFinalize">NOT Installed</Custom>
</InstallUISequence>
```

这不是“用户在目录页点击下一步后校验”，而是 Windows Installer UI Sequence 到达 `CostFinalize` 后直接执行。用户尚未完成目录选择，Custom Action 已经开始运行。

### 4.2 Custom Action 是内联 VBScript

第 96 行开始：

```xml
<CustomAction
  Id="ValidateInstallDirectory"
  Script="vbscript"
  Execute="immediate"
  Return="check">
```

因此目标机器必须有可用脚本运行时，脚本中的任何未处理错误又会被 `Return="check"` 解释为安装失败。

### 4.3 WMI 调用模型混用

第 247 行：

```vbscript
Set result = security.GetSecurityDescriptor()
```

后续却使用：

```vbscript
result.ReturnValue
result.Descriptor.DACL
```

这里混用了两种不同调用模型：

- 直接 VBScript 调用应为 `returnCode = security.GetSecurityDescriptor(descriptor)`；
- 只有 `ExecMethod_("GetSecurityDescriptor")` 才返回带 `ReturnValue` 和输出参数属性的对象。

当前代码既没有传入输出参数，又把返回值当对象使用，足以触发运行时错误或进入“无法证明安全即拒绝”的分支。

### 4.4 策略拒绝通过异常表达

第 143–147 行的 `Reject` 最终执行：

```vbscript
Err.Raise vbObjectError + 513, ...
```

Windows Installer 收到脚本 Custom Action 失败，只能显示通用脚本错误。用户看到的是 Error 1720，而不是具体的“请选择其他安装目录”。

### 4.5 UI 错误属性没有形成可恢复交互

脚本设置 `FYAGENT_INSTALLDIR_ERROR`，但当前模板没有把该属性接到目录页的可恢复提示流程。即使策略原因已经生成，安装器仍以脚本失败退出。

## 5. 当前脚本策略本身的设计缺陷

即使修复 `GetSecurityDescriptor` 调用，当前脚本仍存在以下问题：

1. **目标机依赖**：依赖 VBScript、WMI 和 `Scripting.FileSystemObject`；
2. **执行时机错误**：不是在用户提交最终 `INSTALLDIR` 后运行；
3. **错误通道错误**：把正常策略拒绝当脚本崩溃；
4. **UI/静默语义混杂**：同一个失败返回同时负责提示和终止；
5. **路径标准化不足**：字符串比较不能可靠处理最终路径、挂载点、设备命名空间和重解析点；
6. **ACL 判定过粗**：逐 ACE 扫描没有完整建模泛型权限、继承、对象/回调 ACE、父目录 `DELETE_CHILD` 和路径组件替换；
7. **可能误拒绝默认路径**：对所有祖先一律把“可创建子项”视为目标可被篡改，可能把卷根允许创建无关兄弟目录误判为可替换 Program Files；
8. **TOCTOU**：UI 校验后没有基于同一规范化结果和路径句柄做权威重验；
9. **共享目录风险**：若用户直接选择已有公共目录，后续 `PermissionEx` 可能改变整目录 ACL；
10. **未知状态处理不透明**：对任何 WMI/对象读取错误统一映射成“可写”，用户无法区分策略与系统故障。

这些问题说明“只改一行 VBScript”不能达到正式发布要求。

## 6. 构建流程为什么未提前发现

### 6.1 Linux/Wine 候选验证只检查有限表

`scripts/windows-cross/build-windows-msi.sh` 的 `verify_linux_candidate()` 主要导出：

```text
Property
Directory
Component
Feature
FeatureComponents
File
Registry
Shortcut
Upgrade
InstallExecuteSequence
```

它没有完整验证：

```text
Binary
CustomAction
InstallUISequence
Dialog
Control
ControlEvent
```

也没有真正运行 `ValidateInstallDirectory`。

### 6.2 脚本已明确声明原生验证推迟

当前构建脚本写入的证据说明中包含：

```text
Windows Installer ICE and lifecycle validation are deferred from the Wine build
```

结尾也声明安装、启动、升级、卸载、签名接受等仍待原生 Windows 验证。换言之，流程本身知道交叉构建无法覆盖安装生命周期，但正式发布门禁尚未强制补齐。

### 6.3 当前测试只验证字符串存在

`tests/releaseWorkflow.test.ts` 等测试主要验证模板中存在 `Script="vbscript"`、WMI 类名或工作流文本。此类测试能防止模板片段意外消失，却不能执行脚本，也无法发现返回类型错误。

### 6.4 直接切到 Windows 编译不能自动修复源码

同一份错误 WiX/VBScript 在 Windows runner 上仍会生成错误 MSI。Windows 原生构建的价值是能运行 MSI、捕获失败并阻止发布，而不是编译器会自动纠正脚本。

## 7. 当前 FyAgent 版本分布

源码中的应用版本 `0.2.0` 至少分布如下：

| 文件 | 当前位置/行为 | 问题 |
|---|---|---|
| `package.json:3` | `"version": "0.2.0"` | npm 项目并不发布，但保存一份应用版本 |
| `src-tauri/Cargo.toml:3` | `version = "0.2.0"` | Rust 包版本 |
| `src-tauri/tauri.conf.json:4` | `"version": "0.2.0"` | Tauri 打包版本 |
| `src-tauri/Cargo.lock:1779–1780` | 本地 `fyagent` 包 `0.2.0` | 生成文件中的投影 |
| `tests/versionConsistency.test.ts:13` | 常量 `FYAGENT_V1_0_2_VERSION` | 把当前版本固化为测试里程碑 |
| `scripts/macos-cross/project_metadata.py:186–203` | 比较 package/Tauri/Cargo/lock | 依赖三份人工字段都存在 |
| `scripts/windows-cross/build-windows-msi.sh:531–545` | 从 `tauri.conf.json` 读版本 | 平台脚本绑定重复字段 |
| `.github/workflows/release.yml` | 用 `GITHUB_REF_NAME` 命名资产 | 标签可与内嵌版本漂移，且文件名含 `v` |
| `scripts/generate-download-manifest.mjs:75–79` | 通过去掉标签 `v` 得到版本 | 不验证标签与源码版本一致 |

`pnpm-lock.yaml` 当前不保存根应用版本，不需要纳入更新目标。

## 8. 当前版本测试的缺陷

`tests/versionConsistency.test.ts` 目前：

- 用正则从 Cargo `[package]` 读取字面量；
- 读取 `package.json.version`；
- 读取 `tauri.conf.json.version`；
- 要求三者等于硬编码 `0.2.0`。

这实际上是在测试“重复配置恰好一致”，而不是消除重复。每次版本更新都必须先改生产配置，再改测试常量；测试自身成为版本散布的一部分。

## 9. 发布工作流的版本漂移风险

当前工作流在 Windows/macOS/Linux 资产准备阶段直接使用：

```text
GITHUB_REF_NAME
```

例如标签 `v0.2.1` 会生成含 `v0.2.1` 的文件名，但工作流没有在构建前验证：

```text
标签 v0.2.1 == 应用内嵌版本 0.2.1
```

可能出现：

- 标签是 `v0.2.1`，Cargo/Tauri 仍是 `0.2.0`；
- 文件名写 `v0.2.1`，MSI ProductVersion 是 `0.2.0`；
- 下载 manifest 通过去掉 `v` 宣称 `0.2.1`，但实际二进制不是；
- Windows 改了版本，macOS/Linux 构建仍使用另一字段或旧字段。

## 10. 根因树

```text
用户看到 Error 1720
├─ 直接原因：脚本型 Custom Action 返回失败
│  ├─ GetSecurityDescriptor 调用模型错误
│  ├─ 结果对象访问错误
│  └─ Reject 使用 Err.Raise
├─ 交互原因：校验在目录页前调度
│  └─ 用户没有修改路径和重试机会
├─ 架构原因：安全策略依赖 VBScript/WMI
│  └─ 目标机组件、类型系统和 Win32 语义不可控
└─ 流程原因：正式发布前没有原生 Windows 生命周期门禁
   ├─ Wine 只做有限结构检查
   └─ 单元测试只检查模板字符串

版本更新困难
├─ 多个文件各自保存应用版本
├─ 构建脚本直接读取不同副本
├─ 测试硬编码当前版本
├─ 标签与内嵌版本没有强契约
└─ 缺少精确、原子的一键更新工具
```

## 11. 设计结论

根因决定了本次必须同时完成：

1. 保留 WiX 目录选择能力；
2. 删除内联脚本；
3. 用原生 DLL 实现可测试的路径/ACL 策略；
4. 把策略拒绝改成可恢复 UI 流程；
5. 在 Execute Sequence 做权威重验；
6. 把公开 MSI 的生命周期测试移到原生 Windows 发布门禁；
7. 把 FyAgent 全局版本收敛到 Cargo workspace 单一真源；
8. 让所有平台脚本和标签从单一真源派生并校验。
