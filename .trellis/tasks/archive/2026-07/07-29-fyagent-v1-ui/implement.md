# UI 实施计划

1. 等 Core 交付 JSON fixture 后定义 TypeScript contract、结构化 error parser 与 thin API。
2. 实现 Query keys/mutations、event + get_job race-safe Hook 和 toast/copy/log behaviors。
3. 编写 Card 状态表、zh/en keys、minimal App mount 和 accessibility；本地旧版可更新时
   只显示“更新 Codex”，不渲染次级启动操作。
4. 用 MSW/mock invoke/event 覆盖状态矩阵与无禁止控件审计。
5. 运行 `pnpm typecheck`、受影响 Vitest、`pnpm format:check`，将共享品牌/CLI/updater
   冲突移交 integration。
