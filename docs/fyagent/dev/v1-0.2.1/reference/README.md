# Reference implementation and integration snippets

本目录提供可审阅、可测试的参考文件，目标是减少实施者把设计重新翻译为代码时产生偏差。

## 内容

```text
scripts/version.mjs
  FyAgent 全局应用版本的一键读取、校验、设置和递增参考实现。

tests/version.test.mjs
  隔离 fixture 测试；不会修改实际 FyAgent 源码。

snippets/Cargo.toml.versioning.toml
  根 Cargo workspace 版本结构。

snippets/installer-actions.Cargo.toml
  原生 MSI Custom Action crate 清单。

snippets/package.json.versioning.json
  package.json 应合并的 private/版本命令片段。

snippets/tauri.conf.versioning.json
  删除重复 version 后的 Tauri/WiX 相关结构。

snippets/release-version-contract.yml
  GitHub Actions 版本契约 job 参考。

snippets/wix-native-custom-actions.wxs
  WiX Binary、Type 1 actions、结果属性和 Execute Sequence 概念片段。
```

## 版本脚本落库

把：

```text
reference/scripts/version.mjs
```

复制为目标仓库：

```text
scripts/version.mjs
```

把测试复制为：

```text
tests/version.test.mjs
```

然后合并 package.json 脚本并运行：

```bash
node --test tests/version.test.mjs
pnpm run version:check
```

参考脚本假设目标仓库已完成 Cargo workspace 迁移；它不是旧三版本结构的自动迁移器。

## 本目录测试

在解压后的交付包根目录执行：

```bash
node --test reference/tests/version.test.mjs
```

测试会把脚本复制到临时仓库 fixture，再验证更新范围、标签、非法版本和 dry-run。

## 片段边界

片段是设计接线，不是可直接覆盖完整项目文件：

- `package.json` 片段必须与现有 scripts 合并；
- Cargo 根片段必须保留全部依赖/profile；
- WiX 片段中的条件、Fragment 和本地化要与当前 WiX v3 UI源码对齐；
- release YAML 要与现有权限、签名、公证和 matrix 合并；
- 原生目录校验算法以 `04` 为准，本包不提供未经 Windows 安全测试的伪实现。

不得把片段未经编译/生命周期验证直接作为已完成实现发布。
