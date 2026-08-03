# 集成设计

Integration 是共享边界唯一写者。`AppState::new` 创建 service，启动后 attachment 只向
service 注入 AppHandle/event sink；`generate_handler!` 仅注册薄 commands。scoped proxy
builder 提取现有代理配置的合法部分并由 installer 设置 manual redirect，既有 global client
保持不变。

Updater removal 是全链路断开，不是隐藏按钮：Tauri plugin、permissions、config、command
registration、frontend providers/components/API、DatabaseUpgrade actions 和 release artifact
workflow 必须共同去除。database-too-new 仍拒绝不兼容数据，只显示翻译后的无网络联系支持/
受控分发信息，避免上游 URL、自动更新或 database mutation。

品牌迁移采用 clean break：当前产品身份统一映射为 `fyagent`、Rust lib
`fyagent_lib`、`fyagent.exe`、`~/.fyagent`、`fyagent.db`、`fyagent://` 与
`com.fyagent.desktop`，并同步 Flatpak、安装/自启动、发布脚本、测试夹具和当前维护文档。
不提供旧路径、协议、自启动项或序列化标记的迁移/兼容读取。审计必须区分当前产品身份与
不可伪造的真实仓库 URL、历史 changelog/release notes、LICENSE/版权及必要上游归因；后者
按事实保留。图标以 `assets/fyagent.png` 保存用户提供文件的精确字节，
由仓库现有 `pnpm tauri icon` 生成标准平台集合；About 复用生成的 32×32 PNG。macOS tray
继续使用 template rendering，但三个倍率只取源图非透明边界内的 alpha 轮廓，等比装入
24pt 画布中央的 18pt 内容框并输出黑色 RGBA，避免把彩色 app bitmap 当作菜单栏模板。
DMG 背景、第三方 provider 图标和截图不进入写集。为避免跨层竞态，本任务在所有 children
的 DTO contracts 稳定后才编辑 shared registry/Cargo/entry points。
