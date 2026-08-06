# FyAgent v1.0.2 应用壳层与窗口布局

## 目标

消除 WorkBuddy 页面白边、顶部栏跳动与 `+` 被裁剪问题，使应用在正常和受限
工作区都保持 P0 操作可见、可访问并稳定定位。

## 范围

- UI-001–004：WorkBuddy 标准全宽壳层、卡片响应式布局、单主滚动与稳定滚动条。
- UI-005–014：固定顶部栏顺序、Provider `+` / WorkBuddy 空槽、P0/P1/P2 优先级、
  AppSwitcher ResizeObserver/More、Profile 退化和键盘顺序。
- UI-015–020：最大组合夹具测量、版本化最小宽度、默认尺寸、受限模式、窗口
  状态恢复、工作区/DPI 合并调整。

## 验收标准

- [ ] Provider/WorkBuddy 切换中 AppSwitcher 与主操作槽右边缘偏移不超过
  1 CSS px，空槽没有交互、Tab、tooltip 或辅助技术名称。
- [ ] 正常容量直显九应用与 P0/P1；受限模式按规定顺序收纳，P0 仍可操作，
  无横向滚动、裁剪或“小屏幕”Toast。
- [ ] 页面白边消失，布局、滚动和窗口恢复符合 UI-001–004/015–020；四语、
  缩放与保存状态通过 mock/geometry 测试。
- [ ] 不启动真实 Codex/ChatGPT 或真实窗口自动化；本机仅运行 mock/fixture 的
  renderer 与布局验证。
