# Package metadata

- 生成日期：2026-08-06
- 目标 FyAgent 应用版本：`0.2.1`
- 版本范围：Windows、macOS、Linux、运行时版本、Git 标签、发布资产与下载清单
- Windows 安装器方案：`perMachine + elevated + 用户可选安全目录 + 原生 Custom Action`
- 版本治理方案：Cargo workspace 单一真源 + 一键更新脚本 + CI 版本契约
- 文档语言：简体中文

## 输入基线

| 输入 | SHA-256 |
|---|---|
| `cc-switch-feature-fyagent-v1(1)(2).zip` | `67300c8be0efe00aeb807020c15ed838fdfed3b5e34238542e58d6f962e78f6b` |
| `FyAgent-0.2.0-Windows(1).msi` | `18d6ceb85b2af9f891400f787b19cbb8f574499d36ff3cb59d16d30056190809` |
| `image(20260806-054829).png` | `e65339b740ecfb32caa491a01896f416b147d07a6d310b70b5459609b9487f92` |

## 源码定位基线

- 解包根目录：`cc-switch-feature-fyagent-v1`
- 当前应用版本：`0.2.0`
- WiX 模板：`src-tauri/wix/per-machine-main.wxs`
- 发布工作流：`.github/workflows/release.yml`
- Windows 交叉构建：`scripts/windows-cross/build-windows-msi.sh`
- macOS 元数据检查：`scripts/macos-cross/project_metadata.py`
- 下载清单：`scripts/generate-download-manifest.mjs`

## 交付内容

- 12 份编号需求/设计/实施文档
- 1 份目录索引
- 1 个版本脚本参考实现
- 1 个版本脚本测试文件
- 6 份接入片段
- SHA-256 清单

## 研究依据

方案优先使用 Tauri、Cargo、npm、SemVer、WiX/FireGiant 与 Microsoft Windows Installer/Win32 官方文档；CC Switch 的目录选择行为以其公开 WiX 源码和本次留存快照作为对照。完整链接见 `11-ADR-RISKS-AND-REFERENCES.md`。
