# 07 — 单一真源与一键版本脚本设计

## 1. 设计目标

本设计解决当前版本号分散在 `package.json`、`tauri.conf.json`、`Cargo.toml`、`Cargo.lock`、测试常量和构建脚本中的问题，并确保后续自动化代理只需执行一个受控命令。

核心原则：

1. **一个决策源**：只在 Cargo workspace 根清单维护 FyAgent 当前应用版本；
2. **继承优先**：能由工具原生继承的字段不复制；
3. **读取优先**：构建脚本读取单一真源，不各自解析不同文件；
4. **精确修改**：脚本按结构和包名定位，不按数字搜索替换；
5. **失败可恢复**：结构异常、写入失败、后验校验失败均不留下半更新状态；
6. **发布解耦**：版本更新不自动提交、打标签或发布。

## 2. 目标仓库结构

```text
package.json
scripts/
  version.mjs
src-tauri/
  Cargo.toml                 # workspace 根 + 唯一版本字面量
  Cargo.lock                 # 生成投影
  tauri.conf.json            # 不含 version
  installer-actions/
    Cargo.toml               # version.workspace = true
tests/
  version.test.mjs
  versionConsistency.test.ts # 删除或重写为契约入口测试
```

## 3. Cargo 单一真源

### 3.1 根清单

目标片段：

```toml
[workspace]
members = [".", "installer-actions"]
resolver = "2"

[workspace.package]
version = "0.2.1"

[package]
name = "fyagent"
version.workspace = true
description = "All-in-One Assistant for Claude Code, Codex & Gemini CLI"
authors = ["Jason Young"]
license = "MIT"
edition = "2021"
rust-version = "1.85.0"
```

现有 `src-tauri/Cargo.toml` 同时作为 workspace 根和根 package 是 Cargo 支持的结构。`installer-actions` 作为 member 共享锁文件、目标目录和 workspace package metadata。

`resolver = "2"` 保持与当前 edition/依赖行为兼容；不在本次顺带升级到 resolver 3，以免引入与安装器无关的依赖解析变化。

### 3.2 安装器动作清单

```toml
[package]
name = "fyagent-installer-actions"
version.workspace = true
edition = "2021"
publish = false
```

`publish = false` 防止误发 crate。动作 DLL 版本与 FyAgent 全局版本一致，但它不是独立发布产品。

### 3.3 为什么不以 `package.json` 为真源

- FyAgent 的安装包、原生二进制和运行时版本由 Tauri/Cargo 驱动；
- 项目不发布为 npm 包；
- 以 npm 字段为真源仍需要把值同步到 Cargo，产生第二套更新逻辑；
- `package.json` 中大量依赖版本增加误替换风险。

### 3.4 为什么不以 `tauri.conf.json` 为真源

Tauri 可以在缺省配置版本时使用 Cargo package 版本。保留 `tauri.conf.json.version` 会继续形成重复字段，而且新建原生 workspace member 时仍需另行同步。

Cargo workspace inheritance 更适合同时覆盖主应用和安装器动作 crate。

## 4. package.json 设计

删除：

```json
"version": "0.2.0"
```

增加：

```json
"private": true,
"scripts": {
  "version:get": "node scripts/version.mjs get",
  "version:check": "node scripts/version.mjs check",
  "version:set": "node scripts/version.mjs set",
  "version:bump": "node scripts/version.mjs bump"
}
```

现有脚本原样保留。建议把版本脚本放在靠近 `tauri`/`build` 的位置，便于维护，但 JSON 键顺序不是契约。

`pnpm-lock.yaml` 不保存根应用版本，因此本次不应因 FyAgent 版本更新而修改它。若 pnpm 命令造成无关 lockfile diff，应回退该 diff 并检查命令环境。

## 5. Tauri 配置设计

从 `src-tauri/tauri.conf.json` 删除：

```json
"version": "0.2.0"
```

保留 `productName`、`identifier` 和 bundle 配置。构建前由 `version:check` 确认该字段没有被重新引入。

平台配置文件如果未来出现，同样不得写 FyAgent 应用版本；必须沿用 Cargo/Tauri 解析结果。

## 6. `scripts/version.mjs` 命令接口

### 6.1 接口

```text
node scripts/version.mjs get
node scripts/version.mjs check [--tag vX.Y.Z]
node scripts/version.mjs set X.Y.Z [--dry-run]
node scripts/version.mjs bump patch|minor|major [--dry-run]
```

所有错误写入 stderr，并以非零状态退出。成功时输出可读结果；`get` 的 stdout 仅输出版本。

### 6.2 仅依赖 Node 标准库

脚本使用：

```text
node:fs
node:path
node:process
node:url
```

不新增 npm runtime dependency，不要求安装 TOML 库。选择受限的结构解析器，而不是通用正则替换：

- 精确找到 `[workspace.package]`；
- 要求其中恰好一个字面量 `version = "..."`；
- 精确找到各清单 `[package]` 中 `version.workspace = true`；
- 按 `[[package]]` 块和 `name` 解析 Cargo.lock；
- JSON 使用 `JSON.parse`。

如果仓库结构偏离契约，脚本失败并要求先人工设计迁移，而不是猜测。

## 7. 读写流程

### 7.1 `get`

```text
读取 src-tauri/Cargo.toml
  ↓
定位 [workspace.package]
  ↓
要求恰好一个 version 字面量
  ↓
验证稳定 SemVer + MSI 数值上限
  ↓
输出 APP_VERSION
```

### 7.2 `check`

```text
读取四类文件
  ├─ Cargo.toml
  ├─ Cargo.lock
  ├─ package.json
  └─ tauri.conf.json
  ↓
验证单一真源和继承结构
  ↓
验证本地 lock 包版本
  ↓
可选验证 release tag
  ↓
统一报告所有契约错误
```

`check` 应尽量聚合多个错误，让 Codex 一次修完结构问题；文件无法读取、JSON 无法解析、TOML 目标段缺失等基础错误可立即终止。

### 7.3 `set`

```text
验证目标版本
  ↓
读取并保存原文件
  ↓
执行结构预检
  ↓
在内存中生成新 Cargo.toml / Cargo.lock
  ↓
--dry-run ? 只报告 : 写入两个文件
  ↓
重新执行完整 check
  ↓
失败则恢复原文件；成功则报告 diff 范围
```

### 7.4 `bump`

```text
check 当前契约
  ↓
读取当前版本
  ↓
按 patch/minor/major 计算目标
  ↓
调用 set
```

## 8. Cargo.lock 精确更新

### 8.1 允许更新的包名

固定 allowlist：

```js
const LOCAL_CARGO_PACKAGES = [
  "fyagent",
  "fyagent-installer-actions"
];
```

脚本逐个解析 `[[package]]`，只在包名完全匹配时改该块唯一的 `version` 行。

### 8.2 必须拒绝的异常

- `fyagent` 包块不存在；
- 同名本地包块出现两次；
- 安装器动作 manifest 已存在但 lock 中无该包；
- 目标包块没有或有多个 `version` 行；
- Cargo.lock 结构无法识别。

### 8.3 为什么不运行全量依赖更新

版本迭代不应顺带改变依赖解析。自动执行 `cargo update` 可能在约束允许范围内选择新的依赖版本，扩大 diff 和回归面。因此参考脚本只更新本地包投影；CI 再用 Cargo 命令验证 lockfile 可用。

如果未来 Cargo.lock 格式发生不兼容变化，应升级脚本和测试，不得降级为数字替换。

## 9. 原子性与回滚

### 9.1 写入集合

常规 `set` 最多写入：

```text
src-tauri/Cargo.toml
src-tauri/Cargo.lock
```

脚本在写入前把原始文本保存在内存。任一写入失败时按反序恢复已经写过的文件。

### 9.2 后验失败

即使写入成功，只要后续 `check` 失败，也必须把两个文件恢复到原始内容，再返回失败。

### 9.3 并发边界

脚本不支持两个进程并发更新同一工作树。CI 和 Codex 必须串行运行；发布工作流只做 `check`，不在构建机自动改版本。

如未来需要强并发保证，可增加仓库锁文件和临时文件原子 rename，但不属于 `0.2.1` 必要范围。

## 10. 输出规范

### 10.1 成功设置

```text
0.2.0 -> 0.2.1
updated src-tauri/Cargo.toml
updated src-tauri/Cargo.lock
```

### 10.2 幂等设置

```text
0.2.1 -> 0.2.1
version already matched; no files changed
```

### 10.3 Dry run

```text
0.2.0 -> 0.2.1
would update src-tauri/Cargo.toml
would update src-tauri/Cargo.lock
```

### 10.4 契约失败

```text
[fyagent-version] version contract failed:
  - package.json must not declare the FyAgent application version
  - release tag must be v0.2.1; received "v0.2.0"
```

输出中不得包含秘密；文件路径使用仓库相对路径。

## 11. 首次迁移算法

参考脚本假设目标结构已经建立，因此首次改造由实现提交完成：

1. 编辑根 Cargo manifest，建立 workspace 与继承；
2. 创建 `installer-actions` manifest；
3. 运行 Cargo metadata/check，让 Cargo.lock 出现两个本地包；
4. 迁移到 `0.2.1`；
5. 删除 npm/Tauri 重复版本；
6. 安装 `scripts/version.mjs`；
7. 运行 `node scripts/version.mjs check`；
8. 把旧 `versionConsistency.test.ts` 改成调用脚本或删除其硬编码断言；
9. 执行参考单元测试和项目测试。

首次迁移完成后，脚本成为唯一写入口。

## 12. 测试设计

参考测试至少覆盖：

1. `set` 更新单一真源和两个本地 lock 包；
2. 相同数字的依赖版本不变；
3. `check --tag` 精确匹配；
4. 拒绝 `v` 前缀、prerelease 和 build metadata；
5. `--dry-run` 不修改文件；
6. `bump` 的三种算术；
7. 超出 MSI 上限；
8. 缺失 workspace 段；
9. `package.json.version` 被重新加入；
10. `tauri.conf.json.version` 被重新加入；
11. helper manifest 与 Cargo.lock 不一致；
12. 模拟第二个文件写入失败并验证回滚。

随包 `reference/tests/version.test.mjs` 提供 8 项可执行参考，覆盖精确更新、标签、非法版本、dry-run、读取、递增、MSI 上限和重复字段回归；落库时仍应扩充到上述完整矩阵。

## 13. CI 接入

所有 CI 工作在安装依赖和构建前执行：

```bash
pnpm run version:check
```

标签工作执行：

```bash
pnpm run version:check -- --tag "$GITHUB_REF_NAME"
```

随后只通过：

```bash
APP_VERSION="$(pnpm --silent run version:get)"
```

或 version-contract job output 获得版本。禁止再次从 `package.json`、`tauri.conf.json` 或标签中各自推导。

## 14. 参考实现状态

本交付包中的：

```text
reference/scripts/version.mjs
reference/tests/version.test.mjs
```

是可复制到目标仓库的参考实现。它在隔离 fixture 中验证核心更新、精确标签、非法版本和 dry-run。正式合并前仍需在 FyAgent 实际工作树中运行全部项目测试，并根据最终 Cargo workspace 名称确认 allowlist。
