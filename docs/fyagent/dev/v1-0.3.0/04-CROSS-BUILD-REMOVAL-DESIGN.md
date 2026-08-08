# 本地跨平台构建移除设计

> **交付状态**：Implemented, locally verified, archived / 已实施、本地验证并归档
> **关联决策**：1–5、20、53、55、116
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 删除定义

原始删除范围是开发者从 Linux/WSL 构建 Windows MSI 或 macOS DMG 的本地跨 OS 体系。D116 将活动合同进一步收紧为：任何本地开发、构建、测试、打包和验证只能针对当前宿主 OS/架构；不得以 target 参数、子系统桥接、外来可执行文件、模拟器、复制工具链或本地暂存产物执行非宿主工作。保留的是宿主平台原生开发/本机 bundle，以及 Actions 五平台 Release 和 macOS runner 上的 Universal Binary。

## 2. 删除清单

commit `e8954d97faed1b833a6bce6fb9477b4fc4e2fd83` 已完成本节删除；表中“动作”是已执行结果。

| 路径/对象                                           | 动作                 | 说明                                           |
| --------------------------------------------------- | -------------------- | ---------------------------------------------- |
| `scripts/macos-cross/**`                            | 删除                 | 包含 osxcross、SDK、DMG、自动 trust 等。       |
| `scripts/windows-cross/**`                          | 删除                 | 包含 cargo-xwin、Wine/WiX 和双架构 candidate。 |
| `mise.toml` 中 5 个 cross task                      | 删除                 | 不保留兼容别名。                               |
| `llvm-tools`                                        | 从基础 Rust 组件删除 | 仅交叉构建需要。                               |
| Apple/Windows Rust targets                          | 从本地基础环境删除   | Actions/原生 runner 按 job 添加实际目标。      |
| `tests/macosCrossWorkflow.test.ts`                  | 删除                 | 由新的结构化 task/lock/负向合同覆盖。          |
| `.trellis/spec/backend/wsl-macos-cross-build.md`    | 删除活动 spec        | Git 历史保留。                                 |
| README/CONTRIBUTING/PR/spec 交叉构建说明            | 删除                 | 替换为宿主原生 build 与 Actions Release 边界。 |
| `dist-bundle/windows`、`dist-bundle/macos` 文档入口 | 清理                 | 实际目录若只是 Git-ignored 输出无需提交删除。  |

## 3. 自动 trust

原始输入中的两个 macOS 交叉脚本曾执行 `mise trust --yes`；它们已随脚本删除。长期负向合同：任何项目 task、脚本或 hook 不得执行 `mise trust`、`mise untrust` 或修改 trust 状态；用户在审阅配置后显式运行 `mise trust`。

## 4. 保留的构建入口

```text
mise run dev             宿主平台 Tauri dev
mise run build:binary    宿主平台 release 编译，不 bundle
mise run build           宿主平台 release bundle
mise run build:debug     宿主平台 debug bundle
```

这些任务及其底层 package/Cargo 包装不得接受任意 `--target` 或其他路由绕回非宿主目标。标准本地 test/package/verify 同样受此限制；可移植纯逻辑测试不构成另一平台的原生证据。正式资产和所有非宿主验证只来自匹配的 Actions native runner。

## 5. Windows manifest 分层

目标合同：

```text
Windows 原生宿主 dev/build       FYAGENT_WINDOWS_MANIFEST=dev
Windows 原生宿主 check/test      FYAGENT_WINDOWS_MANIFEST=test
Actions 正式 Release FYAGENT_WINDOWS_MANIFEST=release
```

只有 Windows 是实际宿主时，前两行才是本地入口；当前 Linux x64 环境不得桥接 Windows 进程执行它们。本地使用 Cargo release profile 不代表可嵌入正式管理员 manifest。

删除 Windows cross script 前，其 MSI product/protocol/payload/architecture 与 installer-actions Binary stream 断言已迁入 `scripts/release/verify-windows-msi.ps1`，并由原生 Windows Release matrix 调用。Linux 静态审查已完成，Windows Installer COM/CAB 的真实执行仍是远程 runner 门禁。

本轮曾为定位问题执行的本地 Windows cargo/rustc 与 Light/MSI 路径只产生诊断信息。相关进程已停止、显式诊断临时目录已删除，且这些结果不得写入 Windows x64/ARM64 验收证据。

## 6. 文档与历史

活动规范和当前 README 必须移除旧方案；`docs/fyagent/dev/**`、`.trellis/tasks/archive/**` 中的历史事实保留，并加“非当前权威”入口声明。不能机械改写历史正文。

## 7. 验收

- 活动代码/配置/文档不再包含已删除 task 或脚本路径；
- 基础 `mise install` 不安装交叉 targets 或 `llvm-tools`；
- 宿主原生开发与 build 仍有标准 task；
- 活动本地 entrypoint 对跨 OS/架构 target、bridge、foreign tool 和 emulator 做负向检查；
- Release workflow 静态合同仍生成五平台/10 安装器；真实全矩阵产物待远程 preflight；
- 产品运行时的上游 mise CLI 兼容不因本项被删除。

当前 Linux x64 清理复核：`rustup target list --installed` 仅返回
`x86_64-unknown-linux-gnu`；`src-tauri/target/app` 与
`target/installer-actions` 均已清除，清理前记录占用量分别为 4.1 GiB 与
57.4 MiB。该复核证明本地非宿主残留已清理，不证明 Windows Light、MSI
结构、Windows 生命周期、macOS bundle 或 ARM64 原生行为。上述门禁全部
保持 Pending，等待对应 Actions native runner。
