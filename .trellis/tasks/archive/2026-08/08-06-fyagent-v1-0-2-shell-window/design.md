# 应用壳层与窗口技术设计

顶部栏由 `TopLevelHeader` 组合固定顺序。Trailing primary-action slot 始终占据
相同尺寸：Provider 渲染真实按钮，WorkBuddy 渲染 `aria-hidden` 且不可交互的
占位。P2 descriptor 进入 TopMore；AppSwitcher 持有父槽的 ResizeObserver，保留
当前 app 并将非当前项移至 More。禁止以父级 `overflow-x-hidden` 裁掉关键控件。

页面保持单一纵向滚动，卡片列表内部有界。尺寸测量在稳定最大组合 fixture 上
产生版本化常量，运行时只根据工作区逻辑尺寸选择 normal/constrained，而不把
瞬时 DOM 宽度当作持久产品最小宽度。窗口状态在隐藏阶段读取、版本迁移、钳制
尺寸/位置、恢复最大化；工作区/DPI 事件防抖合并。

纯 renderer 布局可用 DOM/geometry fixture 验证；真实多显示器/DPI 窗口恢复为
受控候选环境人工验收。
