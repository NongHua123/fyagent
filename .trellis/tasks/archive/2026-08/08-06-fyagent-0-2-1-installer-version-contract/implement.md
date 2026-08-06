# FyAgent 0.2.1 实施清单

1. 复核工作树和参考包 manifest；将原始参考包保留为只添加、不改写的输入。
2. 先实现 Cargo workspace、helper manifest、version script 与 Node tests；同时迁移
   package/Tauri/Cargo readers 和旧 version consistency test，运行版本与 metadata
   回归测试，确认中间状态不会破坏 cross/macOS paths。
3. 建立 helper crate 的 portable skeleton、MSI property/log wrapper、stable error
   model、FFI exports 和 Windows policy modules；添加 pure/cfg tests 后运行 workspace
   format/clippy/test。
4. 替换 WiX 的旧 script CA，只修改相关 properties/actions/sequences/dialog path；
   保留无关 WebView/update actions、机器级模型、registry/ACL 和 upgrade logic。
5. 让 Windows cross build 与 release Windows steps 先编译每架构 DLL、注入 path 并
   扩展 MSI table/PE checks；为 WiX/installer structure 增加 regression tests。
6. 增加 release `version-contract`，改平台资产/manifest 读取渠道，保留 manual CI
   trigger、签名/公证和 unsigned branch artifact 路径；更新 workflow/manifest tests。
7. 运行范围化测试后运行完整 typecheck/format/unit/Rust/workspace/cross checks；复核
   diff、reference manifest 和不应出现的旧 validator 字符串。
8. 仅在真实 Windows x64/ARM64 候选环境补做 UI、silent、upgrade、repair、uninstall、
   ICE 与签名证据；在未执行前将其列为 release blocker。

## 回滚点

- 版本迁移完成并通过 `version:check` 后形成第一可验证切片。
- helper crate/policy 与 WiX 接线分别保持独立提交边界，避免目录安全逻辑与工作流
  改动无法隔离回滚。
- 不进行 tag、push、release 或远端 workflow 操作。
