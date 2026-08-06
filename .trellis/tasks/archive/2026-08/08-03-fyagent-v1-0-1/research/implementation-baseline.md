# 实施基线核查

## 文档与任务边界

- `docs/fyagent/dev/v1-0.1/README.md` 将 01 需求、02 架构、03 安全/数据、04 验收定义为 v1-0.1 输入，并明确 `3.18.0` 是历史源码快照。
- 用户已锁定：产品版本 `0.1.0`；仅应用元数据与本地一致性校验；不改 release workflow；超过 1,000 个模型返回前 1,000 个 + `truncated`; 重复模型使用 `duplicatePolicy=updateAll` 重提。

## 当前代码证据

- 应用版本分别位于 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`；`src-tauri/Cargo.lock` 根包和 `tests/components/AboutSection.test.tsx` 也包含旧值。
- `AboutSection` 通过 Tauri `getVersion()` 显示版本；Cargo 的 `CARGO_PKG_VERSION` 已传播到日志、崩溃信息和部分请求标识，避免新增重复版本常量。
- `src-tauri/src/services/model_fetch.rs` 会排序、要求 Key、尝试候选 URL且返回裸列表，不符合 WorkBuddy 固定 endpoint、保序/截断和无 Key 行为。
- 当前通用 Windows `atomic_write` 包含先删除目标再 rename 的路径，不能用于 WorkBuddy 严格替换。
- `src-tauri/src/services/codex_desktop/` 已形成固定来源、可信安装身份和重启占用边界；任何新重启能力必须建立在该边界上。
- 前端当前 App 切换和 `VisibleApps` 与 Provider ID 高度耦合，因此 WorkBuddy 需要显式拆出顶层导航 ID，不能给后端 `AppType` 增加枚举分支。

## 测试与环境

- 新 Rust WorkBuddy 文件测试必须使用 `FYAGENT_TEST_HOME`/临时目录并串行化任何 HOME 覆盖，绝不写真实用户 `.workbuddy`。
- 当前会话已通过 `pnpm install --offline --frozen-lockfile --ignore-scripts` 恢复 node_modules。若后续缓存不足，用户已授权使用同一冻结锁文件与忽略生命周期脚本的正常联网恢复；无需更改 lockfile。
- Cargo、rustfmt、clippy 可用；最终 cargo 检查一律 `--locked --offline`。renderer build 与 cargo target 只写忽略产物，不构成真实 E2E。
