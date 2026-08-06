# FyAgent v1.0.2 桌面验收自动化与证据

## 目标

为 v1.0.2 的 WorkBuddy、Codex 状态、顶部栏/窗口、Windows 发布和四语结果建立
可重复、可追踪的自动化/人工验收体系，而不把真实应用或本机环境当作测试对象。

## 范围

- QA-001–007：有限 WebdriverIO/Tauri desktop E2E、mock IPC、几何断言、关键
  区域视觉基线、四语/缩放/平台矩阵、Git LFS 基线和手动 workflow dispatch。
- `07` 中 WB/CR/CV/UI/WIN/I18N P0 场景的需求—测试映射、fixtures、结果 JSON、
  截图/差异图、脱敏日志与最终验收报告结构。

## 约束

- E2E 中的 Tauri IPC 只可使用替身，绝不关闭、启动或读取真实 Codex/ChatGPT，
  不触发 UAC、安装器、PackageManager、真实第三方端点或真实用户配置。
- 普通测试不得自动接受当前截图为基线；正式基线更新需显式受控命令，PNG 使用
  Git LFS，平台/缩放/语言基线互相隔离。

## 验收标准

- [ ] E2E fixture 固定时间、时区、随机数、网络、字体、动画、版本和模型数据，
  以 mock IPC 覆盖正常/受限窗口及四语状态。
- [ ] UI-E / WB-E 场景至少能证明几何、焦点、滚动和视觉关键区域；不以宽松
  snapshot 掩盖偏移、裁剪或无障碍回归。
- [ ] release/Windows 验证脚本能产出 manifest、签名、安装器、IPC 和配置的
  受控候选环境证据需求，但本机不执行真实发布操作。
- [ ] 验收报告将自动通过、未运行人工项、平台/缩放/语言和已接受风险明确分开。
