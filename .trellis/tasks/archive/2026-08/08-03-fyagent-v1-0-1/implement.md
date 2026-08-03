# FyAgent v1-0.1 执行计划

## 实施顺序

1. 读取本任务 PRD、设计、文档包和相关前后端规范；复核工作树，保持用户需求文档不变。
2. 完成版本链路：三个应用 manifest、Cargo lock、About fixture和版本一致性测试；先用小范围检查验证版本传播没有引入配置漂移。
3. 建立 Codex 原生能力后端：资格判断、无损 TOML 补丁、私有迁移元数据、Responses 校验和测试；再更新前端表单/typed facade/四语言开关。
4. 将 live 配置差异摘要接入 Provider mutation；复用可信 Codex Desktop 安装边界实现受限重启状态机、前端协调器和重启对话框；为正常退出、超时、强制确认、歧义和失败添加测试。
5. 建立 WorkBuddy 后端模块与 Tauri 命令：路径/status、URL、受限 HTTP、模型截断、严格 JSON/revision/duplicatePolicy/backup/原子替换及 Rust 测试。
6. 建立 WorkBuddy 前端独立导航、可见性兼容、页面、typed API/query、截断提示、重复确认、无 Key/Key 清理和四语言资源；添加 TypeScript/Vitest 测试并导入本地图标。
7. 进行跨层审查：验证 WorkBuddy 未进入 Provider 域，所有新 DTO 前后端字段一致，错误和日志不含秘密，Codex 只在实际 live 文件变化时请求重启。
8. 执行本地质量门禁，修复被检查发现的问题，记录无法在当前 Windows 本地环境执行的验证与人工验收项目。

## 验证命令

```powershell
# 优先使用锁定的离线恢复；若本地缓存不完整，用户已明确允许在当前 Windows
# 环境使用同一冻结锁文件进行正常联网安装。
pnpm install --offline --frozen-lockfile --ignore-scripts

pnpm typecheck
pnpm format:check
pnpm test:unit
pnpm run build:renderer

cargo fmt --manifest-path .\src-tauri\Cargo.toml --check
cargo clippy --manifest-path .\src-tauri\Cargo.toml --locked --offline -- -D warnings
cargo test --manifest-path .\src-tauri\Cargo.toml --locked --offline
git diff --check
```

## 风险与停止条件

- 不运行 `pnpm format`、带 coverage 的 Vitest、未冻结的 pnpm install 或无 `--locked` 的最终 Cargo 验证，避免验证意外改写跟踪文件。
- 缓存不完整时，可改用 `pnpm install --frozen-lockfile --ignore-scripts` 联网恢复；
  不更改 lockfile、不运行生命周期脚本，并在验证记录中说明实际使用的恢复路径。
- 不执行 CI、push、tag、release、真实 Tauri E2E、真实第三方 API 或真实用户 WorkBuddy 配置写入。
- macOS/Linux 原生行为、Windows 真实进程重启/ACL/原子替换和实际 UI 可访问性均列为人工验收，不以单元测试替代。
