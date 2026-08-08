# mise、uv 与开发任务环境设计

> **交付状态**：Implemented and locally verified / 已实施并完成 Linux x64 本地验证；其他原生平台待远程验证
> **关联决策**：6–12、20–34、57–80、115
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 管理链路

```text
standard version files
├─ .node-version = 24.19.0
├─ package.json#packageManager = pnpm@10.12.3
├─ rust-toolchain.toml = 1.97.1 + rustfmt + clippy
├─ mise.toml = uv@latest + task includes
└─ .python-version = 3.14.7

mise (local orchestration)
├─ reads Node/pnpm/Rust idiomatic version files
├─ installs/activates uv
└─ exposes mise run <task>

uv
├─ only-managed Python 3.14.7
├─ .venv
├─ pyproject.toml
└─ uv.lock
```

Actions 不安装 mise；它读取同一 Node/Rust/pnpm 标准文件。contracts/frontend job 从 `mise.lock` 解析 uv 版本、执行 `uv sync --locked` 并验证 uv managed Python；不能用系统 Python 代替项目合同。

## 2. 已实施配置

```toml
# mise.toml — current contract excerpt
min_version = "2026.8.0"

[settings]
lockfile = true
task.run_auto_install = false
idiomatic_version_file_enable_tools = ["node", "pnpm", "rust"]
lockfile_platforms = [
  "linux-x64", "linux-arm64", "macos-x64", "macos-arm64",
  "windows-x64", "windows-arm64",
]

[tools]
uv = "latest"

[tool_alias]
pnpm = "github:pnpm/pnpm"
uv = "github:astral-sh/uv"

[task_config]
includes = [
  ".mise/tasks/core.toml",
  ".mise/tasks/frontend.toml",
  ".mise/tasks/rust.toml",
  ".mise/tasks/python.toml",
  ".mise/tasks/trellis.toml",
  ".mise/tasks/upstream.toml",
  ".mise/tasks/contracts.toml",
  ".mise/tasks/release.toml",
  ".mise/tasks/hooks.toml",
]
```

不得在 `mise.toml` 重复 Node、pnpm、Rust 或 Python 版本。`uv=latest` 的团队实际版本由 `mise.lock` 固定；正常 bootstrap 不 bump lock。

`task.run_auto_install=false` 是已实施的 D115 覆盖：它只覆盖历史 D23 的 task 隐式安装部分，不把 mise 的全局工具能力改成禁用。工具与依赖准备由 `mise run bootstrap` 内显式 `mise install --locked`、frozen pnpm install 和 locked uv sync 完成；普通 task 和只读检查不会在执行途中悄悄安装工具。该边界已由 task metadata/side-effect 合同和本地 bootstrap 验证，不是 Release NO-GO。

```toml
# pyproject.toml — current contract excerpt
[project]
name = "fyagent-development-environment"
version = "0.0.0"
requires-python = ">=3.14,<3.15"
dependencies = []

[dependency-groups]
dev = []

[tool.uv]
package = false
python-preference = "only-managed"
python-downloads = "automatic"
```

`.python-version`：

```text
3.14.7
```

## 3. 初始化和检查

### 标准流程

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

### bootstrap 顺序

1. `bootstrap` 显式执行 `mise install --locked`，按已提交 lock 准备缺失工具，不依赖 task auto-install；
2. `pnpm install --frozen-lockfile`；
3. `uv sync --locked`；
4. `mise run env:check`；
5. `mise run tasks:validate` 与项目 task 合同检查。

bootstrap 不 trust、不安装系统包、不改 Git、不 bump lock、不 build/release。

### env:check

strict、只读，验证：版本事实源、实际版本、mise/uv/Python 归属、`.venv`、lock 一致性、Rust components、无 cross targets/llvm-tools、任务定义和 Codex hooks。输出人类摘要和 JSON；偏差非零退出。

### system:check

按宿主检查 Tauri 原生依赖并给出官方安装说明，不提权安装。Linux/Windows/macOS 规则分开维护。

## 4. Python 与临时依赖

- 项目/Trellis 的可重复依赖：`mise run python:add:dev -- "<requirement>"` 默认预演，审阅后追加 `--apply`；删除使用 `mise run python:remove:dev -- <package>` 的同一预演/应用边界，并提交 `pyproject.toml` 和 `uv.lock`；
- 一次性库：`mise run python:with -- "<requirement>" <command> [args...]`；
- 一次性 CLI：`mise run python:tool -- <tool> [args...]`；锁定项目环境中的任意命令使用 `mise run python:run -- <command> [args...]`；
- 禁止活动文档指导 `pip install` 或直接 `uv pip install` 污染 `.venv`；
- 普通 Trellis task 使用 `uv run --locked`，可在缺环境时同步；
- Codex hook 使用 `uv run --locked --no-sync --offline`，不得下载或修改环境。

## 5. task 组织

```text
mise.toml
.mise/tasks/
├─ core.toml
├─ frontend.toml
├─ rust.toml
├─ python.toml
├─ trellis.toml
├─ upstream.toml
├─ contracts.toml
├─ release.toml
└─ hooks.toml
scripts/tasks/
├─ toolchain-check.mjs
├─ lockfile-check.mjs
├─ task-contract-check.mjs
├─ docs-contract-check.mjs
├─ upstream-check.mjs
└─ ...
```

简单叶子 task 包装 package scripts/Cargo；复杂跨平台逻辑使用 Node `.mjs`，避免核心逻辑依赖 Bash。

## 6. 规范 task 目录

| 域                 | 规范 task                                                                                                                                                             |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 初始化             | `bootstrap`、`deps:install`、`env:check`、`system:check`                                                                                                              |
| 开发/构建          | `dev`、`dev:renderer`、`build`、`build:binary`、`build:debug`、`build:renderer`                                                                                       |
| 前端               | `typecheck`、`format`、`format:check`、`test:*`                                                                                                                       |
| Rust               | `rust:fmt`、`rust:fmt:check`、`rust:check`、`rust:clippy`、`rust:test`                                                                                                |
| 聚合               | `check`、`check:frontend`、`check:backend`、`check:contracts`                                                                                                         |
| Python             | `python:sync`、`python:lock`、`python:lock:check`、`python:check`、`python:add:dev`、`python:remove:dev`、`python:update`、`python:with`、`python:run`、`python:tool` |
| Trellis            | `trellis:init-developer`、`trellis:context`、`trellis:task`、`trellis:session:add`、`trellis:validate`                                                                |
| Hooks              | `codex:hook:workflow-state`、`codex:hook:subagent-context`、`codex:hooks:check`                                                                                       |
| 上游               | `upstream:check`、`upstream:fetch`、`upstream:audit`、`upstream:merge:prepare`、`upstream:merge:abort`                                                                |
| 维护               | `deps:update:*`、`toolchain:update:*`、`clean:*`、`tasks:validate`、`tasks:docs:*`                                                                                    |
| 版本/资产/发布合同 | `version:*`、`assets:icons`、`release:check`                                                                                                                          |

完整的 80-task catalog 已由真实 task metadata 生成到 `docs/fyagent/development/mise-tasks.md`，并由 `mise run tasks:docs:check` 做逐字节漂移检查。冻结 overlay 中的旧 catalog 不再应用。

## 7. 参数与副作用

- 有参数 task 必须定义 `usage`；包名、SemVer、枚举值先验证；
- `version:set`/`version:bump` 默认 dry-run，`--apply` 才写；
- 依赖全量更新必须 `--all` 并确认；不提供跨生态 `deps:update:all`；
- `upstream:merge:prepare` 确认后最多执行 `git merge --no-ff --no-commit`；
- `clean:all` 确认并验证路径在仓库根内；不删 lock、Git、Trellis 状态或用户数据；
- `release:check` 只读，不创建 tag/资产/Release。

## 8. 本地 check 与 Actions

`mise run check` 在当前宿主复用同一脚本/合同，但不能证明其它 OS。建议组成：

```text
check
├─ env:check
├─ check:frontend
├─ check:backend
└─ check:contracts
```

前端独立检查可并行；Cargo fmt/check/clippy/test 顺序执行；交互和修改型 task 不进入 check。

## 9. lockfile 治理

- `mise.lock`：结构化验证 Node/pnpm/Rust/uv 的预期条目、平台、URL、checksum/options；无旧 Rust options/cross targets；
- `uv.lock`：`uv lock --check`、`uv sync --locked`；
- `pnpm-lock.yaml`：普通安装 frozen；
- `Cargo.lock`：检查/测试使用 `--locked`；
- 所有升级通过受控 PR，不在 bootstrap 自动发生。

## 10. 实施证据与剩余门禁

实现 commit 为 `3d534710307d538e570c137231b1d80a64ac8ab7`；`mise run bootstrap`、`mise run check`、80-task 合同、lock 二次生成稳定性、hooks 模拟、version `0.3.0` 和 Linux x64 managed-toolchain 检查已通过。`mise.lock` SHA-256 为 `5f0d9df527ec1fdaf5532726ba30d330c74872786ad0380783064a36ceeefd9d`，解析 uv `0.12.2`。

Windows x64/ARM64、Linux ARM64 和 macOS 的 native setup/env/hooks/task 证据仍待 Actions；实际 `v0.3.0` tag 与正式 Release 也未发生。因此 Child 3 保持 `in_progress`，不能从 lock 平台条目推断远程执行成功。
