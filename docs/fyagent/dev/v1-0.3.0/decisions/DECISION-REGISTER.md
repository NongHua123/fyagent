# 决策登记表（1–104）

> **交付状态**：Decision / 已决策  
> **关联决策**：1–104  
> **证据等级**：本文使用 `[Observed / 已核实]`、`[Decision / 已决策]`、`[Proposed / 拟实施]`、`[Pending Verification / 待验证]`。

本表是访谈决策的规范化摘要。若本表与主文档出现冲突，以本表、`01-REQUIREMENTS-AND-DECISIONS.md` 及用户最终授权共同解释；`35–38` 保留编号但明确废止。

| ID | 状态 | 决策 | 责任子任务 | 主要设计文档 |
|---:|---|---|---|---|
| 1 | Confirmed | 仅移除 Linux/WSL 到 Windows、macOS 的本地交叉构建体系；保留宿主平台原生开发与构建。 | Child 2: remove-local-cross-builds | `04-CROSS-BUILD-REMOVAL-DESIGN.md` |
| 2 | Confirmed | 保留五类 GitHub Release 目标：Windows x64/ARM64、Linux x64/ARM64、macOS Universal。 | Child 2: remove-local-cross-builds | `04-CROSS-BUILD-REMOVAL-DESIGN.md` |
| 3 | Confirmed | 保留 macOS runner 上的 universal-apple-darwin 构建。 | Child 2: remove-local-cross-builds | `04-CROSS-BUILD-REMOVAL-DESIGN.md` |
| 4 | Confirmed | 正式 Release 资产只能来自 GitHub Actions；本地产物仅用于开发、诊断和验收。 | Child 2: remove-local-cross-builds | `04-CROSS-BUILD-REMOVAL-DESIGN.md` |
| 5 | Confirmed | 保留历史记录并退役当前规范；不清洗不可变归档中的历史事实。 | Child 2: remove-local-cross-builds | `04-CROSS-BUILD-REMOVAL-DESIGN.md` |
| 6 | Confirmed | 标准版本声明文件是唯一事实源，Actions 是最终执行与验收权威。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 7 | Confirmed | Node.js 基线固定为 24.19.0。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 8 | Confirmed | Rust 基线固定为 1.97.1。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 9 | Confirmed | pnpm 保持 10.12.3，并消除重复版本声明。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 10 | Confirmed | GitHub Actions 不安装 mise，继续使用原生 setup actions。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 11 | Confirmed | 工具链完整补丁版本精确固定，通过受控 PR 更新，不自动合并。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 12 | Confirmed | 统一影响产物的工具链；本地辅助工具可独立存在（后由 uv/Python 决策进一步收敛）。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 13 | Confirmed | Required CI 与 Release 禁止使用 *-latest runner 标签。 | Child 4: modernize-ci-and-release | `06-CI-AND-RELEASE-DESIGN.md` |
| 14 | Confirmed | Linux Release 采用新宿主 runner 加同架构 Ubuntu 22.04 构建容器。 | Child 4: modernize-ci-and-release | `06-CI-AND-RELEASE-DESIGN.md` |
| 15 | Confirmed | 明确 runner：Windows x64=windows-2022、Windows ARM64=windows-11-arm、macOS=macos-15。 | Child 4: modernize-ci-and-release | `06-CI-AND-RELEASE-DESIGN.md` |
| 16 | Confirmed | PR/main/merge_group 自动 CI，并使用稳定的 Required 聚合门禁。 | Child 4: modernize-ci-and-release | `06-CI-AND-RELEASE-DESIGN.md` |
| 17 | Confirmed | 增加工具链静态声明检查与各 job 运行时版本检查。 | Child 4: modernize-ci-and-release | `06-CI-AND-RELEASE-DESIGN.md` |
| 18 | Confirmed | 所有 Actions 固定完整 commit SHA，并采用最小 GITHUB_TOKEN 权限。 | Child 4: modernize-ci-and-release | `06-CI-AND-RELEASE-DESIGN.md` |
| 19 | Confirmed | 正式 Release 采用来源资格、全矩阵预演、精确资产集合、摘要和 provenance 的失败关闭契约。 | Child 4: modernize-ci-and-release | `06-CI-AND-RELEASE-DESIGN.md` |
| 20 | Confirmed | 本地项目操作统一通过 mise run；mise 生命周期、Actions 和宿主包管理命令为明确例外。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 21 | Confirmed | 建立一对一基础 task 与组合 task 两层映射。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 22 | Confirmed | 保留 package.json scripts，由 mise tasks 作为仓库级稳定入口包装。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 23 | Confirmed | 允许 mise 的 auto_install 与 task.run_auto_install；不在项目层过度限制。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 24 | Confirmed | env:check、本地门禁、CI 与 Release 工具链检查统一 strict，偏差即失败。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 25 | Confirmed | 带参数 task 使用正式 usage 契约与输入校验。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 26 | Confirmed | 提供 trellis:* task；Trellis 命令必须使用项目统一的 Python 环境。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 27 | Confirmed | 开发 Python 环境改为 mise 管理 uv、uv 管理 Python/.venv/依赖，pyproject.toml 决定包管理。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 28 | Confirmed | 保留并尽量对齐 CC Switch 上游的产品运行时 mise 可选兼容能力。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 29 | Confirmed | 本次不另行设计 FyAgent 私有的产品运行时 mise 目录架构。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 30 | Confirmed | 产品中的 mise 兼容必须保持可选，不得演变为 FyAgent 启动或核心功能的硬依赖。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 31 | Confirmed | Windows ARM64 是正式支持的本地 mise/uv/Python 开发平台。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 32 | Confirmed | mise.lock 使用生成式、结构化契约测试，不以字符串包含作为充分证据。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 33 | Confirmed | 不启用 mise 全局 locked=true；正常安装优先使用 lockfile。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 34 | Confirmed | mise 最低版本按实际所需能力推导；忽略 mise.local.* 和项目 .venv。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 35 | Revoked | [Revoked / 已废止] 基于“删除产品运行时 mise 支持”的删除边界问题。 | Revoked / no implementation | `01-REQUIREMENTS-AND-DECISIONS.md` |
| 36 | Revoked | [Revoked / 已废止] 基于“拒绝 PATH 中 mise shim”的问题。 | Revoked / no implementation | `01-REQUIREMENTS-AND-DECISIONS.md` |
| 37 | Revoked | [Revoked / 已废止] 基于“删除 mise 来源类型”的问题。 | Revoked / no implementation | `01-REQUIREMENTS-AND-DECISIONS.md` |
| 38 | Revoked | [Revoked / 已废止] 基于“防止上游重新引入 mise 支持”的问题。 | Revoked / no implementation | `01-REQUIREMENTS-AND-DECISIONS.md` |
| 39 | Confirmed | 本次完整合并 CC Switch v3.19.2，而非只移植相关功能切片。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 40 | Confirmed | 以上游正式标签 v3.19.2 作为固定审计基准。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 41 | Confirmed | 上游共享功能默认跟随；FyAgent 差异必须有需求、证据和回归测试。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 42 | Confirmed | 在 Trellis 中建立独立 upstream-sync 长期规范。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 43 | Confirmed | 先完成隔离的 v3.19.2 上游合并，再实施工具链、CI、文档和 DEP0040 改造。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 44 | Confirmed | 使用保留上游祖先关系的显式 merge commit（--no-ff --no-commit）。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 45 | Confirmed | origin 可写、upstream fetch-only 且 push=DISABLED，并提供安全检查。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 46 | Confirmed | 冲突采用分层裁决，不全局使用 ours/theirs。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 47 | Confirmed | FyAgent 继续维护独立的 0.2.x 产品版本线。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 48 | Confirmed | 保留当前混合许可模型；新合入 CC Switch 代码继续按 MIT 来源记录。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 49 | Confirmed | 不建立上游合并专用产品验收层；使用普通 Required CI 和最终 Release workflow。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 50 | Confirmed | 完整合并后删除 CC Switch v3.19.2 release-note 文件，不将其包装为 FyAgent 发布说明。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 51 | Confirmed | 上游 merge commit 只做必要冲突裁决；工程治理重构放在后续独立提交。 | Child 1: merge-cc-switch-v3-19-2 | `03-UPSTREAM-MERGE-DESIGN.md` |
| 52 | Confirmed | 建立一个现代化 parent task 和六个独立 child tasks。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 53 | Confirmed | 重写仍有效的活动 spec，删除已经失效的活动 spec。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 54 | Confirmed | 项目 Trellis workflow 面向开发者/代理的命令统一为 mise run trellis:*。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 55 | Confirmed | 历史版本化开发文档保留原文，只增加归档与当前入口声明。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 56 | Confirmed | 交付 Trellis artifacts、长期 specs 和独立可读综合文档三层成果。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 57 | Confirmed | mise.toml 中 uv 使用 latest 选择器。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 58 | Confirmed | Python 精确开发基线为 3.14.7。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 59 | Confirmed | pyproject.toml 定义非包型开发环境，不把 FyAgent 变成 Python 包。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 60 | Confirmed | mise 只安装/包装 uv；Python 版本、解释器、.venv 和包只由 uv 管理。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 61 | Confirmed | 可重复 Python 依赖进入 pyproject.toml/uv.lock；一次性依赖隔离运行。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 62 | Confirmed | Codex hooks 也通过 mise+uv 执行，但 hook 不负责首次准备环境。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 63 | Confirmed | 采用命名空间化、稳定、可弃用治理的 mise task API。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 64 | Confirmed | 根配置、分域 task TOML、复杂跨平台 Node 脚本分层组织。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 65 | Confirmed | uv=latest 由 mise.lock 固定实际批准版本；升级通过受控 lock bump。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 66 | Confirmed | bootstrap 是首次初始化和环境重新同步的唯一高级入口。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 67 | Confirmed | 统一创建 .venv，但不将其全局注入所有 mise tasks。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 68 | Confirmed | env:check 与 system:check 拆分，二者 strict、只读、非修复。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 69 | Confirmed | 普通 Python/Trellis task 可按 uv.lock 同步；Codex hooks 必须 no-sync/offline。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 70 | Confirmed | mise run check 覆盖当前宿主门禁，但不声称完全等价多平台 Actions。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 71 | Confirmed | 组合 task 显式控制顺序、并行、交互和修改副作用。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 72 | Confirmed | mise 为 Trellis 提供稳定薄包装；详细子命令参数继续由 Python argparse 权威维护。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 73 | Confirmed | 提供宿主平台原生 dev/dev:renderer/build/build:binary/build:debug/build:renderer；禁止跨 OS/arch target。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 74 | Confirmed | 依赖更新按 Node/Rust/Python 生态隔离；全量更新必须显式 --all。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 75 | Confirmed | 提供 Node/Rust/pnpm/uv 独立工具链升级助手，不自动提交。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 76 | Confirmed | version:set/version:bump 默认预演，显式 --apply 才写文件。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 77 | Confirmed | 图标和视觉基线任务是显式修改型任务，不进入 bootstrap/check。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 78 | Confirmed | upstream:merge:prepare 最多进入未提交 merge 状态，不解决冲突、commit 或 push。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 79 | Confirmed | 提供分域 clean task 与需要确认的 clean:all，限制仓库内路径。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 80 | Confirmed | 本地只提供 release:check；正式发布只能由 GitHub Actions 完成。 | Child 3: redesign-mise-uv-development-environment | `05-MISE-UV-DEVELOPMENT-ENVIRONMENT.md` |
| 81 | Confirmed | DEP0040 必须消除根因，不能仅依赖 Node 24 默认隐藏 node_modules 告警。 | Child 5: eliminate-dep0040-punycode | `07-DEP0040-REMEDIATION-DESIGN.md` |
| 82 | Confirmed | 删除 cross-fetch/polyfill 与 cross-fetch 直接依赖，使用 Node 24 原生 Fetch API。 | Child 5: eliminate-dep0040-punycode | `07-DEP0040-REMEDIATION-DESIGN.md` |
| 83 | Confirmed | 不提供 Fetch polyfill 回退；缺少原生 API 时严格失败。 | Child 5: eliminate-dep0040-punycode | `07-DEP0040-REMEDIATION-DESIGN.md` |
| 84 | Confirmed | DEP0040 修复作为完整上游合并后的独立 FyAgent 变更。 | Child 5: eliminate-dep0040-punycode | `07-DEP0040-REMEDIATION-DESIGN.md` |
| 85 | Confirmed | 正常重生成 pnpm-lock.yaml，并验证旧依赖链退出。 | Child 5: eliminate-dep0040-punycode | `07-DEP0040-REMEDIATION-DESIGN.md` |
| 86 | Confirmed | 普通 Node 检查使用 --throw-deprecation；Fetch/MSW 探针额外使用 --pending-deprecation。 | Child 5: eliminate-dep0040-punycode | `07-DEP0040-REMEDIATION-DESIGN.md` |
| 87 | Confirmed | 禁止 NODE_NO_WARNINGS/--no-warnings/过滤 stderr 等抑制，以行为和依赖证据验收。 | Child 5: eliminate-dep0040-punycode | `07-DEP0040-REMEDIATION-DESIGN.md` |
| 88 | Confirmed | 迁移全部当前权威开发文档、Trellis 操作入口和自动化入口。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 89 | Confirmed | 四份 README 保持核心开发语义一致；详细 task 参考只维护一份。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 90 | Confirmed | 标准初始化为 mise trust → mise run bootstrap → mise run system:check → mise run dev。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 91 | Confirmed | 保留一个双语、自适应的 Pull Request 模板。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 92 | Confirmed | 迁移项目实际 Trellis 技能，不批量改写通用 trellis-meta 参考资料。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 93 | Confirmed | Codex hook 环境缺失时可见降级，不阻断用户 prompt/子代理。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 94 | Confirmed | Codex hooks 属于受支持开发集成，运行时无同步、无网络副作用。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 95 | Confirmed | 将当前所有未归档 Trellis tasks 归档。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 96 | Confirmed | 按关注点拆分长期 specs，并建立严格文档/task 漂移门禁。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 97 | Confirmed | 旧 Trellis tasks 归档为 superseded，不伪装成已经实施完成。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 98 | Confirmed | 本次交付只包含文档、Trellis artifacts 和文档级 overlay。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 99 | Confirmed | 文档/Trellis 提供可直接替换的完整稿；代码、配置、workflow 只给精确设计。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 100 | Confirmed | 所有交付统一标注已核实、已决策、拟实施、待验证。 | Child 6: migrate-docs-and-trellis-specs | `08-DOCUMENTATION-AND-TRELLIS-MIGRATION.md` |
| 101 | Confirmed | 分别记录项目方提供的 FyAgent 基线与待真实 Git 验证的上游完整 SHA。 | Child 6: migrate-docs-and-trellis-specs | `10-RISKS-AND-ACCEPTANCE.md` |
| 102 | Confirmed | 建立结构化风险登记与 GO/GO WITH CONDITIONS/NO-GO 门槛。 | Child 6: migrate-docs-and-trellis-specs | `10-RISKS-AND-ACCEPTANCE.md` |
| 103 | Confirmed | 最终 ZIP 使用分层目录并附 MANIFEST.sha256。 | Child 6: migrate-docs-and-trellis-specs | `10-RISKS-AND-ACCEPTANCE.md` |
| 104 | Confirmed | 最终授权仅允许生成文档和 ZIP，不实施代码、合并、远程修改或发布。 | Child 6: migrate-docs-and-trellis-specs | `10-RISKS-AND-ACCEPTANCE.md` |
