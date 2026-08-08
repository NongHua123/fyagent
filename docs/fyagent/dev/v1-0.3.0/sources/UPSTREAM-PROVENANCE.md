# 上游来源与已验证 provenance

> **交付状态**：Verified local Git provenance / 本地 Git provenance 已核实
> **关联决策**：39–51、101  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 已知来源

```text
Upstream repository: https://github.com/farion1231/cc-switch.git
Target tag:          v3.19.2
GitHub short SHA:    43eaf07
Public commit SHA:   43eaf07355af145aebfee301801779e824d4c221
Release date:        2026-08-06
License:             MIT (upstream-derived portions)
```

GitHub 发布页说明本版数据库 schema 保持 v16、没有新增迁移；这不替代 FyAgent 数据兼容性测试。

## 实际 Git 证据

```bash
mise run upstream:check
mise run upstream:fetch -- v3.19.2
mise run upstream:audit -- v3.19.2
```

`upstream:audit` 报告 tag object、peeled commit、merge base、待合入 commits 和 diff summary；下面的 SHA 是真实运行及 merge graph 的冻结证据，而不是要求维护者绕过 task API 重新拼接 Git 命令。

| 字段             | 已核实值                                               |
| ---------------- | ------------------------------------------------------ |
| 实施基线         | `55173d2b32c4acf182b6ec504d7ad326ade2bb9b`             |
| recovery ref     | `refs/backup/fyagent-v0.3.0-baseline`                  |
| tag object       | `f6882b69f0a30968dcc6dbb1153b6b12b50e6b1a`             |
| peeled commit    | `43eaf07355af145aebfee301801779e824d4c221`             |
| merge base       | `28529620f438b2ed25c812f6364825d846a4a9d6` (`v3.19.1`) |
| two-parent merge | `f4462765e9b3a2efd1deb13aabf3ce349166a058`             |
| first parent     | `194edb22ef6896f865e08a21b27d5b846dbaf54d`             |
| second parent    | `43eaf07355af145aebfee301801779e824d4c221`             |

本地 tag object/peeled commit 与 upstream `ls-remote` 一致；`git merge-base --is-ancestor refs/tags/v3.19.2 f4462765` 通过。merge 产生 33 个冲突，均按身份、共享上游逻辑、工程治理和 FyAgent 专属能力分层裁决，没有全局 ours/theirs。

## 证据边界

原始上传包不含 `.git` 的限制已由真实 checkout 核验消除。该 provenance 证明本地 merge graph 与来源，不证明远程 PR/main、`v0.3.0` tag、installer 或 GitHub attestation；这些仍属于后续远程 evidence。
