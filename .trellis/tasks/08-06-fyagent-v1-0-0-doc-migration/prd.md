# 将 FyAgent 最初版设计迁移为 v1.0.0 历史记录

## 目标

将 `docs/fyagent/dev/v1/` 重命名为 `docs/fyagent/dev/v1-0.0/`，使目录名
准确表达其为最初版历史设计；更新仍面向当前开发的规范，使
`docs/fyagent/dev/v1-0.2/` 成为当前范围的权威输入。

## 范围

- 使用 Git 感知重命名迁移全部 19 份 Markdown；保留文件正文、相对链接、
  历史版本、提交和冻结语义的历史事实。仅在 00、02、05、16、17 中补充目录
  已归档为 v1.0.0、当前 v1.0.2 优先的必要说明，不改写原始技术决策。
- 更新 `.trellis/spec/backend/index.md`、
  `.trellis/spec/backend/codex-desktop-installer.md` 和
  `.trellis/spec/backend/fyagent-v1-0-1-config-domains.md` 的活动优先级说明。
- 更新当前父任务所需的路径/优先级描述；归档任务、Git 历史、tags、release
  notes、`docs/fyagent/dev/v1.zip`、深链接 `fyagent://v1` 和 API `/v1` 不改。

## 验收标准

- [ ] `docs/fyagent/dev/v1-0.0/` 存在、`docs/fyagent/dev/v1/` 不存在；其余 14
  个历史 Markdown 与重命名前 `HEAD` 内容逐字相同，00、02、05、16、17 仅含
  v1.0.0 归档定位和 v1.0.2 优先级所需的最小文字更新。
- [ ] 当前 backend 规范明确：v1.0.0 是历史设计；当前 v1.0.2 在其修改契约处
  优先；v1.0.1 仅保留未被覆盖的历史配置域输入。
- [ ] 排除 `.trellis/tasks/archive/` 后，不再有活动文件将
  `docs/fyagent/dev/v1/` 作为可解析的当前路径。
- [ ] `git diff --check` 和迁移路径检查通过，且 Git 历史未被改写。

## 非范围

不重新解释、现代化或改写历史 v1.0.0 文档内容；不修改历史任务证据或协议/API
版本字符串。
