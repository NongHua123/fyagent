# 交付元数据

> **交付状态**：Observed / 已核实（针对文档生成结果）  
> **关联决策**：98–104  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。


| 字段 | 值 |
|---|---|
| 生成日期 | `2026-08-07` |
| 输入压缩包 | `fyagent-feature-fyagent-v1(1)(1).zip` |
| 输入 SHA-256 | `4b5b19856bf927c47aeee521ba4ae20602276d66f0ba59ea7ed4c5aa2de3a473` |
| 项目方提供基线 | `55173d2b` / `feature/fyagent-v1`（上传包无 `.git`，未独立验证 ancestry） |
| 上游目标 | `farion1231/cc-switch` tag `v3.19.2` |
| 公开上游 commit | `43eaf07355af145aebfee301801779e824d4c221`（真实 checkout 仍须本地 Git 验证） |
| 交付目录预计文件数 | `135`（含本文件、验证报告和 manifest） |
| 实施状态 | 仅文档/Trellis artifacts/documentation overlay；未实施源码、配置、Git、CI、Release |

## 完整性模型

- `MANIFEST.sha256` 覆盖交付目录内除清单自身之外的全部文件，避免自引用；
- 外部 `fyagent-modernization-design.zip.sha256` 覆盖最终 ZIP；
- 原始上传 ZIP 在生成前后均校验为 `4b5b19856bf927c47aeee521ba4ae20602276d66f0ba59ea7ed4c5aa2de3a473`；
- ZIP 中不包含源码工作副本、`.git`、secrets、签名证书或构建产物。
