# Windows 实施计划

1. 依照 Core trait 增加 `platform/windows` cfg skeleton 与 fixture parser tests。
2. 完成 MSIX manifest 审计、identity/publisher/arch/version/min-OS gate 与稳定错误映射。
3. 引入 fake PackageManager，完成 current-user inventory、deploy progress、post-check、
   AUMID launch 与 HRESULT tests。
4. 增加严格 headless all-users parser/job verifier/elevation facade tests；不接普通 IPC。
5. 在 Windows x64 与 ARM64 target 上编译/测试，记录不执行真实安装的证据。
6. 向 integration 提交最小 windows crate features、`main.rs` headless hook 与共享注册
   请求，不自行改共享文件。
