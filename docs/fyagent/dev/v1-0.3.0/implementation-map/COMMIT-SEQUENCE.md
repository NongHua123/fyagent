# 实际提交与剩余 PR 序列

> **交付状态**：Implementation and Release recorded; closeout PR pending / 实施与 Release 已记录，closeout PR 待完成
> **关联决策**：43–52、84、97、105–118
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

| 顺序 | 提交/PR                                                                                            | 必须独立的原因                                                                                          | 回退                                     |
| ---: | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
|    1 | `f4462765 merge(upstream): merge CC Switch v3.19.2`                                                | 两父 ancestry 与工程改造隔离。                                                                          | `git revert -m 1`（先评估数据）。        |
|    2 | `e8954d97 build: remove local cross-platform builds`                                               | 删除责任与上游合并分离。                                                                                | 单独 revert。                            |
|    3 | `3d534710 build: modernize mise and uv development environment`                                    | 版本、tasks、locks、hooks 环境成组。                                                                    | 成组 revert。                            |
|    4 | `038675b3 ci: restore automatic required checks and labeling`                                      | 合并门禁先于发布改造稳定。                                                                              | revert CI。                              |
|    5 | `94ff9ee9 ci: build unsigned release transaction`                                                  | 权限、无签名与资产合同单独审查。                                                                        | 正式发布后只允许新补丁版本，不移动 tag。 |
|    6 | `4e407df4 test(node): remove cross-fetch and enforce native fetch`                                 | 明确 FyAgent 上游差异。                                                                                 | 仅在重新接受风险时 revert。              |
|    7 | `eb748f9c` + `58101230` docs/spec/Trellis contract commits                                         | 最终命令稳定后更新活动文档与追踪。                                                                      | revert 对应 commits。                    |
|    8 | `1d3849e6 chore(trellis): archive superseded v1.0.2 tasks`                                         | 五个旧任务原状态与 superseded 语义一次性收口。                                                          | 从 commit 恢复目录/元数据。              |
|    9 | `580c5efa test(trellis): cover active task validation`                                             | 证明旧树归档后全部活动任务可由统一入口验证。                                                            | revert test。                            |
|   10 | `d0af898a` + `d8c26b70` D118 engineering fixes                                                     | 将 MSI/metadata 低层门禁前移，并修复 empty cleanup accumulator 根因。                                   | revert 要求重新打开发布风险审查。        |
|   11 | [PR #7](https://github.com/NongHua123/fyagent/pull/7) → `bde1370bbaffd345c3d9875708615eaf96140591` | 通过 PR/main Required 后以 merge commit 合入 main，保留内部 ancestry。                                  | 已发布 tag 不移动；后续只能新补丁版本。  |
|   12 | preflight `31259905022` + formal `31260931509`                                                     | exact-main preflight、annotated tag、stable/latest exact-13 Release 与独立复核。                        | 发布已完成；不移动/删除 tag。            |
|   13 | closeout PR commits                                                                                | 真实远程证据 → x64/ARM64 locked uv/Python/Trellis + MSI native gate → manifest → task archive/journal。 | 未满足 PR CI 证据不归档 parent。         |

实现 PR 与 Release 已有真实 URL/digest；closeout PR 应引用覆盖决策、Trellis child、风险项与已发生证据。未发生的 closeout CI/merge/archive/journal 不得使用占位值或预期成功表述。
