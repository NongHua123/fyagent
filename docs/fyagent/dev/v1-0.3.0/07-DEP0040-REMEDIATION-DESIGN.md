# DEP0040 / punycode 根因修复设计

> **交付状态**：Implemented, locally verified, archived / 已实施、本地验证并归档
> **关联决策**：81–87  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 原始根因证据

[Observed / 已核实的 2026-08-07 输入]

```text
package.json devDependencies: cross-fetch ^4.1.0
tests/msw/tauriMocks.ts: import "cross-fetch/polyfill"
pnpm-lock.yaml: cross-fetch 4.1.0 → node-fetch 2.7.0 → whatwg-url 5.0.0 → tr46 0.0.3
```

Node 24 中 `DEP0040` 属于 application deprecation 时，默认可能不报告 `node_modules` 的调用；`--pending-deprecation` 会扩大报告范围。因此仅以 stderr 默认是否出现为验收会产生假阴性。

## 2. 已实施变更

```diff
- import "cross-fetch/polyfill";
  import { vi } from "vitest";
```

commit `4e407df4` 已从 `package.json` 删除 `cross-fetch`、删除 polyfill import，并以 pnpm `10.12.3` 正常重生成锁文件。没有增加 `node-fetch`、`undici`、`isomorphic-fetch` 或其它全局 polyfill；Node 24 原生 Fetch API 是项目基线。

## 3. 环境合同

测试 setup 在使用前验证：

```ts
for (const name of ["fetch", "Headers", "Request", "Response"] as const) {
  if (typeof globalThis[name] === "undefined") {
    throw new Error(
      "FyAgent tests require the repository-pinned Node.js runtime; run mise run env:check and mise run bootstrap.",
    );
  }
}
```

实际实现需处理 TypeScript 索引类型，但不得在缺 API 时动态安装 polyfill。

## 4. 行为测试

聚焦测试已经证明：

```text
Tauri invoke mock
→ native global fetch
→ MSW handler
→ Response status/body
→ JSON/text parse
```

测试同时覆盖 JSON 成功与 invoke 证据、非 2xx 错误文本、204 空响应映射为 `undefined` 和跨 jsdom realm 的 `Headers`，避免只验证 `typeof fetch` 或 `instanceof`。

## 5. 依赖图验收

依赖合同由 canonical Release 合同入口运行并保存结构化证据：

```bash
mise run release:check
```

其内部使用 argv 形式的 `pnpm why --json`，并与 manifest、活动 module specifier、pnpm package/snapshot graph 交叉验证。合同不是禁止所有 `whatwg-url`/`tr46` 名称，而是确认旧链路和版本退出；当前允许的路径为 `jsdom@25.0.1 → whatwg-url@14.2.0 → tr46@5.1.1 → punycode@2.3.1`，且必须同时由 lock 与 why graph 解释。

## 6. 告警门禁

- 普通 Node/Vitest 合同：`NODE_OPTIONS=--throw-deprecation`；
- 聚焦 Native Fetch/MSW 探针：`NODE_OPTIONS="--pending-deprecation --throw-deprecation"`；
- 探针尽量只加载相关 setup，避免无关依赖的 pending deprecation 降低可操作性；
- 禁止 `NODE_NO_WARNINGS`、`--no-warnings`、`--no-deprecation`、`--disable-warning=DEP0040`、stderr 过滤。

## 7. 上游差异

完整 merge `v3.19.2` 后再单独删除 `cross-fetch`。在 upstream-sync spec 记录文件、原因、Node 24 前提、行为测试和重新评估条件。若未来上游也删除该依赖，则收敛差异；若上游采用新方案，以实际测试评估。

## 8. 完成证据

1. package manifest、活动 source/import 和 lock 已无 `cross-fetch` 旧链路；
2. Native Fetch/MSW 四个行为测试全部通过；
3. 普通 `--throw-deprecation` 与聚焦 pending+throw 入口通过；
4. 7/7 DEP0040 JSON checks、13/13 DEP0040/CI contracts、全量 140 files/963 tests 通过；
5. 519 个活动 module 文件、2,812 个 specifier 和 68 个可执行配置面完成失败关闭扫描；
6. 独立 Trellis review 已补强 exact Node、AST parse、package/snapshot、alias/reverse-origin、组合 suppression 和跨 realm cleanup 断言；
7. Child 5 已归档；Windows/macOS/Linux ARM64/正式 Release 是 parent 级独立门禁，不回退本地完成结论。
