# 桌面验收自动化实施计划

1. 建立依赖、WDIO 配置、test-build 开关、mock IPC 和稳定 fixture；先验证它们
   无法调用真实 Codex/ChatGPT、用户文件或网络。
2. 为顶部栏/窗口和 WorkBuddy 建立 geometry helpers、稳定化和关键区域 baseline；
   添加 UI-E/WB-E 场景并覆盖四语/正常/受限模式。
3. 建立 CR/CV/WIN 的 fake 服务/静态产物证据收集，保留真实运行/发布为人工项。
4. 配置 LFS、显式更新基线脚本和 workflow_dispatch CI 任务；不添加 push/PR 触发。
5. 输出需求—测试矩阵、人工清单和脱敏制品约定，运行允许的本机 mock 检查。
