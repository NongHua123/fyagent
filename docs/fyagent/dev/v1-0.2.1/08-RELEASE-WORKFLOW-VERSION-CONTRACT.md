# 08 — 发布工作流版本契约

## 1. 目的

本文定义从单一真源到 Windows、macOS、Linux 正式产物的版本传播链，并把标签、文件名、内嵌元数据和下载清单纳入同一个 CI 门禁。

当前 `.github/workflows/release.yml` 多处直接使用 `GITHUB_REF_NAME` 生成资产名，并由不同平台脚本分别读取 `package.json`、`tauri.conf.json` 或 Cargo 版本。目标设计改为：

```text
src-tauri/Cargo.toml
  ↓ version-contract job
APP_VERSION=0.2.1
RELEASE_TAG=v0.2.1
SOURCE_SHA=<commit>
  ↓
所有平台构建、命名、校验、清单和发布
```

## 2. 版本契约 job

### 2.1 位置

在 release workflow 最前面新增 `version-contract` job。所有构建 job 必须 `needs: version-contract`，并只使用其 outputs。

### 2.2 输出

```text
app_version   0.2.1
release_tag   v0.2.1
source_sha    完整 Git commit SHA
```

### 2.3 参考实现

```yaml
jobs:
  version-contract:
    name: Verify FyAgent version contract
    runs-on: ubuntu-latest
    outputs:
      app_version: ${{ steps.version.outputs.app_version }}
      release_tag: ${{ steps.version.outputs.release_tag }}
      source_sha: ${{ steps.version.outputs.source_sha }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: pnpm/action-setup@v4
        with:
          run_install: false

      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm

      - name: Verify repository and release tag
        shell: bash
        run: pnpm run version:check -- --tag "$GITHUB_REF_NAME"

      - id: version
        name: Export immutable version values
        shell: bash
        run: |
          set -euo pipefail
          app_version="$(pnpm --silent run version:get)"
          printf 'app_version=%s\n' "$app_version" >> "$GITHUB_OUTPUT"
          printf 'release_tag=v%s\n' "$app_version" >> "$GITHUB_OUTPUT"
          printf 'source_sha=%s\n' "$GITHUB_SHA" >> "$GITHUB_OUTPUT"
```

具体 Node/pnpm setup 版本应复用仓库现有策略；关键契约是 job 在签名、公证和大规模编译前失败。

## 3. 标签规则

正式发布只能由精确标签触发：

```text
vX.Y.Z
```

必须满足：

```text
GITHUB_REF_TYPE == tag
GITHUB_REF_NAME == "v" + APP_VERSION
```

以下标签均拒绝：

```text
0.2.1
release-0.2.1
v0.2.1-hotfix
v0.2.2     # Cargo 仍为 0.2.1
```

当前不支持 prerelease 标签。未来若支持，需要同步设计 GitHub Release prerelease 标志、Tauri updater 比较、MSI 升级排序和资产通道。

## 4. 构建 job 消费规则

每个 job 顶层设置：

```yaml
env:
  APP_VERSION: ${{ needs.version-contract.outputs.app_version }}
  RELEASE_TAG: ${{ needs.version-contract.outputs.release_tag }}
  SOURCE_SHA: ${{ needs.version-contract.outputs.source_sha }}
```

禁止：

- 从 `GITHUB_REF_NAME` 去掉字符后当版本；
- 从 `package.json` 读取；
- 从 `tauri.conf.json` 读取；
- 各平台使用不同 sed/PowerShell 正则推导；
- 在 job 内修改版本文件。

每个构建 job checkout 后可再次运行只读校验：

```bash
pnpm run version:check -- --tag "$RELEASE_TAG"
```

这不是第二真源，而是防止 checkout、生成脚本或缓存污染。

## 5. 资产命名

### 5.1 统一原则

- 文件名使用 `APP_VERSION`，不含标签前导 `v`；
- 平台和架构后缀固定；
- 不从 Tauri 自动生成文件名解析版本；
- 重命名前验证源产物内嵌版本；
- 同名冲突直接失败，不覆盖。

### 5.2 建议命名

```text
FyAgent-0.2.1-Windows.msi
FyAgent-0.2.1-Windows-arm64.msi
FyAgent-0.2.1-macOS.dmg
FyAgent-0.2.1-macOS.zip
FyAgent-0.2.1-macOS-unsigned.dmg
FyAgent-0.2.1-macOS-unsigned.zip
FyAgent-0.2.1-Linux-x86_64.AppImage
FyAgent-0.2.1-Linux-x86_64.deb
FyAgent-0.2.1-Linux-x86_64.rpm
FyAgent-0.2.1-Linux-arm64.AppImage
FyAgent-0.2.1-Linux-arm64.deb
FyAgent-0.2.1-Linux-arm64.rpm
```

若当前资产命名兼容性要求保留其他后缀，可保留，但版本部分必须来自 `APP_VERSION`。

## 6. Windows 发布契约

### 6.1 构建

正式公开 MSI 使用原生 Windows runner：

```powershell
pnpm tauri bundle --bundles msi
```

跨平台 Wine 脚本仍可产生候选 MSI，用于快速反馈或结构检查，但不能绕过原生 Windows 生命周期门禁。

### 6.2 内嵌版本验证

至少验证：

1. MSI Property 表 `ProductVersion == APP_VERSION`；
2. Upgrade/Product identity 符合升级设计；
3. 主 EXE ProductVersion/FileVersion 与 `APP_VERSION` 的数值投影一致；
4. MSI 中不存在旧 `ValidateInstallDirectory`、VBScript/WMI 字符串；
5. MSI Binary 表包含正确架构的原生动作 DLL；
6. 文件名版本等于 `APP_VERSION`。

MSI 可用 WiX/Windows Installer SDK 工具导出 Property/Binary/CustomAction 表，或通过 PowerShell COM API读取。

### 6.3 生命周期门禁

正式签发/发布前在隔离 Windows VM 执行：

```text
fresh install -> launch -> close -> repair -> upgrade -> uninstall
```

并覆盖默认目录、安全 D 盘目录、不安全目录、静默安装和 ARM64/x64。

## 7. macOS 发布契约

### 7.1 构建脚本

当前 macOS 交叉构建脚本如果同时比较 `package.json`、Tauri 和 Cargo 版本，应改为：

```bash
APP_VERSION="$(pnpm --silent run version:get)"
pnpm run version:check
```

然后只使用 `APP_VERSION` 命名和校验。

### 7.2 内嵌版本

签名/公证前后验证：

```text
CFBundleShortVersionString == APP_VERSION
```

`CFBundleVersion` 若需要纯整数构建号，应另行定义由 CI run number 派生的技术构建号，不能反过来替代 `APP_VERSION`。当前若 Tauri 直接生成兼容值，则保持现状并验收。

### 7.3 资产

DMG、ZIP、unsigned 资产均使用同一 `APP_VERSION`，不得从 tag 直接带入 `v`。

## 8. Linux 发布契约

### 8.1 构建

AppImage、DEB、RPM 构建均由 Tauri 解析 Cargo app version。重命名脚本读取 `APP_VERSION` output。

### 8.2 验证

- DEB：读取 control `Version`；
- RPM：读取 package version；
- AppImage：验证 Tauri 生成元数据或运行 `--version`/应用诊断接口；
- 文件名版本一致；
- 所有架构来自同一 `SOURCE_SHA`。

发行版包可能附加 revision/release 字段。允许包管理器层面的 revision，但其上游应用版本部分必须严格为 `APP_VERSION`，并在清单中分别记录。

## 9. 下载 manifest 契约

下载清单不得再通过“从 tag 去掉 v”作为版本真源，而应由 version-contract output写入：

```json
{
  "version": "0.2.1",
  "tag": "v0.2.1",
  "sourceSha": "<40-char-sha>",
  "assets": [
    {
      "platform": "windows",
      "arch": "x86_64",
      "name": "FyAgent-0.2.1-Windows.msi",
      "sha256": "..."
    }
  ]
}
```

生成时要求：

- `version == APP_VERSION`；
- `tag == RELEASE_TAG`；
- `sourceSha == SOURCE_SHA`；
- 每个文件真实存在；
- SHA-256 在重命名和签名完成后计算；
- 不包含未通过门禁的候选产物。

## 10. 发布阶段划分

推荐 job 依赖图：

```text
version-contract
  ├─ build-macos
  ├─ build-linux-x64
  ├─ build-linux-arm64
  ├─ build-windows-x64
  └─ build-windows-arm64
         ↓
platform-verification
         ↓
windows-lifecycle-test
         ↓
assemble-release-assets
         ↓
generate-manifest-and-checksums
         ↓
publish-release
```

`assemble-release-assets` 只能下载来自当前 workflow run、当前 `SOURCE_SHA` 的 artifacts。

## 11. 版本内嵌验证表

| 平台/对象 | 读取方式 | 期望值 |
|---|---|---|
| Cargo canonical | `version:get` | `APP_VERSION` |
| Tauri resolved | build metadata/API | `APP_VERSION` |
| MSI ProductVersion | MSI Property | `APP_VERSION` |
| Windows EXE | Version resource | 数值对应 `APP_VERSION` |
| macOS app | Info.plist | `APP_VERSION` |
| DEB | package metadata | `APP_VERSION` 或明确 revision 形式 |
| RPM | package metadata | `APP_VERSION` |
| AppImage | app metadata/diagnostic | `APP_VERSION` |
| 文件名 | 受控正则 | `APP_VERSION` |
| manifest | JSON 字段 | `APP_VERSION` / `RELEASE_TAG` |
| GitHub Release | tag name | `RELEASE_TAG` |

任何一项不一致，发布 job 失败。

## 12. 分支与发布操作

### 12.1 准备发布

```bash
pnpm run version:set -- 0.2.1
pnpm run version:check
pnpm test
# 审查并提交源码
```

### 12.2 创建标签前

```bash
git status --short
git show HEAD:src-tauri/Cargo.toml
pnpm run version:check
git tag -s v0.2.1 -m "FyAgent 0.2.1"
git push origin <branch> v0.2.1
```

是否要求签名标签可由仓库策略决定；标签字符串精确匹配是强制要求。

### 12.3 发布失败

构建或验收失败时：

- 不移动/复用同一个公开标签到新 commit；
- 若标签尚未公开，可按团队 Git 策略删除后重新创建；
- 若标签已公开，应修复后递增新 PATCH，例如 `0.2.2`；
- 失败产物不进入下载 manifest。

## 13. 对现有 workflow 的具体修正

当前 workflow 中类似：

```bash
VERSION="${GITHUB_REF_NAME}"
```

和：

```powershell
$VERSION = $env:GITHUB_REF_NAME
```

应统一替换为 job env 中的 `APP_VERSION`。现有资产名包含 `v` 的行为随之修正。

所有对 tag 的清洗：

```bash
sed 's/[^A-Za-z0-9._-]/-/g'
```

不得用于版本推导。标签在 version-contract 已被严格验证，不需要“容错清洗”；出现不合格字符应失败。

## 14. 安全与供应链要求

- version-contract 在使用签名密钥前运行；
- workflow 权限按 job 最小化；
- 构建产物记录 commit SHA、runner OS/arch、工具链锁定信息和 SHA-256；
- Windows MSI/EXE、macOS app/DMG 的签名验证结果进入发布证据；
- 不接受外部 PR 工作流访问正式签名秘密；
- 发布 job 只消费受信分支/tag 产生的 artifact。

## 15. 完成定义

- release workflow 只有 version-contract job读取单一真源；
- 所有平台 job 通过 outputs 获取版本；
- 资产名不再含前导 `v`；
- 标签不匹配在构建前失败；
- 每个平台有内嵌版本校验；
- 下载 manifest 同时记录应用版本、标签、SHA 和资产哈希；
- 公开 MSI 经过原生 Windows 生命周期测试；
- 单个平台不能单独发布与其他平台不同的 FyAgent 应用版本。
