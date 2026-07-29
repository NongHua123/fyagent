# 集成设计

Integration 是共享边界唯一写者。`AppState::new` 创建 service，启动后 attachment 只向
service 注入 AppHandle/event sink；`generate_handler!` 仅注册薄 commands。scoped proxy
builder 提取现有代理配置的合法部分并由 installer 设置 manual redirect，既有 global client
保持不变。

Updater removal 是全链路断开，不是隐藏按钮：Tauri plugin、permissions、config、command
registration、frontend providers/components/API、DatabaseUpgrade actions 和 release artifact
workflow 必须共同去除。database-too-new 仍拒绝不兼容数据，只显示翻译后的无网络联系支持/
受控分发信息，避免上游 URL、自动更新或 database mutation。

品牌迁移采用审计表逐项分类：可见名称/链接/文案更改，许可证/identifier/deep-link/data
directory/internal package 名保留。为避免跨层竞态，本任务在所有 children 的 DTO contracts
稳定后才编辑 shared registry/Cargo/entry points。
