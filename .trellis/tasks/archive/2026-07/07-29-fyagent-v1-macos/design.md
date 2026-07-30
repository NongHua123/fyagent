# macOS 适配设计

adapter 将 local scan、bundle verifier、running detector、DMG mount/discovery 和 safe swap
拆分为可 fake 单元。所有系统命令经过 `CommandRunner`，文件系统操作通过受限 helper，
使 unit tests 无法触及真实 `/Applications`。

scan canonicalize 后仅接受两个标准 parent 下的 top-level app。对目标 ID 继续读取
Info.plist、Team、arch、codesign、spctl；多 Stable 返回 `Ambiguous`。DMG mount 使用
RAII detach guard，从 structured plist 找 mount point，再发现唯一 top-level app 并完整校验。

新装路径由 source basename 选择，existing Stable 更新保持旧实际路径。copy 到 same-volume
staging，重验后新装原子 rename；更新将 existing verified target 暂改为 backup、swap staging
并 post-verify，任意失败立即 restore。所有 staging/backup 名随机且受目标 parent 限制。
