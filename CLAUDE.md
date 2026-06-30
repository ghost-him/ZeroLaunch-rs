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
sdk/                → 平台抽象层 re-export 桥
core/               → 核心层（ConfigManager, Configurable trait, types）
plugin/             → 插件实现
plugin_system/      → 插件框架（SessionRouter, Pipeline, Registry, PluginManager, zlplugin:// 协议）
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
| 后端类型定义        | `src-tauri/src/plugin_system/types.rs` (re-export)                 |
| SessionRouter       | `src-tauri/src/plugin_system/session_router.rs`                    |
| ConfigManager       | `src-tauri/src/core/config/manager.rs`                             |
| HostApi + Builder   | `src-tauri/src/sdk/host_api.rs`                                    |
| 内置组件注册表      | `src-tauri/src/plugin_system/builtin_registry.rs`                  |
| Plugin Inspector    | `src-tauri/src/plugin_system/inspector.rs` (feature = "inspector") |
| PluginHostManager   | `crates/plugin-host/src/manager.rs`                                |
| 第三方插件加载器    | `src-tauri/src/plugin_system/manager.rs`                           |
| CLI HTTP 服务器     | `src-tauri/src/cli_server/server.rs`                               |
| 前端类型契约        | `src-ui-new/bridge/contract.ts`                                    |
| 前端命令封装        | `src-ui-new/bridge/commands.ts`                                    |
| 前端事件监听        | `src-ui-new/bridge/events.ts`                                      |

## IPC 命令清单

搜索/会话（`bridge_` 前缀，8个）：`bridge_query`, `bridge_confirm`, `bridge_wake`, `bridge_reset`, `bridge_get_session_mode`, `bridge_refresh_candidates`, `bridge_get_candidates_count`, `bridge_hide_window`

配置（`config_` 前缀，8个）：`config_get_all_components`, `config_get_schema`, `config_get_settings`, `config_apply_settings`, `config_reset_settings`, `config_set_enabled`, `config_get_actions`, `config_execute_action`

插件管理（`plugin_` 前缀，7个）：`plugin_list`, `plugin_get_manifest`, `plugin_install_local`, `plugin_reload`, `plugin_uninstall`, `plugin_set_enabled`, `plugin_get_logs`

检查器（`inspector_` 前缀，2个）：`inspector_get_state`, `inspector_simulate_query`

资源（`resource_` 前缀，2个）：`resource_get`, `resource_upload`

CLI（`cli_` 前缀，1个）：`cli_get_info`

## 从哪里开始

- **AI 约束规则** → `.claude/rules/`（general / plugin-system / sdk / frontend / commands / config / third-party-plugin / directory-map / data-flow）
- **设计哲学与架构** → `docs/design/`
- **前端架构与需求** → `docs/frontend/`
- **第三方插件开发** → `crates/plugin-api/README.md`
- **内置插件开发** → `docs/dev/built-in-plugin-guide.md`

<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tools** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them. `codegraph_node` returns one symbol's source + callers, or reads a whole file with line numbers. If the tools are listed but deferred, load them by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` and `codegraph node <symbol-or-file>` print the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->
