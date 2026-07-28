# macOS Apple Silicon 实现规格

## 1. 范围

V1 正式支持：

- Apple Silicon；
- macOS 14 或目标 Bundle 更高最低要求；
- 官方 DMG；
- 首次安装、更新、启动；
- `/Applications` 优先，权限不足回退 `~/Applications`；
- 保留 DMG 内官方 `.app` 原始目录名；
- 按 Bundle ID/Team/signature 管理。

不支持：

- Intel；
- Beta；
- ChatGPT Classic 管理；
- Sparkle delta；
- 管理员 helper/密码弹窗；
- 修改 bundle、去 quarantine、重新签名。

## 2. 模块

```text
platform/macos/
├── mod.rs       adapter 装配、CommandRunner trait
├── bundle.rs    扫描、Info.plist、签名、架构、运行、launch
└── dmg.rs       mount、发现、目标计划、copy/swap、detach
```

所有系统命令通过 `CommandRunner` 抽象，测试使用 fake runner，不在测试机真实挂载/复制。

## 3. Stable 身份

硬门禁：

```text
CFBundleIdentifier == com.openai.codex
TeamIdentifier == 2DC432GLL2
CPU 包含 arm64
codesign 验证通过
spctl assessment accepted
```

拒绝：

```text
com.openai.chat            ChatGPT Classic
com.openai.codex.beta      Beta
其他 Bundle ID
Team 相同但 Bundle ID 不同
目录名匹配但身份不匹配
```

目录名和显示名只是外观。当前官方包可能是 `ChatGPT.app`，旧安装可能仍位于 `Codex.app`。

## 4. 本地扫描

只扫描：

```text
/Applications/*.app
~/Applications/*.app
```

“扫描”指直接一级候选，不递归整个 bundle 外的任意目录，不使用 Spotlight 全盘搜索。

对每个候选：

1. 用 `plutil` 或可靠 plist parser 读取 `Contents/Info.plist`；
2. 只对 `CFBundleIdentifier == com.openai.codex` 进一步深检；
3. 读取 `CFBundleVersion`、`CFBundleShortVersionString`、`CFBundleExecutable`、`LSMinimumSystemVersion`；
4. 获取 codesign TeamIdentifier；
5. 确认候选为顶层 `.app` 且 canonical path 仍位于两个标准目录。

结果：

| 数量 | 行为 |
|---|---|
| 0 Stable | NotInstalled |
| 1 Stable | 管理该实际路径，不主动改名或迁移 |
| >1 Stable | `MAC_MULTIPLE_INSTALLATIONS`，阻断更新和启动自动选择 |

`com.openai.chat` 不算 Stable，但用于目标路径冲突保护。

## 5. 运行状态检测

禁止：

```text
pgrep -x Codex
pgrep -x ChatGPT
按显示名猜进程
```

V1 可使用 macOS 自带 JXA/`osascript -l JavaScript` 调用 `NSWorkspace.sharedWorkspace.runningApplications`，输出 JSON，按以下信息匹配：

- `bundleIdentifier == com.openai.codex`；
- `bundleURL.path` canonical 后等于当前管理路径。

命令输出必须：

- 有超时；
- JSON parse；
- 不执行任何来自远程/用户的脚本内容；
- 脚本是内置常量；
- 无法确定时对更新 fail safe：视为不可安全更新并返回诊断，不假设未运行。

首次安装没有现有路径时无需运行检测。更新时发现运行，返回 `MAC_APP_RUNNING`，不自动 quit/kill。

## 6. DMG 挂载

命令：

```bash
hdiutil attach <dmg> -readonly -nobrowse -plist
```

要求：

- 参数独立传给 `Command`，不拼 shell 字符串；
- timeout；
- parse plist 输出获取 mount point；
- mount point canonical；
- 使用 RAII guard 确保最终 detach；
- DMG path 必须位于当前 job temp root；
- 禁止 `-noverify` 等降低验证的参数。

detach：

```bash
hdiutil detach <mount-point>
```

正常 detach 失败可尝试有限次数普通 detach；不默认 force detach。安装结果成功但 detach 失败时记录清理错误并向用户显示适当 warning/错误策略，至少保证日志可诊断。

## 7. DMG 内应用发现

在 mount root 中查找**恰好一个顶层 `.app`**：

- 不要求名为 `Codex.app`；
- 不递归选择多个候选之一；
- symlink 需解析并确保仍在 mount root；
- 0 个 → `MAC_APP_NOT_FOUND`；
- 多个 → `PACKAGE_PARSE_FAILED`/专用诊断；
- 读取 Bundle ID，必须 `com.openai.codex`。

记录 `source_bundle_name`，新安装目标使用该原始名称。

## 8. Bundle 验证

### 8.1 Info.plist

读取：

```text
CFBundleIdentifier
CFBundleVersion
CFBundleShortVersionString
CFBundleExecutable
LSMinimumSystemVersion
```

验证：

- Stable ID exact；
- bundle build 与 ReleaseDescriptor platform version 相符；
- short version用于展示；
- minimum OS 与当前系统比较；
- executable 是 bundle 内合法相对名称，但不作为身份。

### 8.2 架构

使用内置工具，例如：

```bash
lipo -archs <bundle>/Contents/MacOS/<executable>
```

要求包含 `arm64`。Universal 包可包含其他架构，但 V1 下载分支必须是 arm64 artifact。若 executable 不存在，parse failed。

### 8.3 代码签名

```bash
codesign --verify --deep --strict --verbose=2 <app>
codesign --display --verbose=4 <app>
```

从 display 输出提取 `TeamIdentifier`，必须 `2DC432GLL2`。不要仅依赖输出文本“valid on disk”；检查退出码。

### 8.4 Gatekeeper

```bash
spctl --assess --type execute --verbose=4 <app>
```

退出非 0 → `MAC_GATEKEEPER_REJECTED`。只对顶层 `.app` 执行。

### 8.5 禁止修改

不得：

- `xattr -d com.apple.quarantine`；
- 改 Info.plist；
- 替换图标；
- 改目录内资源；
- `codesign --force`；
- ad-hoc 签名；
- 重新打包 DMG。

## 9. 最低系统版本

预检：

```text
当前 CPU == arm64
当前 macOS >= 14.0
当前 macOS >= LSMinimumSystemVersion（如果包提供）
```

取更严格者。由于 Bundle minimum 在下载后才能读，远程 manifest 有字段时下载前预检一次；下载后再次以 Bundle 为权威。

## 10. 目标路径规划

### 10.1 已安装唯一 Stable

更新**原实际路径**：

```text
/Applications/Codex.app             → 更新这里
~/Applications/ChatGPT.app          → 更新这里
```

即使新 DMG 内名改变，也不迁移现有目录，不改变用户 Dock/路径语义。

### 10.2 全新安装

保留 DMG 内原名：

```text
source: ChatGPT.app
candidate 1: /Applications/ChatGPT.app
candidate 2: ~/Applications/ChatGPT.app
```

### 10.3 路径冲突

对 candidate：

- 不存在 → 可用；
- 存在且 Bundle ID `com.openai.codex` → 视为 Stable 检测遗漏/重新检测，不能盲覆盖；
- 存在且 `com.openai.chat` → 不覆盖；
- 存在且 beta/未知 → 不覆盖；
- `/Applications` 被非目标占用 → 尝试用户目录；
- 两个都冲突 → `MAC_TARGET_PATH_CONFLICT`。

“权限不足”和“身份冲突”不同：只有权限不足才正常 fallback；冲突也可按 D65 尝试第二路径，但必须在 UI/日志保留明确原因。

## 11. 权限与目录

不开发管理员 helper，不使用：

```text
osascript ... with administrator privileges
sudo
保存密码
```

路径策略：

1. 计划 `/Applications`；
2. 尝试创建同卷 staging/写入；
3. 若明确是 permission denied，计划 `~/Applications`；
4. 创建 `~/Applications`（必要时）；
5. 其他错误不误判为权限。

## 12. 安全复制与替换

使用 `ditto` 复制 bundle，目标卷 staging：

```text
<target-parent>/.fyagent-codex-install-<job-id>.app
```

流程：

1. 确保 staging 不存在；
2. `ditto <mounted-source.app> <staging.app>`；
3. 对 staging 重新执行 Bundle ID、Team、架构、codesign、spctl；
4. 若首次安装，atomic rename staging → target；
5. 若更新：
   - 再次验证 existing target 仍是 `com.openai.codex` 且未运行；
   - rename existing → 同卷临时 backup；
   - rename staging → target；
   - post-verify target；
   - 成功删除 backup；
   - 立即失败则恢复 backup。

该 backup 是单次事务补偿，不是面向用户的版本回滚功能。不要长期保留旧应用。

注意：

- staging/backup 名不可来自远程；
- canonical parent 必须是计划的 Applications；
- 避免跨卷 rename；
- symlink/TOCTOU 前后重新检查；
- 不使用 `rm -rf` 对未经验证路径；
- 删除只限随机 staging/backup 和已确认目标 Stable。

## 13. 安装后验证

目标路径：

- 存在且是目录 bundle；
- exact Bundle ID；
- Team ID；
- codesign；
- spctl；
- arm64；
- platform version >= target；
- platform version >= pre-install；
- 本地标准目录扫描只发现一个 Stable。

失败执行即时补偿；无法补偿时 `INSTALLATION_VERIFY_FAILED` 并记录 backup/staging 的脱敏状态，人工处理。

## 14. 启动

```bash
open <verified-actual-bundle-path>
```

- 参数数组，不拼 shell；
- 启动前重新扫描并确认唯一 Stable；
- 不使用 `open -a ChatGPT`；
- 不使用目录名推断；
- 启动失败 `LAUNCH_FAILED`。

## 15. 临时与挂载清理

- DMG 安装包在 job temp；
- mount guard 总会 detach；
- staging/backup 在目标卷；
- success 删除 DMG、staging/backup；
- cancel 只可能在安装前，删除 DMG；
- crash 后 temp cleanup 不应误删 target staging；建议 staging 名和 sidecar 标记可在下一次平台检测中识别，但 V1 不做复杂自动修复；发现遗留只记录并阻断目标写入，人工确认。

## 16. 测试

### Bundle fixture

- Stable `com.openai.codex`，目录 `ChatGPT.app`；
- Stable 旧目录 `Codex.app`；
- Classic `com.openai.chat`；
- Beta；
- wrong Team；
- wrong arch；
- min OS > current；
- malformed plist；
- multiple Stable。

### CommandRunner fake

覆盖：

- attach 输出；
- no app/multiple app；
- codesign fail；
- spctl fail；
- lipo output；
- permission fallback；
- both path conflict；
- running Stable；
- copy fail；
- post-verify fail + restore；
- detach fail；
- launch verified path。

### 重要断言

- 不出现 `xattr -d`；
- 不出现 `codesign --force`；
- 新安装保持 source app basename；
- 更新保持 existing target path；
- Classic 永不被删除/覆盖；
- multiple Stable 不自动选择；
- process name 不参与身份判断。

## 17. 完成定义

- macOS runner 编译、Clippy、测试通过；
- Intel 分支明确 unsupported；
- 保持官方 bundle 原名和内容；
- exact Bundle ID/Team/signature/Gatekeeper；
- 标准目录扫描；
- 权限 fallback 与路径冲突区分；
- running app 阻断且不 kill；
- 同卷 staging + post-verify +即时补偿；
- 启动使用 verified path；
- 未执行真实 `/Applications` 自动化写入。
