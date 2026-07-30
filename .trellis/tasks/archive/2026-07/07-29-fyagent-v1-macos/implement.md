# macOS 实施计划

1. 建立 cfg module、fake command runner、fixture parser 和 unsupported/Intel tests。
2. 实现标准目录 scan、Bundle ID/Team/Info.plist/version/arch/codesign/spctl verifier。
3. 实现运行检测、DMG attach plist parser、unique app discovery 和 detach guard。
4. 实现 target planner、permission-only fallback、conflict detection、same-volume staging/swap/
   restore 和 post-install scan。
5. 实现 verified-path launch 与完整 command/filesystem failure matrix。
6. 在 macOS runner 完成编译/fixture tests，记录无真实 DMG 挂载或 Applications 写入。
