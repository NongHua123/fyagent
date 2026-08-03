# FyAgent V1 macOS 平台适配

## Goal

在 Core trait 上完成 macOS Apple Silicon 的 Stable Bundle 检测、DMG 验证、无管理员
事务式安装、安装后重检与按已验证实际路径启动。macOS Intel 必须明确 unsupported。

## Requirements

- 只扫描 `/Applications/*.app` 与 `~/Applications/*.app` 的一级候选。只有 exact
  `com.openai.codex`、允许 Team ID、arm64、codesign 与 Gatekeeper 均通过的 Bundle
  才是 Stable；Classic、Beta、未知 Bundle 绝不覆盖或选作 target。
- 通过 fakeable `CommandRunner` 调用 `plutil`、`hdiutil -plist`、`lipo`、`codesign`、
  `spctl`、`ditto`、`open`，所有参数数组化并有 timeout/有限输出。
- DMG 内只能有一个 top-level `.app`；新安装保留 DMG 原始 basename，优先
  `/Applications`，仅在 permission denied 时回退 `~/Applications`；更新保留现有
  Stable 实际路径。
- 使用同卷随机 staging/backup、复制后重验、atomic rename、post-verify 和即时 restore；
  update 时若 app 运行或无法安全判断运行状态则阻断，不 quit/kill。
- 禁止 sudo、helper、管理员密码、xattr 去 quarantine、修改 plist/资源、重新签名、
  `open -a ChatGPT`、按目录名/进程名判定身份和真实 `/Applications` 自动化写入。

## Acceptance Criteria

- [ ] fixture/fake 覆盖 Stable 不同 basename、Classic/Beta、错误 Team/arch/ID、最低 OS、
  multiple Stable、错误 plist、DMG 0/多 app、codesign/spctl/attach/copy/detach failure。
- [ ] 目标路径策略覆盖 permission fallback、Classic 冲突、双路径冲突、已安装路径更新，
  且不删除/改名非 Stable Bundle。
- [ ] safe swap 覆盖新装、更新、rename 前/后失败、restore success/fail、symlink 拒绝和
  cleanup 边界；launch only receives a verified actual Bundle path。
- [ ] macOS runner 编译和 fake tests 可重复；Intel/非 macOS 返回 explicit unsupported，
  未真实挂载或写用户 Applications 目录。

## Ownership

仅修改 `src-tauri/src/codex_desktop/platform/macos/**` 与专属 fixture/test。不得修改
shared registration、Cargo、`main.rs`、UI 或 Windows code。需要 `CommandRunner`/filesystem
契约变化时先提给 Core。
