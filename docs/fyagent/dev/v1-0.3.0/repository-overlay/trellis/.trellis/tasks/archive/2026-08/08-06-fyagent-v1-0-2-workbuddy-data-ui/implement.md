# WorkBuddy v1.0.2 实施计划

1. 先为对象根、非法结构、已有 ID、覆盖预检和 Key 生命周期补失败测试。
2. 改造 `services/workbuddy`：新增 document/overwrite 边界，更新 types/error/
   config/mod 和 `commands/workbuddy.rs`；保留现有 revision、backup、原子写入。
3. 更新 typed API/query 与 mock handler，新增 ID query，替换旧 duplicate DTO。
4. 拆分 WorkBuddy 页面为状态、已有 ID、连接和远程模型卡片；实现有限筛选 hook、
   覆盖确认和固定下载动作；同步四语言。
5. 运行 Rust/React 目标测试、typecheck、format 与静态秘密审查；禁止真实文件/
   网络调用。

主要文件：`src-tauri/src/services/workbuddy/*`、`src-tauri/src/commands/workbuddy.rs`、
`src/lib/api/workbuddy.ts`、`src/lib/query/workbuddy.ts`、
`src/components/workbuddy/*`、`src/i18n/locales/*.json` 及对应测试。
