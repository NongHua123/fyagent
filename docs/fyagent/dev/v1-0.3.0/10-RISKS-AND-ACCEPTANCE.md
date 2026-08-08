# 风险登记与验收门槛

> **交付状态**：Active risk register / 活动风险登记；正式发布当前 NO-GO
> **关联决策**：19、24、31、39–49、81–87、97–115
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 风险评分

概率/影响使用 1–5；风险分数为乘积。`15–25` 默认 NO-GO，除非降低后重新评审；`8–14` 至少 GO WITH CONDITIONS；`1–7` 可 GO，但仍需证据。

| ID   | 风险                                                                    |   P |   I | 预防/检测                                                                   | 回退                                      | 所有者         | 门槛                                                 |
| ---- | ----------------------------------------------------------------------- | --: | --: | --------------------------------------------------------------------------- | ----------------------------------------- | -------------- | ---------------------------------------------------- |
| R-01 | 上游覆盖 FyAgent 品牌、数据目录或许可                                   |   3 |   5 | 分层冲突表、身份负向扫描、许可审查                                          | revert merge                              | Child 1        | NO-GO if unresolved                                  |
| R-02 | 上游安全/正确性修复在冲突中丢失                                         |   3 |   5 | commit/diff 台账、语义冲突记录、普通 CI                                     | 重做 merge                                | Child 1        | NO-GO                                                |
| R-03 | 删除交叉构建时误删正式发布能力                                          |   2 |   5 | 区分 scripts vs Actions、10 资产合同                                        | revert Child 2                            | Child 2        | NO-GO                                                |
| R-04 | task 名/API 漂移导致文档和代理失效                                      |   3 |   4 | task metadata docs、contracts scan                                          | 兼容 alias/回退                           | Child 3/6      | conditional                                          |
| R-05 | uv/Python 3.14.7 在 Windows ARM64 不可用                                |   3 |   4 | 原生资产、bootstrap/env check 平台验证                                      | 受控回退 Python 3.13（需新决策）          | Child 3        | NO-GO for claimed support                            |
| R-06 | mise `latest` 在 lock 刷新时引入破坏                                    |   3 |   3 | lock 精确、单独 bump PR、不在 bootstrap                                     | revert lock PR                            | Child 3        | conditional                                          |
| R-07 | CI 聚合 gate 将 skipped 误判成功                                        |   3 |   5 | `if: always()`、结果枚举测试                                                | revert workflow                           | Child 4        | NO-GO                                                |
| R-08 | Linux 容器挂载/工具导致包失败                                           |   3 |   4 | x64/ARM64 unsigned preflight、固定 digest                                   | 暂停发布，修容器                          | Child 4        | NO-GO for release                                    |
| R-09 | runner/action 固定信息过期                                              |   4 |   3 | Dependabot PR、月度评估、明确版本注释                                       | revert update                             | Child 4        | conditional                                          |
| R-10 | attestation 或 draft→stable 发布权限/身份错误                           |   3 |   5 | job 级最小权限、正式事件/仓库/workflow/CI 身份复核、13 附件回读             | 不 publish；保留未公开 draft 供人工审计   | Child 4        | NO-GO                                                |
| R-11 | Node 24 默认隐藏 DEP0040，误报已修复                                    |   4 |   4 | pending-deprecation 探针、依赖图                                            | 阻断合并                                  | Child 5        | NO-GO                                                |
| R-12 | Native Fetch 与 MSW/jsdom realm 不兼容                                  |   2 |   4 | 真实行为测试，不只检查全局存在                                              | 评估 MSW/jsdom 升级，不回退旧 polyfill    | Child 5        | conditional                                          |
| R-13 | 旧 Trellis 任务归档被误解为完成                                         |   4 |   3 | superseded 元数据、archive note、no-commit                                  | 恢复任务目录                              | Child 6        | conditional                                          |
| R-14 | overlay 覆盖上游合并后的新产品文档                                      |   3 |   4 | 三方比较、仅应用开发章节、人工审阅                                          | revert docs commit                        | Child 6        | conditional                                          |
| R-15 | 产品运行时 mise 被误删或变硬依赖                                        |   2 |   4 | upstream-first、无 mise 启动测试、可选路径测试                              | revert runtime change                     | Child 1/6      | NO-GO                                                |
| R-16 | `merge_group` 真实事件无法产生                                          |   5 |   4 | 明确个人账户与 Merge Queue 前提，不以静态合同冒充事件证据                   | 若要 live run，迁移组织仓库并允许保护规则 | Parent         | Accepted D114 verification exception; live run N/A   |
| R-17 | 无 ruleset/branch protection/tag protection，仅 workflow 验证来源       |   3 |   5 | exact repo/workflow/tag/main ancestry/CI SHA 检查、一次性发布、记录残余风险 | 停止 tag；后续独立治理任务                | Parent/Child 4 | Accepted residual risk, not administrator protection |
| R-18 | 原 pre-merge preflight 与 merge-commit/标准 attestation provenance 冲突 |   5 |   4 | 使用 merge→main CI→exact-main-SHA preflight→tag 顺序                        | 偏离已接受顺序则不发布                    | Parent/Child 4 | Accepted D113; remote execution pending              |

## 2. GO 条件

- 真实 tag 完整 SHA、merge base 和远程边界已验证；
- 所有 P0/NO-GO 风险有通过证据；
- Required gate 可证明失败关闭；
- 工具链在要求的平台实际解析到目标版本；
- Native Fetch/MSW 与 deprecation 探针通过；
- post-merge exact-main-SHA unsigned 全矩阵预演生成正确资产、metadata 与 attestation；
- 文档/任务合同扫描通过。

## 3. GO WITH CONDITIONS

只允许对不影响数据安全、身份、许可、正式发布来源和工具链可复现性的剩余风险使用。Artifact attestation、精确附件集合、同 SHA CI 和正式来源资格是 v0.3.0 强制门禁，不得降级为 GO WITH CONDITIONS。无管理员保护规则的风险已被明确接受，但不得宣称 main/tag 受保护。D114 只豁免当前治理下不可能产生的 live `merge_group` 证据；替代证据仍必须包含 YAML trigger、失败关闭合同/静态测试和真实 PR/main/manual 运行。

## 4. NO-GO

任一以下情况必须停止：

- 上游 tag/SHA/远程无法确认；
- 品牌、许可、数据目录或 FyAgent 独立版本被上游覆盖；
- CI 可以在依赖失败/取消/异常跳过时通过；
- 正式 Release 可以部分发布或本地产物可进入正式资产；
- Windows manifest 分层失效；
- Linux 包不能在计划支持的旧系统运行；
- `cross-fetch` 旧链路或 warning suppression 仍存在；
- Windows ARM64 被声称支持但 bootstrap/env check 未验证；
- 旧任务被记录为完成而非 superseded。
- D113 的已接受 post-merge 顺序未被遵守，或 D114 的替代证据（YAML/失败关闭静态合同及真实 PR/main/manual 运行）不完整；
- 任一 installer、manifest、build metadata 或 attestation bundle 缺失；
- 文档出现伪造 run/Release URL、digest、签名状态或 Released 状态。

## 5. 验收证据分层

| 层             | 证据                                                                                                |
| -------------- | --------------------------------------------------------------------------------------------------- |
| 静态           | 配置解析、文件扫描、lock 结构、workflow contract、task docs diff。                                  |
| 本地当前宿主   | `mise run check`、本机原生 build、hooks 模拟。                                                      |
| Actions 多平台 | Required CI matrix、实际版本和 runner metadata。                                                    |
| Release 预演   | post-merge exact-main-SHA 的 unsigned 10 安装器、2 个 JSON evidence 与 mandatory attestation。      |
| 正式 Release   | tag/main/CI/workflow 来源、精确 13 附件、摘要、attestation/manifest、Windows/macOS 无签名负向证据。 |

The project owner accepted D113/D114 on 2026-08-08：D113 确认 post-merge exact-main/workflow-SHA preflight 顺序；D114 确认 live `merge_group` 在当前个人仓库/无保护治理下为 N/A 的验证例外，且不是成功运行。两项决策门禁已解除，但本地实现与合同验证不产生远程 Actions 或正式 Release 证据；真实 PR/main/manual、preflight、tag、Release、资产、attestation 与 closeout 等 pending 项完成前，正式发布与 parent 归档仍保持 NO-GO。
