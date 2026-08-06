# 应用壳层与窗口实施计划

1. 建立固定测试夹具与几何断言，先固定现有白边、裁剪和跳动失败案例。
2. 从 `App.tsx` 提取 header、trailing slot、P2 overflow 和 AppSwitcher overflow；
   迁移 Profile 紧凑/截断并验证键盘顺序。
3. 移除 WorkBuddy 根级窄栏，建立标准页面壳和卡片/ScrollArea 布局。
4. 增加 window layout constants、Rust window state service 与 renderer layout-mode
   hook，更新 Tauri 配置并以 mock 工作区测试迁移/钳制。
5. 运行 Vitest/geometry、typecheck、format；将真实窗口/DPI 结果标为人工验收。

主要文件：`src/App.tsx`、`src/components/AppSwitcher.tsx`、
`src/components/profiles/ProfileSwitcher.tsx`、`src/components/workbuddy/*`、
`src/lib/layout/*`、`src-tauri/src/lib.rs`、`src-tauri/tauri*.conf.json` 及测试。
