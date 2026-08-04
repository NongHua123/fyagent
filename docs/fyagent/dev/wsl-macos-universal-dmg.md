# 在 WSL2 中一键构建 FyAgent macOS Universal DMG

本文记录仓库内受支持的 WSL macOS 交叉构建流程。它面向内部测试和后续手动跨平台构建研究，不是正式 macOS 发布流程。

## 结论与边界

安装全局 mise 并把仓库放在 WSL ext4 文件系统后，唯一构建入口是：

```bash
./scripts/macos-cross/build-universal-dmg.sh
```

首次非交互执行使用：

```bash
./scripts/macos-cross/build-universal-dmg.sh --accept-risk
```

该命令只生成一个同时包含 `arm64` 与 `x86_64` 切片的 Universal DMG。App 和 DMG 都是 ad-hoc 签名，未经过 Developer ID 签名或 Apple 公证；DMG 由非 Apple 的实验性工具生成。因此它不能作为公开发行包，也不能把 WSL 静态验证描述为已经在 macOS 上安装、启动或通过 Gatekeeper。

## 支持矩阵与前置条件

| 项目            | 支持范围                                                      |
| --------------- | ------------------------------------------------------------- |
| Windows/WSL     | Windows 11 + WSL2                                             |
| WSL CPU         | x86_64                                                        |
| Linux 发行版    | Ubuntu 22.04、24.04                                           |
| 仓库文件系统    | WSL ext4；拒绝 `/mnt/c` 等 DrvFS 路径                         |
| mise            | 用户全局安装，版本 `>= 2026.8.0`，Linux 路径且不能位于 `/mnt` |
| macOS target    | `universal-apple-darwin`                                      |
| 最低 macOS 版本 | 12.0                                                          |

全局 mise 是 WSL 开发环境的一部分，不仅供本工作流使用。先按照 [mise 官方安装说明](https://mise.jdx.dev/getting-started.html)完成一次全局安装，然后验证：

```bash
command -v mise
mise --version
mise trust
mise install
```

`command -v mise` 必须返回 WSL/Linux 路径，不能是 `/mnt/<drive>` 下的 Windows 可执行文件。项目脚本不会下载第二份 mise，也不会设置私有的 `MISE_DATA_DIR`、`MISE_CARGO_HOME` 或 `RUSTUP_HOME`。Node、pnpm、Python 和 Rust 的具体版本仍由仓库的 `mise.toml` 与 `mise.lock` 决定。

首次运行可能产生以下系统或用户级变更：

- 缺少 apt 依赖时执行 `sudo apt-get update` 和 `apt-get install`；apt 包是共享系统依赖，不会自动卸载。
- 全局 mise 执行 `mise install`，在用户的正常 mise/Rust 数据目录中安装仓库声明的开发工具和两个 macOS Rust targets；这些工具可能由其他项目共享。
- SDK、OSXCross、libdmg、rcodesign、下载文件和风险确认只写入 FyAgent 专用 XDG 目录。
- 依赖安装在 mise 环境内以 `CI=true mise exec -- pnpm install --frozen-lockfile`
  的等效方式运行；若现有 `node_modules` 属于不同 pnpm store，会自动按锁文件重建而不会询问。

本次机器上的 FyAgent cross cache/data、全局 mise/Rust 数据、Rust target、`node_modules` 合计约 9 GiB。新环境建议至少预留 15 GiB；网络速度、SDK 下载和 OSXCross/Rust 冷编译会显著影响耗时。本次已有 cross-tool 缓存、但首次切换到全局 Cargo home 的构建用时 762.69 秒；随后的缓存复用构建用时 566.15 秒。完全无缓存环境应预留 30–60 分钟以上。

## 固定版本与供应链输入

| 组件             | 固定值                                                                                                                                         |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| 全局 mise        | 最低版本 2026.8.0；二进制由用户管理，不由项目下载                                                                                              |
| Node.js          | 22.12.0                                                                                                                                        |
| pnpm             | 10.12.3                                                                                                                                        |
| Python           | 3.12.8                                                                                                                                         |
| Rust             | 1.95.0 + rustfmt + clippy                                                                                                                      |
| Rust targets     | `aarch64-apple-darwin`、`x86_64-apple-darwin`                                                                                                  |
| macOS SDK        | `MacOSX14.5.sdk.tar.xz`                                                                                                                        |
| SDK URL          | `https://github.com/joseluisq/macosx-sdks/releases/download/14.5/MacOSX14.5.sdk.tar.xz`                                                        |
| SDK SHA256       | `6e146275d19f027faa2e8354da5e0267513abf013b8f16ad65a231653a2b1c5d`                                                                             |
| OSXCross         | `https://github.com/tpoechtrager/osxcross.git`，commit `27d21e4977c9751d01199c7a226a6faf494c3dd9`                                              |
| OSXCross 配置    | flavor `llvm`、arches `arm64 x86_64`、deployment target `12.0`                                                                                 |
| libdmg-hfsplus   | `https://github.com/planetbeing/libdmg-hfsplus.git`，commit `7ac55ec64c96f7800d9818ce64c79670e7f02b67`                                         |
| rcodesign URL    | `https://github.com/indygreg/apple-platform-rs/releases/download/apple-codesign/0.29.0/apple-codesign-0.29.0-x86_64-unknown-linux-musl.tar.gz` |
| rcodesign SHA256 | `dbe85cedd8ee4217b64e9a0e4c2aef92ab8bcaaa41f20bde99781ff02e600002`                                                                             |

远程 cross-build 输入的单一来源是 `scripts/macos-cross/constants.sh`。`mise.toml` 和 `mise.lock` 固定项目开发工具；自动化测试会检查它们与 `.node-version`、`package.json#packageManager` 和 `rust-toolchain.toml` 一致。

第三方 SDK 的 checksum 只能证明下载文件与选定发布一致，不能证明 Apple 授权或消除许可限制。运行前必须自行审阅适用的 Apple 条款。

`libdmg-hfsplus` 是 GPL-3.0、本地编译、上游标记为高度实验性的工具。工作流只构建未加密 DMG 所需的 `dmg-bin`，明确禁用 FileVault 并拒绝链接宿主 `libcrypto`；其源码和二进制不会提交到仓库或放入 DMG。GPL 工具只存在于本地构建工具边界内。

## 从新 WSL 到一条命令产物

1. 安装受支持的 WSL2 Ubuntu x86_64，把仓库放在例如 `/projects/cc-switch` 的 ext4 路径。
2. 全局安装 mise `>= 2026.8.0`，确认 `command -v mise` 是 Linux 路径。
3. 进入仓库；无需预装 Node、pnpm、Python 或 Rust，工作流会通过全局 mise 安装仓库声明版本。
4. 审阅本文的 SDK、Apple 条款、GPL 和 experimental 风险。
5. 执行：

   ```bash
   ./scripts/macos-cross/build-universal-dmg.sh --accept-risk
   ```

6. 成功后校验：

   ```bash
   cd dist-macos
   sha256sum -c FyAgent-*-macOS-universal-adhoc-unnotarized-experimental.dmg.sha256
   ```

交互式终端首次执行可以省略 `--accept-risk`，然后按提示输入 `ACCEPT`。风险确认指纹绑定 SDK URL/hash 和 OSXCross/libdmg 提交；这些输入变化时会要求重新确认。

## 阶段与日志标识

入口依次执行以下阶段，日志统一以 `[macos-cross]` 开头：

1. `stage 1/3`：验证全局 mise、WSL2、Ubuntu、x86_64、ext4，安装缺少的 apt 包并记录版本，处理风险确认。
2. `stage 2/3`：执行全局 `mise install`，校验运行时/targets 与 `mise which` 路径；校验或构建 SDK、OSXCross、libdmg、rcodesign。
3. `stage 3/3`：项目预检、mise 环境内的 frozen pnpm 安装、Tauri Universal 构建、App/DMG 组装签名和最终验证。

核心构建命令只有：

```bash
mise exec -- pnpm tauri build --target universal-apple-darwin --no-bundle --ci
```

Tauri 内部依次构建两种架构并调用工作流提供的 `lipo` 包装器合并。合并后必须满足：

- `file` 识别为含两个架构的 Mach-O Universal binary。
- `lipo -archs` 的集合精确等于 `arm64` 与 `x86_64`。
- 两个切片都记录最低 macOS 12.0。
- `otool -L` 不含 `.so`、`/home/`、`/mnt/` 或 Linux 系统库路径。
- App 和 DMG 的 rcodesign 输出包含 `CodeSignatureFlags(ADHOC)`，且 `cms: null`。
- DMG 最后 512 字节以 `koly` 开头，并通过仓库的 UDIF magic、checksum 和 manifest 一致性检查。

最终目录只应包含：

```text
dist-macos/FyAgent-<version>-macOS-universal-adhoc-unnotarized-experimental.dmg
dist-macos/FyAgent-<version>-macOS-universal-adhoc-unnotarized-experimental.dmg.sha256
dist-macos/FyAgent-<version>-macOS-universal-adhoc-unnotarized-experimental.dmg.manifest.json
```

构建不会检查 Git 工作树是否干净。manifest 的 `gitCommit` 仅提供源码上下文，`worktreePolicy` 固定为 `unchecked`，不能据此声称产物可从该提交逐字节复现。

## 缓存、更新与清理

macOS cross-build 专用默认路径是：

```text
${XDG_CACHE_HOME:-$HOME/.cache}/fyagent-macos-cross
${XDG_DATA_HOME:-$HOME/.local/share}/fyagent-macos-cross
${XDG_STATE_HOME:-$HOME/.local/state}/fyagent-macos-cross
```

这些目录不再包含 mise 或语言运行时。下载完成并校验后才进入正式缓存；SDK hash、工具提交、部署目标、Ubuntu 版本或宿主架构改变会产生新的缓存键。第二次运行复用 SDK、OSXCross、libdmg 和 rcodesign，但仍重新调用应用构建并重复所有验证。

完全清理 FyAgent cross-build 缓存时，优先使用可恢复的回收站操作：

```bash
gio trash "${XDG_CACHE_HOME:-$HOME/.cache}/fyagent-macos-cross"
gio trash "${XDG_DATA_HOME:-$HOME/.local/share}/fyagent-macos-cross"
gio trash "${XDG_STATE_HOME:-$HOME/.local/state}/fyagent-macos-cross"
```

执行前必须确认展开后的路径精确以 `/fyagent-macos-cross` 结尾。不要删除整个 `~/.cache`、`~/.local/share` 或 `~/.local/state`。上述命令也会移除风险确认标记，下一次运行需重新确认。

全局 mise 与其安装的 Node/Python/Rust 可能被其他项目共享，不属于此工作流的清理范围；不要从 macOS 构建脚本中自动卸载。确需删除某个全局 mise 工具时，应先检查其他项目依赖，再显式使用 mise 自身的卸载命令。

更新固定输入时必须在同一变更中：

1. 更新 `mise.toml` 以及 `.node-version`、`package.json#packageManager`、`rust-toolchain.toml` 中相应兼容声明。
2. 使用受支持的全局 mise 运行 `mise lock --platform linux-x64`，提交更新后的 `mise.lock`。
3. 更新 `constants.sh` 中 SDK/工具 URL、commit 与 SHA256；禁止使用 `latest`、`main` 或 `master`。
4. 更新本文与 Trellis spec，运行全部静态/失败关闭测试和两次真实 WSL 构建。

## 常见失败与恢复

| 阶段      | 典型原因                                        | 处理方式                                                                       |
| --------- | ----------------------------------------------- | ------------------------------------------------------------------------------ |
| mise      | 未安装、低于 2026.8.0，或解析到 `/mnt`          | 安装/升级 Linux 全局 mise，修复 PATH；不要恢复私有 mise                        |
| host      | WSL1、非 Ubuntu、ARM 主机、DrvFS                | 使用受支持的 WSL2 Ubuntu x86_64，并把仓库复制到 ext4                           |
| risk      | 非交互首次运行未确认                            | 审阅风险后显式传 `--accept-risk`                                               |
| download  | 网络中断或 checksum 不一致                      | 重试；checksum 不一致时不要手工绕过                                            |
| tools     | `mise.toml`、兼容声明或 Rust targets 不一致     | 运行 `mise install`，修复声明并重新生成 `mise.lock`                            |
| pnpm      | store 身份不同导致需重建 `node_modules`         | 脚本已设置 `CI=true`；若仍出现提示，视为缺陷并停止，不要手工应答               |
| preflight | 新增 resources/sidecars/frameworks/entitlements | 先扩展组装器和测试，不能跳过预检                                               |
| compile   | crate 不支持 macOS 交叉编译或链接器错误         | 检查完整日志、target-specific 编译器变量和 SDK；不要加入 Linux pkg-config 路径 |
| binary    | 缺少切片或出现 `.so`、`/home`、`/mnt`           | 修复构建环境，禁止继续组装 App                                                 |
| dmg       | libdmg 失败、无 `koly` 或签名失败               | 旧成功产物保持不变；清理 FyAgent cross cache 后重试，不能改名发布中间文件      |
| manifest  | 文件名、大小或 SHA 与 DMG 不一致                | 失败关闭；重新运行完整入口，不能手工修改 manifest                              |

## WSL 实测记录（2026-08-04）

### 受支持的全局 mise 最终验收

宿主与源码上下文：

| 项目             | 实测值                                                 |
| ---------------- | ------------------------------------------------------ |
| WSL              | WSL2，kernel `6.18.33.2-microsoft-standard-WSL2`       |
| Ubuntu           | 24.04.1 LTS                                            |
| CPU              | x86_64                                                 |
| Git HEAD         | `a0391283a04ff98b33361ec94875bc7de0fb30b2`，仅作上下文 |
| 全局 mise        | `/root/.local/bin/mise`，2026.8.1                      |
| Node/pnpm/Python | 22.12.0 / 10.12.3 / 3.12.8                             |
| Rust/Cargo       | 1.95.0 / 1.95.0                                        |

第一次全局 mise 构建命令：

```bash
PATH=/root/.local/bin:/usr/bin:/bin \
  ./scripts/macos-cross/build-universal-dmg.sh --accept-risk
```

- 退出码：0。
- 用时：762.69 秒；最大 RSS：6,175,948 KiB。
- 全局 mise 校验通过；OSXCross、libdmg、rcodesign 命中固定缓存。
- 产物静态验证、checksum 和 manifest 检查全部通过。

紧接着以同一命令进行缓存复用验收：

- 退出码：0。
- 用时：566.15 秒；最大 RSS：6,173,272 KiB。
- 日志再次明确记录 OSXCross、libdmg、rcodesign cache hit。
- Tauri Universal 构建仍被调用，最终五次 `static artifact verification passed`，应用和 DMG 重新签名/验证。

缓存复用运行留下的最终产物：

| 项目                | 实测值                                                                     |
| ------------------- | -------------------------------------------------------------------------- |
| 文件                | `FyAgent-0.1.0-macOS-universal-adhoc-unnotarized-experimental.dmg`         |
| 大小                | 28,034,561 bytes                                                           |
| SHA256              | `3879d893a4afd29d11e19746fd472e958412039e3127b98cb966f4ba9cc19db1`         |
| Universal slices    | `x86_64 arm64`，集合精确匹配                                               |
| Deployment target   | 两个切片均为 macOS 12.0                                                    |
| Mach-O dependencies | 仅 macOS frameworks 与 `/usr/lib/*.dylib`；无 Linux/WSL 路径               |
| App signature       | ad-hoc，`CodeSignatureFlags(ADHOC)`，无 CMS                                |
| DMG signature       | ad-hoc，`CodeSignatureFlags(ADHOC)`，`cms: null`                           |
| UDIF                | `file` 识别 Apple Disk Image，trailer 为 `koly`                            |
| Checksum            | `sha256sum -c` 通过                                                        |
| Manifest            | schema v1；`wslStaticValidation: passed`；`macosNativeValidation: pending` |

`dist-macos/` 实测只包含 DMG、`.sha256` 和 `.manifest.json` 三项。标准 FyAgent XDG 目录中已复核不存在 `mise`、`mise-data` 或 mise 下载文件；项目运行时来自全局 mise。

### 全局 mise 改造前基线

切换前的私有 mise 版本也曾按相同静态门禁成功构建，最终输出包含三次末端 checksum/manifest 验证和完成提示。迁移前基线 DMG SHA256 为 `b4335ce625a79ff5183416484eda98e0243b5a9c10bb854104db1ebaddcb0b8a`。该记录只用于证明全局 mise 改造没有破坏产物流程；私有 mise 已不再受支持，相关残留已移入系统回收站。

## 后续真 Mac 验收

以下步骤当前明确为 pending，必须在真实 macOS 上完成后才能更新结论：

```bash
hdiutil verify FyAgent-*.dmg
hdiutil attach -readonly -nobrowse FyAgent-*.dmg
codesign --verify --deep --strict --verbose=2 /Volumes/FyAgent/FyAgent.app
spctl --assess --type execute --verbose=4 /Volumes/FyAgent/FyAgent.app
```

还需完成：

1. 只读挂载后确认 DMG 顶层只有 `FyAgent.app` 和指向 `/Applications` 的链接。
2. 把 App 复制到 `/Applications`，确认可启动主窗口。
3. 验证 `fyagent://` deep-link 和基础功能。
4. 记录 `codesign` 的真 Mac 验证输出。
5. 如实记录 `spctl` 结果。由于该包未公证，Gatekeeper 很可能拒绝它；不得把手工绕过安全策略描述为 Gatekeeper 通过。

正式公开发布仍使用 `.github/workflows/release.yml` 的真 macOS runner、Developer ID 签名、公证、Staple 和 Gatekeeper 验证流程。本地 WSL 脚本不得读取发布证书、Apple ID、App Store Connect 或 notarization secrets，也不得接入该 release workflow。
