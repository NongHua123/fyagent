# mise、uv 与开发任务环境设计

> **交付状态**：Implemented and native-remote verified / 已实施并完成 Linux x64 本地验证及匹配 runner 的原生远程验证
> **关联决策**：6–12、20–34、57–80、115–117
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
├─ host-native.mjs
├─ toolchain-check.mjs
├─ lockfile-check.mjs
├─ task-contract-check.mjs
├─ docs-contract-check.mjs
├─ upstream-check.mjs
└─ ...
```

简单无目标叶子 task 可直接包装 package scripts/Cargo；Tauri dev/build 与 Cargo check/clippy/test 必须先经过 `host-native.mjs` 运行时边界。复杂跨平台逻辑使用 Node `.mjs`，避免核心逻辑依赖 Bash。

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
- `dev`/`build`/`build:binary`/`build:debug` 是固定操作，不接受任何 forwarded argv；`rust:test` 只接受一个非 option 测试名过滤器；
- caller target/compiler/rustdoc/wrapper/runner/linker env、任意 target-specific Rust/rustdoc flags、普通/build/encoded flags 中的 `--target`，以及 `LD_PRELOAD`、DYLD loader 路径、`NODE_OPTIONS`/`NODE_PATH` 等 loader/runtime injection 控制（均含大小写变体）在启动 rustc/rustdoc/Cargo/Tauri 前失败关闭。

## 8. 本地 check 与 Actions

`mise run check` 在当前宿主复用同一脚本/合同，但不能证明其它 OS 或架构。`pnpm dev`/`pnpm build` 及 canonical `mise run dev`、`build*`、`rust:check`、`rust:clippy`、`rust:test` 都先把 `process.platform`/`process.arch` 严格映射到六个受支持宿主之一，把 PATH 中的 rustc/rustdoc 解析为绝对路径并要求两者 `-vV` 的 host/release/commit 与当前宿主一致。随后 child env 显式 own 两个工具、清空 compiler wrapper 与 Rust/rustdoc flag 来源，并给 Tauri/Cargo 传 `--target <current-host>`；在工具链启动前还会遍历仓库、祖先目录、Cargo home 及递归 include 的有效 Cargo config，拒绝 build target/compiler/rustdoc/wrapper/flags 和 target runner/linker/flags，也拒绝 required-missing include、symlink 与 include cycle。Cargo config `[env]` 中的对应 protected key 按大小写无关规则统一拒绝，不允许 string/table/force 形式从 include 恢复已清空控制。Cargo test 另外通过 CLI TOML array 注入当前 target 的原生直通 runner，数组元素是当前绝对 Node、同一 `host-native.mjs`、固定子命令和 host target，含空格路径仍保持独立 argv。Cargo 追加 binary/filter argv 后，runner 验证 host target、仓库 target 路径边界、regular non-symlink、原生格式以及 ELF `e_machine`、PE `Machine` 或 thin Mach-O `cputype` 与目标完全一致，再使用 `spawnSync(..., shell:false)` 直接执行；不使用 cmd/PowerShell/Bash 等 shell。canonical task 不保留 caller 的安全自定义 flags。所有标准本地 dev/build/test/package/verify 命令只能使用实际宿主 OS/架构；不得通过 target 参数、linker、WSL/子系统桥接、外来可执行文件、模拟器、复制工具链或本地暂存的非宿主产物改变证据归属。建议组成：

```text
check
├─ host-native guard（无子进程）
├─ env:check
├─ check:frontend
├─ check:backend
└─ check:contracts
```

前端独立检查可并行；Cargo fmt/check/clippy/test 顺序执行；交互和修改型 task 不进入 check。
`check` 的第一步是无子进程的 host-native guard，在 `env:check` 探测 rustc
之前拒绝 caller compiler/wrapper/runner/linker/target environment 与 target-bearing
flags；后续 Rust wrapper 仍独立核验绝对 rustc/rustdoc 身份并固定 Cargo env。

低层 `pnpm tauri` 仅保留给经过审阅、刻意绕开本地 task API 的 Actions/维护命令，不是本地标准入口。合同不声称能拦截任意手写 `cargo`、`rustc` 或 `pnpm tauri` 命令；这类命令的输出也不能作为项目验收证据。

纯逻辑/可移植测试可以在当前宿主验证合同，但不能升级为另一平台的原生结果。Windows、macOS、ARM64 及其他非宿主验证由匹配的 GitHub Actions native runner 独占。

## 9. lockfile 治理

- `mise.lock`：结构化验证 Node/pnpm/Rust/uv 的预期条目、平台、URL、checksum/options；无旧 Rust options/cross targets；
- `uv.lock`：`uv lock --check`、`uv sync --locked`；
- `pnpm-lock.yaml`：普通安装 frozen；
- `Cargo.lock`：检查/测试使用 `--locked`；
- 所有升级通过受控 PR，不在 bootstrap 自动发生。

## 10. 实施证据与剩余门禁

基础实现 commit 为 `3d534710307d538e570c137231b1d80a64ac8ab7`；`mise run bootstrap`、`mise run check`、80-task 合同、lock 二次生成稳定性、hooks 模拟、version `0.3.0` 和 Linux x64 managed-toolchain 检查已通过。D116 runtime hardening 由 `a1c1238c4f7ec8f80238edfb2618823bcedf49f5` 补充共享 host planner、package/mise wrapper 路由、caller env/argv 拒绝和真实 wrapper smoke。`mise.lock` SHA-256 为 `5f0d9df527ec1fdaf5532726ba30d330c74872786ad0380783064a36ceeefd9d`，解析 uv `0.12.2`。

D116 收口时已停止为本地 Windows 诊断启动的 cargo/rustc 进程，删除显式诊断临时目录，并清理 `src-tauri/target/app`（清理前 4.1 GiB）与 `target/installer-actions`（清理前 57.4 MiB）。当前复核确认两个目录均不存在，`rustup target list --installed` 只有 `x86_64-unknown-linux-gnu`。此前本地 Windows Light/MSI 输出仅是诊断信息，不是 Windows 原生验收。

PR #7、exact-main Required CI `31259389682`、五 target preflight
`31259905022`、annotated `v0.3.0` 和 formal Release run `31260931509`
已经证明实现与原生打包合同。closeout [PR #8](https://github.com/NongHua123/fyagent/pull/8)
补齐 Windows x64/ARM64 Required smoke：锁定 uv/Python、执行
`uv sync --locked --managed-python`、校验 Python native platform，并通过
managed Python 调用 Trellis JSON task list。首次 run `31264604075` 的 x64 job
`93120609402` 成功，但 version-only `setup-uv` 在 Windows on ARM 选择
`win-amd64`，ARM64 job `93120609411` 和 Required `93121912798` 因此失败。
commit `4645668d5860cb67f2ae70a3a2eba1fc9afe6ecd` 改用 uv 官方完整
`implementation-version-os-arch-libc` request 并强制 managed Python；修复后的
run `31265504901` 中 x64 `93122857985`、ARM64 `93122858012` 与 Required
`93123992476` 均成功。因此 Child 3/D116 原生 smoke 已远程验证；task 仍因后续
manifest/archive/journal 门禁保持 `in_progress`。

## 11. Actions 运行观察纪律

D117 规定：仅在触发已单独获授权后，由发起主流程同步等待整次 Actions run 到 `completed`；不得启动后台/异步监控代理，也不得反复执行 run/job/check 状态查询。等待结束后只读取一次最终 run/job 结果；成功时不批量抓日志，失败时才获取一次失败 job 日志。PR #7、exact-main、preflight、formal Release，以及 closeout PR 的首次失败 run `31264604075` 和修复成功 run `31265504901` 均按该纪律观察。该观察流程本身不扩张 rerun、cancel、tag 或 publish 权限。
