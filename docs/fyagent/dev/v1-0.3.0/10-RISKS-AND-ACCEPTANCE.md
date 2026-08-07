# 风险登记与验收门槛

> **交付状态**：Proposed / 拟实施  
> **关联决策**：19、24、31、39–49、81–87、97–104  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 风险评分

概率/影响使用 1–5；风险分数为乘积。`15–25` 默认 NO-GO，除非降低后重新评审；`8–14` 至少 GO WITH CONDITIONS；`1–7` 可 GO，但仍需证据。

| ID | 风险 | P | I | 预防/检测 | 回退 | 所有者 | 门槛 |
|---|---|---:|---:|---|---|---|---|
| R-01 | 上游覆盖 FyAgent 品牌、数据目录或许可 | 3 | 5 | 分层冲突表、身份负向扫描、许可审查 | revert merge | Child 1 | NO-GO if unresolved |
| R-02 | 上游安全/正确性修复在冲突中丢失 | 3 | 5 | commit/diff 台账、语义冲突记录、普通 CI | 重做 merge | Child 1 | NO-GO |
| R-03 | 删除交叉构建时误删正式发布能力 | 2 | 5 | 区分 scripts vs Actions、10 资产合同 | revert Child 2 | Child 2 | NO-GO |
| R-04 | task 名/API 漂移导致文档和代理失效 | 3 | 4 | task metadata docs、contracts scan | 兼容 alias/回退 | Child 3/6 | conditional |
| R-05 | uv/Python 3.14.7 在 Windows ARM64 不可用 | 3 | 4 | 原生资产、bootstrap/env check 平台验证 | 受控回退 Python 3.13（需新决策） | Child 3 | NO-GO for claimed support |
| R-06 | mise `latest` 在 lock 刷新时引入破坏 | 3 | 3 | lock 精确、单独 bump PR、不在 bootstrap | revert lock PR | Child 3 | conditional |
| R-07 | CI 聚合 gate 将 skipped 误判成功 | 3 | 5 | `if: always()`、结果枚举测试 | revert workflow | Child 4 | NO-GO |
| R-08 | Linux 容器挂载/工具导致包失败 | 3 | 4 | x64/ARM64 unsigned preflight、固定 digest | 暂停发布，修容器 | Child 4 | NO-GO for release |
| R-09 | runner/action 固定信息过期 | 4 | 3 | Dependabot PR、月度评估、明确版本注释 | revert update | Child 4 | conditional |
| R-10 | 签名/公证/attestation 权限错误 | 3 | 5 | environment 审批、最小权限、signed dry-run | 不 publish | Child 4 | NO-GO |
| R-11 | Node 24 默认隐藏 DEP0040，误报已修复 | 4 | 4 | pending-deprecation 探针、依赖图 | 阻断合并 | Child 5 | NO-GO |
| R-12 | Native Fetch 与 MSW/jsdom realm 不兼容 | 2 | 4 | 真实行为测试，不只检查全局存在 | 评估 MSW/jsdom 升级，不回退旧 polyfill | Child 5 | conditional |
| R-13 | 旧 Trellis 任务归档被误解为完成 | 4 | 3 | superseded 元数据、archive note、no-commit | 恢复任务目录 | Child 6 | conditional |
| R-14 | overlay 覆盖上游合并后的新产品文档 | 3 | 4 | 三方比较、仅应用开发章节、人工审阅 | revert docs commit | Child 6 | conditional |
| R-15 | 产品运行时 mise 被误删或变硬依赖 | 2 | 4 | upstream-first、无 mise 启动测试、可选路径测试 | revert runtime change | Child 1/6 | NO-GO |

## 2. GO 条件

- 真实 tag 完整 SHA、merge base 和远程边界已验证；
- 所有 P0/NO-GO 风险有通过证据；
- Required gate 可证明失败关闭；
- 工具链在要求的平台实际解析到目标版本；
- Native Fetch/MSW 与 deprecation 探针通过；
- unsigned 全矩阵预演生成正确资产；
- 文档/任务合同扫描通过。

## 3. GO WITH CONDITIONS

只允许对不影响数据安全、身份、许可、正式发布来源和工具链可复现性的剩余风险使用。例如 artifact attestations 因 GitHub 套餐暂不可用，可在强制 SHA-256 manifest、明确阻塞项和后续计划下条件通过。

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

## 5. 验收证据分层

| 层 | 证据 |
|---|---|
| 静态 | 配置解析、文件扫描、lock 结构、workflow contract、task docs diff。 |
| 本地当前宿主 | `mise run check`、本机原生 build、hooks 模拟。 |
| Actions 多平台 | Required CI matrix、实际版本和 runner metadata。 |
| Release 预演 | unsigned 10 资产、signed 签名/公证（受保护环境）。 |
| 正式 Release | tag/main/CI 来源、资产集合、摘要、attestation/manifest。 |

本设计包自身只完成“文档交付验收”，不产生上述工程证据。
