# 后续版本路线图

> 本文只记录已讨论但明确不进入 V1 的目标。Agent 不得在 V1 顺手实现。

## 1. Clean break 后续边界

### V1.x / V2 候选

- 对新 FyAgent 身份的安装、卸载与数据清理体验；
- 新旧应用并行存在时的明确人工支持文档；
- 仅在未来获得独立产品批准时，评估隔离的手动导入/转换工具。

V1 已采用独立 Identifier、AppUserModelID/Bundle ID、`fyagent://`、
`~/.fyagent`、FyAgent 数据库/日志/主题 key、图标与内部包名。
当前合同是 clean break：不对旧目录、协议、自启动值或序列化标记做迁移或兼容读取。

## 2. FyAgent 自更新

- 自有 release渠道；
- 自有 updater签名密钥；
- 不复用上游 CC Switch endpoint/pubkey；
- 中国大陆用户运行时可达的更新分发；
- 灰度、回滚、签名和发布流程。

V1 关闭更新，不用占位 endpoint。

## 3. 下载能力

- 断点续传；
- 跨重启恢复；
- 分块校验；
- 下载速度/ETA；
- 离线安装包；
- 下载缓存；
- 带宽限制。

前置：持久化 Job、Range契约、镜像稳定性和隐私设计。

## 4. 多源

可能包括：

- 第二个中国大陆友好镜像；
- 官方源作为显式可选线路；
- 企业内部镜像；
- source健康探测。

约束：任何默认或自动线路仍需满足用户运行时中国大陆友好。官方/GitHub fallback不得仅因为“存在”就自动加入；需要明确可达性和用户预期。

## 5. 平台扩展

- macOS Intel（以官方包持续存在和真机为前提）；
- Windows all-users正式支持；
- Store license/依赖处理；
- 企业部署文档；
- Beta通道；
- Stable/Beta并存管理。

## 6. 生命周期

- 修复安装；
- 卸载；
- 干净卸载可选项；
- 版本回滚；
- 历史版本选择；
- 保留/清理用户数据选项；
- 安装完成引导；
- 独立安装中心页面。

这些功能涉及破坏性操作，必须有独立采访和验收，不从 V1隐含推导。

## 7. macOS 更新增强

- Sparkle appcast；
- delta选择；
- EdDSA验证；
- 失败回退完整包；
- 版本间回滚；
- 原子 replacement helper。

V1只做完整 DMG。

## 8. 诊断与支持

- 一键诊断 ZIP；
- 可选匿名崩溃上报；
- 用户明确授权上传；
- 环境快照；
- 安装历史；
- 可观测性 dashboard。

必须先定义隐私政策和脱敏测试。

## 9. 发行与签名

- Apple Developer Program；
- Developer ID Application；
- Notarization；
- Windows Authenticode OV/EV/云签名；
- CI密钥托管；
- SBOM；
- provenance；
- 安装器自身的供应链安全。

这些针对 FyAgent宿主，不改变目标 OpenAI包保持官方原签名的原则。

## 10. 授权系统

若未来恢复商业授权：

- 独立 `InstallPolicy`接口；
- 后端/硬件策略可插拔；
- V1默认无鉴权；
- 不直接恢复旧 VibeKey设备耦合；
- 必须重新做离线、失败、隐私和可用性设计。

## 11. Provider 联动

候选：

- 安装后打开 FyAgent Codex配置；
- 检测官方登录状态；
- 提示 Provider配置；
- 可选启动工作目录。

不得自动覆盖 `~/.codex` 或切换 Provider，除非未来明确确认。

## 12. 优先级建议

| 优先级 | 候选 |
|---|---|
| P1 | FyAgent自更新、中国大陆分发、代码签名 |
| P2 | 断点续传、诊断 ZIP、正式 all-users、修复/卸载 |
| P3 | 多源、历史版本、rollback、Sparkle delta、Intel Mac |
| P4 | 授权系统、Provider联动、安装中心 |

## 13. V1 防越界检查

出现以下词时，Agent应确认是否误入 roadmap：

```text
resume
persistent job
uninstall
repair
rollback
delta
appcast
beta channel
custom source
source selector
telemetry
license server
hardware key
migration wizard
```
