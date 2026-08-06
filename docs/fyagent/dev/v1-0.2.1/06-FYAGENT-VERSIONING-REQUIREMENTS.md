# 06 — FyAgent 全局应用版本治理需求

## 1. 目的

本文把“版本号”限定为 **FyAgent 应用版本**，并定义其跨平台语义、递增规则、受控范围和操作契约。

这里的版本不是 Windows 安装器的独立版本，也不是某个前端包、Rust crate、数据库、配置格式或外部工具的版本。一个正式发布提交只有一个 FyAgent 应用版本，并由 Windows、macOS、Linux、运行时接口、发布标签、下载清单和安装包元数据共同使用。

## 2. 术语

| 术语 | 定义 |
|---|---|
| `APP_VERSION` | 不带前导 `v` 的 FyAgent 应用版本，例如 `0.2.1` |
| `RELEASE_TAG` | 与应用版本严格对应的 Git 标签，例如 `v0.2.1` |
| 单一真源 | 唯一允许人工修改 FyAgent 当前应用版本的位置 |
| 投影值 | 由单一真源自动继承、生成、读取或校验的版本值 |
| 历史版本文本 | CHANGELOG、旧发布说明、迁移文档、测试夹具等用于描述过去版本的值 |
| 依赖版本 | Cargo、npm、系统组件或工具链依赖的版本，不属于 `APP_VERSION` |

## 3. 全局版本语义

### 3.1 一次发布，一个版本

同一 Git commit 构建的正式产物必须共享同一个 `APP_VERSION`：

```text
FyAgent Windows x64       0.2.1
FyAgent Windows ARM64     0.2.1
FyAgent macOS             0.2.1
FyAgent Linux x86_64      0.2.1
FyAgent Linux ARM64       0.2.1
运行时 getVersion()       0.2.1
Git release tag           v0.2.1
```

不允许出现以下状态：

```text
Windows = 0.2.1
macOS   = 0.2.0
Linux   = 0.2.0
```

也不允许为安装器动作 DLL、下载 manifest 或某个平台单独维护一个“同步更新”的 FyAgent 版本字段。

### 3.2 当前发布策略

`0.2.1` 采用稳定三段式 SemVer：

```text
MAJOR.MINOR.PATCH
```

当前脚本只接受：

```regex
^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$
```

因此以下值无效：

```text
v0.2.1
0.2
0.2.1-beta.1
0.2.1+build.4
00.2.1
```

限制为稳定版本是有意设计：Windows Installer 的产品版本只有三个数值字段，当前项目又没有定义预发布通道、升级排序、签名清单和自动更新兼容规则。未来需要 alpha/beta/rc 时，应单独建立预发布版本设计，而不是放宽现有脚本正则。

## 4. 递增规则

### 4.1 PATCH

递增 `PATCH` 适用于向后兼容的修复和小范围工程改进，例如：

- 修复安装器脚本错误；
- 修复崩溃、错误提示或兼容性问题；
- 不改变用户已有数据语义的内部重构；
- 不破坏既有 CLI、配置或存储兼容性的安全加固。

本次从 `0.2.0` 到 `0.2.1` 属于 PATCH：它修复 MSI 安装，并收敛版本维护方式，不改变 FyAgent 面向用户的核心兼容契约。

### 4.2 MINOR

递增 `MINOR` 适用于向后兼容的新能力，例如：

- 新增可选功能；
- 新增向后兼容的配置字段；
- 新增平台支持；
- 新增可回退且不破坏旧数据的协议能力。

递增后 PATCH 归零：

```text
0.2.9 -> 0.3.0
```

### 4.3 MAJOR

递增 `MAJOR` 适用于已确认的破坏性变化，例如：

- 无法自动迁移的配置/数据格式变更；
- 移除既有兼容能力；
- 改变公开协议或 CLI 契约；
- 安装范围、身份标识或更新通道发生不兼容迁移。

递增后 MINOR、PATCH 归零：

```text
0.9.8 -> 1.0.0
1.7.3 -> 2.0.0
```

### 4.4 决策与机械操作分离

版本脚本只负责机械更新，不负责判断应递增哪一段。负责人或发布变更说明先确定目标版本，再执行：

```bash
pnpm run version:set -- 0.2.1
```

`version:bump` 仅作为明确选择后的快捷方式：

```bash
pnpm run version:bump -- patch
```

## 5. 单一真源与投影模型

### 5.1 唯一人工维护位置

单一真源固定为：

```toml
# src-tauri/Cargo.toml
[workspace.package]
version = "0.2.1"
```

仓库审查规则：当前 FyAgent 应用版本的人工维护字面量只能出现在这一处。

### 5.2 直接继承

以下本地 Rust 包直接继承 workspace 版本：

```toml
[package]
name = "fyagent"
version.workspace = true
```

```toml
[package]
name = "fyagent-installer-actions"
version.workspace = true
```

### 5.3 Tauri 投影

`src-tauri/tauri.conf.json` 删除顶层 `version`。Tauri 在该字段缺省时读取 Cargo 包版本，因此 Windows MSI、macOS bundle、Linux package 和运行时版本 API 都由同一 Cargo 值派生。

不得在以下位置重新加入应用版本：

- `tauri.conf.json`；
- 平台覆盖配置；
- WiX 模板硬编码；
- shell/PowerShell 脚本常量；
- GitHub Actions `env` 常量。

### 5.4 npm 元数据

该仓库不是要发布到 npm registry 的 JavaScript 包。`package.json` 删除 `version` 并设置：

```json
{
  "private": true
}
```

这样 `package.json` 只承担 Node 工具、脚本和依赖清单职责，不再充当第二应用版本源。

### 5.5 Cargo.lock

`src-tauri/Cargo.lock` 中本地 workspace 包版本是生成投影，不是决策源。版本脚本只允许精确更新：

```text
name = "fyagent"
name = "fyagent-installer-actions"
```

对应包块的 `version` 字段。即使某个依赖恰好也是 `0.2.0`，也不得修改。

## 6. 纳入与排除范围

### 6.1 必须由 `APP_VERSION` 驱动

| 对象 | 方式 |
|---|---|
| 主 Rust package | workspace 继承 |
| 安装器动作 crate | workspace 继承 |
| Tauri runtime version | Cargo/Tauri 继承 |
| MSI ProductVersion | Tauri/WiX 构建投影 |
| Windows EXE Product/File version | 构建投影并验收 |
| macOS `CFBundleShortVersionString` | Tauri bundle 投影 |
| DEB/RPM/AppImage 元数据或资产名 | 构建投影/脚本读取 |
| 发布标签 | `v${APP_VERSION}` |
| 发布资产名 | `${APP_VERSION}`，不带 `v` |
| 下载 manifest | `version=${APP_VERSION}`、`tag=v${APP_VERSION}` |
| 版本显示与诊断 | 运行时 API/构建信息读取 |

### 6.2 明确不纳入

以下值即使长得像 `X.Y.Z`，也不得由脚本替换：

- `@tauri-apps/*`、React、Vitest 等 npm 依赖版本；
- `tauri`、`serde`、`windows` 等 Cargo 依赖版本；
- `packageManager: pnpm@...`；
- Rust `rust-version`；
- Node、pnpm、WiX、WebView2、mise、rcodesign 版本；
- 数据库 schema 版本；
- 配置文件格式版本；
- IPC/API/DTO/协议版本；
- 外部 Claude、Codex、Gemini 模型或 CLI 版本；
- 旧版本迁移测试中的 `0.2.0`；
- CHANGELOG、旧发布说明、归档文档；
- 文档中的示例版本，除非本次需求明确要求更新该示例。

该边界是禁止“全仓库搜索替换”的核心原因。

## 7. 操作需求

### 7.1 标准命令

迁移完成后，唯一受支持的版本操作是：

```bash
pnpm run version:get
pnpm run version:check
pnpm run version:set -- X.Y.Z
pnpm run version:bump -- patch|minor|major
```

### 7.2 `version:get`

- 读取 `[workspace.package].version`；
- 验证格式与 MSI 数值范围；
- 仅在 stdout 输出纯版本值，便于脚本捕获；
- 不修改文件。

### 7.3 `version:check`

至少校验：

- 单一真源存在且仅有一个字面量；
- 主 crate 和可选安装器动作 crate 均继承 workspace；
- `package.json.version` 不存在且 `private=true`；
- `tauri.conf.json.version` 不存在；
- 本地 Cargo.lock 包版本一致；
- 可选 `--tag` 与 `v${APP_VERSION}` 精确相等。

### 7.4 `version:set`

- 接受一个目标 `X.Y.Z`；
- 默认只修改 Cargo 单一真源和本地 Cargo.lock 投影；
- 写入前完成结构预检；
- 写入后再次执行完整契约校验；
- 失败时回滚；
- 相同版本时幂等成功；
- 支持 `--dry-run`；
- 不运行 Git 命令。

### 7.5 `version:bump`

- 先执行 `version:check`；
- 按 SemVer 算术计算下一个稳定版本；
- 复用 `version:set` 的写入和回滚路径；
- 超出 MSI 数值上限时失败。

## 8. Windows Installer 数值约束

Windows Installer `ProductVersion` 使用 `major.minor.build`，上限为：

```text
major <= 255
minor <= 255
patch/build <= 65535
```

因此全局 FyAgent 稳定版本必须同时满足该范围。这里选择让所有平台共享 MSI 可表达的版本域，而不是为 Windows 再设计一个映射版本，以避免升级、日志、资产和支持信息出现双版本。

当未来产品接近这些上限时，应先设计新的产品版本映射或安装技术；当前脚本直接拒绝越界值。

## 9. 状态不变量

任意可发布提交必须满足：

```text
canonical Cargo version
  == main crate version
  == installer-actions crate version
  == local Cargo.lock package versions
  == Tauri resolved app version
  == all embedded package versions
  == release asset filename version
  == download manifest version
  == release tag without leading v
```

另外必须满足：

```text
package.json.version is absent
tauri.conf.json.version is absent
release tag == "v" + canonical version
```

## 10. 从 0.2.0 迁移到 0.2.1

迁移不是简单运行脚本，因为旧仓库尚未建立目标结构。首次实施顺序固定为：

1. 在 `src-tauri/Cargo.toml` 建立 workspace 和 `[workspace.package] version = "0.2.1"`；
2. 主 crate 改为 `version.workspace = true`；
3. 新增安装器动作 crate，并同样继承 workspace；
4. 删除 `tauri.conf.json.version`；
5. 删除 `package.json.version`，增加 `private` 和四个版本脚本命令；
6. 生成/更新 Cargo.lock，使两个本地包为 `0.2.1`；
7. 删除旧的三源一致性测试和硬编码 `FYAGENT_V1_0_2_VERSION`；
8. 落库 `scripts/version.mjs` 和新测试；
9. 执行 `pnpm run version:check`；
10. 改造所有构建/发布脚本只读取 `version:get` 或 CI 输出 `APP_VERSION`。

迁移完成后，后续版本更新不得再重复以上结构改造，只运行标准命令。

## 11. 验收标准

- 仓库中当前 FyAgent 应用版本只有一个人工维护字面量；
- `pnpm run version:set -- 0.2.2` 只改变预期的 Cargo 文件内容；
- 恰好为 `0.2.1` 的依赖版本、旧发布文本和测试夹具保持不变；
- 无效、预发布、带 `v` 和越界版本均失败；
- 写入中途失败可恢复原文件；
- 标签 `v0.2.1` 通过，`0.2.1`、`v0.2.2` 均失败；
- Windows、macOS、Linux 正式产物内嵌版本一致；
- Codex 接到“更新 FyAgent 版本号为 X.Y.Z”时无需搜索版本字段，只执行 runbook 指定命令并报告 diff/校验结果。
