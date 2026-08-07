# 本地跨平台构建移除设计

> **交付状态**：Proposed / 拟实施  
> **关联决策**：1–5、20、53、55  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 删除定义

删除的是开发者从 Linux/WSL 构建 Windows MSI 或 macOS DMG 的本地跨 OS 体系。保留：宿主平台原生开发、本机 bundle、Actions 五平台 Release、macOS runner 上的 Universal Binary。

## 2. 删除清单

| 路径/对象 | 动作 | 说明 |
|---|---|---|
| `scripts/macos-cross/**` | 删除 | 包含 osxcross、SDK、DMG、自动 trust 等。 |
| `scripts/windows-cross/**` | 删除 | 包含 cargo-xwin、Wine/WiX 和双架构 candidate。 |
| `mise.toml` 中 5 个 cross task | 删除 | 不保留兼容别名。 |
| `llvm-tools` | 从基础 Rust 组件删除 | 仅交叉构建需要。 |
| Apple/Windows Rust targets | 从本地基础环境删除 | Actions/原生 runner 按 job 添加实际目标。 |
| `tests/macosCrossWorkflow.test.ts` | 删除或重写为负向合同 | 不再保留旧文本包含测试。 |
| `.trellis/spec/backend/wsl-macos-cross-build.md` | 删除活动 spec | Git 历史保留。 |
| README/CONTRIBUTING/PR/spec 交叉构建说明 | 删除 | 替换为宿主原生 build 与 Actions Release 边界。 |
| `dist-bundle/windows`、`dist-bundle/macos` 文档入口 | 清理 | 实际目录若只是 Git-ignored 输出无需提交删除。 |

## 3. 自动 trust

两个 macOS 交叉脚本当前执行 `mise trust --yes`。它们随脚本删除。长期负向合同：任何项目 task、脚本或 hook 不得执行 `mise trust`、`mise untrust` 或修改 trust 状态；用户在审阅配置后显式运行 `mise trust`。

## 4. 保留的构建入口

```text
mise run dev             宿主平台 Tauri dev
mise run build:binary    宿主平台 release 编译，不 bundle
mise run build           宿主平台 release bundle
mise run build:debug     宿主平台 debug bundle
```

这些任务不得接受任意 `--target` 绕回非宿主目标。正式资产只来自 Actions。

## 5. Windows manifest 分层

目标合同：

```text
本地 dev/build       FYAGENT_WINDOWS_MANIFEST=dev
本地 check/test      FYAGENT_WINDOWS_MANIFEST=test
Actions 正式 Release FYAGENT_WINDOWS_MANIFEST=release
```

本地使用 Cargo release profile 不代表可嵌入正式管理员 manifest。

## 6. 文档与历史

活动规范和当前 README 必须移除旧方案；`docs/fyagent/dev/**`、`.trellis/tasks/archive/**` 中的历史事实保留，并加“非当前权威”入口声明。不能机械改写历史正文。

## 7. 验收

- 活动代码/配置/文档不再包含已删除 task 或脚本路径；
- 基础 `mise install` 不安装交叉 targets 或 `llvm-tools`；
- 宿主原生开发与 build 仍有标准 task；
- Release workflow 仍生成五平台/10 资产；
- 产品运行时的上游 mise CLI 兼容不因本项被删除。
