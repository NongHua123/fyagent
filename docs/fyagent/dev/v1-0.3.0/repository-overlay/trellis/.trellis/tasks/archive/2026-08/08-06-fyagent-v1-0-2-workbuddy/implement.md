# FyAgent v1.0.2 父任务实施计划

## 顺序与完成定义

1. **文档迁移**：先完成 `v1` → `v1-0.0` Git 重命名、活动规范的优先级更新和
   逐字内容验证。完成后，任何后续实现者都以 v1.0.2 作为当前契约。
2. **WorkBuddy 数据与 UI**：先建立双根文档模型、保存事务、覆盖 token、DTO 和
   Rust 测试，再接前端 query、卡片、过滤/选择、密钥生命周期及四语文案。
3. **Codex 版本状态与重启协议**：版本状态可与 WorkBuddy 并行；重启先改私有
   平台候选集合、固定比较器与 fake 服务测试，再升级 Tauri DTO、协调器和对话框。
4. **顶部栏与窗口**：冻结容量夹具和布局常量后重构 header，接入受限模式及窗口
   恢复；不能靠隐藏 overflow 通过测试。
5. **Windows 安全与发布**：只在普通权限 fake/单测稳定后调整 manifest、WiX、
   单实例、降权启动、autolaunch、IPC/Capability/CSP 和 workflow。
6. **验收自动化**：为已冻结接口配置 mock desktop/E2E、几何、视觉、四语和
   发布证据。真实应用/安装器/签名项目列为候选环境人工验收。
7. **集成审查**：核对所有需求编号、DTO 前后端字段、密钥/日志边界、活动文档
   优先级、静态格式与可执行检查；记录不能在本机完成的验证。

## 每个子任务的启动前检查

- 完成该子任务的 `prd.md`；复杂子任务还必须有 `design.md`、`implement.md`。
- 其 `implement.jsonl` 和 `check.jsonl` 各至少含一条真实规范/研究项。
- 根据受影响层读取对应 Trellis 规范，且先搜索现有 API/类型/测试模式。
- 复核工作树，不覆盖用户已有的 v1.0.2 文档输入或其他子任务变更。

## 统一验证命令

根据改动层选择运行，且仅在不会触发真实 Codex/ChatGPT 操作的前提下执行：

```bash
mise exec -- pnpm typecheck
mise exec -- pnpm format:check
mise exec -- pnpm test:unit
mise exec -- pnpm run build:renderer
mise exec -- cargo fmt --manifest-path src-tauri/Cargo.toml --check
mise exec -- cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline -- -D warnings
mise exec -- cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
git diff --check
```

新加的 Codex 服务测试必须使用假平台/假时钟/fixture。不得执行真实进程、
PackageManager、安装包、用户配置或 GUI 操作；Windows/macOS 发布候选验证另列
人工结果，不用本机命令替代。

## 文档迁移验证

```bash
test ! -d docs/fyagent/dev/v1
test -d docs/fyagent/dev/v1-0.0
rg -n -F 'docs/fyagent/dev/v1/' . \
  --glob '!docs/fyagent/dev/v1-0.0/**' \
  --glob '!.trellis/tasks/**'
git diff --check
git diff --find-renames -- docs/fyagent/dev .trellis/spec/backend
```

重命名前后比较 `HEAD` 中每个旧 Markdown 与新路径内容，确认仅路径发生变化。

## 风险停止条件

- 缺少 macOS 精确 Bundle ID、签名证书或受控测试环境时，继续完成可隔离代码与
  fake 测试，但不得虚构真实运行/发布通过。
- 发现需要放宽身份匹配、把密钥写入持久状态、以管理员权限启动普通应用、或
  修改归档/Git 历史才能推进时，停止该子任务并回到设计审查。
- 子任务间接口变化必须先更新父任务设计和所有受影响子任务计划，再恢复实施。
