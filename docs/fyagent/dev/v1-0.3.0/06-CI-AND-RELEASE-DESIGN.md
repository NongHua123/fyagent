# GitHub Actions CI 与无签名正式 Release 设计

> **交付状态**：本地实现并通过静态/行为验证；远端 PR/main/preflight/tag/Release 证据待完成
>
> **目标版本**：FyAgent `0.3.0` / tag `v0.3.0`
>
> **关联决策**：13–19、43、49、70、80、105–117
> **证据等级**：本文区分 `[Implemented / 已实施]`、`[Verified Locally / 本地已验证]`、`[Pending Remote Verification / 远端待验证]`、`[Accepted Governance Exception / 已接受治理例外]`。

## 1. 已实施的边界

- Actions 恢复自动 PR/main CI 和安全自动 Labeler；
- CI Required 名称固定为 `CI / Required`，并以失败关闭方式聚合依赖结果；
- Release 仅支持“指定 immutable main SHA 的无签名全矩阵预演”和“精确 `v0.3.0` 正式发布”；
- Windows/macOS 签名、公证、staple、签名 secrets 和 Release environment 均不属于 v0.3.0；
- 不配置 branch/tag ruleset、branch protection 或 environment 审批；来源资格仅由 workflow 验证；
- 正式安装资产精确为 10 个；另有 2 个机器证据文件和 1 个 Sigstore attestation bundle；
- 所有直接第三方 Action 固定完整 40 位 commit SHA，runner 不使用 `*-latest`；
- Actions 不安装或执行 mise。
- 本地标准命令只允许当前宿主 OS/架构；所有非宿主原生验证只由匹配的 Actions runner 执行。
- 授权触发后的 run 由发起主流程同步等待整次完成，再一次读取结果；不使用后台/异步监控代理或重复状态轮询。

## 2. CI 拓扑

```text
Repository Contracts (ubuntu-24.04) ──────┐
Frontend Checks (ubuntu-24.04) ───────────┤
Desktop Acceptance Contract (ubuntu-24.04)├─> CI / Required
Backend Checks Linux (ubuntu-24.04) ───────┤    ubuntu-24.04, if: always()
Backend Checks Windows (windows-2022) ─────┤
Backend Checks macOS (macos-15) ───────────┘
```

触发合同：

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

`CI / Required` 读取显式 `needs` 结果，只接受完整且全为 `success` 的依赖集合；`failure`、`cancelled`、缺失依赖和未经合同允许的 `skipped` 均失败。workflow 不使用顶层 path filter 隐藏 Required context。

标准工具链事实为 Node 24.19.0、pnpm 10.12.3、Rust 1.97.1、Python 3.14.7 和 `mise.lock` 中锁定的 uv。CI 使用标准文件与锁文件解析这些值，在 job 内验证实际运行时版本，而不是把镜像预装版本当作证据。

## 3. Labeler 安全边界

- 自动路径使用 `pull_request_target`，只读取 base repository 的 label 配置；
- 不 checkout、不构建、不执行 PR head 代码；
- job 仅获得 `pull-requests: write`；
- `actions/labeler` 固定完整 SHA；
- `workflow_dispatch` 保留带明确 PR 编号的维护者补跑入口；
- 自动标签只引用仓库已存在 label，不能借 PR 输入创建任意标签。

远端自动 Labeler 只有在新 workflow 已进入默认分支后才能产生真实 `pull_request_target` 证据；feature branch 的静态测试不能替代该证据。

## 4. Release 触发与来源资格

```yaml
on:
  push:
    tags:
      - "v0.3.0"
  workflow_dispatch:
    inputs:
      source_sha:
        required: true
```

### 4.1 无签名预演

- 输入必须是小写 40 位 commit SHA，且与受信任 main workflow 的
  `GITHUB_SHA` / `GITHUB_WORKFLOW_SHA` 完全一致；
- 预演不伪造 Required CI 绑定，`build-metadata.json.requiredCi=null`；
- 执行 Windows x64/ARM64、Linux x64/ARM64、macOS Universal 五个 target group；
- 完成结构验证、exact-10、manifest、metadata 和 attestation；
- 只保留 workflow artifacts，publish job 必须跳过；
- 不存在 signed、mac-only、partial 或手动 tag 模式。

标准 `actions/attest` 的 provenance 绑定 workflow `GITHUB_SHA`。因此本轮
不能把未合并 candidate 的 bytes 冒充为 main provenance：首次安全预演在
实现 PR 合并后对精确 main SHA 执行。若未来要支持未合并 candidate，必须
新建 trusted reusable workflow 或 custom predicate 任务。

[Decision / 已决策] The project owner accepted D113/D114 on 2026-08-08。
D113 确认上述 post-merge exact-main/workflow-SHA preflight 顺序；该接受只
解除顺序决策门禁，不代表 preflight、tag 或 Release 已实际发生。

### 4.2 正式发布

正式来源资格逐项验证：

1. repository path=`NongHua123/fyagent` 且 repository ID=`1313497021`；
2. workflow path/name/ref 与 `.github/workflows/release.yml@refs/tags/v0.3.0` 一致；
3. event/ref/tag name 精确为 `push` / `refs/tags/v0.3.0` / `v0.3.0`；
4. event SHA、peeled tag commit、checkout HEAD 与冻结 source SHA 相同；
5. `version:check --tag v0.3.0` 与 Cargo 权威版本 `0.3.0` 一致；
6. source SHA 是最新获取的 `origin/main` 祖先；
7. `.github/workflows/ci.yml` 处于 active；
8. 该 SHA 最新 main-push run/attempt 已完成且成功；旧 success 不能掩盖更新的 failure/cancelled/in-progress；
9. 选定 attempt 恰有一个成功的 `CI / Required` job；
10. 选定 check suite 恰有一个来自 `github-actions` app、details URL 绑定同 run 的成功 `CI / Required` check-run。

## 5. Runner 与平台构建

| Target group    | Runner             | 用户空间                          | 原生/结构门禁                                                                 |
| --------------- | ------------------ | --------------------------------- | ----------------------------------------------------------------------------- |
| Windows x64     | `windows-2022`     | 原生 x64                          | release manifest、helper PE、MSI tables/payload/protocol、EXE/MSI `NotSigned` |
| Windows ARM64   | `windows-11-arm`   | 原生 ARM64                        | 同上；禁止用 x64 代构建                                                       |
| macOS Universal | `macos-15`         | macOS + 两个 Apple Rust targets   | Universal slices、Info.plist、ZIP/DMG 同源、无 Developer ID/Team/ticket       |
| Linux x64       | `ubuntu-24.04`     | `ubuntu:22.04` amd64 child digest | 原生 uname、AppImage/DEB/RPM 版本与架构                                       |
| Linux ARM64     | `ubuntu-24.04-arm` | `ubuntu:22.04` arm64 child digest | 原生 uname、无 QEMU、三格式完整                                               |

Linux 容器使用 child manifest digest，而不是只固定 multi-arch index：

```text
amd64 sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e
arm64 sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149
```

job 开始即验证 Ubuntu 22.04 和 `uname -m`。ARM runner 暂时不可用时允许对同一 SHA 重跑，但不得切回 QEMU、本地 cross-build 或少资产发布。

本地不得提前执行或复刻表中任一非宿主 package/verify gate。当前 Linux x64 环境中的 Windows、macOS 与 ARM64 都必须保持远程；子系统桥接、foreign executable、emulator、copied toolchain 或 staged artifact 均不能改变这一证据归属。

## 6. Windows 无签名安全门禁

两个 Windows job 都在真实 app build 和 MSI bundle 命令上设置 `FYAGENT_WINDOWS_MANIFEST=release`。验证分层如下：

1. `verify-windows-release-manifest.ps1` 在 bundle 前后读取目标 EXE 的 PE/RT_MANIFEST，要求精确 x64/ARM64 Machine、唯一 `requireAdministrator` / `uiAccess=false`，并验证产品版本；
2. workflow 独立构建 architecture-matched installer-actions DLL，并校验 PE Machine；
3. `verify-windows-msi-structure.ps1` 承接旧 workflow 内联结构门禁，并通过只读 `_Streams` 读取 embedded cabinet，只让系统 `expand.exe` 提取固定 File key `Path` 到全新目录，拒绝额外/reparse/逃逸输出，再按 size/SHA-256/PE Machine/`NotSigned` 将 MSI 主 EXE 绑定到已验证 built EXE；不运行 `msiexec` 或 MSI action；
4. 既有 `verify-windows-msi.ps1` 验证 Binary stream 与 helper SHA/PE、ProductName/Version、ARPNOREPAIR、fyagent URL protocol、唯一 fyagent.exe payload、架构和 host residue；
5. `verify-windows-unsigned.ps1` 要求 EXE 与 MSI 都是 `NotSigned`，且 signer/timestamper certificate 均为空。

上述“未签名”是发布事实和风险，不是临时绕过。未来恢复 Authenticode 必须新建任务，不能把证书逻辑重新混入 v0.3.0。

## 7. macOS 无签名边界

- 只接受一个 `FyAgent.app`；二进制同时包含 `arm64` 和 `x86_64`；
- `CFBundleShortVersionString=0.3.0`，`CFBundleIdentifier=com.fyagent.desktop`；
- 真正 unsigned 或 ad-hoc signature 均允许，但任何 `Authority=...`、Developer ID 或真实 TeamIdentifier 失败；
- workflow 使用负向 `stapler validate` 证明 app/DMG 无 ticket，但不执行 `stapler staple` 或 `notarytool`；
- ZIP 与 DMG 都从同一 app 生成；ZIP 展开、DMG verify + read-only mount 后，Info.plist 版本和主可执行文件 SHA-256 都必须与源 app 一致；
- workflow 不引用 Apple secret，不宣称 Gatekeeper 信任。

## 8. 精确资产、manifest、metadata 与 attestation

安装资产 exact allowlist：

```text
FyAgent-0.3.0-macOS.dmg
FyAgent-0.3.0-macOS.zip
FyAgent-0.3.0-Windows.msi
FyAgent-0.3.0-Windows-arm64.msi
FyAgent-0.3.0-Linux-x86_64.AppImage
FyAgent-0.3.0-Linux-x86_64.deb
FyAgent-0.3.0-Linux-x86_64.rpm
FyAgent-0.3.0-Linux-arm64.AppImage
FyAgent-0.3.0-Linux-arm64.deb
FyAgent-0.3.0-Linux-arm64.rpm
```

平台 artifact 下载时保持五个命名目录，不使用 `merge-multiple` 直接覆盖。collector 检查目录与文件归属，拒绝缺失、额外、错组、nested、symlink 和 duplicate，再用 no-overwrite 复制成 exact-10 平面集合。

附属证据：

```text
download-manifest.json
build-metadata.json
artifact-attestation.sigstore.json
```

- `download-manifest.json` 是 SHA-256 manifest，schema 为 `fyagent-download-manifest/v2`；覆盖 exact-10 的 filename/platform/architecture/format/sizeBytes/sha256/final URL；
- `build-metadata.json` 覆盖五 target group、runner image/toolchain、Linux host `RUNNER_ARCH`/child digest、repository/workflow ref+SHA/run/source；formal 记录 Required CI run/attempt，preflight 明确为 `null`；
- attestation subjects 精确为 10 installer + 2 JSON = 12；
- `actions/attest` v4.2.2 输出独立 Sigstore bundle，形成第 13 个 Release attachment；
- attestation bundle 不计入 10 个安装资产。

## 9. 权限与一次发布

默认权限：

```yaml
permissions:
  contents: read
```

- eligibility 才增加 `actions: read` / `checks: read`；
- attest 才增加 `id-token: write` / `attestations: write` / `artifact-metadata: write`；
- publish 是唯一 `contents: write` job，并依赖 eligibility、全部 native builds、exact asset/evidence、attestation 全成功；
- publish 先重验 formal event/tag/source、13 文件和英文 Release Notes，再通过认证后的全分页 Release 列表确认既无同 tag draft 也无 published Release；
- 创建一个带 run/source marker 的私有 draft，上传 13 文件后从 API 全量列举并逐个重新下载，验证 exact-13、非空与 SHA-256 相同；最终 PATCH 前再次读取并比对 draft ID/tag/marker/state 与 exact asset IDs，最后只执行一次 PATCH 公开为 stable/non-prerelease/latest；PATCH 响应成功后仍须按 ID 重读 published identity/exact assets 并确认 latest，才可判定事务完成；
- 失败不自动删除 draft：Release DELETE 没有可防 TOCTOU 的条件删除，自动清理可能误删并发发布结果。失败会报告 Release ID/URL 并要求独立人工决策；一旦尝试 PATCH，退出处理只读查询并报告 `draft`、`published` 或 `unknown`，不得重试 PATCH，也不得在结果不确定时声称仍为私有 draft。已有 draft/published Release 时重跑失败关闭；任何路径都不移动或删除 tag。

GitHub 对该 unsafe PATCH 不提供通用条件请求，管理员仍可能在最后一次
读取与 PATCH 之间竞态修改状态。这是 workflow-only、无 ruleset 决策下
保留的窄残余风险，不能描述为原子或管理员保护。

## 10. workflow-only 残余风险

[Decision / 已决策] 本版本不配置 branch/tag ruleset、branch protection 或 Release environment。workflow 内的 repo ID、workflow ref、main ancestry、same-SHA CI/check-run 和 exact asset/attestation 验证显著降低误发布风险，但管理员仍可能改写未受保护的 main/tag/workflow。这一残余风险必须出现在验证报告和 Release 决策记录中，不能写成“main/tag 已受保护”。

## 11. 本地边界与远程运行观察

[Decision / 已决策] D116 要求所有本地开发、构建、测试、打包和验证仅针对当前宿主 OS/架构；Windows、macOS、ARM64 及任何非宿主验收必须由匹配的 GitHub Actions native runner 产生。此前本地 Windows cargo/rustc、Light/MSI 路径只用于诊断，相关进程、显式临时目录与 `src-tauri/target/app`、`target/installer-actions` 输出已清理，不能引用为 acceptance。

[Decision / 已决策] D117 要求授权触发后由发起主流程执行一次同步 whole-run wait，直到 run 为 `completed`；随后只读取一次最终 run/job 结果。不得派后台/异步 monitor，也不得反复轮询 run/job/check 状态；仅最终失败才获取失败 job 日志。观察授权不等于 rerun、cancel、tag 或 publish 授权。

## 12. 当前验证状态与治理例外

- `[Verified Locally]` Release 定向 Vitest、Prettier、version contract 与 actionlint 已通过；
- `[Pending Remote Verification]` Windows ARM64、Linux ARM64、macOS Universal、full-matrix preflight、正式 tag 和公开资产尚须真实 Actions 证据；
- `[Pending Remote Verification]` 自动 Labeler 需 workflow 进入 default branch 后通过真实 PR 验证；
- `[Accepted Governance Exception]` D114 确认：当前仓库属于个人账户且明确不启用 ruleset/branch protection，GitHub Merge Queue 无法启用，因而真实 `merge_group` 运行在当前治理下为 N/A，而不是成功。接受的替代证据为 YAML trigger、失败关闭合同/静态测试和真实 PR/main/manual 运行；这些远程运行仍待验证。
- `[Verified Local Cleanup]` 当前只安装 `x86_64-unknown-linux-gnu` Rust target，两个诊断 build 目录已不存在；该状态不证明 Windows/macOS/ARM64 原生门禁。

本地通过与 D116/D117 文档落地不等于发布完成。Child4 在 main CI、unsigned preflight、tag-triggered workflow、公开 Release、独立下载复核和 attestation URL 全部取得真实证据前保持 `in_progress`；本轮未触发、监控或轮询 Actions。
