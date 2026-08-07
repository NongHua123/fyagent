# 建议提交与 PR 序列

> **交付状态**：Proposed / 拟实施  
> **关联决策**：43–51、52、84、97  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

| 顺序 | 提交/PR | 必须独立的原因 | 回退 |
|---:|---|---|---|
| 1 | `merge(upstream): merge cc-switch v3.19.2` | 保留 ancestry，区分上游来源。 | `git revert -m 1`（评估数据）。 |
| 2 | `chore(build): remove local cross-platform build toolchains` | 删除责任与上游合并分离。 | 单独 revert。 |
| 3 | `build(dev): establish mise task and uv python environment` | 版本、tasks、locks、hooks 环境成组。 | 成组 revert。 |
| 4 | `ci: modernize required checks` | 合并门禁先于发布改造稳定。 | revert CI。 |
| 5 | `ci(release): enforce actions-only release transaction` | 权限/签名/资产合同单独审查。 | 禁止 publish 后修复。 |
| 6 | `test(node): remove cross-fetch and enforce native fetch` | 明确 FyAgent 上游差异。 | 仅在重新接受风险时 revert。 |
| 7 | `docs(trellis): migrate development contracts` | 最终命令稳定后更新文档。 | revert docs。 |
| 8 | `chore(trellis): archive superseded tasks` | 防止归档自动提交混入其它改动。 | 恢复目录/元数据。 |

每个 PR 应引用 decision IDs、Trellis child task、风险项与验收证据。
