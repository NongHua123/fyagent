## Scope and rationale / 范围与原因

<!-- What problem does this PR solve, why this approach, and what is explicitly out of scope? -->
<!-- 本 PR 解决什么问题、为何采用此方案，以及哪些内容明确不在范围内？ -->

## Related work / 关联工作

<!-- Link the Issue and Trellis task. Use "Fixes #123" only when appropriate. -->
<!-- 关联 Issue 与 Trellis task；仅在适用时使用 "Fixes #123"。 -->

Fixes #

Trellis task:

## Evidence / 验证证据

<!-- List exact commands, results, platforms, screenshots/logs, and anything not verified. -->
<!-- 列出精确命令、结果、平台、截图/日志，以及尚未验证的内容。 -->

- Local gate / 本地门禁: `mise run check`
- Focused checks / 聚焦检查:
- Native or remote evidence / 原生或远程证据:
- Not verified / 未验证:

## Risk and rollback / 风险与回退

<!-- Cover user-visible behavior, compatibility/data/security, release impact, and the narrow rollback. -->
<!-- 说明用户可见行为、兼容性/数据/安全、发布影响和最小回退方式。 -->

## Contract and provenance impact / 契约与来源影响

- [ ] Durable rules update the owning `.trellis/spec/**` and enforcing test, or
      this PR explains why none changed / 长期规则已更新对应 spec 与测试，或已说明无需修改
- [ ] Upstream changes record tag, tag object/peeled commit, merge/conflict
      decisions, and preserved FyAgent contracts, or are not applicable /
      上游变更已记录 tag、SHA、merge/冲突与 FyAgent 契约，或不适用
- [ ] CI/Release changes record triggers, permissions, runner/platform evidence,
      exact assets, remaining remote gates, and rollback, or are not applicable /
      CI/Release 变更已记录触发、权限、平台、资产、远程门禁与回退，或不适用

## Screenshots / 截图

<!-- If applicable, add before/after screenshots. / 如适用，请添加修改前后的截图。 -->

| Before / 修改前 | After / 修改后 |
| --------------- | -------------- |
|                 |                |

## Checklist / 检查清单

- [ ] `mise run check` passes on the current host / 当前宿主完整门禁通过
- [ ] Tests cover observable success and failure behavior / 测试覆盖可观察成功与失败路径
- [ ] User-visible text updates all four locales and accessibility states /
      用户可见文本已同步四份 locale 与无障碍状态
- [ ] No secret, certificate, personal config, `.venv`, or user data is included /
      未包含 secret、证书、个人配置、`.venv` 或用户数据
- [ ] Claims distinguish local checks from native runners and published Release
      evidence / 结论明确区分本地检查、原生 runner 与正式发布证据
