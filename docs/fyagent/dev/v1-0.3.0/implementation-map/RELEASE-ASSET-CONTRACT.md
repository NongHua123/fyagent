# FyAgent v0.3.0 正式 Release 资产与事务合同

> **交付状态**：已实施并由原生预演、正式发布及独立重下载/attestation 复核验证；Trellis closeout 仍在进行
>
> **产品/tag**：`0.3.0` / `v0.3.0`
> **发布性质**：公开、稳定、无签名、手动下载安装

实现证据冻结在 exact main SHA
`bde1370bbaffd345c3d9875708615eaf96140591`：full-matrix preflight
`31259905022`、formal Release run `31260931509` 和 stable Release
[`v0.3.0`](https://github.com/NongHua123/fyagent/releases/tag/v0.3.0)
均成功。Release ID `367220197` 恰好包含本合同的 10 个安装资产与 3
个证据附件；独立重下载的 allowlist、size、SHA-256、URL 以及全部 12
个 attestation subjects 已复核。完整台账见
[`DELIVERY-METADATA.md`](../DELIVERY-METADATA.md) 与
[`VALIDATION-REPORT.md`](../VALIDATION-REPORT.md)。

## 1. 唯一来源

正式资产只来自 `.github/workflows/release.yml` 对 exact main SHA 的原生平台构建。任何本地 `mise run build*`、WSL/cross-build、手工重命名后上传、未绑定 source SHA 的文件或部分平台成功结果都不得成为正式 `v0.3.0` 资产。

workflow 支持：

| 模式        | 来源资格                                                                        | 平台             | 产出                                                          | GitHub Release         |
| ----------- | ------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------- | ---------------------- |
| `preflight` | dispatch 指定小写 40 位 SHA，且等于 main workflow/event SHA；不声明 Required CI | 五 target groups | 10 installers + 2 JSON + attestation bundle workflow artifact | 禁止创建/更新          |
| `formal`    | exact `v0.3.0` tag push；tag/version/source/main/CI/workflow identity 全一致    | 五 target groups | 同一 13 文件事务                                              | 一次创建 stable/latest |

不存在 signed、partial、mac-only、manual-tag-dispatch 或本地 publish 模式。

## 2. 精确十安装资产

|   # | Target group    | 文件名                                | Runner / 用户空间                                    | 核心验证                                                                            |
| --: | --------------- | ------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------- |
|   1 | macOS Universal | `FyAgent-0.3.0-macOS.dmg`             | `macos-15`                                           | universal app；DMG verify/read-only mount；同源 digest；无 Developer ID/Team/ticket |
|   2 | macOS Universal | `FyAgent-0.3.0-macOS.zip`             | `macos-15`                                           | 展开 app；Info.plist；同源 executable digest                                        |
|   3 | Windows x64     | `FyAgent-0.3.0-Windows.msi`           | `windows-2022`                                       | release manifest；helper/MSI tables/payload/protocol；`NotSigned`                   |
|   4 | Windows ARM64   | `FyAgent-0.3.0-Windows-arm64.msi`     | `windows-11-arm`                                     | 原生 ARM64；同上；`NotSigned`                                                       |
|   5 | Linux x64       | `FyAgent-0.3.0-Linux-x86_64.AppImage` | `ubuntu-24.04` + Ubuntu 22.04 amd64 child digest     | exact raw count；ELF x86-64                                                         |
|   6 | Linux x64       | `FyAgent-0.3.0-Linux-x86_64.deb`      | 同上                                                 | DEB version/amd64                                                                   |
|   7 | Linux x64       | `FyAgent-0.3.0-Linux-x86_64.rpm`      | 同上                                                 | RPM version/x86_64                                                                  |
|   8 | Linux ARM64     | `FyAgent-0.3.0-Linux-arm64.AppImage`  | `ubuntu-24.04-arm` + Ubuntu 22.04 arm64 child digest | native aarch64；无 QEMU                                                             |
|   9 | Linux ARM64     | `FyAgent-0.3.0-Linux-arm64.deb`       | 同上                                                 | DEB version/arm64                                                                   |
|  10 | Linux ARM64     | `FyAgent-0.3.0-Linux-arm64.rpm`       | 同上                                                 | RPM version/aarch64                                                                 |

安装资产数量必须恰好为 10。任一缺失、额外、空文件、错版本、错架构、错 target group、nested/symlink 或重复均失败。

## 3. 五个 artifact 目录到 exact-10

构建 job 先冻结每类 raw output 数量：

- Windows 每架构恰好一个 raw MSI；
- Linux 每架构恰好一个 AppImage、一个 DEB、一个 RPM；
- macOS 恰好一个 raw `FyAgent.app`，再从同一 app 生成一个 ZIP 和一个 DMG。

normalized outputs 分别上传到：

```text
installers-macos-universal/
installers-windows-x64/
installers-windows-arm64/
installers-linux-x64/
installers-linux-arm64/
```

verify job 下载时保留这五个目录，不使用可能覆盖同名文件的 `merge-multiple`。`collect-workflow-artifacts.mjs` 先核对目录与每组文件归属，再以 `COPYFILE_EXCL`/no-overwrite 复制为平面 exact-10。重复文件不能被静默覆盖成“看起来数量正确”。

## 4. 机器可读 SHA-256 manifest

`download-manifest.json` 是附属供应链证据，不计入十安装资产。schema：

```text
fyagent-download-manifest/v2
```

顶层字段：

```text
product = FyAgent
version = 0.3.0
tag = v0.3.0
sourceSha = lowercase 40-char commit
publishedAt = ISO-8601 instant
assets = exact 10 records
```

每条 asset 记录：

```text
name
platform
architecture
format
sizeBytes
sha256
url
```

generator 在读取摘要前再次验证 exact-10 且每个文件非空，以流式 SHA-256 计算真实构建后 bytes。它不从 tag 截取版本，也不忽略未知文件。

## 5. 构建元数据

`build-metadata.json` 要求恰好五个 `fyagent-platform-build/v1` 输入，并输出 `fyagent-build-metadata/v1`。至少绑定：

- product version、tag、source SHA；
- repository path 与不可变 repository ID；
- Release workflow path/ref/run ID/run attempt/event/mode；
- trusted workflow ref/SHA；formal 的 `.github/workflows/ci.yml` 与选定 Required CI run ID/attempt；preflight 的 `requiredCi=null`；
- 五 target group 的 platform/architecture；
- `runner.requestedLabel`：matrix 请求的 runner 路由 label，不表述为运行时发现的 host label 或固定 hosted image；
- `runner.context.os/arch`：只来自 workflow 显式映射的 `${{ runner.os }}` / `${{ runner.arch }}`；
- 实际 Node/pnpm/rustc；
- Windows/macOS 的 `container` 精确为 `null`；
- Linux 的 `container.configuredImage.reference/manifestDigest` 绑定 workflow 配置的 fully-qualified Ubuntu 22.04 child，`container.observed.osRelease.id/versionId` 与 `.unameMachine` 绑定 metadata emission 前实际观察。

native target 的 runner context 映射为：`windows-x64 = windows-2022 / Windows / X64`，`windows-arm64 = windows-11-arm / Windows / ARM64`；`macos-universal = macos-15 / macOS / ARM64`。macOS 的 output architecture 仍是 `universal`，该产物事实不能覆盖或放宽当前 hosted-runner label 的 ARM64 来源合同。Linux 精确映射为：

```text
linux-x64:
  runner = ubuntu-24.04 / Linux / X64
  configured image = docker.io/library/ubuntu:22.04@sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e
  observed = ubuntu / 22.04 / x86_64

linux-arm64:
  runner = ubuntu-24.04-arm / Linux / ARM64
  configured image = docker.io/library/ubuntu:22.04@sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149
  observed = ubuntu / 22.04 / aarch64
```

每个 input record 的唯一形状为：

```text
schema, targetGroup, platform, architecture,
runner {
  requestedLabel,
  context { os, arch }
},
container = null | {
  configuredImage { reference, manifestDigest },
  observed {
    osRelease { id, versionId },
    unameMachine
  }
},
toolchain { node, pnpm, rustc },
identity { exact release/workflow/CI fields }
```

root 与每个 nested object 都拒绝 missing/extra/retired keys；aggregate 验证后从 allowlist 重建 target，不把 input `identity` 或任意原始字段 spread 到受 attestation 的 `targets`。`ImageOS` / `ImageVersion` 不属于 schema，也不以 `null` 保留；不写 `verified`、伪造 actual-image digest 或猜测 hosted-image version。配置 reference 是 workflow configuration evidence；`/etc/os-release`、`uname -m` 与 attestation 都不能独立证明 OCI digest 或自定义字段语义。

2026-08-08 本地只读证据确认此前失败的 preflight 未进入 aggregation/attestation/publication，且没有已发布或已消费的 v1 record，因此这两个 v1 schema 在首次发布前原位定稿。若首次发布前发现任何 public v1 consumer，`fyagent-platform-build/v1` 与 `fyagent-build-metadata/v1` 必须连同所有 writer、validator、type、test 和文档原子升级为 v2，formal 只接受 v2；禁止兼容 reader、默认值或将缺失事实合成为等价 v2。

任何 target 缺失、附加 JSON、runner/digest/source/workflow/CI identity 不一致都阻止 attestation 和 publish。

## 6. Attestation subject 与最终附件 allowlist

```text
10 installers
+ download-manifest.json
+ build-metadata.json
= 12 actions/attest subjects
```

`actions/attest` v4.2.2 必须成功，为这 12 个 subject 生成一个 Sigstore bundle。bundle 固定命名：

```text
artifact-attestation.sigstore.json
```

最终 GitHub Release attachment allowlist 恰好 13 个：10 installer + 2 JSON + 1 bundle。attestation bundle 和两个 JSON 都不是安装资产；README/Release Notes 不得把“13 个附件”写成“13 个安装包”。

## 7. 无签名事实与风险

### Windows

EXE 和 MSI 都必须通过 `Get-AuthenticodeSignature` 证明 `Status=NotSigned`，且 signer/timestamper certificate 为空。SmartScreen/未知发布者警告是预期用户体验，不能用脚本关闭或绕过。

### macOS

允许 truly unsigned 或 ad-hoc app，但禁止 Developer ID Authority、真实 TeamIdentifier、notarization ticket 和 staple。用户文档只能引导 Apple 支持的“Privacy & Security → Open Anyway”流程，不提供关闭 Gatekeeper 或移除 quarantine 的自动脚本。

这些是 v0.3.0 明确接受的分发风险。未来签名、公证或证书 secret 恢复必须进入新任务、新版本和独立决策。

## 8. 权限与 publish 事务

```text
eligibility/build/verify -> contents: read
attest                  -> contents: read + id-token/attestations/artifact-metadata write
publish                 -> actions: read + contents: write
```

publish 只在 formal 模式且前置全部成功时运行。发布前再次验证：

1. exact push/ref/tag/version/source；
2. peeled `v0.3.0` tag commit 等于 frozen source；
3. 13 附件 exact allowlist；
4. 英文 `docs/release-notes/v0.3.0-en.md` 已存在；
5. GitHub 上尚不存在 `v0.3.0` Release。

publish 先用认证的全分页 Release 列表排除同 tag draft/published Release，再创建仅本 run 标记的私有 draft。13 个文件全部上传后，从 Release API 列举并逐个重新下载，复核 exact allowlist、非空和 SHA-256；PATCH 前再次读取并比对 draft ID/tag/marker/state 与 exact asset IDs，最后只执行一次 PATCH 公开为 stable/non-prerelease/latest。PATCH 响应成功后还要按 ID 重读 published identity/exact assets 并确认 latest，才算完成。失败不自动 DELETE（避免无条件 DELETE 的 TOCTOU）；报告 Release ID/URL 并要求独立人工决策。若已尝试 PATCH，退出处理只读查询并报告 `draft`、`published` 或 `unknown`，不得自动重试 PATCH，也不得在结果不确定时声称仍为私有 draft。GitHub 不支持对该 unsafe PATCH 使用通用条件请求，最后一次读取/PATCH 仍有管理员竞态残余风险；不得把它写成原子保护，也不得更新既有 Release 或移动/删除 tag。

## 9. 本地验证与远端完成定义

本地合同检查包括：

```bash
pnpm exec prettier --check .github/workflows/release.yml scripts/release tests/writePlatformMetadata.test.ts tests/releaseWorkflow.test.ts tests/downloadManifest.test.ts tests/releaseAssets.test.ts
actionlint .github/workflows/release.yml
pnpm exec vitest run tests/writePlatformMetadata.test.ts tests/releaseWorkflow.test.ts tests/downloadManifest.test.ts tests/releaseAssets.test.ts
pnpm run version:check -- --tag v0.3.0
```

本地成功只证明合同形状和纯函数行为。完成仍要求：

- exact main SHA 的成功 `CI / Required`（formal 来源门禁）；
- 合并后同一 exact main/workflow SHA 的 full-matrix unsigned preflight；
- `v0.3.0` tag-triggered formal run；
- public stable Release 上 exact 13 attachments；
- 独立重新下载后的数量/摘要/版本/架构/Windows `NotSigned`/macOS unsigned 复核；
- attestation URL/bundle 验证；
- 将真实 run/release/digest 证据写回 validation report/traceability 后归档 Trellis 任务。
