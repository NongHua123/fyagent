# 上游来源与验证计划

> **交付状态**：Observed / 已核实 + Pending Verification / 待验证  
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

## 实施时必须记录

```bash
git fetch upstream tag v3.19.2
git rev-parse v3.19.2^{commit}
git show --no-patch --format='%H %D' v3.19.2
git merge-base 55173d2b v3.19.2
git log --oneline --left-right --cherry-pick <merge-base>...v3.19.2
git diff --stat <merge-base>...v3.19.2
```

验证 tag 实际解析到公开 commit `43eaf07355af145aebfee301801779e824d4c221`，并记录真实 merge base、commit set、diff stat、merge commit SHA、冲突清单和许可更新。

## 不确定性

上传包无 `.git`，本设计无法验证 `55173d2b` 是否确实包含上游 v3.19.1 ancestry，但公开 commit 页面已给出 `43eaf07355af145aebfee301801779e824d4c221`；仍不得只凭网页假定本地 tag/ref 未被污染，供应链清单必须记录本地 Git 验证结果。
