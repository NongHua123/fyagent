# CC Switch v3.19.2 完整合并设计

> **交付状态**：Implemented, locally verified, archived / 已实施、本地验证并归档
> **关联决策**：28–30、39–51  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 目标与边界

[Decision / 已决策] 从项目方提供的 FyAgent `55173d2b` 基线完整合并 `upstream/v3.19.2`，保留上游祖先关系。该 merge commit 不承载本次工具链、CI、交叉构建或 `DEP0040` 重构。

[Observed / 已核实] 真实仓库已完成以下核验：

```bash
mise run upstream:check
mise run upstream:fetch -- v3.19.2
mise run upstream:audit -- v3.19.2
```

预期远程：

```text
origin fetch/push = https://github.com/NongHua123/fyagent.git
upstream fetch    = https://github.com/farion1231/cc-switch.git
upstream push     = DISABLED
```

实际 tag object 为 `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`，peeled commit 为 `43eaf07355af145aebfee301801779e824d4c221`，merge base 为 `28529620f438b2ed25c812f6364825d846a4a9d6`。两父 merge commit `f4462765e9b3a2efd1deb13aabf3ce349166a058` 的第二父为上述 peeled commit。

## 2. Git 流程

```text
55173d2b
  └─ create isolated integration branch
      └─ fetch verified v3.19.2 tag
          └─ mise run upstream:merge:prepare -- v3.19.2 --apply
              ├─ resolve semantic conflicts
              ├─ run ordinary CI-equivalent checks available locally
              └─ create one explicit merge commit
                  └─ follow-up modernization commits
```

禁止：squash、rebase FyAgent 到上游、逐提交 cherry-pick 代替完整合并、全局 ours/theirs、自动 push。

## 3. 冲突裁决

| 层级 | 规则                     | 典型内容                                                                                 |
| ---- | ------------------------ | ---------------------------------------------------------------------------------------- |
| 1    | 必须保留 FyAgent 身份    | 名称、仓库 URL、bundle ID、deep link、数据/日志目录、`FYAGENT_*`、图标、资产命名、许可。 |
| 2    | 默认跟随上游共享产品逻辑 | 安全、数据正确性、Provider/CLI/MCP/Skills、平台兼容、代理、性能。                        |
| 3    | 以本项目工程决策覆盖     | Node/Rust/Actions/mise/uv、交叉构建移除、Release 契约、DEP0040。                         |
| 4    | 保留 FyAgent 专属功能    | 上游没有实现不是删除本地功能的理由。                                                     |

每个语义冲突应记录：文件、上游行为、FyAgent 行为、最终裁决、理由、覆盖测试、重新评估条件。

## 4. 上游产品运行时 mise

[Decision / 已决策] 保留上游对 mise-managed CLI 的**可选兼容**：shims、安装来源诊断、来源锚定升级等。不得将其误写为 FyAgent 启动硬依赖。验收包括“未安装 mise 的设备正常启动”和“使用 mise 的用户仍可发现 CLI”。

## 5. 许可与来源

- CC Switch 直接合入的代码仍属于 MIT-derived；
- FyAgent 自有代码/修改继续按仓库现有许可边界；
- 更新 `THIRD_PARTY_NOTICES.md` 和简洁来源台账；
- 冲突解决不得删除上游版权/许可声明以规避边界；
- 具体商业分发争议需法律专业意见，本设计不是法律意见。

## 6. Release notes 处理

[Observed / 已核实] merge commit 正常接收了上游 release-note 文件；Child 6 已在 `58101230` 中删除 `docs/release-notes/v3.19.2-*.md`，并将上游来源收口到 provenance 与 FyAgent CHANGELOG。保留：

- merge ancestry；
- `docs/upstream/cc-switch-v3.19.2.md`；
- FyAgent CHANGELOG 简述；
- tag/完整 SHA/许可来源。

不得将 CC Switch release notes 替换品牌后当作 FyAgent 发布说明。

## 7. 验证与回退

根据决策 49，没有新增上游产品专用 workflow。merge 隔离提交已通过当时可用的本地前端/Rust/身份/许可/数据路径合同；完整跨平台和无签名包路径仍由最终 CI/Release workflow 验证。

回退要求：上游 merge commit 与后续重构分离，使维护者可以在未叠加后续提交时 `git revert -m 1 <merge-commit>`。回退前仍需评估数据库/数据行为，不得只看 Git 文件差异。

## 8. 任务自动化边界

`mise run upstream:*` 只允许安全检查、fetch、audit 和进入未提交 merge 状态。`upstream:merge:prepare` 必须在内部完成确认后才执行 merge，且不得通过 `depends` 在确认前隐藏副作用。
