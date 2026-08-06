# 05 — WorkBuddy 数据模型与安全设计

## 1. 设计目标

WorkBuddy 配置服务应满足四个同时成立的目标：

1. 兼容项目旧根数组和当前对象根格式；
2. 对未知字段和历史重复条目保持非破坏性；
3. 对已有模型覆盖提供明确确认和原子事务；
4. 限制 API Key 在内存、日志和文件系统中的暴露范围。

本文只设计用户级 `~/.workbuddy/models.json`，不处理项目级配置、WorkBuddy 安装检测或完整模型管理。

## 2. 官方格式与兼容基线

**[官方事实]** 当前公开 WorkBuddy 配置示例使用对象根：

```json
{
  "models": [
    {
      "id": "hy3",
      "name": "hy3",
      "vendor": "Custom",
      "url": "https://example/v1",
      "apiKey": "..."
    }
  ]
}
```

**[源码事实]** 当前 FyAgent 仅接受根数组。为兼容已有用户和现行格式，目标解析器必须是双格式，而不能强制迁移全部旧文件。

## 3. 内部文档模型

建议保留整个根值并用枚举标记形状：

```rust
enum WorkBuddyRootKind {
    LegacyArray,
    ObjectRoot,
}

struct LoadedWorkBuddyDocument {
    exists: bool,
    root_kind: WorkBuddyRootKind,
    root: serde_json::Value,
    original_bytes: Vec<u8>,
    revision: Option<String>,
    model_locations: Vec<ModelLocation>,
}

struct ModelLocation {
    index: usize,
    id: String,
}
```

`serde_json` 已启用 `preserve_order`，应继续使用该特性，避免未知对象键因重序造成不必要差异。

### 3.1 根数组

```json
[
  { "id": "model-a", "custom": 1 }
]
```

- `root_kind = LegacyArray`；
- 模型数组就是根本身；
- 写回继续为数组；
- 不引入 `availableModels`；
- 不自动迁移为对象根。

### 3.2 对象根

```json
{
  "models": [
    { "id": "model-a", "custom": 1 }
  ],
  "availableModels": ["model-a"],
  "futureField": true
}
```

- `root_kind = ObjectRoot`；
- 只修改 `models` 和满足规则时的 `availableModels`；
- `futureField` 等未知字段原样保留；
- 顶层字段顺序尽量保留。

### 3.3 文件不存在

构造：

```json
{
  "models": []
}
```

随后执行同一新增算法。默认不创建 `availableModels`。

## 4. 解析和校验

### 4.1 根校验

| 输入 | 结果 |
|---|---|
| 根数组 | 接受，旧格式 |
| 根对象且 `models` 为数组 | 接受，对象格式 |
| 根对象且无 `models` | 接受，视为空数组，保留其他字段 |
| 根对象且 `models` 非数组 | 错误，禁止写入 |
| 其他根类型 | 错误，禁止写入 |

### 4.2 模型条目校验

每个 `models` 条目必须：

- 是 JSON 对象；
- `id` 是字符串；
- `id.trim()` 非空。

ID 在业务比较中使用 `trim()` 结果；原条目的 `id` 值不因普通读取而自动重写。只有该条目被目标覆盖时，仍保持原 ID 大小写和字符，不把 ID 统一小写。

发现非法条目时：

- 已有模型读取返回结构化错误；
- 保存操作失败关闭；
- 不自动删除或跳过非法条目；
- 原文件字节保持不变。

### 4.3 重复 ID

重复定义为：

```text
trim 后，大小写敏感精确相等
```

例如：

```text
" gpt-5 " 与 "gpt-5" 相同
"GPT-5" 与 "gpt-5" 不同
```

重复条目不是本次自动修复目标：

- 读取 ID 时静默去重；
- 保存其他模型时允许重复继续存在；
- 覆盖该 ID 时更新所有精确匹配条目；
- 不合并、不删除、不提示重复数量。

## 5. 已有模型 ID 投影

```rust
fn unique_model_ids(document: &LoadedWorkBuddyDocument) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for model in models(document) {
        let id = model.id.trim();
        if seen.insert(id.to_owned()) {
            output.push(id.to_owned());
        }
    }
    output
}
```

要求：

- 第一次出现顺序；
- 大小写保持；
- 不返回对象内容；
- 不返回条目数量；
- 不读取或反序列化 API Key 到 DTO。

`get_workbuddy_status.modelCount` 和 `get_workbuddy_model_ids.ids.length` 必须一致。

## 6. 目标 ID 规范化

目标集合由远程选择和手动输入拼接：

```text
selectedModelIds
+ manualModelIds
```

规范化规则：

1. 每项 `trim()`；
2. 删除空值；
3. 按大小写敏感精确值去重；
4. 保持第一次出现顺序；
5. 最终为空则返回 `WORKBUDDY_CONFIG_NO_TARGET_MODELS`。

不得排序，以保持用户选择和 `availableModels` 追加顺序。

## 7. 保存事务

### 7.1 第一次提交：预检

```text
read latest file
→ parse and validate
→ compare expectedRevision
→ normalize target IDs
→ find all existing target IDs
```

若存在已有 ID：

- 不创建备份；
- 不写临时文件；
- 不改 `availableModels`；
- 返回唯一 ID 列表和短期覆盖令牌。

若无已有 ID：直接进入原子新增事务。

### 7.2 确认提交

前端提交原请求和覆盖令牌。后端：

1. 消费并验证令牌；
2. 重新读取文件；
3. 验证最新 revision 与令牌绑定 revision 一致；
4. 重新解析并计算已有集合；
5. 确保请求摘要与令牌一致；
6. 在内存副本中更新/新增全部目标；
7. 更新 `availableModels`；
8. 创建备份；
9. 原子替换；
10. 返回新 revision 和统计。

任何步骤失败时，正式目标文件保持原状；备份创建成功但正式替换失败时，返回写入失败，不声称保存成功。

### 7.3 事务原子性

新模型与已有模型混合示例：

```text
A：已有
B：新增
C：已有
```

确认后必须一次完成：

```text
update all A matches
append B once
update all C matches
update availableModels
write one final document
```

不得产生“B 已保存但 A/C 未覆盖”的部分状态。

## 8. 新建模型模板

只有新条目使用受管模板。建议保留当前项目模板语义：

```json
{
  "id": "model-a",
  "name": "model-a",
  "vendor": "Custom",
  "url": "https://example/v1",
  "apiKey": "...",
  "supportsToolCall": true,
  "supportsImages": true,
  "supportsReasoning": true,
  "useCustomProtocol": false,
  "reasoning": {
    "defaultEffort": "max",
    "supportedEfforts": ["low", "medium", "high", "xhigh", "max"],
    "canDisableThinking": false
  }
}
```

注意：这是 FyAgent 的新建模板，不应描述为 WorkBuddy 对所有模型能力的自动检测结果。用户仍需根据上游实际能力自行修正配置。

## 9. 覆盖已有模型

### 9.1 允许修改的字段

仅：

```text
url
apiKey
```

不得调用当前用于新建条目的 `apply_managed_fields()`。

建议分离：

```rust
fn create_managed_model(...)
fn patch_existing_connection_fields(...)
```

### 9.2 URL

所有精确匹配条目统一写入本次规范化 Base URL。

### 9.3 API Key

```rust
fn patched_api_key(existing: Option<&str>, request: &Request) -> String {
    if !request.api_key.trim().is_empty() {
        request.api_key.clone()
    } else if request.clear_existing_api_keys {
        String::new()
    } else {
        existing.unwrap_or_default().to_owned()
    }
}
```

结果：

| 输入 | 已有条目 |
|---|---|
| 当前 Key 非空 | 全部改为当前 Key |
| 当前 Key 空，未选择清空 | 每个条目分别保留原 Key |
| 当前 Key 空，明确清空 | 全部改为空字符串 |

若历史重复条目原 Key 不同，空输入且未清空时应各自保留，不能用第一个条目的 Key 覆盖其他条目。

### 9.4 未知字段

以下字段无论值是否“看起来不合理”都必须保留：

```text
name
vendor
supportsToolCall
supportsImages
supportsReasoning
useCustomProtocol
onlyReasoning
reasoning
maxInputTokens
maxOutputTokens
任意未知扩展字段
```

只有用户另行编辑该模型时才可改变；本次 WorkBuddy 批量配置不是完整编辑器。

## 10. `availableModels` 算法

仅对象根适用。

```rust
fn update_available_models(root: &mut Map<String, Value>, target_ids: &[String]) {
    match root.get_mut("availableModels") {
        None => {}
        Some(Value::Array(items)) if items.is_empty() => {}
        Some(Value::Array(items)) => append_missing_string_ids(items, target_ids),
        Some(_) => return error_without_writing,
    }
}
```

规则说明：

- 缺失表示用户未启用限制，保持缺失；
- 空数组按已确认语义保持为空；
- 非空数组表示限制列表，保存目标必须追加，否则用户可能保存成功却无法在 WorkBuddy 中看到；
- 原有非字符串项属于配置错误，禁止写入，不自动删除；
- 比较采用大小写敏感精确值；
- 不删除已不存在于 `models` 的旧值，因为本次不是配置整理器。

## 11. revision 与覆盖令牌

### 11.1 revision

继续使用当前进程内 HMAC revision：

- 不暴露文件哈希细节；
- renderer 重载后必须重新获取；
- 文件字节任何变化都会导致 revision 变化；
- 适合检测 WorkBuddy 或用户编辑器的并发修改。

### 11.2 覆盖令牌

后端状态：

```rust
struct PendingOverwrite {
    request_digest: [u8; 32],
    expected_revision: Option<String>,
    existing_ids: Vec<String>,
    expires_at: Instant,
}
```

不应在 token 中可逆编码 API Key 或模型列表。应用退出、令牌过期或消费后立即删除。

建议有效期短于普通表单停留时间，例如 2–5 分钟；具体值作为后端常量和测试夹具，不在 UI 显示倒计时。

## 12. 备份与原子替换

### 12.1 备份时机

- 只有即将执行正式写入时创建；
- 覆盖预检不创建；
- 备份内容是本次事务前的原始字节；
- 继续保留单份备份策略；
- 文件不存在时无需创建空备份。

### 12.2 临时文件

- 与目标同目录；
- 随机唯一文件名；
- Unix 权限 `0600`；
- 写入、flush、fsync；
- 目标替换后同步父目录；
- 失败时尽力删除临时文件；
- 普通日志不写临时文件完整路径。

### 12.3 序列化

- 使用 `serde_json` preserve_order；
- 统一以换行结束；
- 可以使用 pretty JSON，但必须保持未知键顺序；
- 不承诺保留原始空白缩进，因为 JSON 不是注释保留格式；
- 文档中应明确“字段和值非破坏性”优先于“字节级排版完全不变”。

## 13. URL 规范化和模型获取安全

继续使用独立 WorkBuddy 获取服务，避免通用模型服务尝试多个未授权路径。

### 13.1 URL

- 只允许 `http`/`https`；
- 去除不影响语义的尾斜杠；
- 模型请求路径固定按当前 WorkBuddy 服务约定生成；
- 拒绝用户名/密码嵌入 URL；
- 限制 URL 总长度；
- 不允许 `file:`、`javascript:`、`data:` 等协议。

### 13.2 API Key

- `allowNoApiKey=false` 且 Key 为空时，在发请求前失败；
- Authorization 只发送给经验证的最终目标；
- 重定向必须执行同源/允许策略，不把 Key 发给未验证跨源目标；
- Key 不进入错误正文、响应摘要或日志。

### 13.3 响应

- 限制超时和最大响应体；
- 只接受预期 JSON schema；
- 限制模型数量，例如现有 1000 上限；
- 对 ID trim、去空、精确去重并保序；
- 不把上游 HTML 或错误正文直接展示给用户。

## 14. 前端 API Key 生命周期

建议状态边界：

```text
WorkBuddyPage mount
→ apiKey state + ref available
→ fetch/save/conflict/retry retain
→ route switches away and component unmounts
→ cleanup overwrite string and ref
```

防止泄漏：

- 不放入 URL、localStorage、sessionStorage、Tauri Store；
- 不放入 React Query key 或 cache data；
- 不放入全局状态管理；
- 不在 `console.log` 或错误对象序列化；
- 不在视觉测试夹具使用真实 Key；
- 测试使用固定占位符并验证日志中不存在该值。

React 字符串无法保证物理内存立即清零，因此文档只承诺缩短可达生命周期，不宣称可在 JavaScript 垃圾回收前完成安全擦除。

## 15. 用户可见错误映射

| 内部错误 | 用户文案原则 |
|---|---|
| invalid URL | `请输入有效的 HTTP 或 HTTPS 地址。` |
| Key required | `请输入 API Key，或启用“允许无 API Key”。` |
| fetch timeout/network | `获取模型失败，请检查网络和连接地址。` |
| invalid response | `模型接口返回了无法识别的数据。` |
| invalid JSON | `WorkBuddy 配置文件不是有效的 JSON。` |
| unsupported root/models | `WorkBuddy 配置结构无法识别，请检查 models.json。` |
| invalid entry | `现有模型配置包含无效条目，未进行写入。` |
| concurrent modification | `配置已被其他程序修改，请检查后重新保存。` |
| backup/write failure | `保存 WorkBuddy 配置失败，原配置未被替换。` |
| overwrite token invalid | 关闭旧对话框，刷新后提示重新保存 |

不展示 `invalidEntryIndex` 等专业细节；索引可以进入诊断日志。

## 16. 数据流示例

### 16.1 对象根覆盖

原始：

```json
{
  "models": [
    {
      "id": "model-a",
      "name": "自定义名称",
      "url": "https://old.example/v1",
      "apiKey": "old-a",
      "supportsImages": false,
      "custom": { "x": 1 }
    },
    {
      "id": "model-a",
      "url": "https://second.example/v1",
      "apiKey": "old-b",
      "maxInputTokens": 999
    }
  ],
  "availableModels": ["existing-filter"],
  "theme": "custom"
}
```

请求：

```text
ID=model-a
URL=https://new.example/v1
API Key=new-key
```

结果：

```json
{
  "models": [
    {
      "id": "model-a",
      "name": "自定义名称",
      "url": "https://new.example/v1",
      "apiKey": "new-key",
      "supportsImages": false,
      "custom": { "x": 1 }
    },
    {
      "id": "model-a",
      "url": "https://new.example/v1",
      "apiKey": "new-key",
      "maxInputTokens": 999
    }
  ],
  "availableModels": ["existing-filter", "model-a"],
  "theme": "custom"
}
```

### 16.2 空 Key 默认保留

同一原始文件，请求 Key 为空、未选择清空：

- 第一条继续 `old-a`；
- 第二条继续 `old-b`；
- URL 两条都更新；
- 不把第一条 Key 复制到第二条。

### 16.3 旧根数组新增

原始：

```json
[
  { "id": "old" }
]
```

新增 `new` 后仍为：

```json
[
  { "id": "old" },
  { "id": "new", "name": "new", "vendor": "Custom" }
]
```

不得自动改成对象根。

## 17. 已接受风险与限制

1. 历史重复 ID 继续保留，WorkBuddy 对同一数组内部重复项的最终生效规则未被官方明确保证；
2. 新建模板中的能力值是 FyAgent 默认值，不等于自动能力探测；
3. 深链接仍允许明文 API Key，可能在浏览器或第三方传递链路中暴露；
4. API Key 在页面内存中保留至页面卸载，便利性优先于最短内存生命周期；
5. JSON 写回不承诺原始缩进字节完全不变，但必须保留字段、值、顺序和未知扩展。

这些风险不得被实现文案包装成第三方官方保证。

## 18. 单元测试不变量

必须以属性/表驱动测试覆盖：

- 两种根形状读取与同形状写回；
- 对象根未知字段保留；
- 首次顺序静默去重；
- 大小写敏感 ID 比较；
- 历史重复全部更新且不删除；
- 已有条目只改 URL/Key；
- 空 Key 三种分支；
- `availableModels` 缺失、空、非空和非法类型；
- revision 变化中止；
- 覆盖令牌一次性、过期、请求摘要不符；
- 备份和正式写入失败不破坏原文件；
- 错误和日志不含固定测试 API Key。
