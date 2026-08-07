# WorkBuddy v1.0.2 技术设计

## 后端模型与事务

将配置读取抽象为保留原根的文档模型：legacy array、object root 与 missing。
模型数组访问、唯一 ID 投影、严格条目校验和同形状序列化放在 `document.rs`；
`config.rs` 只编排锁、revision、目标归一化、备份和原子替换。对象根未知字段、
条目未知字段、JSON 键顺序和数组顺序必须保留。

首次保存锁内重读、验证 revision 和所有目标 ID。任一已有目标则生成后端存储的
短期一次性 `PendingOverwrite`，令牌绑定规范化目标、revision、base URL 摘要、
API Key 的进程内不可逆摘要、清空 Key 标记及 nonce；返回唯一现有 ID，不写文件。
消费令牌后再次锁内验证，创建和覆盖在同一事务中完成。

## IPC 与前端状态

`WorkBuddyStatus` 只返回 exists、唯一模型数、opaque revision、backup 和 format；
“配置位置”使用稳定相对显示值而非真实全路径。新增 `get_workbuddy_model_ids`，
返回首次出现顺序的唯一 ID 和 revision。状态、ID 与保存结果使用独立 query key；
query/cache 从不持有 Key。

WorkBuddy 页面是唯一持有 API Key 的组件。其卸载 cleanup 清空 key；成功保存只
invalidate status/IDs，不能清空 key。通用筛选行为从现有模式中提炼为有限 hook：
`Set` 保存选择、输入即时过滤、全选/清空只影响当前可见项。页面卡片各管局部
展示；不要建需要大量 render prop 的万能列表。

## 测试与回滚

Rust 使用临时 HOME/注入路径和 mock HTTP，验证文档、token、竞争、backup、
redaction；React 使用 Tauri mock，验证 key 生命周期、筛选和覆盖快照。后端 DTO
与页面 UI 必须成组回滚；卡片布局可以独立回滚。
