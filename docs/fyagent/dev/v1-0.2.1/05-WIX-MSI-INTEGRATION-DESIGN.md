# 05 — WiX/MSI 集成、时序与升级设计

## 1. 目标

在保留当前 Tauri 自定义 WiX 模板和标准目录选择体验的基础上：

- 移除内联 VBScript；
- 嵌入对应架构的原生动作 DLL；
- 把目录安全校验接到 `InstallDirDlg` 的 Next 事件链；
- 在 Execute Sequence 做第二次权威校验；
- 以可恢复提示替代 Error 1720；
- 保持升级、修复、卸载、签名和 Tauri bundler 兼容。

## 2. 保留的现有结构

以下配置继续使用：

```xml
<Package
  InstallScope="perMachine"
  InstallPrivileges="elevated" />

<Property Id="WIXUI_INSTALLDIR" Value="INSTALLDIR" />

<Directory Id="$(var.PlatformProgramFilesFolder)">
  <Directory Id="INSTALLDIR" Name="{{product_name}}" />
</Directory>

<Feature
  Id="MainProgram"
  ConfigurableDirectory="INSTALLDIR">
```

以及：

- 当前 Product/UpgradeCode 体系；
- HKLM 产品键；
- 安装目录注册值；
- `MsiLockPermissionsEx`/`PermissionEx` 对最终目录的受保护 ACL；
- 桌面/开始菜单快捷方式；
- ProgramData 运行时目录 ACL；
- Tauri `bundle.windows.wix.template`。

## 3. 必须删除的内容

从 `src-tauri/wix/per-machine-main.wxs` 删除：

1. `<CustomAction Id="ValidateInstallDirectory" Script="vbscript" ...>` 整块；
2. `InstallUISequence` 中 `ValidateInstallDirectory` 调度；
3. `InstallExecuteSequence` 中旧调度；
4. WMI、`Scripting.FileSystemObject`、`GetSecurityDescriptor`、`Err.Raise` 相关文本；
5. 只由旧脚本使用的常量和属性；
6. 对旧脚本存在性的测试断言。

删除后 MSI 的 CustomAction/Binary 表中不得再出现 FyAgent 自定义脚本类型。

## 4. 原生 Binary 和 CustomAction

概念片段：

```xml
<Binary
  Id="FyAgentInstallerActions"
  SourceFile="$(env.FYAGENT_INSTALLER_ACTIONS_DLL)" />

<CustomAction
  Id="ValidateFyAgentInstallDirUi"
  BinaryKey="FyAgentInstallerActions"
  DllEntry="ValidateFyAgentInstallDirUi"
  Execute="immediate"
  Return="check" />

<CustomAction
  Id="ValidateFyAgentInstallDirExecute"
  BinaryKey="FyAgentInstallerActions"
  DllEntry="ValidateFyAgentInstallDirExecute"
  Execute="immediate"
  Return="check" />
```

这是 Windows Installer Type 1 DLL Custom Action：DLL 存储在 MSI Binary 表，执行时提取到临时位置，并以当前安装会话句柄调用导出入口。

### 4.1 为什么两个 CustomAction 指向同一 DLL

- 入口职责不同，核心函数相同；
- UI 入口始终把策略拒绝转成属性；
- Execute 入口提供独立阶段日志和更严格上下文；
- WiX 条件和序列更清晰；
- 不需要通过额外模式属性复用同一个入口。

## 5. UI 对话框定制

WiX 官方建议：需要改变内置 WixUI 对话框事件顺序时，复制对应 Fragment 并修改 `Publish`。本方案应把当前简单 `<UIRef Id="WixUI_InstallDir"/>` 改为受版本控制的定制流程。

可以：

- 在同一个 `per-machine-main.wxs` 中内嵌定制 UI；或
- 使用 Tauri `wix.fragmentPaths` 引入独立 `.wxs` Fragment。

优先建议独立 Fragment：

```text
src-tauri/wix/per-machine-main.wxs
src-tauri/wix/fyagent-install-dir-ui.wxs
```

并在 `tauri.conf.json` 中配置 `fragmentPaths`。若当前 Tauri/WiX 模板与 Fragment 合并验证存在兼容问题，则回退为同文件内嵌，但逻辑保持一致。

## 6. InstallDirDlg Next 事件顺序

目标事件链：

```text
Order 1  SetTargetPath
Order 2  WixUIValidatePath
Order 3  标准路径无效 → InvalidDirDlg
Order 4  标准路径有效 → ValidateFyAgentInstallDirUi
Order 5  FyAgent 策略无效 → FyAgentUnsafeInstallDirDlg
Order 6  两者都有效 → VerifyReadyDlg
```

概念 WiX：

```xml
<Publish Dialog="InstallDirDlg"
         Control="Next"
         Event="SetTargetPath"
         Value="[WIXUI_INSTALLDIR]"
         Order="1">1</Publish>

<Publish Dialog="InstallDirDlg"
         Control="Next"
         Event="DoAction"
         Value="WixUIValidatePath"
         Order="2">NOT WIXUI_DONTVALIDATEPATH</Publish>

<Publish Dialog="InstallDirDlg"
         Control="Next"
         Event="SpawnDialog"
         Value="InvalidDirDlg"
         Order="3"><![CDATA[
  WIXUI_INSTALLDIR_VALID <> "1"
]]></Publish>

<Publish Dialog="InstallDirDlg"
         Control="Next"
         Event="DoAction"
         Value="ValidateFyAgentInstallDirUi"
         Order="4"><![CDATA[
  WIXUI_INSTALLDIR_VALID = "1"
]]></Publish>

<Publish Dialog="InstallDirDlg"
         Control="Next"
         Event="SpawnDialog"
         Value="FyAgentUnsafeInstallDirDlg"
         Order="5"><![CDATA[
  WIXUI_INSTALLDIR_VALID = "1"
  AND FYAGENT_INSTALLDIR_VALID <> "1"
]]></Publish>

<Publish Dialog="InstallDirDlg"
         Control="Next"
         Event="NewDialog"
         Value="VerifyReadyDlg"
         Order="6"><![CDATA[
  WIXUI_INSTALLDIR_VALID = "1"
  AND FYAGENT_INSTALLDIR_VALID = "1"
]]></Publish>
```

实际条件要与当前 WiX v3 `WixUI_InstallDir.wxs` 源码完全对照，尤其是 Maintenance/Resume 分支。上面是时序契约，不应不经编译验证直接粘贴。

## 7. 自定义错误对话框

`FyAgentUnsafeInstallDirDlg` 只需要：

- 标题：`无法使用该安装目录`；
- 文本控件绑定 `[FYAGENT_INSTALLDIR_ERROR_MESSAGE]`；
- 一个“确定”按钮；
- 关闭后返回 `InstallDirDlg`；
- 不退出安装；
- 不显示技术详情按钮。

错误消息属性只能从固定本地化表选择，不允许把未转义路径、SID 或系统错误文本直接注入 RichText/控件。

## 8. UI 状态复位

每次调用 UI 动作前，DLL 或 Type 51 SetProperty 必须清空：

```text
FYAGENT_INSTALLDIR_VALID
FYAGENT_INSTALLDIR_ERROR_CODE
FYAGENT_INSTALLDIR_ERROR_MESSAGE
FYAGENT_INSTALLDIR_NORMALIZED
```

否则用户第一次失败后修改路径，旧属性可能阻止成功导航。

用户返回上一页再回来时也必须重新验证，不缓存旧结论。

## 9. Execute Sequence

目标时序：

```text
CostFinalize
  ↓
ValidateFyAgentInstallDirExecute
  ↓
AbortUnsafeFyAgentInstallDir（仅 VALID<>1）
  ↓
InstallValidate
  ↓
InstallInitialize / 标准安装动作
```

概念 WiX：

```xml
<CustomAction
  Id="AbortUnsafeFyAgentInstallDir"
  Error="[FYAGENT_INSTALLDIR_ERROR_MESSAGE]" />

<InstallExecuteSequence>
  <Custom Action="ValidateFyAgentInstallDirExecute"
          After="CostFinalize"><![CDATA[
    NOT (REMOVE~="ALL")
  ]]></Custom>

  <Custom Action="AbortUnsafeFyAgentInstallDir"
          After="ValidateFyAgentInstallDirExecute"><![CDATA[
    NOT (REMOVE~="ALL")
    AND FYAGENT_INSTALLDIR_VALID <> "1"
  ]]></Custom>
</InstallExecuteSequence>
```

Type 19 Error Action 以明确文案终止。必须通过 MSI 日志和真实 UI 验证它不会退化为通用 Custom Action 错误。

## 10. 安装上下文条件

### 10.1 首次安装

```text
NOT Installed
AND NOT WIX_UPGRADE_DETECTED（按实际属性）
```

允许显示目录页和选择新目录。

### 10.2 Major Upgrade

- 在 AppSearch/CostFinalize 前恢复旧目录；
- 不显示目录页；
- Execute 校验现有目录；
- `INSTALLDIR` 外部覆盖不改变位置。

### 10.3 Maintenance/Repair

- 不显示目录页；
- Execute 校验；
- 修复失败时给明确重新安装建议。

### 10.4 Uninstall

条件必须排除：

```text
REMOVE~="ALL"
```

防止不安全/损坏目录阻断卸载。

### 10.5 Rollback

校验动作是 immediate、只读，不需要 rollback Custom Action。实际目录/ACL 变更仍由 MSI 标准事务管理。

## 11. INSTALLDIR 属性安全

`INSTALLDIR` 是大写 Public Property，命令行可以覆盖。因为 MSI 是 elevated per-machine：

- 把它作为 `Secure` 属性跨 UI/Execute 传递；
- 仍把内容视为不可信并在服务端执行序列重验；
- 不依赖 Hidden 属性提供安全性；
- 升级时通过条件禁止外部属性迁移目录；
- 设置 `SecureCustomProperties`/WiX `Secure="yes"` 的具体方式应通过生成 MSI Property 表验证。

## 12. 保存和恢复安装目录

当前模板已写 HKLM：

```xml
<RegistryValue
  Name="InstallDir"
  Type="string"
  Value="[INSTALLDIR]" />
```

改造要求：

1. 保存规范化后的最终目录；
2. 使用稳定制造商/产品键；
3. 读取采用 `RegistrySearch`，Root=`HKLM`；
4. 仅在相关产品升级/修复时应用旧路径；
5. 对读取值重新校验；
6. 不能从 HKCU 或用户环境变量恢复提权程序目录；
7. 卸载时按产品数据保留策略移除该值。

## 13. 最终目录 ACL

当前 `PermissionEx` 设计应保留并重新审查。目标是：

- `SYSTEM`：Full Control；
- `Administrators`：Full Control；
- 普通 Users：只读/执行；
- DACL protected，避免继承父目录的可写 ACE；
- 所有者为可信主体；
- 文件和子目录继承适配应用运行所需。

注意：最终 ACL 不能弥补不安全父目录的替换能力，因此前置校验仍必须检查父目录 `DELETE_CHILD`、重解析点和抢先创建。

## 14. 构建接线

### 14.1 构建顺序

每个 Windows 架构：

```text
构建 frontend/主 EXE
  ↓
cargo build -p fyagent-installer-actions --target <target> --release
  ↓
验证动作 DLL PE 架构和版本
  ↓
设置 FYAGENT_INSTALLER_ACTIONS_DLL
  ↓
Tauri bundle --bundles msi
  ↓
验证 MSI Binary/CustomAction/UI 表
  ↓
签名 MSI
```

主 EXE 的构建顺序可按现有签名流程微调，但动作 DLL必须在 Candle/Light 处理模板前可用。

### 14.2 x64

```text
x86_64-pc-windows-msvc
```

### 14.3 ARM64

```text
aarch64-pc-windows-msvc
```

不能把 x64 DLL嵌入 ARM64 MSI 后指望 WOW64 执行；Windows Installer 进程架构和 DLL加载必须匹配。

### 14.4 交叉构建

现有 cargo-xwin/Wine 脚本可以增加动作 DLL构建与 MSI 静态验证，但公开发布仍要求原生 Windows 生命周期测试。

## 15. Tauri 配置

保留：

```json
"windows": {
  "wix": {
    "template": "wix/per-machine-main.wxs"
  }
}
```

若采用 Fragment：

```json
"windows": {
  "wix": {
    "template": "wix/per-machine-main.wxs",
    "fragmentPaths": [
      "wix/fyagent-install-dir-ui.wxs"
    ]
  }
}
```

`tauri.conf.json` 中不再保存应用 `version`，这与 WiX模板无冲突；模板的 `{{version}}` 由 Tauri 解析后的 Cargo 应用版本提供。

## 16. MSI 静态验证

每个候选 MSI 必须导出/检查：

```text
Property
Binary
CustomAction
InstallUISequence
InstallExecuteSequence
Dialog
Control
ControlEvent
Directory
Component
Feature
Registry
LockPermissions / MsiLockPermissionsEx 相关表
Upgrade
```

断言：

- Binary 表有 `FyAgentInstallerActions`；
- CustomAction 表仅有预期 Type 1 入口；
- 不存在 Type 5/6/21/22/37/38 的 FyAgent 目录脚本；
- UI Next 事件顺序符合设计；
- Execute 校验在 `InstallValidate` 前；
- 卸载条件排除校验；
- `INSTALLDIR` 可配置且默认 Program Files；
- ProductVersion 等于全局应用版本；
- MSI Template Summary 和 DLL PE Machine 一致。

还应对 MSI 字符串运行负面扫描：

```text
ValidateInstallDirectory（旧 ID）
Scripting.FileSystemObject
Win32_LogicalFileSecuritySetting
GetSecurityDescriptor
FyAgentInstallDirectoryPolicy
```

这些旧实现标识不得存在。

## 17. ICE 与原生测试

在 Windows runner 上运行适用 ICE 检查。需要注意：

- ICE 警告不能全部无条件忽略；
- 对已知 Tauri/WiX 模板例外应记录精确规则和原因；
- 新 Custom Action、Binary、组件位数、目录和注册表项不应引入 ICE03/ICE38/ICE64/ICE80 等结构问题；
- ICE 不能替代实际安装、升级、修复和卸载。

## 18. 签名顺序

推荐：

1. 构建动作 DLL；
2. 对动作 DLL签名并验证（若采用）；
3. 构建主 EXE；
4. 对主 EXE签名并验证；
5. 生成 MSI并嵌入已签名 DLL/EXE；
6. 对 MSI签名并时间戳；
7. 安装前后再次验证签名。

不能在 MSI签名后重新修改 Binary 表或资源。

## 19. 错误恢复

- UI 失败：只返回目录页；
- Execute 失败：安装尚未进入实质文件复制，明确终止；
- 安装过程中其他错误：Windows Installer 标准 rollback；
- 动作 DLL加载失败：视为安装器缺陷，发布测试阻断；
- 旧目录不安全：不自动修 ACL/搬迁，给出卸载重装路径。

## 20. 参考接入片段

随包提供：

```text
reference/snippets/wix-native-custom-actions.wxs
reference/snippets/installer-actions.Cargo.toml
```

它们用于表达结构与时序，不替代实施者对当前 WiX v3/Tauri 生成模板、Localization 和维护模式条件的编译验证。
