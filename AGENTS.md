# AGENTS.md — AI 快速导航

ZeroLaunch-rs 是基于 Bun + Tauri 2.x + Vue 3 + Native UI 的 Windows 快捷启动器。新系统采用三层架构。

## 架构

```
sdk/          → 平台抽象层（HostApi, traits, platform impls）
core/         → 核心层（ConfigManager, Configurable trait, types）
plugin/       → 插件实现（DataSource, Executor, SearchEngine, KeywordOptimizer, ScoreBooster, Plugin）
plugin_system/→ 插件框架（SessionRouter, PluginRegistry, CandidatePipeline, SearchPipeline, ExecutorRegistry）
```

**依赖方向**: `sdk/ ← core/ ← plugin/ ← plugin_system/` — 禁止反向依赖。

**前后端边界**: 前端负责数据显示与用户交互，后端负责数据持久化与逻辑控制。前端是"薄"展示层，所有业务逻辑、平台操作必须通过 IPC 委托后端。

## 关键文件

| 类别             | 路径                                            |
| ---------------- | ----------------------------------------------- |
| 后端 Bridge 命令 | `src-tauri/src/commands/bridge.rs`              |
| 后端 Config 命令 | `src-tauri/src/commands/config_file.rs`         |
| 后端类型定义     | `src-tauri/src/plugin_system/types.rs`          |
| SessionRouter    | `src-tauri/src/plugin_system/session_router.rs` |
| ConfigManager    | `src-tauri/src/core/config/manager.rs`          |
| 前端类型契约     | `src-ui-new/bridge/contract.ts`                 |
| 前端命令封装     | `src-ui-new/bridge/commands.ts`                 |
| 前端事件监听     | `src-ui-new/bridge/events.ts`                   |

## IPC 命令清单（15个）

搜索/会话（`bridge_` 前缀）：`bridge_query`, `bridge_confirm`, `bridge_wake`, `bridge_reset`, `bridge_get_session_mode`, `bridge_refresh_candidates`, `bridge_get_candidates_count`

配置（`config_` 前缀）：`config_get_all_components`, `config_get_schema`, `config_get_settings`, `config_apply_settings`, `config_reset_settings`, `config_set_enabled`, `config_get_actions`, `config_execute_action`

## 从哪里开始

- **AI 约束规则** → `.claude/rules/`（general / plugin-system / sdk / commands / config / data-flow）
- **设计哲学与架构** → `docs/design/`
- **前端架构与需求** → `docs/frontend/`
