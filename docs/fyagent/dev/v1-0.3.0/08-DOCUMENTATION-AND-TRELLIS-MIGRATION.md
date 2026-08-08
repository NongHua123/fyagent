# 文档与 Trellis 迁移设计

> **交付状态**：Migration verified; repository closeout in progress / 迁移已验证，仓库收尾进行中
> **关联决策**：50、52–56、88–118
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 目标

使当前权威文档、AI 操作入口、Trellis 长期规范和新任务计划一致地表达：`mise run`、uv/Python、Actions 正式发布、交叉构建退役、上游同步和 DEP0040 合同。

## 2. 活动文档范围

活动文件正在真实仓库树中通过三方比较修订。`repository-overlay/documentation/` 是 2026-08-07 冻结快照，只用于比对，不再是替换源：

- `README.md`、`README_ZH.md`、`README_JA.md`、`README_DE.md`；
- `CONTRIBUTING.md`；
- `.github/pull_request_template.md`；
- `flatpak/README.md`；
- `tests/e2e/visual-baselines/README.md`；
- `docs/upstream/cc-switch-v3.19.2.md`；
- `docs/fyagent/development/mise-tasks.md`（已由真实 task metadata 生成）；
- 历史版本目录 README 的归档声明。

四份 README 只展示核心流程；详细 task 表只维护一份，命令名不翻译。

## 3. Trellis specs

### 已由前序 child 实施，Child 6 只做索引/命令收口

- `backend/development-environment.md`；
- `backend/github-release-workflow.md`；
- `backend/windows-release-boundary.md`；
- `frontend/quality-guidelines.md`。

### 已由前序 child 新增

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
mise run trellis:init-developer -- <name>
mise run trellis:context
mise run trellis:task -- <subcommand> [args]
mise run trellis:session:add -- --title <title> --commit <hashes> --summary <summary>
```

修改 `.trellis/workflow.md` 与 trellis-start/continue/before-dev/brainstorm/check/finish-work；新增 `fyagent-trellis` 本地 skill。通用 `trellis-meta/references/**` 不批量改写，以避免复制模板架构文档。

## 5. Codex hooks

`.codex/hooks.json` 与 wrappers 已实施：hook 命令使用 `mise run --silent codex:hook:*`，内部使用 `uv run --locked --no-sync --offline`。环境未准备时输出合法 JSON 和可见警告、继续用户操作；脚本损坏/非法 JSON 仍失败关闭。

## 6. 旧任务归档

五个旧任务已由 `1d3849e6` 按四个 child、旧 parent 的顺序归档到
`.trellis/tasks/archive/2026-08/`。每项均保留原始状态并写入以下元数据：

```json
{
  "archiveDisposition": "superseded",
  "supersededBy": "08-07-fyagent-upstream-toolchain-release-modernization",
  "archiveReason": "Replaced by the approved FyAgent v0.3.0 upstream, toolchain, CI, release, dependency, and documentation modernization task tree.",
  "historicalStatusBeforeArchive": "<in_progress|planning>"
}
```

归档使用 canonical `mise run trellis:task -- archive <task> --no-commit`；
`status=completed` 只是工具移动目录时写入的技术生命周期标记，
`ARCHIVE-NOTE.md` 与 `historicalStatusBeforeArchive` 明确旧需求未被冒充完成。
已经归档的 shell-window 与 desktop-acceptance 未被重建或重复归档。冻结
overlay 中的 archive 只保留原始建议，不可复制到真实 archive。

## 7. 文档漂移门禁

当前 `tasks:docs:check` / `check:contracts` 检查活动范围：

- 无 `mise exec --`；
- 不直接调用项目 pnpm/Cargo/Trellis Python；
- 无已删除 cross task/脚本；
- hook 不使用系统 `python3`；
- 文档 task 名存在；
- 生成 task 文档与元数据一致；
- spec index 完整。

排除：Actions（明确例外）、历史 `docs/fyagent/dev/**` 正文、`.trellis/tasks/**` 过程证据、通用 trellis-meta、真实产品用户命令。

## 8. 应用 overlay 的边界

overlay 已于 2026-08-07 冻结，共 111 个文件：108 个冻结 payload 和 3 个中央冻结声明。真实实施必须在当前树与上游 merge 结果上选择性移植仍有效内容；禁止复制、同步或整文件覆盖 108 个 payload。本轮 diff 只更新 3 个中央声明；当前树和长期 specs 是唯一活动事实源。

## 9. 完成边界

正式 Release 已于 [formal run `31260931509`](https://github.com/NongHua123/fyagent/actions/runs/31260931509) 完成；[stable/latest v0.3.0](https://github.com/NongHua123/fyagent/releases/tag/v0.3.0) 的 13 附件、manifest/metadata/bundle digest、独立重下载与 12-subject attestation 复核已写入本设计包，因此 Release 可标记为 `Released / Verified`。

本 closeout PR 的 Windows x64/ARM64 locked uv-managed Python/Trellis
Required gate 已由修复后 run `31265504901` 远程验证，真实证据已经写回；
最终 `MANIFEST.sha256` 也已在设计包字节冻结后重建并复验。Child 6 只在实际
归档前保持 `in_progress`，并在同一 PR 内按 Child 3 → Child 4 → Child 6
归档，最后归档 parent。随后才记录 journal、运行最终 PR CI、merge、验证
exact-main CI，并清理非 main 分支。后四项是 parent-level 后续阶段，不是
Child 6 的循环 archive 前置条件。每个阶段都必须使用真实结果，不得用占位
URL、预期成功或提前 archive 替代。
