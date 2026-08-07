> **历史设计包：** 以下正文按原样保留，不再是当前版本、开发环境、任务 API 或
> CI/Release 合同的权威来源。当前入口见
> [v1-0.3.0](../v1-0.3.0/00-README.md) 与
> [mise task catalog](../../development/mise-tasks.md)。

# CC Switch `feature/fyagent-v1` 功能需求与设计文档包

> 文档基线核查日期：2026-08-03（Asia/Shanghai）  
> 文档性质：需求规格、架构与详细设计、配置与安全设计、验收场景  
> 明确不包含：实施任务拆解、工期、人员分工、开发排期、PR 清单

## 1. 文档包内容

| 文件 | 内容 |
|---|---|
| `01-需求规格说明.md` | 产品范围、业务规则、交互要求、兼容性和非功能要求 |
| `02-系统架构与详细设计.md` | 对当前私有分支的源码映射、组件边界、状态机、数据流和平台设计 |
| `03-配置数据模型与安全设计.md` | Codex TOML、WorkBuddy JSON、URL 规范化、凭据保护、原子写入与备份策略 |
| `04-验收标准与测试场景.md` | 可判定的验收口径、平台矩阵、异常与回归场景 |
| `assets/workbuddy-icon-original.webp` | 需求方指定 URL 下载的原始图标 |
| `assets/workbuddy-icon-512.png` | 512×512、透明外角、适用于高 DPI 的本地图标资源 |
| `assets/workbuddy-icon-256.webp` | 256×256、带 Alpha 的界面资源 |

## 2. 设计基线

### 2.1 项目方提供的 Git 基线声明

以下 Git 信息由项目方提供。上传 ZIP 不包含 `.git`，因此本文档不会声称已从 ZIP 或远程私有仓库独立验证这些 Git 对象。

| 项目 | 值 |
|---|---|
| 私有远程仓库 | `https://github.com/NongHua123/cc-switch.git` |
| 分支 | `feature/fyagent-v1` |
| 上游 | `origin/feature/fyagent-v1` |
| HEAD | `d6a21a0b5bb32533562e3d0099ea438b5e3c7ea4` |
| 短 SHA | `d6a21a0b` |
| 提交标题 | `ci: disable automatic PR checks` |
| 作者 / 提交者 | `pythonrust <pythonrust@foxmail.com>` |
| 作者 / 提交时间 | `2026-07-30 18:29:21 +08:00` |
| HEAD tree | `7851d323c491cd6c3c7a0e5099d3aa53e607f87f` |
| 根提交 | `e0a9c1ab4c46ecadf665dfb31dd967ce6f0019ac` |
| 与上游关系 | Ahead 0 / Behind 0 |
| 工作树 | 干净，无未提交或未跟踪文件 |
| tag / git describe | 无指向 HEAD 的 tag；`d6a21a0b` |

### 2.2 实际审阅的源码快照

- 上传文件：`cc-switch-feature-fyagent-v1(1).zip`
- ZIP SHA-256：`054cd9e07c714b78e8fd64e6562179b7b2feb77ff47e5e6abd92c77088141162`
- 前端版本：`3.18.0`
- Tauri/Rust 版本：`3.18.0`
- ZIP 中未包含 `.git`

本文档的目录、模块和改造边界均以该 ZIP 为源码基线；公开上游只作为 Codex、WorkBuddy 和平台 API 的协议资料，不作为代码合并基线。

## 3. 最终范围摘要

### 3.1 Codex

在所有 Codex 供应商新增/编辑表单的“高级选项”中，增加两个**供应商级**开关。高级区初始保持折叠；官方、普通第三方、保留名、托管 OAuth 和代理接管场景均可查看并修改：

1. **启用内置生图扩展**
   - 固定官方供应商默认关闭，其他 Codex 供应商默认开启；显式 TOML 始终优先。
   - 写入当前供应商的 `http_headers`：

     ```toml
     http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
     ```

   - 历史供应商使用延迟迁移，不在升级时批量改写。
   - 用户主动操作开关时，大小写不敏感地归一化或删除全部同名请求头；非法请求头字段按开关动作替换或删除，普通保存不做破坏性修复。

2. **启用 WebSocket 传输**
   - 默认关闭。
   - 开启时写入 `supports_websockets = true`；关闭时删除该字段。
   - Responses、Chat、Anthropic、官方、托管 OAuth 和代理场景均允许保存，不因格式切换自动关闭。

固定官方供应商只在首次实际开启能力时延迟生成保持 ChatGPT 登录语义的 `custom` Provider 表；全部关闭后，仅安全清理由本功能生成且未被用户扩展的骨架。

WebSocket 开启后，添加或编辑成功结果会按最终 Provider 返回结构化风险：任一已配置模型不是 `gpt-` 系列时提示模型风险，代理接管开启时提示链路风险；两项风险合并为一条 warning。提示不阻止保存，也不表示本地代理或第三方上游已经实现 WebSocket。当前本地代理仍只提供 HTTP/SSE，不实现 WebSocket Upgrade。

当一次操作实际改变当前生效的 `~/.codex/config.toml`，且可信 Codex 桌面应用正在运行时，弹出模态框询问是否重启。重启仅管理 Codex 桌面应用，不处理 CLI、IDE 插件或名称相似的其他进程。

### 3.2 WorkBuddy

在顶层应用切换器中，将 WorkBuddy 放在 Codex 后方并默认显示。WorkBuddy 是独立的一键配置页，不接入现有供应商、MCP、Skills、Prompt、Profile 或本地代理接管体系。

页面提供：

- URL；
- API Key；
- “允许无 API Key”开关，默认关闭；
- 自动获取上游模型；
- 模型自由勾选、全选、取消全选；
- 手动多行模型 ID；
- 保存到 `~/.workbuddy/models.json`。

生成或更新的目标模型默认：

- 工具调用：开启；
- 图片输入：开启；
- 思考模式：开启；
- 思考强度：`low`、`medium`、`high`、`xhigh`、`max`；
- 默认思考强度：`max`；
- 允许关闭思考：关闭；
- 仅思考模式：关闭；
- 自定义协议：关闭。

不安装、不检测、不升级、不启动、不关闭、不重启 WorkBuddy。

## 4. 关键工程结论

1. **WorkBuddy 不应加入后端现有 `AppType` 供应商域。** 它只需要顶层导航和独立配置命令。强行加入会使供应商查询、切换、MCP、Skills、用量、代理和当前供应商状态产生大量无意义分支。
2. **Codex 开关必须通过保留注释的 TOML 编辑器修改。** 当前项目已依赖 `toml_edit = "0.22"`；目标设计使用后端结构化补丁，不整体重新序列化用户配置。
3. **是否弹出重启框由后端返回的 live 配置实际变化结果决定。** 前端不能根据“点击了保存”进行猜测。
4. **WorkBuddy 获取模型使用独立的受限 HTTP 客户端。** 现有通用模型获取服务会尝试多个候选 URL、要求非空 API Key 并排序模型，不符合本需求。
5. **Windows 当前通用 `atomic_write` 会先删除目标文件再重命名，不能直接作为严格原子替换依据。** WorkBuddy 凭据文件需要更强的同目录原子替换实现。

## 5. 证据等级

| 等级 | 含义 | 本文采用方式 |
|---|---|---|
| A | 官方公开文档或标准 | Codex 配置键、WorkBuddy 产品行为、HTTP/平台 API |
| B | 上传源码直接审阅 | 现有模块、依赖、调用链、能力缺口 |
| C | 项目方真实样例或基线声明 | WorkBuddy JSON 字段、私有 Git 基线 |
| D | 设计推导 | 状态机、接口、冲突处理、原子事务方案 |

本文不会将 C/D 级信息表述为公开官方 Schema。

## 6. 外部资料

访问日期均为 2026-08-03。

1. OpenAI Codex Configuration Reference  
   https://developers.openai.com/codex/config-reference
2. OpenAI Codex Advanced Configuration  
   https://learn.chatgpt.com/docs/config-file/config-advanced
3. WorkBuddy 模型配置  
   https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Model
4. 腾讯云 WorkBuddy Enterprise 模型配置  
   https://cloud.tencent.com/document/product/1831/134445
5. WorkBuddy 更新日志  
   https://www.codebuddy.cn/docs/workbuddy/Changelog
6. Apple `NSRunningApplication`  
   https://developer.apple.com/documentation/appkit/nsrunningapplication
7. RFC 9110 — HTTP Semantics  
   https://datatracker.ietf.org/doc/html/rfc9110

## 7. 图标资产说明

原始 URL：

```text
https://ts3.tc.mm.bing.net/th/id/OIP-C.dD7vZFBOyBlyDPBExNfGrwHaGI?r=0&rs=1&pid=ImgDetMain&o=7&rm=3
```

原图为 474×392 WebP，包含白色主体细节。处理时不采用自动抠白算法，以避免把图标内部白色猫形误删；改为在透明外角的圆角正方形画布内等比适配原图，保留原始绿色渐变和白色细节。

## 8. 交付文件完整性

| 文件 | 字节 | SHA-256 |
|---|---:|---|
| `01-需求规格说明.md` | 18031 | `5237f4b8170d7a10c7d657eb6fce1637c614aad89cde104c4576514714bbb16f` |
| `02-系统架构与详细设计.md` | 20729 | `6ecb2b9fc434fe64e4b4812d3b5fc2b4f2a9ac6ad8273a0fe440d4c71e724e3e` |
| `03-配置数据模型与安全设计.md` | 14045 | `03bc31ca9f589d632fceb26808c5a817f5cc6a854ffe9b7f5be52f6c98b14cf3` |
| `04-验收标准与测试场景.md` | 15223 | `f4802d8fe4957051b90e0a71c7af05a64f704219c32a5cf8ac1608bb9f0b7b36` |
| `assets/workbuddy-icon-256.webp` | 26856 | `a595260ebae70e9064ebdbb4facda07c9f66cb60c864426bfe27e94b7cd7736a` |
| `assets/workbuddy-icon-512.png` | 91948 | `060e5e0fe1fce063e24b809a2d655df5a32ef36d97a7322e33b22c245570b868` |
| `assets/workbuddy-icon-original.webp` | 4442 | `224973c3bf4551c656f22fa4a59f0fd962f186619e1e8b5d7e299c867834edb3` |

> README 自身在追加本表后哈希会变化，因此不在表内自引用。
