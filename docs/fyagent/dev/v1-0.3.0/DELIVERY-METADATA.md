# 交付与修订元数据

> **状态**：v0.3.0 released and verified; repository closeout in progress / v0.3.0 已发布验证，仓库收尾进行中
> **原始生成日期**：2026-08-07
> **真实实施授权日期**：2026-08-08

## 原始输入（不可改写的来源记录）

| 字段           | 值                                                                           |
| -------------- | ---------------------------------------------------------------------------- |
| 输入压缩包     | `fyagent-feature-fyagent-v1(1)(1).zip`                                       |
| 输入 SHA-256   | `4b5b19856bf927c47aeee521ba4ae20602276d66f0ba59ea7ed4c5aa2de3a473`           |
| 项目方提供基线 | `55173d2b` / `feature/fyagent-v1`；输入 ZIP 不含 `.git`                      |
| 原设计目录     | 134 个非 manifest 文件；135 个含 `MANIFEST.sha256` 文件                      |
| 原设计授权     | 仅文档/Trellis artifacts/documentation overlay；已被 2026-08-08 实施授权覆盖 |

## 当前真实 checkout

| 字段                | 值                                                                                                                    |
| ------------------- | --------------------------------------------------------------------------------------------------------------------- |
| 完整实施基线        | `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`                                                                            |
| Release source      | `bde1370bbaffd345c3d9875708615eaf96140591`                                                                            |
| 当前收尾分支        | `codex/fyagent-v0.3.0-closeout`                                                                                       |
| 恢复引用            | `refs/backup/fyagent-v0.3.0-baseline`                                                                                 |
| origin              | `https://github.com/NongHua123/fyagent.git`（公开仓库）                                                               |
| upstream            | fetch `https://github.com/farion1231/cc-switch.git`；push `DISABLED`                                                  |
| 上游 tag object     | `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`                                                                            |
| 上游 peeled commit  | `43eaf07355af145aebfee301801779e824d4c221`                                                                            |
| 上游 merge commit   | `f4462765e9b3a2efd1deb13aabf3ce349166a058`                                                                            |
| 产品版本 / tag 合同 | `0.3.0` / `v0.3.0`；tag object `e6706d4bdc33a184cf641204574df1fc2962ca4c`                                             |
| Release 合同        | stable unsigned；10 installers / 13 attachments；mandatory attestation                                                |
| 真实 Release        | [stable/latest v0.3.0](https://github.com/NongHua123/fyagent/releases/tag/v0.3.0)；ID `367220197`                     |
| closeout PR         | [PR #8](https://github.com/NongHua123/fyagent/pull/8)；native-success head `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` |
| closeout native CI  | run `31265504901`；x64 `93122857985`、ARM64 `93122858012`、Required `93123992476` 全成功                              |
| 来源保护            | workflow-only；无 ruleset、branch protection 或 Release environment                                                   |

## 修订审计

| 边界                      | 实现提交                           | 证据/归档提交                                  | 当前状态                                              |
| ------------------------- | ---------------------------------- | ---------------------------------------------- | ----------------------------------------------------- |
| 任务树与执行授权          | `194edb22`                         | —                                              | active parent + six-child history                     |
| CC Switch v3.19.2 merge   | `f4462765`                         | `487995e0`, `20a4bc65`                         | locally verified, archived                            |
| 本地 cross-build 退役     | `e8954d97`                         | `b8e50c4a`, `5e0dc678`                         | locally verified, archived                            |
| mise/uv/task/version      | `3d534710`                         | `8bd54f6b`, `4645668d`; run `31265504901`      | native remotely verified; task archive pending        |
| CI/Labeler                | `038675b3`                         | PR/main runs `31258884239` / `31259389682`     | remotely verified                                     |
| unsigned Release workflow | `94ff9ee9`, `d0af898a`, `d8c26b70` | preflight/formal `31259905022` / `31260931509` | released and independently verified                   |
| DEP0040                   | `4e407df4`                         | `e5c543f7`, `6be28965`                         | locally verified, archived                            |
| docs/Trellis/设计包       | `eb748f9c`, `58101230`             | `1d3849e6`, `580c5efa`; closeout 待补          | Release evidence recorded; PR/archive/journal pending |

## 完整性模型

- 原始输入 ZIP 的 SHA-256 永久保留，不因真实实施而改写。
- `repository-overlay/` 共 111 个文件：108 个冻结 payload 加 3 个中央冻结声明；本轮 overlay diff 只更新三份声明，108 个 payload 保持原始字节。
- `MANIFEST.sha256` 已在全部 closeout native 证据与设计包文档字节冻结后确定性重建并复验，覆盖包内除清单自身外的 134 个普通文件；路径按 `LC_ALL=C` 排序，包内无 symlink。任务归档目录不在该清单范围内，清单自身也不参与哈希以避免自引用。
- Release evidence SHA-256：`download-manifest.json`=`d1d81b973aea506d369e21b385ee60b993b88121b946cf68dc254e825b4abea1`；`build-metadata.json`=`7ae0631b77059d05a8866ec9602f8afc7f8493a092f6c32a3e4a161a3fc98079`；`artifact-attestation.sigstore.json`=`4802f1e9b5eca3eb0cc2a03530b86057d79e9d5828a97615d8ea5e5430ce0576`。
- 13 附件已独立重下载并核对 exact allowlist/manifest/metadata；本地 bundle 已用官方 checksum 核验的 GitHub CLI 2.97.0 验证 12/12 subjects。
