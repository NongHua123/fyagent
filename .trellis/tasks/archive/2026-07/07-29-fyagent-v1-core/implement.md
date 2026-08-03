# Core 实施计划

1. 建立 fixture 与 raw manifest/checksum 解析测试，记录 schema v5 事实和不接受字段。
2. 实现 DTO、版本比较、canonical release ID、稳定错误和诊断 formatter，锁定 JSON
   contract fixture。
3. 实现 Job controller、状态转换、取消/并发/sequence/event tests 和 unsupported adapter。
4. 实现 source cache、固定 endpoint、checksums/manifest 交叉校验、metadata drift tests。
5. 实现 scoped HTTP、redirect/retry/stream/progress/cancel/temp/size/hash/disk validation，
   全部以 mock transport/filesystem 测试。
6. 实现可注入 service 和普通 command 薄壳，并以 fake platform 完成 service paths。
7. 运行受影响 Rust tests、format/clippy；向 integration 提交共享注册/依赖需求和冻结的
   DTO、command、event 清单。

## Validation

至少运行受影响 `cargo test`，最终在集成分支运行完整 Rust 质量门。不得执行真实下载、
安装、UAC 或写系统 Applications 目录。

## 2026-07-29 集成验证记录

以下结果在 Windows 开发主机、分支 `feature/fyagent-v1` 的未提交 V1 工作树执行，
基线提交为 `2400031a85f6b45b4db7aec89394b997a88826a8`；Rust 目标为
`x86_64-pc-windows-msvc`。自动化没有触发真实安装、UAC、PackageManager
Stage/Provision、卸载或 macOS `/Applications` 写入。

| 类别             | 命令 / 证据                                                                                                                                                                                                                                      | 退出码与结果                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| 前端类型与格式   | `pnpm typecheck`；`pnpm format:check`                                                                                                                                                                                                            | 0；均通过                                                                                         |
| 前端全测         | `pnpm test:unit`                                                                                                                                                                                                                                 | 0；85 files / 556 tests 通过                                                                      |
| 前端生产构建     | `pnpm build:renderer`                                                                                                                                                                                                                            | 0；Vite production build 通过                                                                     |
| Rust 格式与 lint | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`；`cargo clippy --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --lib --no-deps -- -D warnings`                                                                  | 0；均通过                                                                                         |
| Rust 全测        | `cargo test --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc`                                                                                                                                                                | 0；library suite 报告 2337 tests，所有仓库 integration test binaries 也完成                       |
| Rust 编译        | `cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc`                                                                                                                                                               | 0；通过                                                                                           |
| macOS 静态证据   | 下载当前 arm64 DMG 到临时目录后计算 SHA-256、只读提取顶层 Info.plist/Mach-O header；`cargo test --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --lib recorded_macos_identity_fixture_matches_the_exact_stable_allowlists`  | 0；hash 与当前 manifest 一致，fixture/allowlist 回归通过；未把 Windows 结果当作 native trust 验收 |
| 关键回归         | checksum mismatch 后强制 metadata refresh、direct local same/newer launch-only、restart claim、all-users TOCTOU、普通 IPC 七命令、DTO fixture、更新态只显示“更新 Codex”、metadata R2 redirect 与 `REDIRECT_REJECTED` 分类、无 updater 签名资产链 | 由 full Rust/Vitest suite 覆盖，均通过                                                            |

Rust 全测会输出现有 test-only dead-code warnings；前端会输出 browsers data、`punycode`、
MSW mock/预期错误等既有警告，均未改变上述命令的 exit code。已成功执行
`rustup target add aarch64-pc-windows-msvc`；但
`cargo check --manifest-path src-tauri/Cargo.toml --target aarch64-pc-windows-msvc`
退出码为 1，在 `aws-lc-sys` 构建阶段因缺少 ARM64 MSVC `cl.exe` 停止。本机 VS2019
Build Tools 仅有 x86/x64 工具，未安装 `Microsoft.VisualStudio.Component.VC.Tools.ARM64`，
因此该结果不构成 ARM64 编译证据，也未归因为 V1 Rust 源码。当前主机也不能运行 Apple
的 `codesign`/`spctl`。

最终仍需：真实 Windows x64、Windows ARM64、macOS Apple Silicon 以及中国大陆网络
人工矩阵；all-users 实验还需 reparse/中间目录替换、UNC/映射盘、UAC、ProgramData ACL 和
真实签名 MSIX 的 Stage/Provision HIL。任务保持 `in_progress`，不得以自动化结果宣称
V1 已完成平台验收。

## 2026-07-29 审查修正复验

最终只读审查发现当前固定 metadata endpoint 会以 HTTPS 302 跳转到 AgentsMirror R2。
因此 metadata transport 改用与下载相同的最多五跳 HTTPS/manual redirect policy；拒绝
HTTP、userinfo、缺失 Location 或第六跳时，source 保留不可重试的 `REDIRECT_REJECTED`
与 `OpenLogs`，不降级为 `SOURCE_UNAVAILABLE`。相应 runtime 测试使用 Tokio runtime，
不再依赖 `block_on` 下 cancellation branch 的调度时机。

复验结果：

- `cargo test --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --lib codex_desktop::`：0，148 passed；
- `pnpm exec vitest run tests/releaseWorkflow.test.ts --reporter=dot`：0，3 passed；
- 上表的完整前端和 Rust 命令均在该修正后重新执行并通过。
