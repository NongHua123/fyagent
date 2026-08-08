## Summary / 概述

<!-- What problem does this PR solve, and why is this approach appropriate? -->
<!-- 该 PR 解决什么问题，为什么采用当前方案？ -->

## Related Issue or Trellis Task / 关联 Issue 或 Trellis Task

Fixes #

Task: `.trellis/tasks/...`

## Scope and Risk / 范围与风险

- Affected layers / 影响层：
- User/data/security impact / 用户、数据或安全影响：
- Rollback / 回退方式：

## Evidence / 验证证据

<!-- List exact tasks, platforms and results. Do not label mock-only evidence as native. -->
<!-- 列出准确任务、平台和结果；不要把 mock 结果描述为原生平台证据。 -->

```text
mise run check
```

## Screenshots / 截图

| Before / 修改前 | After / 修改后 |
| --- | --- |
|  |  |

## Documentation and Contract / 文档与契约

- [ ] Commands, workflow or onboarding changes update maintained documentation / 命令、流程或初始化变化已更新活动文档
- [ ] A durable engineering rule updates the owning Trellis spec and test / 长期工程规则已更新对应 Trellis spec 与测试
- [ ] Generated files have both generation and drift checks / 生成文件已运行生成与一致性检查

## Conditional Information / 条件信息

### Upstream merge / 上游合并

- Tag and full SHA / 标签和完整 SHA：
- Conflict decisions / 冲突裁决：
- MIT provenance updated / MIT 来源已更新：

### CI or Release / CI 或 Release

- Runner/matrix impact / Runner 或矩阵影响：
- Token/secret/permission impact / Token、Secret 或权限影响：
- Expected asset contract / 预期产物契约：

## Checklist / 检查清单

- [ ] `mise run check` passes on the current host / 当前宿主 `mise run check` 通过
- [ ] `mise run system:check` passes when native build behavior changed / 涉及原生构建时 `system:check` 通过
- [ ] Required locale files are updated / 已更新全部所需语言文件
- [ ] No secret, certificate, user data, `.venv`, or `mise.local.*` is committed / 未提交敏感数据或个人环境文件
- [ ] Local output is not represented as a formal Release asset / 未将本地产物描述为正式 Release 资产
- [ ] Not-applicable checks are explained with evidence / 不适用检查已说明原因
