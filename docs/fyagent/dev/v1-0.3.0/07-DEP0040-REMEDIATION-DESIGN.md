# DEP0040 / punycode 根因修复设计

> **交付状态**：Proposed / 拟实施  
> **关联决策**：81–87  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

## 1. 当前证据

[Observed / 已核实]

```text
package.json devDependencies: cross-fetch ^4.1.0
tests/msw/tauriMocks.ts: import "cross-fetch/polyfill"
pnpm-lock.yaml: cross-fetch 4.1.0 → node-fetch 2.7.0 → whatwg-url 5.0.0 → tr46 0.0.3
```

Node 24 中 `DEP0040` 属于 application deprecation 时，默认可能不报告 `node_modules` 的调用；`--pending-deprecation` 会扩大报告范围。因此仅以 stderr 默认是否出现为验收会产生假阴性。

## 2. 目标变更

```diff
- import "cross-fetch/polyfill";
  import { vi } from "vitest";
```

从 `package.json` 删除 `cross-fetch`，正常运行 pnpm 更新锁文件。不得增加 `node-fetch`、`undici`、`isomorphic-fetch` 或其它全局 polyfill；Node 24 原生 Fetch API 是项目基线。

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

新增聚焦测试证明：

```text
Tauri invoke mock
→ native global fetch
→ MSW handler
→ Response status/body
→ JSON/text parse
```

测试同时覆盖成功、非 2xx 错误文本和空响应，避免只验证 `typeof fetch`。

## 5. 依赖图验收

锁文件重生成后运行并保存证据：

```bash
pnpm why cross-fetch
pnpm why node-fetch
pnpm why whatwg-url
pnpm why tr46
```

合同不是禁止所有 `whatwg-url`/`tr46` 名称，而是确认旧链路和版本退出；jsdom 等可合法使用不同版本。

## 6. 告警门禁

- 普通 Node/Vitest 合同：`NODE_OPTIONS=--throw-deprecation`；
- 聚焦 Native Fetch/MSW 探针：`NODE_OPTIONS="--pending-deprecation --throw-deprecation"`；
- 探针尽量只加载相关 setup，避免无关依赖的 pending deprecation 降低可操作性；
- 禁止 `NODE_NO_WARNINGS`、`--no-warnings`、`--no-deprecation`、`--disable-warning=DEP0040`、stderr 过滤。

## 7. 上游差异

完整 merge `v3.19.2` 后再单独删除 `cross-fetch`。在 upstream-sync spec 记录文件、原因、Node 24 前提、行为测试和重新评估条件。若未来上游也删除该依赖，则收敛差异；若上游采用新方案，以实际测试评估。

## 8. 完成定义

1. package manifest 无 `cross-fetch`；
2. 活动源码/测试无 polyfill import；
3. 锁文件无已知旧链路；
4. Native Fetch/MSW 行为测试通过；
5. 全量前端和 desktop mock 测试通过；
6. 两级 deprecation 门禁通过；
7. 仓库无相关 suppression；
8. frontend quality 与 upstream-sync spec 已更新。
