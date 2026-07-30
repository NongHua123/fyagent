# Windows 适配设计

Windows trait implementation 将 manifest parser、PackageManager facade、AUMID launcher 与
实验 elevation 分层。WinRT/Win32 type 永不泄漏到通用 DTO；系统调用均通过可 fake 的
facade。正常 adapter 只暴露 `install_current_user(VerifiedPackage)`。

manifest parser 只读取固定 root entry，产出 exact identity、publisher、version、arch、
Application ID 和目标 OS 信息。PackageManager facade 从 current user inventory 读取
stable package，使用 `AddPackageByUriAsync(file://verified-msix)` 部署，并通过重新查询
确认 identity 与 `>= target` 版本。应用运行中决不使用 ForceApplicationShutdown 或
`taskkill`。

所有用户实验路径处于 Tauri runtime 之前。parent 产生受限、过期、nonce-bound job
描述；elevated child 只接受精确 command，确认 job 的 canonical temp path、ACL/重解析点、
hash 和 manifest 后才执行 stage/provision。普通 service 和 Tauri command 没有 scope
参数，因此不能误调用实验路径。
