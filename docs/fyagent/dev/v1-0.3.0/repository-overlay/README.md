# Repository overlay 冻结说明

> **状态**：Frozen design snapshot / 2026-08-07 冻结设计快照
> **取代日期**：2026-08-08

本目录共 111 个文件：108 个冻结 payload，加上本 README 与两个子目录 `OVERLAY-MAP.md` 共 3 个中央冻结声明。108 个 payload 保留原始需求与建议，已由真实仓库树、实际提交和 `.trellis/spec/` 活动合同取代。禁止复制、同步、脚本化应用或整文件覆盖到仓库根；README 和 Trellis 内容尤其会覆盖上游合并后的产品变化与 Child 1–5 已实施合同。

三方比较时只能把单条仍有效意图作为审计输入，并以当前文件、测试、覆盖决策和 child evidence 决定是否移植。本轮 overlay diff 只包含上述 3 个中央冻结声明；108 个 payload 字节未修改。设计包 `MANIFEST.sha256` 已在正式 Release closeout 证据冻结后统一重建并复验，中央声明的状态更新不改变 108 个冻结 payload。
