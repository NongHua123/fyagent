# 下载源、元数据与验证设计

## 1. 运行时网络原则

本文件中的“中国大陆友好”仅针对最终产品用户运行时：

- FyAgent 安装器只依赖 `codexapp.agentsmirror.com`；
- 不把 GitHub、OpenAI 官网、Microsoft Store 网页或 Apple 页面放入必需运行链路；
- 开发 Agent 仍可正常使用全球开发资源；
- 已安装应用本身的登录和服务连接不由 FyAgent 控制。

## 2. 唯一 source provider

V1 只有一个实现：

```rust
AgentsMirrorSource
```

内置端点：

| 目的             | URL                                                  |
| ---------------- | ---------------------------------------------------- |
| Release manifest | `https://codexapp.agentsmirror.com/latest/manifest`  |
| SHA-256 清单     | `https://codexapp.agentsmirror.com/latest/checksums` |
| Windows x64      | `https://codexapp.agentsmirror.com/latest/win-x64`   |
| Windows ARM64    | `https://codexapp.agentsmirror.com/latest/win-arm64` |
| macOS ARM64      | `https://codexapp.agentsmirror.com/latest/mac-arm64` |

不实现：

- source 切换；-测速；
- official fallback；
- GitHub fallback；
- 自定义 URL；
- Intel endpoint 消费；
- Sparkle appcast 消费。

## 3. Source trait

```rust
#[async_trait]
pub trait ReleaseSource: Send + Sync {
    async fn resolve_latest(
        &self,
        platform: DesktopPlatform,
        arch: CpuArchitecture,
        cache_mode: CacheMode,
    ) -> Result<ReleaseDescriptor, InstallerError>;
}
```

`download_url` 必须在 source 内根据枚举选择，不能从 manifest 的任意外部 URL 直接信任，也不能接受 UI 输入。

## 4. 元数据获取顺序

推荐一致性流程：

```text
GET /latest/checksums
GET /latest/manifest
校验 manifest 原始字节哈希
解析 schema 和当前平台分支
交叉校验当前 artifact hash
生成 ReleaseDescriptor
```

开始安装时再次执行并绕过缓存：

```text
UI expected_release_id
      ↓
强制重新拉取 checksums + manifest
      ↓
重新计算 release_id
      ↓
一致 → 下载
不一致 → METADATA_CHANGED，刷新 UI
```

下载完成后不再次请求新的 checksum；使用该 Job 已锁定的 descriptor 验证，避免下载中途 latest 推进导致将新清单错误套到旧包。若服务端短链在元数据锁定后已切换到新包并导致 hash mismatch：

1. 删除文件；
2. 重新获取元数据；
3. 若 `release_id` 已变化，返回 `METADATA_CHANGED`，让用户确认新版本；
4. 若 `release_id` 未变化但 hash 仍不匹配，返回 `CHECKSUM_MISMATCH`；
5. 不把验证失败当普通网络重试。

## 5. Manifest 解析

镜像当前 manifest 是结构化 JSON，V1 parser 应：

- 仅接受已实现的 schema major/version；
- 未知 schema fail closed：`RELEASE_METADATA_INVALID`；
- 忽略已知 schema 下的未知字段；
- 只反序列化当前平台需要字段，不把外部 JSON 直接作为领域 DTO；
- 必需字段缺失、类型错误、空字符串、无效版本、无效 hash、size 为 0 均失败；
- Windows x64、ARM64 分支独立；
- macOS 使用 arm64 分支；
- root 聚合版本只能作为明确标注为跨平台的非操作性参考；当前平台卡片的“最新版本”展示和更新比较都使用该平台/架构已校验的
  版本。

建议保留镜像响应 fixture，并在开发时根据真实 schema 调整专用 raw DTO：

```rust
struct RawReleaseManifest { ... }
struct RawWindowsArchitecture { ... }
struct RawMacArchitecture { ... }
```

Raw DTO 不跨越 source 模块。

## 6. Checksums 解析

支持常见格式：

```text
<64-hex><two spaces><filename>
<64-hex><space>*<filename>
```

规则：

- UTF-8 文本；
- 单文件大小上限，例如 1 MiB；
- 忽略空行；
- 拒绝重复 filename 对应不同 hash；
- filename 按字节/精确字符串匹配，不做路径归一化；
- 拒绝包含 `/`、`\`、NUL 或 `..` 的目标 artifact 名；
- hash 转小写后比较；
- 必须包含 manifest 自身条目和目标 artifact 条目；
- manifest 内部 derived hash、checksums 文件和平台分支 hash 若同时存在，三者必须一致。

## 7. Manifest 自身完整性

使用 `checksums` 中 `release-manifest.json` 的 hash 校验从 `/latest/manifest` 获得的**原始响应字节**。不要先 parse/re-serialize 后再 hash。

顺序：

```text
bytes = GET manifest
expected = checksum_map["release-manifest.json"]
actual = sha256(bytes)
constant_time-ish equality for fixed-size digest
parse bytes as JSON only after match
```

若镜像 endpoint 的 Content-Disposition 名称不同，以明确协议字段为准；Agent 必须用真实响应 fixture 固化，不得临时放宽为“只要 JSON 可解析”。

## 8. Artifact 名称与平台映射

### Windows

从 manifest 当前架构分支读取 package moniker/filename，必须：

- 扩展名 `.msix`，大小写不敏感；
- 名称与架构分支一致；
- 下载仍使用固定短链，不把临时 CDN URL写入 descriptor；
- 实际 MSIX 内部 identity 最终确认产品。

### macOS ARM64

使用 manifest/derived checksum 对应的镜像 artifact 名；下载 URL 固定 `/latest/mac-arm64`。下载得到的 DMG 顶层 `.app` 可以叫 `ChatGPT.app` 或未来其他官方名称，不能据此拒绝；Bundle ID 是身份锚。

## 9. ReleaseDescriptor 缓存

```text
缓存内容：已完成所有元数据交叉校验的 ReleaseDescriptor
缓存位置：内存
TTL：5 分钟
key：platform + architecture
```

- 页面进入可以读未过期缓存；
- 刷新按钮 `force=true`；
- `start_install` 永远强制重验证；
- 失败不覆盖仍有效的本地状态；
- 不缓存预签名重定向 URL；
- 不写 SQLite、settings 或文件。

## 10. HTTP Client

安装器 client 必须复用 FyAgent 全局代理配置，但需要自己的安全参数：

```text
connect timeout: 30s
metadata request total timeout: 30s（或同量级固定值）
download total timeout: none / 足够长
per-read idle timeout: 60s
redirect: disabled in reqwest, manual handling
user-agent: FyAgent/<version> codex-desktop-installer
compression: metadata 可接受；artifact 最好 identity，避免长度歧义
```

不要给镜像请求附加：

- API Key；
- Codex/ChatGPT OAuth token；
- Provider token；
- Machine Key；
- 用户 ID；
- Cookie。

## 11. 重定向策略

初始 URL 必须精确来自内置常量。每一跳：

- 最多 5 次；
- 仅 `https → https`；
- 拒绝 userinfo；
- 正确解析相对 `Location`；
- 拒绝无效 URI；
- 不把初始 host 的敏感 header 转发到新 host；本实现本来就无敏感 header；
- 不永久硬编码最终 S3/R2 预签名 host；
- 可选防护：拒绝 URL 中的 loopback、link-local、私网 IP literal；不要对 DNS 结果做容易误伤 CDN 的固定 IP 白名单；
- 日志只记录 scheme/host/port/path，不记录 query/fragment。

最终内容仍必须通过 hash 和包身份验证。

## 12. 重试

总尝试：最多三次，即首次 + 自动重试两次。

### 可重试

- DNS/连接失败；
- connect timeout；
- read idle timeout；
- HTTP 408；
- HTTP 429；
- HTTP 5xx；
- 下载流意外中断。

### 不可重试

- 用户取消；
- HTTP 400/401/403/404；
- 重定向规则失败；
- manifest/checksum 解析失败；
- checksum 缺失/不匹配；
- identity/architecture/signature 错误；
- OS/policy/permission 错误；
- 磁盘不足。

退避建议：

```text
attempt 1 → immediate
attempt 2 → 1s + small jitter
attempt 3 → 3s + small jitter
```

429/503 有合理 `Retry-After` 时可尊重，但设置上限，避免 UI 长时间无响应。快照显示当前 attempt。

## 13. 下载写入

```text
<temp>/fyagent-codex-installer/<job-id>/installer.<msix|dmg>.part
```

完成后：

1. flush；
2. `sync_all`（合理情况下）；
3. 原子 rename 为最终 temp 文件；
4. 进入 hash 验证。

规则：

- 不写配置目录；
- 不写 `~/.codex`；
- 不把远程 filename 直接拼接为路径；
- 本地使用固定文件名，由平台枚举决定扩展名；
- 每次 stream chunk 检查取消令牌；
- 进度事件节流，例如每 100ms 或 1 MiB，避免淹没 WebView；
- 若实际字节超过 `expected_size` 的合理容差，立即停止；对于内容长度协议，应要求精确等于 expected size；
- hash 在 stream 中增量计算，并在文件完成后可选重新读取校验，至少确保磁盘最终文件的 digest 被验证。

## 14. 磁盘空间

D93 最终规则：

```text
required_free_space = expected_installer_size × 3
```

在所有参与卷上检查：

- 临时目录所在卷；
- macOS 目标 Applications 所在卷（若不同）；
- Windows all-users staging 的系统卷（实验路径，若不同）。

同一卷只检查一次。使用 checked arithmetic；溢出视为元数据错误。实际安装空间由 OS 最终判断，平台返回空间错误仍映射 `INSUFFICIENT_DISK_SPACE`。

## 15. 通用验证流水线

```text
1. descriptor 已验证
2. 预检 OS/arch/minimum version
3. 检查涉及卷空间 ≥ 3×size
4. 下载到固定 temp path
5. 实际 size == expected size
6. SHA-256 == expected hash
7. 平台包可解析
8. 平台 identity 精确匹配 Stable
9. 包架构匹配当前 CPU
10. 包版本与 descriptor 匹配
11. 平台发布者/Team/signature/OS trust 验证
12. 紧接实际平台消费前，重新打开受控 job 目录内的固定 artifact，确认 regular/non-link、
    size 和 SHA-256 仍精确匹配同一锁定 descriptor
13. 执行安装
14. OS 查询本地 Stable identity
15. 安装版本 >= 目标版本且不低于原本版本
```

任一步失败均不进入后续步骤。

## 16. 临时目录清理

```text
成功：立即删除整个 job 目录
取消：立即删除
失败：删除安装包和 metadata 原文；错误写日志
应用启动：删除超过 24 小时的 fyagent-codex-installer 子目录
```

清理必须：

- 只操作 canonicalized temp root 下的直接子目录；
- 不跟随外部 symlink；
- job ID 必须为 UUID；
- 清理失败记录 warning，不把成功安装改成失败。

## 17. 镜像不可用体验

本地已安装：

```text
已安装版本：x
暂时无法检查更新
[启动 Codex] [重试]
```

本地未安装：

```text
无法获取最新版
请检查网络或 FyAgent 的全局代理设置后重试
[重试]
```

不得提供“前往 GitHub/OpenAI 下载”作为 V1 必需补救路径。

## 18. 安全测试重点

- manifest hash 在 parse 前验证；
- malformed/oversized JSON；
- checksum duplicate/conflict；
- `../evil.msix` filename；
- redirect loop、HTTP downgrade、userinfo；
- query redaction；
- size overflow；
- release ID canonicalization collision；
- metadata changed between page check and start；
- short link switches artifact during download；
- package verification 后、平台 mount/deploy 前替换同长度 artifact；必须在重新校验时
  拒绝，且不得调用平台部署或挂载；
- cancel race before rename；
- old job event cannot mutate new job。
