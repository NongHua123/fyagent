# 实际提交与剩余 PR 序列

> **交付状态**：Local implementation/docs/archive commits recorded; PR and remote closeout pending / 本地实施、文档与旧任务归档提交已记录，PR 与远程 closeout 待完成
> **关联决策**：43–52、84、97、105–115
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

| 顺序 | 提交/PR                                                            | 必须独立的原因                                 | 回退                                     |
| ---: | ------------------------------------------------------------------ | ---------------------------------------------- | ---------------------------------------- |
|    1 | `f4462765 merge(upstream): merge CC Switch v3.19.2`                | 两父 ancestry 与工程改造隔离。                 | `git revert -m 1`（先评估数据）。        |
|    2 | `e8954d97 build: remove local cross-platform builds`               | 删除责任与上游合并分离。                       | 单独 revert。                            |
|    3 | `3d534710 build: modernize mise and uv development environment`    | 版本、tasks、locks、hooks 环境成组。           | 成组 revert。                            |
|    4 | `038675b3 ci: restore automatic required checks and labeling`      | 合并门禁先于发布改造稳定。                     | revert CI。                              |
|    5 | `94ff9ee9 ci: build unsigned release transaction`                  | 权限、无签名与资产合同单独审查。               | 正式发布后只允许新补丁版本，不移动 tag。 |
|    6 | `4e407df4 test(node): remove cross-fetch and enforce native fetch` | 明确 FyAgent 上游差异。                        | 仅在重新接受风险时 revert。              |
|    7 | `eb748f9c` + `58101230` docs/spec/Trellis contract commits         | 最终命令稳定后更新活动文档与追踪。             | revert 对应 commits。                    |
|    8 | `1d3849e6 chore(trellis): archive superseded v1.0.2 tasks`         | 五个旧任务原状态与 superseded 语义一次性收口。 | 从 commit 恢复目录/元数据。              |
|    9 | `580c5efa test(trellis): cover active task validation`             | 证明旧树归档后全部活动任务可由统一入口验证。   | revert test。                            |
|   10 | implementation PR merge commit                                     | 合入 main 且保留内部两父 merge ancestry。      | 合入前停止；不 squash/rebase。           |
|   11 | closeout PR commits                                                | 真实远程证据 → 剩余 task archive → journal。   | 未满足证据不归档 parent。                |

实现 PR 和 closeout PR 应引用覆盖决策、Trellis child、风险项与已发生证据。不存在的 URL/digest 不得使用占位值。
