---
status: final
version: v1
usage: reusable implementation and review checklists
---

# FyAgent V1.0.0 历史维护者实施检查清单

> 归档说明：以下提示词按 V1.0.0 当时的范围原样保留作历史参考，不是当前实施指令。当前实现先使用 `../v1-0.2/` 文档；需要追溯初版假设时再阅读本目录。

## 1. 总控集成

```text
在当前 `NongHua123/cc-switch` 仓库 checkout 中实施 FyAgent V1“一键安装 Codex”。先完整阅读 `v1-0.0/00-AGENT-START-HERE.md` 至 `v1-0.0/18-REFERENCES.md`，并把它们视为该历史版本的实现契约。

目标：基于当前仓库的 CC Switch 3.18.0 衍生代码，按最小侵入方式新增 Windows x64、Windows ARM64、macOS Apple Silicon 的官方 ChatGPT 桌面应用（包含 Codex）镜像安装功能。最终用户运行时必须只走中国大陆友好的 agentsmirror 内置端点；开发阶段联网不受限制。

开始前执行并记录：git status --short、git rev-parse HEAD、当前分支、mise 版本，以及通过 mise 解析的 Node/pnpm/Rust/Cargo 版本。不要丢弃任何现有用户修改，不要同步上游 CC Switch。

如采用 worktree 并行，先由 Core 冻结领域/IPC 契约，再让 Windows、macOS、UI 在该契约上工作；共享注册文件、Cargo 配置、品牌/更新边界和旧 Codex CLI 清理必须由集成负责人统一协调，避免并行覆盖。

严格约束：

- FyAgent 采用 clean break：当前 identifier/deep-link/数据目录/数据库/日志/内部包名/
  安装与序列化标记只使用 FyAgent/fyagent，不迁移、不回退读取、不注册旧别名、
  不清理旧自启动值；
- 真实仓库 `NongHua123/cc-switch`、历史证据、LICENSE/版权、必要上游引用以及
- 只下载并安装 OpenAI官方原始未修改包，不重打包/重签/改名Bundle内容。
- 普通 Windows UI只做当前用户安装，不显示scope、不触发UAC；所有用户仅隐藏实验CLI/headless。
- 不新增数据库表，不修改 ~/.codex，不接管官方应用自更新。
- 不做断点续传、跨重启恢复、多源、fallback、修复/卸载/回滚/向导。
- 严格校验 SHA256、身份、架构、签名、最低系统和安装后结果。
- 不执行真实生产安装，不自动卸载/强杀应用。
- Codex CLI保留只读版本/latest/诊断，删除并后端拒绝install/update/repair。
- 保持 Commands→Services→Domain/Platform 与 API→Query→Hook→Component 分层；App.tsx只挂载。

每个原子提交必须说明文件范围、新依赖、测试、未执行真实安装和已知限制。完成后运行：
mise exec -- pnpm install --frozen-lockfile
mise exec -- pnpm typecheck
mise exec -- pnpm format:check
mise exec -- pnpm test:unit
mise exec -- cargo fmt --check --manifest-path src-tauri/Cargo.toml
mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml

不得声称人工平台验收通过。输出最终commit列表、测试结果、契约偏差、已知限制和人工验收待办。
```

## 2. Core 实施

```text
Core 实施者只修改 `v1-0.0/03-AGENT-WORKTREE-EXECUTION.md` 分配的文件。先阅读 01、02、04、05、06、10、11、12、13。

实现：领域DTO、稳定错误、版本/平台模型、agentsmirror manifest/checksum adapter、canonical release_id、5分钟缓存输入、redirect/retry策略、流式downloader、三倍磁盘预检、临时目录清理、单Job状态机、取消、完整快照事件、service编排、薄Tauri命令契约和mock platform trait。不要修改共享注册文件、Cargo、App.tsx或平台实现。

运行纯单元/服务测试；不得联网下载完整生产包，不得安装。提交一个或少量原子commit，并在正文列出冻结的Rust/TS字段、命令名、事件名和平台trait。
```

## 3. Windows 实施

```text
Windows 实施者基于已冻结 Core 契约工作，并阅读 02、05、06、07、10、11、13、14。

实现 Windows x64/ARM64：安全解析AppxManifest、官方Stable身份/Publisher/PFN/架构/MinVersion/签名校验、本地PackageManager查询、四段版本比较、AddPackageByUriAsync当前用户部署、progress和HRESULT映射、安装后重查、AUMID启动。禁止PowerShell/winget主路径、x64 fallback、降级、强杀应用、绕过策略。

另实现隐藏的all-users实验：同一FyAgent受限headless参数、runas、nonce/job过期/路径/TOCTOU重验证、stage+Provision。普通Tauri UI不能选择all_users。

只修改Windows独占目录和测试，不改共享注册/Cargo。使用mock PackageManager，不执行真实安装。当前身份常量必须由签名fixture验证；变化时停止并报告证据，不能放宽allowlist。
```

## 4. macOS 实施

```text
macOS 实施者基于 Core 契约工作，并阅读 02、05、06、08、10、11、13、14。

只支持Apple Silicon。扫描/Applications和~/Applications，按Bundle ID识别Stable；保护Classic/Beta，多个Stable阻断。挂载DMG使用hdiutil -plist，保持DMG原始.app名；已有Stable更新原路径。验证arm64、macOS>=14与LSMinimumSystemVersion、codesign --deep --strict、Team ID、spctl。/Applications权限不足回退~/Applications；路径被不同Bundle占用时不覆盖。ditto到同卷临时sibling、复制后重验、可恢复替换、RAII detach；应用运行中阻断，不quit/kill；用verified path open。

只修改macOS独占目录和测试，不改共享注册。所有系统命令通过mockable runner，测试不得写真实Applications或下载生产DMG。身份常量变化时报告，不放宽allowlist。
```

## 5. UI 实施

```text
UI 实施者阅读 02、05、09、10、11、13，并遵循其中的共享契约。

按现有前端分层新增：TS types、invoke API、TanStack Query、useCodexDesktopInstaller、CodexDesktopInstallerCard和测试。Card位于Codex/provider页顶部；Linux隐藏、Intel Mac不支持。显示install/update/launch/progress/cancel/retry；remote失败时local仍可launch；成功Toast一次；不显示scope、source selector、URL或向导。

完整维护zh/en文案，其他语言fallback英文。不要把业务逻辑塞App.tsx，只提供最小挂载patch。Codex CLI UI改成只读、排除bulk和安装命令；后端清理由集成Agent处理。mock invoke/event，覆盖状态矩阵。不要新增前端生产依赖。
```

## 6. 最终审查

```text
审查 feature/fyagent-v1 与基线差异，对照v1全部文档。重点找：
1) 用户运行时是否存在非agentsmirror关键路径、fallback或自定义URL；
2) 是否任何校验可绕过；
3) 普通UI能否触发all-users/UAC；
4) mac是否按文件名覆盖Classic；
5) Windows是否下载错误架构/直接启动exe；
6) Codex CLI写操作是否仍可经直接IPC触发；
8) App.tsx/commands是否承担业务逻辑；
9) 日志是否泄漏query/home/token；
10) 实施记录是否误执行或声称真实 E2E。

运行完整质量命令，输出按严重度排序的问题、文件/行号、复现和建议。不要修改代码，除非明确收到修复任务。
```
