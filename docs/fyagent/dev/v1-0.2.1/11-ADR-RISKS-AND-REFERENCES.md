# 11 — 架构决策、风险与参考资料

## 1. 文档状态

以下决策已经由需求确认，实施者不得在没有新证据和显式设计评审的情况下改回被否决方案。

## 2. ADR-001：保留 per-machine，并允许选择“安全目录”

**状态**：Accepted  
**决策**：Windows MSI 保持 `perMachine + elevated`，默认 Program Files；用户可以选择其他本机固定磁盘上的 FyAgent 专用安全目录。

### 理由

- 用户明确要求可选择安装目录；
- FyAgent 当前是管理员权限应用，不能照搬 CC Switch 的用户可写 per-user 路径；
- WiX 标准 `WixUI_InstallDir` 已满足目录选择交互；
- 目录选择与固定 Program Files 不是二选一：默认值固定，最终值可在安全边界内修改。

### 否决方案

- **固定 Program Files、无目录页**：不满足产品需求；
- **允许任意目录**：管理员二进制可被普通用户篡改；
- **直接改成 per-user**：与当前权限、HKLM/ProgramData 和运行模型冲突；
- **GUI/privileged broker 拆分**：长期合理，但超出 `0.2.1` 小版本范围。

## 3. ADR-002：删除 VBScript/WMI，使用 Rust 原生 Type 1 Custom Action

**状态**：Accepted  
**决策**：删除内联 VBScript，新增独立 Rust `cdylib`，以 MSI Binary 表内嵌 DLL，并提供 UI/Execute 两个入口。

### 理由

- 当前脚本存在确定性的 `GetSecurityDescriptor` 调用错误；
- `Err.Raise` 把可恢复的目录策略拒绝转换成 Error 1720；
- VBScript 已进入弃用路径，不能作为新长期安全边界；
- ACL、reparse point、最终路径和有效权限需要 Win32 API 与强类型错误处理；
- Type 1 是 Windows Installer 对 Binary 表 DLL 的标准模型。

### 否决方案

- **只修一行 WMI 调用**：仍保留脚本依赖、错误模型和薄弱测试；
- **PowerShell/外部 EXE**：增加目标机依赖、命令行和安全边界；
- **把主 Tauri crate 改成 cdylib**：会混合应用和安装器职责，并可能影响 bundler 资源；
- **自定义 bootstrapper**：交互成本和发布面过大。

## 4. ADR-003：UI 与 Execute 双重校验，共享核心

**状态**：Accepted  
**决策**：用户点击 Next 时提供即时反馈；执行序列在 `InstallValidate` 前重新校验。两个入口调用同一核心策略。

### 理由

- 仅 UI 校验可被 `/qn` 和命令行属性绕过；
- UI 通过后路径/ACL 可能变化；
- 仅 Execute 校验会让用户到安装后期才看到失败；
- 同一核心避免 UI/静默规则漂移。

### 约束

- UI 中的预期拒绝返回 MSI success，用属性控制导航；
- Execute 中的拒绝使用明确 Type 19 消息终止；
- 卸载排除校验；
- 校验只读且 fail closed。

## 5. ADR-004：一个补丁版本 `0.2.1`

**状态**：Accepted  
**决策**：安装器修复、原生目录校验和全局版本治理在同一个 FyAgent `0.2.1` 中完成，不先发布无安全校验的过渡版本。

### 理由

- 用户明确不需要拆成两个版本；
- 公开发布“先删除校验、以后补安全”会形成已知风险窗口；
- 版本治理先落地后可让新 helper crate 从一开始共享版本。

## 6. ADR-005：FyAgent 全局版本单一真源设为 Cargo workspace

**状态**：Accepted  
**决策**：`src-tauri/Cargo.toml [workspace.package].version` 是唯一人工维护版本。

### 理由

- Tauri 可在配置版本缺省时读取 Cargo package 版本；
- 主应用和 Rust helper 可用 Cargo workspace 原生继承；
- 项目不发布 npm 包，`package.json.version` 没有必要；
- 避免脚本在 JSON、TOML 和文本之间同步多个决策源。

### 否决方案

- **package.json 为真源**：仍需同步 Cargo，且易混淆 npm 依赖版本；
- **tauri.conf.json 为真源**：helper crate 不能原生继承；
- **独立 VERSION 文件**：Tauri/Cargo 均需额外生成步骤，增加间接层；
- **保留三处并用测试比较**：发现不一致但仍要求人工多点修改。

## 7. ADR-006：稳定三段 SemVer，不支持预发布

**状态**：Accepted  
**决策**：脚本只接受 `X.Y.Z`，并施加 MSI 数值上限。

### 理由

- 当前正式发布没有 alpha/beta/rc 通道契约；
- MSI ProductVersion 只有三个受限数值字段；
- 允许 prerelease 但没有升级/更新比较设计会造成跨平台语义不一致；
- 稳定格式使标签、资产名和支持诊断简单明确。

## 8. ADR-007：正式 MSI 必须经过原生 Windows 生命周期门禁

**状态**：Accepted  
**决策**：Wine/Linux 交叉构建只产生候选 MSI；公开 MSI 必须在原生 Windows runner/VM 上完成表检查、安装、升级、修复、卸载和签名验证。

### 理由

- 本次故障恰好是静态生成成功但目标 Windows 执行失败；
- 自定义动作加载、UAC、ACL、ARM64 和 Windows Installer UI/Execute 行为需要真实环境；
- 结构检查不是生命周期测试。

## 9. 主要风险登记

| ID | 风险 | 影响 | 缓解 | 发布状态 |
|---|---|---|---|---|
| R-001 | ACL 有效权限算法错误 | 允许提权程序落入可篡改路径 | Authz/真实 token、负向 ACL fixture、安全审查 | 阻断 |
| R-002 | 父目录 `DELETE_CHILD`/抢先创建未覆盖 | 目录可被替换 | 检查最近存在祖先、父目录有效权限、竞态测试 | 阻断 |
| R-003 | reparse point/路径规范化绕过 | 指向用户可控目标 | handle-based、`OPEN_REPARSE_POINT`、逐祖先检查、Execute 重验 | 阻断 |
| R-004 | 目录策略过严 | 合法 D 盘目录被拒绝 | 明确定义受保护专用目录；收集稳定错误码，不放宽 fail-closed | 可接受但需用例 |
| R-005 | 目录策略过松 | 本地提权攻击面 | 安全 review、负向矩阵、默认只允许可证明安全 | 阻断 |
| R-006 | Rust panic/FFI 未定义行为 | msiexec 崩溃或 1723 | `catch_unwind`、最小 unsafe、FFI 测试 | 阻断 |
| R-007 | x64/ARM64 DLL错配 | Custom Action 加载失败 | PE Machine 静态门禁 + 两架构真实安装 | 阻断 |
| R-008 | 安全软件拦截临时 CA DLL | 企业环境安装失败 | 代码签名/信誉测试、最小依赖、真实企业策略试验、保留日志 | 残余风险 |
| R-009 | WiX 自定义 Fragment 与 Tauri版本不兼容 | MSI 无法构建/UI异常 | 锁定上游 UI 源版本；必要时内嵌同逻辑；表测试 | 阻断 |
| R-010 | Major Upgrade 路径恢复错误 | 搬迁/重复安装/升级失败 | HKLM 搜索条件、0.2.0 fixture、禁止外部迁移 | 阻断 |
| R-011 | 卸载被校验阻断 | 用户无法移除损坏安装 | `REMOVE~="ALL"` 明确排除，损坏路径测试 | 阻断 |
| R-012 | Cargo workspace 改造影响构建 | 非 Windows CI失败 | cfg/target 控制、`cargo --workspace` 矩阵、无依赖升级 | 阻断 |
| R-013 | 版本脚本解析器随文件结构漂移 | 后续无法更新版本 | 未知结构直接失败、单元测试、文档化迁移 | 可接受 |
| R-014 | 手工重新加入重复版本字段 | 再次分叉 | CI `version:check` 阻断 | 阻断 |
| R-015 | 标签重用/多 SHA 混入 | 发布不可追踪 | exact tag、source SHA outputs、artifact provenance | 阻断 |
| R-016 | 资产名变化影响下载方 | 旧链接/自动化失效 | 明确迁移说明、manifest 为稳定发现接口、必要时兼容别名 | 需评估 |
| R-017 | package.json 无 version 与某工具不兼容 | 前端工具失败 | 先在 CI/本地验证；若确有硬依赖，设计生成投影而非人工字段 | 低 |
| R-018 | 直接编辑 Cargo.lock 与未来格式不兼容 | 脚本失败 | 精确结构检查，失败不写；升级脚本时测试新格式 | 可接受 |
| R-019 | 校验日志泄漏用户路径/身份 | 隐私问题 | 规范化脱敏、稳定 code、日志审查 | 阻断 |
| R-020 | 0.2.0 已装在不安全目录 | 用户无法原地升级 | 明确提示卸载重装；不自动接管/搬迁 | 已知迁移风险 |

## 10. 剩余风险与边界

### 10.1 自定义动作不是理想的长期产品架构

管理员权限 GUI 从可选目录运行本身要求复杂的安装安全策略。长期最小权限架构仍是普通 GUI + 固定受保护的 privileged service/broker。但本次用户需求是允许选择其他软件目录，并保持现有架构，因此选择受限目录策略。

### 10.2 无法证明安全时拒绝

域策略、第三方文件系统、特殊卷和复杂 ACL 可能导致误拒绝。为避免把“不理解”当安全，`0.2.1` 明确 fail closed。支持流程应通过错误码和 verbose log定位，而不是让用户关闭校验。

### 10.3 Windows Installer ProductVersion 上限

当前让全平台版本受 MSI 数值上限约束。对现阶段版本空间足够；若未来超过上限，需要新的映射/产品代码设计。

### 10.4 预发布版本

本方案没有 prerelease。临时 CI build 可用 run number、SHA 作为非产品构建标识，但不得写入 `APP_VERSION` 或冒充可升级正式版本。

## 11. 实施时必须验证的假设

以下不是待用户确认项，而是工程验证项：

1. 当前 Tauri 版本对 `wix.fragmentPaths` 和自定义模板的实际合并方式；
2. WiX v3 `WixUI_InstallDir` 的确切 Publish 条件和 maintenance 分支；
3. 当前 UpgradeCode/ProductCode 策略能完成 `0.2.0 -> 0.2.1` major upgrade；
4. 主应用 manifest 的 `requireAdministrator` 及其构建注入路径；
5. HKLM 产品键制造商/产品名稳定；
6. x64/ARM64 Windows runner 可构建 Rust `cdylib` 并被 MSI 加载；
7. `package.json.version` 删除不会触发现有 Vite/pnpm 插件依赖；
8. Cargo workspace 引入后现有 CI 命令的默认 package行为；
9. 目标支持的最低 Windows 版本具备所用 Win32 API；
10. 最终 PermissionEx SDDL 与应用运行时写入需求兼容。

验证失败时先调整实现细节，不改变已确认的产品和安全决策。

## 12. 官方与主源参考

检索日期：2026-08-06。实施时应以锁定工具版本对应文档和源码为准。

### 12.1 Tauri

- [Tauri v2 Configuration — `version` 缺省时使用 Cargo package 版本](https://v2.tauri.app/reference/config/)
- [Tauri v2 Windows Installer — WiX template、fragments 与本地化](https://v2.tauri.app/distribute/windows-installer/)

### 12.2 Cargo 与 npm

- [The Cargo Book — Workspaces and workspace package inheritance](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [The Cargo Book — Manifest format](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [npm package.json — 非发布包的 name/version 可选](https://docs.npmjs.com/cli/v8/configuring-npm/package-json/)
- [npm registry guidance — `private: true`](https://docs.npmjs.com/cli/v6/using-npm/registry/)

### 12.3 版本规范

- [Semantic Versioning 2.0.0](https://semver.org/)
- [Microsoft — Windows Installer ProductVersion](https://learn.microsoft.com/en-us/windows/win32/msi/productversion)

### 12.4 WiX 与 Windows Installer

- [WiX Toolset v3 Manual](https://docs.firegiant.com/wix3/)
- [WiX v3 `WixUI_InstallDir` dialog set](https://docs.firegiant.com/wix3/wixui/dialog_reference/wixui_installdir/)
- [Microsoft — Custom Action Type 1](https://learn.microsoft.com/en-us/windows/win32/msi/custom-action-type-1)
- [Microsoft — Summary of custom action types](https://learn.microsoft.com/en-us/windows/win32/msi/summary-list-of-all-custom-action-types)
- [Microsoft — Guidelines for authoring secure installations](https://learn.microsoft.com/en-us/windows/win32/msi/guidelines-for-authoring-secure-installations)
- [Microsoft — Windows Installer best practices](https://learn.microsoft.com/en-us/windows/win32/msi/windows-installer-best-practices)
- [Microsoft — Error messages / Error 1720](https://learn.microsoft.com/en-us/windows/win32/msi/windows-installer-error-messages)
- [Microsoft — Windows Installer logging / msiexec](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec)

### 12.5 Win32 路径与安全 API

- [GetDriveTypeW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdrivetypew)
- [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
- [GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew)
- [GetFileInformationByHandleEx](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandleex)
- [GetNamedSecurityInfoW](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getnamedsecurityinfow)
- [AuthzAccessCheck](https://learn.microsoft.com/en-us/windows/win32/api/authz/nf-authz-authzaccesscheck)
- [DeleteFile / parent `DELETE_CHILD` semantics](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-deletefilew)

### 12.6 VBScript 退出路径

- [Microsoft Windows IT Pro Blog — VBScript deprecation timelines](https://techcommunity.microsoft.com/blog/windows-itpro-blog/vbscript-deprecation-timelines-and-next-steps/4148301)

### 12.7 参考项目

- [CC Switch repository](https://github.com/farion1231/cc-switch)
- [CC Switch per-user WiX template](https://raw.githubusercontent.com/farion1231/cc-switch/main/src-tauri/wix/per-user-main.wxs)

CC Switch 仅作为目录选择交互和路径记忆的主源参考；FyAgent 不继承其 per-user 权限模型。
