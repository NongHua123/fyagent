# 交付与修订元数据

> **状态**：Implementation audit in progress / 实施审计进行中
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

| 字段                | 值                                                                     |
| ------------------- | ---------------------------------------------------------------------- |
| 完整实施基线        | `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`                             |
| 集成分支            | `codex/fyagent-v0.3.0`                                                 |
| 恢复引用            | `refs/backup/fyagent-v0.3.0-baseline`                                  |
| origin              | `https://github.com/NongHua123/fyagent.git`（公开仓库）                |
| upstream            | fetch `https://github.com/farion1231/cc-switch.git`；push `DISABLED`   |
| 上游 tag object     | `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`                             |
| 上游 peeled commit  | `43eaf07355af145aebfee301801779e824d4c221`                             |
| 上游 merge commit   | `f4462765e9b3a2efd1deb13aabf3ce349166a058`                             |
| 产品版本 / tag 合同 | `0.3.0` / `v0.3.0`；tag 尚未创建                                       |
| Release 合同        | stable unsigned；10 installers / 13 attachments；mandatory attestation |
| 来源保护            | workflow-only；无 ruleset、branch protection 或 Release environment    |

## 修订审计

| 边界                      | 实现提交               | 证据/归档提交                         | 当前状态                                |
| ------------------------- | ---------------------- | ------------------------------------- | --------------------------------------- |
| 任务树与执行授权          | `194edb22`             | —                                     | active parent + six-child history       |
| CC Switch v3.19.2 merge   | `f4462765`             | `487995e0`, `20a4bc65`                | locally verified, archived              |
| 本地 cross-build 退役     | `e8954d97`             | `b8e50c4a`, `5e0dc678`                | locally verified, archived              |
| mise/uv/task/version      | `3d534710`             | `8bd54f6b`                            | local verified, remote pending          |
| CI/Labeler                | `038675b3`             | `2526588a`（与 Release 共用）         | local verified, remote pending          |
| unsigned Release workflow | `94ff9ee9`             | `2526588a`                            | local verified, remote pending          |
| DEP0040                   | `4e407df4`             | `e5c543f7`, `6be28965`                | locally verified, archived              |
| docs/Trellis/设计包       | `eb748f9c`, `58101230` | `1d3849e6`, `580c5efa`; closeout 后补 | local verified, remote closeout pending |

## 完整性模型

- 原始输入 ZIP 的 SHA-256 永久保留，不因真实实施而改写。
- `repository-overlay/` 共 111 个文件：108 个冻结 payload 加 3 个中央冻结声明；本轮 overlay diff 只更新三份声明，108 个 payload 保持原始字节。
- 当前 `MANIFEST.sha256` 仍对应原设计包快照，不能证明正在修改的目录。正式 Release、远程证据和 closeout 内容稳定后才重新生成，清单自身不参与哈希以避免自引用。
- Release run/URL、13 附件 digest 和 attestation 证据目前不存在；不得填入占位值。
