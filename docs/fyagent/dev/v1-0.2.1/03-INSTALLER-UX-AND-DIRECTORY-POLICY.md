# 03 — Windows 安装交互与目录安全策略

## 1. 设计原则

本方案同时满足：

1. 用户可以选择 FyAgent 的安装目录；
2. 提权运行的 FyAgent 不能从普通用户可篡改的位置加载。

因此产品承诺是：

> 用户可以选择本机固定磁盘上的 **FyAgent 专用安全目录**，而不是任意目录。

安全约束必须转化为简单、可恢复的交互。用户不需要理解 ACL、SID、重解析点或 Windows Installer 内部动作。

## 2. 首次安装完整 UI 流程

```text
启动 MSI
  │
  ├─ 欢迎页
  │
  ├─ 安装目录页
  │    ├─ 默认：C:\Program Files\FyAgent
  │    ├─ 用户输入或浏览选择
  │    └─ 点击“下一步”
  │          ├─ 标准路径格式检查
  │          ├─ FyAgent 原生安全检查
  │          ├─ 失败：显示简明提示，仍停留在目录页
  │          └─ 成功：进入确认安装页
  │
  ├─ UAC/执行序列
  │    └─ 再次执行同一安全检查
  │          ├─ 失败：明确终止并写详细日志
  │          └─ 成功：复制文件、设置 ACL、注册产品
  │
  └─ 完成页
```

### 2.1 为什么 UI 与 Execute 都要校验

- UI 校验提供即时反馈和重选机会；
- Execute 校验覆盖静默安装、命令行属性注入和 UI 后的路径变化；
- 两者调用同一个核心函数，不是重复维护两套规则。

## 3. 静默与企业部署流程

示例：

```cmd
msiexec /i FyAgent-0.2.1-Windows.msi /qn INSTALLDIR="D:\Applications\FyAgent" /L*V fyagent.log
```

行为要求：

- 安全目录：返回成功；
- 不安全目录：返回非零，日志包含稳定错误代码；
- 未指定目录：使用默认 Program Files；
- 不显示交互弹窗；
- 不能因 `/qn` 缺少 UI 而跳过策略。

对 SCCM、Intune、组策略和其他部署工具，`INSTALLDIR` 被视为管理员输入，但仍是不可信数据，必须校验。

## 4. 升级、修复和卸载

### 4.1 从 0.2.0 升级到 0.2.1

```text
检测相关旧产品
  ↓
从受保护 HKLM 读取既有 InstallDir
  ↓
恢复为本次 INSTALLDIR
  ↓
不显示目录选择页，不允许通过命令行迁移目录
  ↓
运行 0.2.1 原生安全检查
  ├─ 通过：原地升级
  └─ 不通过：停止，提示卸载后重新安装到安全目录
```

补丁版本不进行隐式搬迁。自动迁移会增加文件占用、回滚、快捷方式、注册表、签名和用户预期风险。

### 4.2 修复

- 使用 Windows Installer 已知的现有目录；
- 不显示目录选择页；
- 在非卸载路径运行只读安全检查；
- 目录安全边界已被破坏时停止修复，不在未知位置继续写入管理员程序。

### 4.3 卸载

- 不运行目录准入检查；
- 路径部分缺失或 ACL 异常时仍尽量卸载；
- 不因新校验导致旧版本无法移除；
- 遵守既有用户数据保留策略。

## 5. 路径输入模型

### 5.1 接受的语法形式

接受规范的绝对 DOS 路径，例如：

```text
C:\Program Files\FyAgent
D:\Applications\FyAgent
```

输入可以有尾部反斜杠，但内部规范化后只保留统一形式。

### 5.2 直接拒绝的语法形式

```text
FyAgent                         # 相对路径
C:FYAGENT                       # 驱动器相对路径
\\server\share\FyAgent        # UNC
\\?\C:\FyAgent                # 用户直接传入扩展命名空间
\\.\C:\FyAgent                # 设备命名空间
C:\                             # 卷根
```

设备命名空间不是因为 Windows API 不能处理，而是用户输入层不接受绕过常规路径语义的形式。内部可使用 `\\?\` 进行 Win32 调用，但必须由校验器自己生成。

## 6. 目录准入策略

### 6.1 分层模型

对目标 `D:\Applications\FyAgent`，校验器把路径分为：

1. 卷根：`D:\`；
2. 既有受保护父目录：`D:\Applications`；
3. 目标目录：`D:\Applications\FyAgent`；
4. 若目标尚不存在，则 `Applications` 是最近存在祖先；
5. 若目标存在，则目标本身也必须被检查。

### 6.2 通过条件

只有全部成立才通过：

- 卷为 `DRIVE_FIXED`；
- 文件系统声明支持持久 ACL；
- 规范化路径与最终句柄路径一致；
- 任何既有组件都不是 reparse point；
- 不在禁止的 known folder 树下；
- 最近存在祖先的所有者可信；
- DACL 可完整读取且结构可解释；
- 非可信主体不能创建目标的第一个缺失组件；
- 非可信主体不能修改、删除、改 DACL 或改所有者；
- 每个既有组件的直接父目录不能允许非可信主体通过 `DELETE_CHILD` 删除/重命名它；
- 现有目标为空，或可识别为当前/旧版 FyAgent 安装；
- 第二次一致性复核没有发现路径或安全描述符变化。

### 6.3 可信主体

默认可信主体按 SID 判断，不按本地化账户名称判断：

```text
LOCAL_SYSTEM                 S-1-5-18
BUILTIN\Administrators       S-1-5-32-544
NT SERVICE\TrustedInstaller  服务 SID
```

0.2.1 不自动把任意域组、单个管理员用户或未知服务账户视为可信。企业定制 ACL 如果无法映射到白名单，可能被保守拒绝；这是 fail-closed 策略的已知取舍。

### 6.4 非可信主体的危险能力

对目标/最近父目录，以下能力视为危险：

```text
FILE_ADD_FILE / FILE_WRITE_DATA
FILE_ADD_SUBDIRECTORY / FILE_APPEND_DATA
FILE_WRITE_EA
FILE_WRITE_ATTRIBUTES
DELETE
WRITE_DAC
WRITE_OWNER
GENERIC_WRITE
GENERIC_ALL
```

对每个既有组件的直接父目录，还检查：

```text
FILE_DELETE_CHILD
```

泛型权限必须先按文件/目录 Generic Mapping 展开，再比较具体位。

### 6.5 为什么不对所有祖先一律禁止“创建子目录”

`C:\Program Files\FyAgent` 的安全性取决于普通用户能否改变 `Program Files` 或在其中抢先创建 `FyAgent`，而不是能否在 `C:\` 下创建无关的兄弟目录。

因此：

- 最近存在父目录：检查创建首个缺失目标组件的能力；
- 目标和既有组件：检查写入、删除、改安全描述符的能力；
- 每个组件的直接父目录：检查 `DELETE_CHILD`；
- 更高层只可创建无关兄弟项不应直接导致误拒绝。

这是对当前 VBScript “扫描所有祖先并拒绝任意写权限”策略的重要修正。

## 7. 允许示例

| 路径 | 前提 | 结果 |
|---|---|---|
| `C:\Program Files\FyAgent` | 标准 Windows ACL 未损坏 | 允许 |
| `D:\Program Files\FyAgent` | `D:\Program Files` 已由管理员保护 | 允许 |
| `D:\Applications\FyAgent` | `D:\Applications` 已存在，可信所有者，普通用户无危险权限 | 允许 |
| `E:\CompanyApps\FyAgent` | 固定磁盘、支持 ACL、路径无链接 | 允许 |

## 8. 拒绝示例

| 路径/状态 | 原因类别 | 用户提示 |
|---|---|---|
| `\\server\share\FyAgent` | 网络位置 | 请选择本机固定磁盘 |
| 映射网络盘 | 网络位置 | 请选择本机固定磁盘 |
| USB/exFAT | 磁盘/ACL 能力不足 | 请选择受支持的本机磁盘 |
| `C:\Users\Alice\FyAgent` | 用户目录 | 请选择受保护的应用文件夹 |
| `C:\Users\Alice\Desktop\FyAgent` | 用户目录 | 请选择受保护的应用文件夹 |
| `%TEMP%\FyAgent` | 临时目录 | 请选择受保护的应用文件夹 |
| `C:\Windows\FyAgent` | 系统目录 | 请选择其他应用文件夹 |
| `D:\PublicWritable\FyAgent` | 父目录可写 | 该文件夹可能被普通用户修改 |
| `D:\AppsLink\FyAgent`，`AppsLink` 是 junction | 重解析点 | 该路径包含链接或重定向 |
| 目标已有其他软件文件 | 非专用目录 | 请选择新的 FyAgent 文件夹 |
| 读取 DACL 失败 | 无法确认 | 无法确认该文件夹是否安全 |

## 9. 目标目录存在性策略

### 9.1 目标不存在

- 找到最近存在祖先；
- 验证普通用户不能在该祖先抢先创建首个缺失组件；
- Windows Installer 后续创建目标并应用受保护 DACL。

### 9.2 目标存在且为空

- 检查目标所有者、DACL、重解析点和父目录替换能力；
- 通过后允许安装。

### 9.3 目标存在且非空

只在以下情况允许：

- 是 Windows Installer 已识别的 FyAgent 现有安装目录；或
- 含可验证的 FyAgent 安装标识，且处于升级/修复路径。

首次安装选择一个普通非空目录一律拒绝，防止覆盖其他应用或对共享目录应用 FyAgent ACL。

## 10. 禁止目录树

通过 `SHGetKnownFolderPath` 等 API 获取本机实际路径，至少禁止：

- Profile；
- Desktop；
- Documents；
- Downloads；
- LocalAppData；
- RoamingAppData；
- LocalAppDataLow；
- Temp；
- Windows；
- System/SystemX86；
- ProgramData 根目录本身。

“Program Files 下 FyAgent 子目录”允许，但不能让用户直接选择 Program Files 根目录；“ProgramData 下运行时数据目录”由安装器固定管理，不作为用户可选程序目录。

## 11. MSI 属性设计

| 属性 | 类型 | 用途 | 规则 |
|---|---|---|---|
| `INSTALLDIR` | Public/Secure | 最终安装目录 | 视为不可信输入，必须重验 |
| `WIXUI_INSTALLDIR` | UI | 指向 `INSTALLDIR` | 固定为 `INSTALLDIR` |
| `FYAGENT_PREVIOUS_INSTALLDIR` | Secure | HKLM 读取的旧目录 | 仅升级/修复使用 |
| `FYAGENT_INSTALLDIR_VALID` | 内部 | `1`/`0` | 每次动作前先清空 |
| `FYAGENT_INSTALLDIR_ERROR_CODE` | 内部 | 稳定错误码 | 不包含秘密 |
| `FYAGENT_INSTALLDIR_ERROR_MESSAGE` | UI/Formatted | 用户文案 | 只使用固定本地化文本 |
| `FYAGENT_INSTALLDIR_CHECK_ID` | 内部 | 日志关联 ID | 短随机/递增标识 |
| `FYAGENT_INSTALLDIR_NORMALIZED` | 内部 | 规范化路径 | 不作为外部可覆盖属性 |

## 12. 路径优先级

### 12.1 干净首次安装

```text
完整 UI 用户选择
  或
静默安装管理员显式 INSTALLDIR
  或
默认 Program Files
```

所有路径最终都通过 Execute 校验。

### 12.2 升级/修复

```text
HKLM 中的既有 InstallDir
  > Windows Installer 已知目录
  > 其他输入
```

升级时不得接受外部 `INSTALLDIR` 改变安装位置。改变目录必须卸载后重新安装。

## 13. 错误码设计

建议稳定枚举：

```text
FYDIR001 EMPTY_OR_INVALID_PATH
FYDIR002 DEVICE_OR_UNC_PATH
FYDIR003 NOT_FIXED_DRIVE
FYDIR004 FILESYSTEM_WITHOUT_PERSISTENT_ACL
FYDIR005 FORBIDDEN_KNOWN_FOLDER
FYDIR006 ROOT_OR_SYSTEM_DIRECTORY
FYDIR007 REPARSE_POINT_DETECTED
FYDIR008 FINAL_PATH_MISMATCH
FYDIR009 OWNER_NOT_TRUSTED
FYDIR010 DACL_UNREADABLE
FYDIR011 UNSUPPORTED_ACE
FYDIR012 UNTRUSTED_WRITE_ACCESS
FYDIR013 PARENT_DELETE_CHILD_ACCESS
FYDIR014 TARGET_NOT_DEDICATED
FYDIR015 PATH_CHANGED_DURING_CHECK
FYDIR099 INTERNAL_VALIDATION_FAILURE
```

用户文案按 5–6 个类别归并；日志保留具体码。

## 14. 错误呈现规则

- UI 策略拒绝：动作返回成功，设置 `VALID=0` 和错误属性；WiX 弹出自定义提示并阻止导航；
- Execute 策略拒绝：动作设置属性并返回成功，紧随其后的 Type 19 Error Action 终止安装；
- DLL 无法加载/入口崩溃：属于安装器缺陷，可以失败并记录，但必须由 CI 阻止发布；
- 不使用 `Err.Raise`、脚本错误或原始 Win32 文本直接面向用户。

## 15. 本地化要求

0.2.1 至少提供当前安装器支持语言的固定文案。错误码不本地化；文案从 `.wxl` 或 WiX 本地化资源读取，不由 DLL拼接技术文本。

## 16. 可访问性与操作性

- 错误弹窗聚焦确认按钮；
- 关闭后焦点返回目录编辑框；
- 不清空用户已输入路径；
- 支持键盘返回和重新浏览；
- 文案不依赖颜色；
- 不显示重复统计、检查数量或“高级详情”入口，详细信息由日志提供。
