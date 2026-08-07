# 正式 Release 资产与发布事务合同

> **交付状态**：Proposed / 拟实施  
> **关联决策**：2–4、13–19、49、80  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。


## 1. 唯一来源

正式资产只来自受保护的 GitHub Actions Release workflow。任何本地 `mise run build*`、手工上传文件、未关联 source SHA 的产物或部分平台成功产物都不得成为正式 Release 资产。

## 2. 精确安装资产集合

| # | 平台 | 架构 | 格式 | 数量 | Runner / 用户空间 | 必要验证 |
|---:|---|---|---|---:|---|---|
| 1 | macOS | Universal | DMG | 1 | `macos-15` | Developer ID、notarization、staple、mount/app identity |
| 2 | macOS | Universal | ZIP | 1 | `macos-15` | 解包、签名、app identity、版本 |
| 3 | Windows | x64 | MSI | 1 | `windows-2022` | `release` manifest、架构、MSI metadata、签名/时间戳 |
| 4 | Windows | ARM64 | MSI | 1 | `windows-11-arm` | 同上，且原生 ARM64 |
| 5 | Linux | x64 | AppImage | 1 | `ubuntu-24.04` + 同架构 Ubuntu 22.04 digest-pinned container | ELF 架构、启动/结构、glibc 基线证据 |
| 6 | Linux | x64 | DEB | 1 | 同上 | package metadata、架构、安装文件清单 |
| 7 | Linux | x64 | RPM | 1 | 同上 | package metadata、架构、安装文件清单 |
| 8 | Linux | ARM64 | AppImage | 1 | 明确 ARM64 host + 同架构 Ubuntu 22.04 digest-pinned container | 原生 ARM64，无 QEMU |
| 9 | Linux | ARM64 | DEB | 1 | 同上 | package metadata、架构 |
| 10 | Linux | ARM64 | RPM | 1 | 同上 | package metadata、架构 |

总计必须是 **10 个安装资产**。签名 sidecar、摘要清单或 attestation 属于附属供应链证据，不计入这 10 个安装资产；verify job 应分别维护“安装资产 allowlist”和“允许的附属证据 allowlist”。最终精确文件名/正则必须从合并后 FyAgent 版本与现有资产命名合同推导，不能在设计包中臆造。

## 3. 发布模式

| 模式 | 来源 | 签名 secrets | 产出 | 是否创建 GitHub Release |
|---|---|---:|---|:---:|
| `unsigned` manual preflight | 任意明确 SHA（建议 PR/main） | 否 | 五 target groups 的 workflow artifacts + 验证报告 | 否 |
| `signed` manual preflight | 受保护 main 的明确 SHA | 受保护 environment + 审批 | 已签名/公证 workflow artifacts + 验证报告 | 否 |
| formal `vX.Y.Z` | 受保护 main、Required CI 成功、tag/version 匹配 | 是 | 10 安装资产 + manifest + attestation/能力说明 | 是，且一次性失败关闭 |

## 4. 来源资格

formal eligibility job 必须证明：

1. tag 语法和 `version:check --tag` 一致；
2. source SHA 是受保护 `main` 的祖先；
3. source SHA 对应稳定 `CI / Required` 成功；
4. 所有 platform jobs 使用相同 immutable source SHA；
5. `v*` tag ruleset 禁止覆盖/删除；
6. workflow/ref/repository 与预期 FyAgent 身份一致。

## 5. 机器可读 manifest

每个资产至少记录：

```text
filename
sha256
size_bytes
platform
architecture
format
product_version
tag
source_sha
workflow_run_id / attempt
node_version
pnpm_version
rust_version
runner_image_os / image_version
container_digest (Linux)
signing/notarization result where applicable
```

manifest 自身也进入 Release，并由 verify/publish job 在发布前重新校验。

## 6. 权限与 job 事务

```text
eligibility/build/test/verify  -> contents: read
signing jobs                   -> 仅必要 secrets；无 Release 写权限
attestation job                -> id-token: write + attestations: write（仅需要时）
publish job                    -> contents: write（且依赖全部成功）
```

任何 build/sign/verify/attest 失败、取消、异常跳过、资产缺失/重复/额外错误命名、source/version 不一致，都必须阻止 publish。不得让单个平台 job 直接先行创建或更新 Release。

## 7. Artifact attestations

GitHub 平台与套餐支持时，对最终资产摘要生成 attestation。若不可用：

- SHA-256 manifest 仍是强制合同；
- 在运行摘要和风险登记中明确能力阻塞；
- 不把“未生成 attestation”静默描述为成功生成 provenance。
