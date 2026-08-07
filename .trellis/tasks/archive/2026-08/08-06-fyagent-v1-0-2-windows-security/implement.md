# Windows 安全与发布实施计划

1. 先添加静态 manifest/WiX/workflow/handler 审计测试和 fake process/activation
   测试，明确现有 per-user、Portable、宽 IPC 和继承权限的失败行为。
2. 实现正式/测试 manifest 分离、per-machine WiX、目录/ACL/旧安装校验与发布
   签名门禁；不要运行真实 installer 或签名。
3. 在最早入口实现单业务实例/activation pipe；隔离启动顺序并用 fake 测试。
4. 建立普通用户启动服务，迁移已存在的浏览器、终端、编辑器、目录和 Codex
   业务动作；白名单记录 CLI 管理员例外。
5. 迁移 Windows autolaunch/设置显示，审计 `invoke_handler`、Capability、CSP、
   deep link 和 redaction；添加每类拒绝/令牌测试。
6. 运行静态/Rust/TS 检查，输出真实发布候选环境的手工验证清单。

主要文件：`src-tauri/build.rs`、`src-tauri/windows/*`、`src-tauri/wix/*`、
`src-tauri/src/{main,lib}.rs`、`src-tauri/src/platform/*`、
`src-tauri/src/security/*`、`src-tauri/src/auto_launch.rs`、
`src-tauri/capabilities/default.json`、`.github/workflows/*` 和对应测试。
