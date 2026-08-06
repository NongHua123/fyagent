# 04 — 原生安装目录校验器设计

## 1. 目标

用独立原生 DLL 替代当前 VBScript/WMI Custom Action，并提供：

- 可在 Windows x64/ARM64 构建的 Rust `cdylib`；
- UI 与 Execute 入口共享的纯核心策略；
- 只读、fail-closed 的路径和 ACL 验证；
- 稳定错误码、简明用户消息和详细 MSI 日志；
- 可单元测试的 Win32 封装层；
- 不依赖目标机脚本引擎、PowerShell、WMI 或外部进程。

## 2. crate 布局

```text
src-tauri/
├─ Cargo.toml                         # workspace root + fyagent 主包
└─ installer-actions/
   ├─ Cargo.toml
   └─ src/
      ├─ lib.rs                       # MSI 导出、panic 边界
      ├─ msi.rs                       # MSI 属性和日志封装
      ├─ policy.rs                    # 纯策略编排
      ├─ path.rs                      # 输入、规范化、known folders
      ├─ volume.rs                    # 驱动器和文件系统能力
      ├─ reparse.rs                   # 路径组件/句柄/重解析点
      ├─ security.rs                  # owner、DACL、ACE 和权限位
      ├─ error.rs                     # 稳定错误码与分类
      └─ tests/                       # Windows 集成辅助（可放 crate tests）
```

主应用 crate 保持现有 `staticlib/rlib`。安装器动作是单独的 workspace member，避免 Tauri 主包的 crate-type 和 MSI 资源规则相互影响。

## 3. Cargo 配置

设计片段：

```toml
[package]
name = "fyagent-installer-actions"
version.workspace = true
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
  "Win32_Foundation",
  "Win32_Security",
  "Win32_Security_Authorization",
  "Win32_Storage_FileSystem",
  "Win32_System_Memory",
  "Win32_System_Msi",
  "Win32_UI_Shell",
] }
```

实际 feature 名称必须以实施时锁定的 `windows` crate 版本验证。非 Windows 目标可以通过 `compile_error!` 或仅在 Windows 构建该 crate，避免产生无意义产物。

## 4. DLL 导出接口

### 4.1 建议入口

```rust
#[no_mangle]
pub unsafe extern "system" fn ValidateFyAgentInstallDirUi(
    install: MSIHANDLE,
) -> u32;

#[no_mangle]
pub unsafe extern "system" fn ValidateFyAgentInstallDirExecute(
    install: MSIHANDLE,
) -> u32;
```

两个入口只负责：

1. 建立 `catch_unwind` 边界；
2. 从 MSI 读取 `INSTALLDIR` 和安装上下文；
3. 调用同一 `validate_install_directory()`；
4. 写属性和日志；
5. 按 UI/Execute 通道返回约定值。

### 4.2 不直接从核心返回 MSI 状态

核心接口：

```rust
pub fn validate_install_directory(
    requested: &OsStr,
    context: &ValidationContext,
) -> Result<ValidatedDirectory, DirectoryPolicyError>;
```

这样可在不启动 Windows Installer 的情况下测试大部分逻辑。

### 4.3 panic 边界

任何 Rust panic 不得跨 FFI：

```rust
match std::panic::catch_unwind(|| run_action(...)) {
    Ok(code) => code,
    Err(_) => {
        log_fatal(..., "FYDIR099");
        ERROR_INSTALL_FAILURE
    }
}
```

策略拒绝不是 panic，也不是 DLL 故障。

## 5. 数据模型

### 5.1 ValidationContext

```rust
struct ValidationContext {
    phase: ValidationPhase,       // Ui | Execute | Upgrade | Repair
    is_first_install: bool,
    is_upgrade: bool,
    is_repair: bool,
    expected_existing_product: bool,
    trusted_install_dir: Option<PathBuf>,
    product_marker: ProductMarkerPolicy,
}
```

### 5.2 ValidatedDirectory

```rust
struct ValidatedDirectory {
    requested_display: PathBuf,
    normalized_dos_path: PathBuf,
    volume_root: PathBuf,
    nearest_existing_ancestor: PathBuf,
    target_exists: bool,
    target_state: TargetState,
    check_id: String,
}
```

### 5.3 DirectoryPolicyError

```rust
enum DirectoryPolicyError {
    EmptyOrInvalidPath,
    DeviceOrUncPath,
    NotFixedDrive,
    PersistentAclUnsupported,
    ForbiddenKnownFolder,
    RootOrSystemDirectory,
    ReparsePointDetected { component_index: usize },
    FinalPathMismatch,
    OwnerNotTrusted,
    DaclUnreadable { win32: u32 },
    UnsupportedAce,
    UntrustedWriteAccess,
    ParentDeleteChildAccess,
    TargetNotDedicated,
    PathChangedDuringCheck,
    InternalFailure { stage: &'static str, win32: Option<u32> },
}
```

错误对象可以保留内部字段，但写入 MSI 公共属性时只暴露稳定代码和固定文案类别。

## 6. 校验总流程

```text
读取 INSTALLDIR
  ↓
语法与命名空间检查
  ↓
规范化为绝对 DOS 路径
  ↓
识别卷根、驱动器类型和文件系统能力
  ↓
known-folder / 系统目录排除
  ↓
分解路径并寻找最近存在祖先
  ↓
对每个既有组件使用句柄检查 reparse point 和最终路径
  ↓
读取 owner + DACL，执行可信主体和危险权限策略
  ↓
检查目标是否为空/受认可旧安装
  ↓
重复关键路径与安全信息检查
  ↓
返回规范化结果或稳定错误
```

## 7. 输入与规范化

### 7.1 输入限制

在调用任何文件系统 API 前拒绝：

- 空字符串；
- NUL；
- 相对路径；
- 驱动器相对路径；
- UNC；
- 用户输入的 `\\?\`、`\\.\`、NT device path；
- ADS 语法或非盘符位置中的冒号；
- 目标等于卷根；
- 超过内部上限的路径。

### 7.2 规范化策略

1. 使用宽字符 API；
2. 将 `/` 归一为 `\`；
3. 解析 `.`、`..`，禁止越过卷根；
4. 保留用户展示路径，但安全比较使用大小写不敏感、标准分隔符的规范形式；
5. 内部调用需要长路径时由代码添加 `\\?\`，不接受用户自己提供；
6. 不通过 `PathCanonicalize` 等会隐藏错误或受 MAX_PATH 限制的旧接口承担全部安全语义；
7. 对最终存在组件用句柄路径确认，而不是只信任字符串规范化。

## 8. 卷与文件系统检查

### 8.1 驱动器类型

对卷根调用 `GetDriveTypeW`，只接受 `DRIVE_FIXED`。

拒绝：

```text
DRIVE_REMOTE
DRIVE_REMOVABLE
DRIVE_CDROM
DRIVE_RAMDISK
DRIVE_NO_ROOT_DIR
DRIVE_UNKNOWN
```

“USB 设备”可能被系统报告为 fixed，因此若产品必须排除所有可拔设备，需要后续增加设备移除策略查询。0.2.1 的最低契约是 Win32 报告为 `DRIVE_FIXED` 且文件系统/ACL/路径策略全部通过。

### 8.2 文件系统能力

通过 `GetVolumeInformationW` 获取标志，必须包含 `FILE_PERSISTENT_ACLS`。不按文件系统名称硬编码 NTFS，因为 ReFS 等也可能提供符合要求的 ACL 语义；能力标志是主判断。

### 8.3 卷根策略

目标不得等于卷根。对最近存在祖先为卷根的场景，只有当卷根本身不允许非可信主体创建首个目标组件且其他检查通过时才允许。普通消费级数据盘根目录通常会因此被拒绝，推荐用户选择管理员预先创建的 `D:\Applications\FyAgent`。

## 9. known folders 与系统路径

使用 `SHGetKnownFolderPath` 获取实际路径，避免硬编码本地化目录名。至少收集：

- `FOLDERID_Profile`；
- `FOLDERID_Desktop`；
- `FOLDERID_Documents`；
- `FOLDERID_Downloads`；
- `FOLDERID_LocalAppData`；
- `FOLDERID_RoamingAppData`；
- `FOLDERID_LocalAppDataLow`；
- `FOLDERID_ProgramData`；
- `FOLDERID_Windows`；
- `FOLDERID_System`；
- `FOLDERID_SystemX86`；
- 系统临时目录。

规则：

- 用户目录和 Temp 的自身及后代一律拒绝；
- Windows/System 目录自身及后代一律拒绝；
- ProgramData 根及通用数据树不作为可选二进制安装目录；
- Program Files 根本身拒绝，但其 `FyAgent` 专用子目录允许；
- known folder API 失败属于无法确认，拒绝并记录 Win32/HRESULT。

## 10. 路径组件与重解析点

### 10.1 逐组件检查

对每个已存在路径组件：

1. 使用 `CreateFileW` 打开目录；
2. 标志至少包含：
   - `FILE_FLAG_BACKUP_SEMANTICS`；
   - `FILE_FLAG_OPEN_REPARSE_POINT`；
3. 共享模式允许读/写/删除共享，降低与其他进程不必要冲突；
4. 使用 `GetFileInformationByHandleEx` 或属性 API读取 reparse tag/属性；
5. 任何 `FILE_ATTRIBUTE_REPARSE_POINT` 都拒绝，不尝试对白名单 tag 放行；
6. 使用 `GetFinalPathNameByHandleW` 取得最终路径，并与预期组件比较。

0.2.1 采取“任何路径组件有重解析点即拒绝”的保守策略，避免对 junction、symlink、mount point、cloud placeholder 等逐类解释。

### 10.2 最近存在祖先

从目标向上寻找第一个已存在目录。所有更下层是将由 MSI 创建的缺失组件。最近存在祖先的 ACL 决定攻击者能否抢先创建首个缺失组件。

### 10.3 句柄与 TOCTOU

尽可能在验证期间保持关键祖先目录句柄打开，并在结束前重新读取：

- 最终路径；
- 文件 ID/卷序列；
- reparse 属性；
- 安全描述符摘要。

Windows Installer 创建目录前仍存在不可完全消除的竞态，因此 Execute Sequence 必须紧邻 `InstallValidate`/目录创建前运行，最终目录创建后还要由受保护 DACL约束。设计目标是显著缩小和检测竞态，不宣称完全原子地锁定任意路径树。

## 11. 所有者与 DACL 检查

### 11.1 读取方式

优先对已打开句柄使用 `GetSecurityInfo`；按名称只能作为补充。读取：

```text
OWNER_SECURITY_INFORMATION
DACL_SECURITY_INFORMATION
```

不需要读取 SACL，也不启用额外审计权限。

### 11.2 所有者策略

所有者 SID 必须在可信白名单中。NULL owner、读取失败或未知结构均拒绝。

### 11.3 DACL 基本条件

- 安全描述符有效；
- DACL present；
- NULL DACL 一律拒绝；
- ACL revision/size 合法；
- 每个 ACE 边界可解析；
- 未支持的 callback/object/conditional ACE 默认拒绝，除非实现能够证明其不授予危险权限。

### 11.4 权限评估策略

目标是证明：**只有可信主体具有影响该程序路径完整性的危险能力**。

推荐实现两层：

1. 对常见低权限主体（Everyone、Authenticated Users、Builtin Users、当前标准用户/其组）使用 `AuthzAccessCheck` 或等价有效权限计算；
2. 再扫描所有 Allow ACE：任何非可信 trustee 获得危险权限且无法被准确证明无效时，按不安全处理。

这样既考虑 deny/group 语义，也不遗漏显式授予某个未知用户/域组的危险 Allow ACE。

### 11.5 危险权限集合

对目标和最近存在父目录：

```text
FILE_ADD_FILE
FILE_ADD_SUBDIRECTORY
FILE_WRITE_EA
FILE_WRITE_ATTRIBUTES
DELETE
WRITE_DAC
WRITE_OWNER
GENERIC_WRITE
GENERIC_ALL
```

对既有组件的直接父目录额外检查：

```text
FILE_DELETE_CHILD
```

对应用二进制目录，任何能写入文件、创建可加载文件、删除/重命名组件或改变安全描述符的能力都应视为完整性风险。

### 11.6 ACE 继承处理

- `INHERIT_ONLY_ACE` 不作用于当前对象，但可能作用于将创建的子对象；
- 对最近存在父目录，继承给新 FyAgent 目录的 ACE 也必须分析；
- `CREATOR_OWNER` 的继承 ACE不能简单全部忽略：需要结合 MSI 最终 protected DACL 是否会阻断继承；
- 当前模板会对最终 `INSTALLDIR` 应用受保护 DACL，但在目录创建与权限落地之间仍要避免不受信任父目录；
- 不能仅按 ACE 顺序做位运算，需遵守 Windows 访问检查语义或采用保守拒绝。

### 11.7 不修改 ACL

校验器不调用 `SetNamedSecurityInfo`，不接管所有权。最终 `INSTALLDIR` 的安全描述符由 WiX `PermissionEx`/MSI 标准动作事务性设置。若父目录不安全，拒绝而不是“自动修好”。

## 12. 目标专用性检查

### 12.1 首次安装

允许：

- 目标不存在；
- 目标存在且为空。

拒绝：

- 目标存在且含任意无关内容；
- 用户直接选择 Program Files、`D:\Applications` 等共享父目录本身。

### 12.2 升级/修复

允许非空目标的前提是：

- Windows Installer 检测到相关 FyAgent 产品；
- HKLM `InstallDir` 与目标规范化路径一致；
- 可验证产品标识/主 EXE/组件注册与该路径关联；
- 不是仅凭目录名等弱信号。

## 13. UI/Execute 入口行为

### 13.1 UI 入口

无论策略通过或拒绝，正常执行都返回 `ERROR_SUCCESS`：

```text
通过：
  FYAGENT_INSTALLDIR_VALID=1
  清空错误属性

拒绝：
  FYAGENT_INSTALLDIR_VALID=0
  FYAGENT_INSTALLDIR_ERROR_CODE=FYDIR...
  FYAGENT_INSTALLDIR_ERROR_MESSAGE=<固定本地化类别>
```

只有 DLL 无法正常运行、MSI 属性 API 失败或 panic 才返回安装失败；此类情况应由 CI 发现。

### 13.2 Execute 入口

同样设置属性并写日志。正常策略拒绝建议仍返回 `ERROR_SUCCESS`，随后由 WiX Type 19 Error Action 基于 `VALID<>1` 明确终止。这能避免 Windows Installer 把拒绝描述成 DLL Custom Action 崩溃。

另一可接受实现是 Execute 入口返回 `ERROR_INSTALL_FAILURE` 并通过 MSI 记录自定义消息，但必须验证最终用户看到的不是通用 1723。Type 19 方案更可控。

## 14. MSI API 封装

`msi.rs` 提供：

```rust
fn get_property(handle, name) -> Result<OsString, MsiError>;
fn set_property(handle, name, value) -> Result<(), MsiError>;
fn log_info(handle, code, fields) -> Result<(), MsiError>;
fn log_error(handle, code, fields) -> Result<(), MsiError>;
```

要求：

- 正确处理 `MsiGetPropertyW` 两次调用的缓冲区长度；
- 不假设 MAX_PATH；
- 所有属性名为编译期常量；
- 写日志使用 `MsiProcessMessage`，不写任意磁盘文件；
- 路径日志可保留完整安装路径，但不得包含额外用户文件枚举。

## 15. 日志格式

建议单行结构：

```text
FyAgentInstallDir check_id=8F21 phase=execute result=reject code=FYDIR012 path="D:\Public\FyAgent" stage=dacl win32=0
```

成功：

```text
FyAgentInstallDir check_id=8F21 phase=execute result=allow path="D:\Applications\FyAgent" volume=fixed target=missing
```

不要输出完整 ACE 列表到默认日志，避免泄漏域账户结构并造成噪音。调试构建可增加受控详细级别，但正式 MSI 默认只记录决定性摘要。

## 16. 构建产物

建议输出：

```text
src-tauri/target/x86_64-pc-windows-msvc/release/fyagent_installer_actions.dll
src-tauri/target/aarch64-pc-windows-msvc/release/fyagent_installer_actions.dll
```

WiX 通过环境变量接收对应路径：

```text
FYAGENT_INSTALLER_ACTIONS_DLL=<absolute path>
```

模板使用 `$(env.FYAGENT_INSTALLER_ACTIONS_DLL)`，避免把架构路径硬编码在 `.wxs`。

## 17. 签名和防护软件

- MSI 外层必须签名；
- 主 EXE 必须签名；
- 建议动作 DLL也签名后再嵌入 MSI，便于 ASR/EDR 识别和取证；
- 签名后再打包，打包后再签 MSI；
- 在企业 Windows Defender/ASR 策略下验证从 Binary 表临时提取的 DLL 可执行；
- 若动作 DLL被安全软件阻止，日志和发布测试应明确区分策略拦截与目录拒绝。

## 18. 测试接口与可测试性

### 18.1 纯单元测试

- 路径语法；
- known folder 子路径判定；
- 错误码映射；
- 目标状态判断；
- 版本/架构元数据。

### 18.2 Windows 集成测试

在临时 NTFS 卷/目录创建 ACL 场景：

- 管理员控制父目录；
- Users 可创建子目录；
- Users 可写文件；
- 父目录仅有 `DELETE_CHILD`；
- 目标自身受保护但父目录可替换；
- NULL DACL；
- unknown/callback ACE；
- symlink/junction；
- ACL 读取拒绝；
- 路径在检查中变化。

测试必须恢复 ACL 和目录，建议在隔离 Windows VM 中以管理员测试账户运行。

### 18.3 FFI 冒烟测试

用小型 MSI 测试或 Windows Installer harness 验证：

- 导出名无装饰且能被 Type 1 CA 调用；
- `extern "system"` 调用约定正确；
- x64/ARM64 架构匹配；
- MSI 属性读写和日志正常；
- panic 被捕获；
- UI 策略拒绝返回成功。

## 19. 安全审查重点

实施评审必须逐项确认：

- 是否在任何路径上跟随了 reparse point；
- 是否错误地把“当前管理员用户”默认视为可信 owner；
- 是否漏检父目录 `DELETE_CHILD`；
- 是否把 NULL DACL 当作无权限；
- 是否对未知 ACE 默认为允许；
- 是否只在 UI 校验；
- 是否通过创建测试文件来判断可写；
- 是否在校验器里修改 ACL；
- 是否因 MAX_PATH 截断比较；
- 是否在 FFI 中让 panic/异常跨边界；
- 是否用账户名称而不是 SID；
- 是否允许选择共享目录本身。

## 20. 设计限制

该校验器提供严格的安装时完整性准入，但不能替代：

- 运行时 DLL 搜索路径加固；
- 主程序签名与完整性检查；
- Windows 服务/IPC 权限设计；
- 企业终端安全策略；
- 对安装后管理员主动降低 ACL 的持续监控。

这些属于更广泛的提权应用安全边界，不应由 MSI 目录校验单独承担。
