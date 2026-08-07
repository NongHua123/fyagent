# GitHub Actions CI 与正式 Release 设计

> **交付状态**：Proposed / 拟实施  
> **关联决策**：13–19、43、49、70、80  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 设计原则

- Actions 是合并与发布执行权威，但版本号来自标准文件；
- Required/Release runner 使用明确标签；镜像内部仍会更新，日志必须记录 image 版本；
- action 引用固定完整 SHA，行尾注释审阅时的版本标签；
- 默认权限只读，写权限按 job 最小授予；
- CI 具体 job 划分可在实施时按合并后仓库调整，但不得削弱下述合同。

## 2. 建议 CI 拓扑

```text
contracts (ubuntu-24.04) ───────────┐
frontend (ubuntu-24.04) ────────────┤
desktop-contract (ubuntu-24.04) ────┼─> CI / Required (ubuntu-24.04, if: always())
backend-linux (ubuntu-24.04) ────────┤
backend-windows (windows-2022) ──────┤
backend-macos (macos-15) ────────────┘
```

触发：

```yaml
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  merge_group:
    types: [checks_requested]
  workflow_dispatch:
```

`CI / Required` 必须读取 `needs` 结果并拒绝任何 `failure`、`cancelled` 或非合同允许的 `skipped`。Required workflow 不使用会让整个 workflow 不出现的 path filter；可在 job 内计算影响范围，但聚合 gate 必须始终结束。

## 3. 安装与版本

- `actions/setup-node` 使用 `.node-version`；
- pnpm setup 不写 `version`，读取 `packageManager`；
- Rust setup 读取 `rust-toolchain.toml`；
- 每个相关 job 运行版本合同，比较实际 `node/pnpm/rustc`；
- Actions 不安装 mise；
- 所有第三方 action 在实施 PR 中解析为完整 40 位 SHA，不能在设计文档虚构 SHA。

[Pending Verification / 待验证] 对选定 action 的“读取标准文件”行为和 ARM64 支持须在实施 PR 中验证；若 action 不满足合同，应替换而不是回退到重复硬编码。

## 4. 权限与供应链

```yaml
permissions:
  contents: read
```

- PR/CI 不读取发布 secrets；
- label workflow 仅 `pull-requests: write`；
- publish job 才 `contents: write`；
- provenance job 才 `id-token: write` 与 `attestations: write`；
- secrets 只在受保护 environment 的签名 job 中可用；
- Dependabot/同等工具提出 action SHA 更新 PR，不自动合并。

## 5. Release 架构

### 5.1 触发模式

- `push tags: v*`：正式发布；
- `workflow_dispatch`：`unsigned` 或 `signed` 预演，指定 source SHA；不创建 GitHub Release。

### 5.2 来源资格

version/eligibility job 验证：

1. `version:check`；
2. tag=`v${app_version}`；
3. source SHA 是 `origin/main` 祖先；
4. source SHA 对应 Required CI 成功；
5. 所有平台共享完全相同 SHA；
6. 正式 tag 受 ruleset 防删除/覆盖（仓库管理员配置）。

### 5.3 runner

| 目标 | 宿主 | 构建环境 |
|---|---|---|
| Windows x64 | `windows-2022` | 原生 x64；`FYAGENT_WINDOWS_MANIFEST=release` |
| Windows ARM64 | `windows-11-arm` | 原生 ARM64 target |
| macOS Universal | `macos-15` | 原生 ARM runner + x86_64/aarch64 targets，签名/公证 |
| Linux x64 | `ubuntu-24.04` | 同架构 `ubuntu:22.04@sha256:<reviewed>` 容器 |
| Linux ARM64 | `ubuntu-24.04-arm` | 同架构 `ubuntu:22.04@sha256:<reviewed>` 容器 |

容器 digest 通过受控 PR 更新。不得使用 QEMU 或把 x64/ARM64 相互交叉编译。Ubuntu 22.04 用户空间用于控制 glibc 基线。

## 6. 资产合同

正式发布精确包含：

```text
macOS:  DMG, ZIP
Windows: x64 MSI, ARM64 MSI
Linux x64: AppImage, DEB, RPM
Linux ARM64: AppImage, DEB, RPM
```

共 10 个安装资产。verify-assets job 下载所有 job artifact，拒绝缺失、重复、额外错误命名、版本/架构不一致；生成机器可读 manifest，记录 SHA-256、size、platform、arch、version、tag、source SHA、Node/pnpm/Rust、runner image。

## 7. 签名与 publish 事务

- Windows x64/ARM64 签名与 MSI 结构检查；
- macOS Developer ID 签名、公证、staple、DMG/ZIP 验证；
- Linux 包结构和架构验证；
- 所有 build/verify/attest 成功后，publish job 才获得写权限；
- 任一失败不得创建或更新 Release；
- publish job 应以一次性、幂等策略创建目标 Release，禁止部分平台先发布。

## 8. provenance

GitHub 平台/套餐支持时对最终资产生成 artifact attestations；不支持时明确记录能力阻塞，但 SHA-256 manifest 仍强制。Attestation 与 asset digest、repository、workflow、event、commit SHA 绑定。

## 9. 规则集（仓库外配置）

[Pending Verification / 待验证] ZIP 无法包含 GitHub ruleset，管理员需配置：

- main 要求 `CI / Required`；
- Merge Queue（若启用）使用同一 required context；
- 禁止未经审计的管理员绕过；
- `v*` tag 禁止覆盖/删除；
- release environment 对 signed 预演/正式发布要求审批。
