# FyAgent V1 Windows 平台适配

## Goal

在 Core 冻结的 trait/DTO 上完成 Windows x64 与 ARM64 的 Stable 官方桌面应用检测、
MSIX 验证、当前用户部署、安装后检查和 AUMID 启动；另提供无法经普通 IPC/UI 触达的
隐藏 all-users 实验 headless 路径。

## Requirements

- 在 Windows 专属模块安全解析 root `AppxManifest.xml`，限制大小、拒绝重复/加密/
  DTD/external entity/path 变体，精确验证 `OpenAI.Codex`、Publisher、架构、四段版本、
  MinVersion 与可启动 Application ID。
- 用 fakeable PackageManager facade 查询当前用户 Stable 包，拒绝 Beta/其他 OpenAI
  包/歧义结果，不扫描磁盘或以 exe/进程名推断身份。
- 普通安装只使用已验证本地 `file://` MSIX 的 WinRT PackageManager current-user
  路径；进度、HRESULT、策略、缺依赖、签名、占用和后验检测映射到稳定错误。
- 启动从已验证包的 Package Family Name 与 manifest Application ID 建造 AUMID；不直接
  启动 WindowsApps exe。
- ARM64 只接受 ARM64 latest，不回退 x64；all-users 只能是严格参数的同一程序
  headless + UAC child 测试路径，并重新验证 job file、nonce、path、hash、identity、
  publisher 和 arch。

## Acceptance Criteria

- [ ] fixture 覆盖 Stable x64/ARM64、Beta、错误 Publisher/Identity/Arch、多个 app、
  错误 min OS、malformed/D TD/duplicate manifest，且每项 fail closed。
- [ ] fake PackageManager 覆盖 local query、current-user success/progress、package-in-use、
  policy/dependency/signature/unknown HRESULT、post-check mismatch 与 AUMID launch。
- [ ] 普通 Tauri IPC/DTO 不含 scope、URL、path 或 bypass 字段，且无法触达 all-users。
- [ ] 所有 users 的受限测试覆盖 nonce mismatch、过期、temp-root escape、文件替换和
  UAC 取消；它不阻断 V1，但不能只是无验证 stub。
- [ ] Windows x64 与 `aarch64-pc-windows-msvc` 有明确编译/target evidence；不执行真实
  PackageManager、UAC、安装、卸载或强杀目标应用。

## Ownership

仅修改 `src-tauri/src/codex_desktop/platform/windows/**` 和专属 fixture/test。不得修改
Cargo、`main.rs`、`lib.rs`、普通 command 注册或其他平台文件；所需 target dependency、
headless 注册、shared contract patch 以小型说明交给 integration/Core。

## Evidence Gate

Publisher allowlist 必须由当前官方已签名 package 的匿名 manifest fixture 与系统信任
取证建立。`OpenAI.Codex`、PFN 后缀或镜像 metadata 不能替代该证据；其未到位时不得
降低验证或宣称 Windows P0 完成。
