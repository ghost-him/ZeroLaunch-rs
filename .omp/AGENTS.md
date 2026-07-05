# AGENTS.md — AI 快速导航

ZeroLaunch-rs 是基于 Bun + Tauri 2.x + Vue 3 + Naive UI 的 Windows 快捷启动器。采用 Cargo workspace 多 crate 架构。

## 架构（Cargo Workspace）

```
crates/plugin-api/        → zerolaunch-plugin-api: traits, 数据类型, HostApi/PluginHandle
crates/plugin-protocol/   → zerolaunch-plugin-protocol: JSON-RPC 协议定义, manifest schema
crates/plugin-host/       → zerolaunch-plugin-host: 子进程管理, RemotePluginAdapter, 双向 RPC
crates/plugin-sdk-rust/   → zerolaunch-plugin-sdk-rust: 第三方 Rust 插件 SDK
crates/platform-windows/  → zerolaunch-platform-windows: Windows 平台实现 + 工厂函数
zerolaunch-cli/           → zerolaunch-cli: 独立 CLI 工具 (zl query / zl plugins)
plugin-template/          → 第三方 Rust 插件项目模板 (不在 workspace)
src-tauri/                → zerolaunch-rs: 主程序、内置插件、配置管理、IPC 命令
```

**依赖方向**: `plugin-api ← plugin-protocol ← plugin-host ← src-tauri` 和 `plugin-api ← plugin-sdk-rust` — 禁止反向依赖。

**内部模块**（src-tauri 内）:
```
bootstrap/          → 应用启动初始化
sdk.rs              → 平台抽象层 re-export 桥（单文件）
core/               → 核心层（ConfigManager, Configurable trait, types）
builtin_plugin/     → 内置插件实现
plugin_framework/   → 插件框架（SessionRouter, Pipeline, Registry, PluginManager, zlplugin:// 协议）
cli_server/         → 本地 HTTP API 服务器（axum）
commands/           → IPC 命令（bridge, config, plugin, resource, inspector, cli）
state/              → AppState 定义
utils/              → 通用工具
logging/            → 日志初始化
```

**前后端边界**: 前端负责数据显示与用户交互，后端负责数据持久化与逻辑控制。前端是"薄"展示层，所有业务逻辑、平台操作必须通过 IPC 委托后端。

## 关键文件

| 类别                | 路径                                                               |
| ------------------- | ------------------------------------------------------------------ |
| Plugin SDK (traits) | `crates/plugin-api/src/services/`                                  |
| Plugin SDK (host)   | `crates/plugin-api/src/host/`                                      |
| Windows 平台实现    | `crates/platform-windows/src/`                                     |
| 后端 Bridge 命令    | `src-tauri/src/commands/bridge.rs`                                 |
| 后端 Config 命令    | `src-tauri/src/commands/config_file.rs`                            |
| 后端类型定义        | `src-tauri/src/plugin_framework/mod.rs` (re-export)               |
| SessionRouter       | `src-tauri/src/plugin_framework/session_router.rs`                |
| ConfigManager       | `src-tauri/src/core/config/manager.rs`                             |
| HostApi + Builder   | `src-tauri/src/sdk.rs`                                             |
| 内置组件注册表      | `src-tauri/src/plugin_framework/builtin_registry.rs`              |
| Plugin Inspector    | `src-tauri/src/plugin_framework/inspector.rs` (feature = "inspector") |
| PluginHostManager   | `crates/plugin-host/src/manager.rs`                                |
| 第三方插件加载器    | `src-tauri/src/plugin_framework/manager.rs`                       |
| CLI HTTP 服务器     | `src-tauri/src/cli_server/server.rs`                               |
| 前端类型契约        | `src-ui/bridge/contract.ts`                                    |
| 前端命令封装        | `src-ui/bridge/commands.ts`                                    |
| 前端事件监听        | `src-ui/bridge/events.ts`                                      |

## IPC 命令清单

搜索/会话（`bridge_` 前缀，8个）：`bridge_query`, `bridge_confirm`, `bridge_wake`, `bridge_reset`, `bridge_get_session_mode`, `bridge_refresh_candidates`, `bridge_get_candidates_count`, `bridge_hide_window`

配置（`config_` 前缀，9个）：`config_get_version`, `config_get_all_components`, `config_get_schema`, `config_get_settings`, `config_apply_settings`, `config_reset_settings`, `config_set_enabled`, `config_get_actions`, `config_execute_action`

插件管理（`plugin_` 前缀，7个）：`plugin_list`, `plugin_get_manifest`, `plugin_install_local`, `plugin_reload`, `plugin_uninstall`, `plugin_set_enabled`, `plugin_get_logs`

检查器（`inspector_` 前缀，2个）：`inspector_get_state`, `inspector_simulate_query`

资源（`resource_` 前缀，2个）：`resource_get`, `resource_upload`

CLI（`cli_` 前缀，1个）：`cli_get_info`

## 目录结构与文件放置规范

### Workspace 结构（crate 维度）

```
ZeroLaunch-rs/                          ← Cargo workspace 根
├── Cargo.toml                          ← [workspace] 定义 + [workspace.dependencies]
├── crates/
│   ├── plugin-api/                     ← zerolaunch-plugin-api
│   │   └── src/
│   │       ├── config/                 ← Configurable trait, SettingDefinition, ComponentType 等
│   │       ├── host/                   ← HostApiError, OpenTarget, CacheLevel, PluginHandle, PluginSdkConfig
│   │       ├── platform/               ← PlatformCapability, PlatformCapabilities
│   │       ├── plugin/                 ← Plugin trait, Query/QueryResponse, 组件 trait, CachedCandidateData
│   │       ├── services/               ← 能力域 trait（具体数量以代码为准；新增域时**必须**在此登记 + sdk.md 同步更新）
│   │       ├── common/                 ← DirUtils, ImageUtils
│   │       └── mock/                   ← Stub 实现 + mock_plugin_handle()（feature = "mock"）
│   ├── plugin-protocol/                ← zerolaunch-plugin-protocol
│   │   └── src/                        ← JSON-RPC 消息体, 方法名常量, manifest schema, 错误码, codec (LSP Content-Length 帧编解码)
│   ├── plugin-host/                    ← zerolaunch-plugin-host
│   │   └── src/                        ← 子进程管理, transport, JsonRpcClient, RemotePluginAdapter
│   ├── plugin-sdk-rust/                ← zerolaunch-plugin-sdk-rust
│   │   └── src/                        ← Rust 第三方插件 SDK (run() + HostProxy)
│   └── platform-windows/               ← zerolaunch-platform-windows
│       └── src/                        ← Windows 平台 trait 实现 + windows_capabilities()
├── zerolaunch-cli/                     ← zerolaunch-cli (独立 bin crate)
├── plugin-template/                    ← Rust 第三方插件项目模板（不在 workspace，显式 exclude）
└── src-tauri/                          ← zerolaunch-rs（主程序）
    └── src/
        ├── bootstrap.rs               ← 应用启动初始化（从 lib.rs 提取）
        ├── sdk.rs                      ← re-export 桥（类型本体在 plugin-api / platform-windows）
        ├── bridge_error.rs            ← BridgeError（IPC 错误）
        ├── core/                       ← ConfigManager, ConfigStore, 核心配置组件
        ├── builtin_plugin/             ← 内置插件实现（具体数量以代码为准）
        ├── plugin_framework/           ← SessionRouter, Pipeline, Registry, PluginManager
        ├── tray/                       ← 系统托盘管理
        ├── window/                     ← 窗口位置工具函数
        ├── commands/                   ← IPC 命令薄代理
        ├── state/                      ← AppState
        └── utils/                      ← 通用工具
```

**依赖方向**: `plugin-api ← plugin-protocol ← plugin-host ← src-tauri`、`plugin-api ← platform-windows ← src-tauri`、`plugin-api ← plugin-sdk-rust` — 禁止反向依赖。

- **第三方插件作者**只依赖 `zerolaunch-plugin-api`，不需要 Tauri / Windows / 主程序源码
- **新增 SDK trait**：在 `crates/plugin-api/src/services/<domain>/` 定义
- **新增 Windows 实现**：在 `crates/platform-windows/src/` 实现对应的 trait
- **src-tauri 中的 sdk.rs** 现为 re-export 桥，类型本体已迁至 plugin-api

### 后端 (src-tauri/src/) 顶层目录职责

| 目录 | 职责 | 可引用 | 禁止 |
|------|------|--------|------|
| `sdk.rs` | re-export 桥（类型本体在 plugin-api / platform-windows） | 无外部依赖 | 引用 core/、builtin_plugin/、plugin_framework/ |
| `core/` | 业务核心：ConfigManager、Configurable trait、类型定义 | sdk.rs | 引用 builtin_plugin/、plugin_framework/ |
| `builtin_plugin/` | 内置插件实现：DataSource、Executor、SearchEngine 等 | sdk.rs、core/ | 引用 plugin_framework/ |
| `plugin_framework/` | 插件框架：SessionRouter、Pipeline、Registry、PluginManager | sdk.rs、core/、builtin_plugin/ | 被其他层反向引用 |
| `tray/` | 系统托盘管理 | state/ | 包含业务逻辑 |
| `window/` | 窗口位置工具函数 | 无外部依赖 | 包含业务逻辑 |
| `commands/` | IPC 命令：薄代理层，仅委托 | 全部 | 包含业务逻辑 |
| `state/` | AppState 定义 | core/、plugin_framework/ | 包含业务方法 |
| `utils/` | 通用工具（locale、font_database 等） | 无限制 | 包含业务逻辑 |

### 各目录详细说明

#### `core/` — 业务核心
```
core/
├── constants.rs         ← 应用常量
├── config/              ← 配置系统
│   ├── manager.rs       ← ConfigManager 主调度器
│   ├── store.rs         ← ConfigStore（JSON 持久化）
│   ├── models.rs        ← 配置数据模型
│   ├── registry.rs      ← ConfigurableRegistry
│   ├── event.rs         ← ConfigEvent + PluginRuntimeEvent 广播
│   ├── setting_builders.rs ← SchemaBuilder API
│   └── mod.rs           ← 模块入口
```

- **核心配置组件**（`builtin_plugin/config/` 下的组件，如 AppearanceConfig、HotkeyConfig 等）：不属于任何插件的系统级配置
- **新增核心配置组件** 时放 `builtin_plugin/config/`。**新增插件** 时放 `builtin_plugin/` 对应子目录

#### `builtin_plugin/` — 内置插件实现
```
builtin_plugin/
├── config/               ← 核心配置组件（非插件系统级配置）
├── _template/            ← 内置插件模板（不被编译或 glob 扫描）
├── data_source/          ← 数据源（具体数量以代码为准）
├── executor/             ← 执行器（具体数量以代码为准）
├── keyword_optimizer/    ← 关键词优化器（具体数量以代码为准）
├── score_booster/        ← 分数增强器（具体数量以代码为准）
├── search_engine/        ← 搜索引擎（具体数量以代码为准）
└── triggerable/          ← 可触发插件（具体数量以代码为准）
```

- 每个插件实现 `Configurable` trait（配置）+ 对应的领域 trait（如 `DataSource`、`ActionExecutor`）。**必须** 通过 `PluginHandle` 访问平台能力
- 新增插件在对应目录添加 .rs 文件 + `inventory::submit!` 块即自动注册，**无需** 修改 `lib.rs`

#### `plugin_framework/` — 插件框架
```
plugin_framework/
├── builtin_registry.rs   ← inventory 自动发现与注册编排器
├── builtin.rs            ← 内置插件定义
├── inspector.rs          ← Plugin Inspector 调试面板 (feature = "inspector")
├── session_router.rs     ← 搜索会话路由（核心调度器）
├── candidate_pipeline.rs ← 候选项采集管道
├── search_pipeline.rs    ← 搜索排序管道
├── executor_registry.rs  ← 执行器注册表
├── manager.rs            ← 第三方插件生命周期管理（单入口）
├── host_handler.rs       ← 子进程 Host 管理
├── plugin_installer.rs   ← 插件安装/卸载逻辑（从 manager.rs 提取）
├── plugin_info.rs        ← 插件信息类型
├── zlplugin_protocol.rs  ← zlplugin:// 自定义协议处理（从 manager.rs 提取）
├── service.rs            ← PluginService
├── registry.rs           ← PluginRegistry
└── mod.rs                ← 模块入口（含 re-export，类型定义在各自模块内，已消除冗余 types.rs shim）
```

- **SessionRouter** 是运行时的中枢。所有 bridge 命令通过它路由
- **builtin_registry** 通过 `inventory` 在编译期收集所有内置组件，启动时统一注册
- **manager.rs** 是第三方插件生命周期的唯一入口，通过 `PluginRuntimeEvent` 广播通道与 ConfigManager/SessionRouter 事件驱动解耦
- **禁止** 在此层定义配置 schema 或持久化逻辑（那属于 core/）

#### `cli_server/` — 本地 HTTP API 服务器
```
cli_server/
├── mod.rs                ← 模块入口
├── server.rs             ← axum 服务器启动与配置
├── middleware.rs          ← 认证中间件（Bearer token 校验）
├── token.rs              ← CLI token 管理
└── routes/
    ├── mod.rs            ← 路由聚合
    ├── query.rs          ← /v1/query 查询端点
    ├── session.rs        ← /v1/session 会话端点
    ├── plugins.rs        ← /v1/plugins 插件管理端点
    └── config.rs         ← /v1/config 配置端点
```

### 新文件放置决策树

```
新增一个功能 →
├─ 需要平台 API？
│  └─ 是 → 在 crates/plugin-api/src/services/ 定义 trait
│         → 在 crates/platform-windows/src/ 实现
│         → 在 HostApi 添加方法，通过 PluginHandle 暴露
├─ 是系统级配置（非插件）？
│  └─ 是 → 放 builtin_plugin/config/
├─ 是新的数据源/执行器/搜索引擎？
│  └─ 是 → 放 builtin_plugin/ 对应子目录
├─ 需要新的 IPC 命令？
│  └─ 是 → 放 commands/ 对应文件（按前缀规则）
├─ 需要前端新页面？
│  └─ 是 → 放 views/
├─ 是可复用的前端逻辑？
│  └─ 是 → 放 composables/
└─ 其他
   └─ 放 utils/
```

## 文件命名约定

- Vue 组件文件名使用 `PascalCase`（如 `DynamicForm.vue`）
- TypeScript 工具文件使用 `camelCase`（如 `schemaTypes.ts`）
- Store 文件使用 `kebab-case` + `-store` 后缀（如 `config-store.ts`）
- Composable 文件使用 `camelCase` + `use` 前缀（如 `useKeyboard.ts`）

## 从哪里开始

- **AI 约束规则** → `.omp/RULES.md`（始终生效的工程纪律）+ `.omp/rules/`（TTSR 条件规则，按领域触发）
- **设计哲学与架构** → `docs/design/`
- **前端架构与需求** → `docs/frontend/`
- **第三方插件开发** → `crates/plugin-api/README.md`
- **内置插件开发** → `docs/dev/built-in-plugin-guide.md`

## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tools** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them. `codegraph_node` returns one symbol's source + callers, or reads a whole file with line numbers. If the tools are listed but deferred, load them by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` and `codegraph node <symbol-or-file>` print the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
