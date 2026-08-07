# 文档与 Trellis 迁移设计

> **交付状态**：Proposed / 拟实施  
> **关联决策**：50、52–56、88–100  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 目标

使当前权威文档、AI 操作入口、Trellis 长期规范和新任务计划一致地表达：`mise run`、uv/Python、Actions 正式发布、交叉构建退役、上游同步和 DEP0040 合同。

## 2. 活动文档范围

完整修订稿位于 `repository-overlay/documentation/`：

- `README.md`、`README_ZH.md`、`README_JA.md`、`README_DE.md`；
- `CONTRIBUTING.md`；
- `.github/pull_request_template.md`；
- `flatpak/README.md`；
- `tests/e2e/visual-baselines/README.md`；
- `docs/upstream/cc-switch-v3.19.2.md`；
- `docs/fyagent/development/mise-tasks.md`（标记为 proposed catalog）；
- 历史版本目录 README 的归档声明。

四份 README 只展示核心流程；详细 task 表只维护一份，命令名不翻译。

## 3. Trellis specs

### 重写

- `backend/development-environment.md`；
- `backend/github-release-workflow.md`；
- `backend/windows-release-boundary.md`；
- `frontend/quality-guidelines.md`。

### 新增

- `backend/task-runner-contract.md`；
- `backend/upstream-sync.md`；
- `backend/development-hooks.md`。

### 删除活动 spec

- `backend/wsl-macos-cross-build.md`。

### 更新索引

backend/frontend index 必须列出新规范并统一使用 `mise run check` 等入口。

## 4. Trellis workflow 与 skills

项目实际操作入口统一包装：

```text
mise run trellis:init-developer
mise run trellis:context
mise run trellis:task
mise run trellis:session:add
```

修改 `.trellis/workflow.md` 与 trellis-start/continue/before-dev/brainstorm/check/finish-work；新增 `fyagent-trellis` 本地 skill。通用 `trellis-meta/references/**` 不批量改写，以避免复制模板架构文档。

## 5. Codex hooks

配置文件的实际实施不在本 overlay 中，但目标合同明确：hook 命令改为 `mise run codex:hook:*`；内部 `uv run --locked --no-sync --offline`。环境未准备时输出合法 JSON 和可见警告、继续用户操作；脚本损坏/非法 JSON 仍失败。

## 6. 旧任务归档

当前五个未归档任务拟复制到 `archive/2026-08/`，先写入：

```json
{
  "archiveDisposition": "superseded",
  "supersededBy": "08-07-fyagent-upstream-toolchain-release-modernization",
  "archiveReason": "Replaced by the consolidated modernization plan"
}
```

使用现有 `task.py archive --no-commit` 的技术关闭行为；文档统一称 superseded，不称“已实施完成”。完整 proposed archive 位于 Trellis overlay。

## 7. 文档漂移门禁

未来 `tasks:docs:check` / `check:contracts` 检查活动范围：

- 无 `mise exec --`；
- 不直接调用项目 pnpm/Cargo/Trellis Python；
- 无已删除 cross task/脚本；
- hook 不使用系统 `python3`；
- 文档 task 名存在；
- 生成 task 文档与元数据一致；
- spec index 完整。

排除：Actions（明确例外）、历史 `docs/fyagent/dev/**` 正文、`.trellis/tasks/**` 过程证据、通用 trellis-meta、真实产品用户命令。

## 8. 应用 overlay 的边界

overlay 是完整文档草稿，不是自动 patch。实施者必须在上游合并和代码配置改造后复核产品功能/版本/路径，再选择性替换。尤其 README 产品段落可能因 v3.19.2 合并而变化，不能不经比较直接覆盖。
