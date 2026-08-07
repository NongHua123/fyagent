# Repository overlay 使用说明

> **交付状态**：Proposed / 拟实施  
> **关联决策**：53–56、88–100  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

- `trellis/`：可直接审阅的 Trellis specs、workflow、skills、parent/child tasks 和 proposed archive。
- `documentation/`：活动 README/CONTRIBUTING/PR/测试说明以及新 provenance/task 参考草稿。
- 不含 `mise.toml`、`.mise/tasks/**`、`pyproject.toml`、锁文件、`.codex/hooks.json`、workflow 或产品代码的伪实现；这些必须在真实上游 merge 后按主设计实施。
- 应用 overlay 前执行三方比较；README 产品功能段可能因上游合并发生变化，优先移植开发章节和治理声明，而不是盲目覆盖整个文件。
