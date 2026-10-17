---
description: 插件系统总览：inventory 自动注册、Configurable 生命周期、CandidatePipeline、SearchPipeline、事件驱动解耦
condition: "inventory::submit|collect_all_builtin_entries|PluginRuntimeEvent|Configurable|CandidatePipeline|SearchPipeline|builtin_registry"
scope: "tool:read(src-tauri/src/builtin_plugin/**), tool:edit(src-tauri/src/builtin_plugin/**), tool:write(src-tauri/src/builtin_plugin/**), tool:read(src-tauri/src/plugin_framework/**), tool:edit(src-tauri/src/plugin_framework/**), tool:write(src-tauri/src/plugin_framework/**), tool:read(src-tauri/src/core/**), tool:edit(src-tauri/src/core/**), tool:write(src-tauri/src/core/**), tool:read(crates/plugin-api/src/plugin/**), tool:edit(crates/plugin-api/src/plugin/**), tool:write(crates/plugin-api/src/plugin/**), tool:read(crates/plugin-api/src/host/**), tool:edit(crates/plugin-api/src/host/**), tool:write(crates/plugin-api/src/host/**)"
---

# 插件系统总览

## 内置组件自动注册 (inventory)

- 内置组件通过 `inventory` crate 实现编译期自动发现。新增组件 **无需** 修改 `lib.rs`
- 每个组件文件底部通过 `::inventory::submit!` 块注册。启动时 `bootstrap.rs` 的 `init_plugin_system` 经 `PluginManager::init_builtins` 调用 `builtin_registry::collect_all_builtin_entries()` 遍历所有条目并统一注册（注册编排的唯一权威是 PluginManager）
- 8 种 Entry 类型对应 8 种组件类别：`ExecutorEntry`、`DataSourceEntry`、`KeywordOptimizerEntry`、`KeywordInjectorEntry`、`SearchEngineEntry`、`ScoreBoosterEntry`、`PluginEntry`、`ConfigEntry`
- `InventoryContext` 负责懒创建并缓存 `PluginHandle`，相同 `handle_key` 的组件共享同一个 handle
- `ConfigEntry` 配合 `ConfigComponentFactory` 用于纯配置组件（仅实现 `Configurable`，无其他领域 trait）
- 新增组件的步骤：在对应目录创建 .rs 文件 → 实现 trait → 添加 `inventory::submit!` 块 → `cargo build` 即自动生效

## Plugin Inspector

- 开发用调试面板，**始终编译**，录制开关由 `AppState::is_debug_mode()` 运行时控制
- 后端：`plugin_framework/inspector.rs` 维护 ring buffer (容量 200)，记录每次 `bridge_query` 的 trace_id、raw_query、mode、耗时
IPC：`inspector_get_state` 返回已注册组件清单 + 最近查询日志
- 前端：设置页 > 插件检查器 tab，包含组件清单表格、查询日志

## Configurable 生命周期

配置变更遵循 5 步流水线：校验（validate_settings）→ 写入（apply_settings）→ 副作用（on_settings_changed）→ 广播 ConfigEvent → 持久化。

## 候选项管道（CandidatePipeline）

- `CandidatePipeline::collect()` 是异步方法，按注册顺序调用每个 DataSource 的 `fetch_candidates()`
- 采集后的候选项经过 KeywordOptimizer 链处理关键词
- 候选项缓存在 `SessionDispatcher` 中，通过 `refresh_candidates()` 触发重新采集（`bridge_refresh_candidates` 仅是 IPC 触发入口之一，配置变更/插件加载等经事件通道同样触发）。搜索 **必须** 使用缓存数据

## SearchPipeline

- `SearchPipeline::search()` 接收查询和缓存候选项，返回排序后的 top_k 结果
- 当前搜索引擎通过 `config_set_enabled` 切换。**仅** 允许一个搜索引擎同时启用
- ScoreBooster 在搜索引擎打分后追加分数修正（历史频率、查询亲和度）

## 事件驱动解耦

- `PluginManager`（`plugin_framework/manager.rs`）不再持有 `ConfigManager` 直接引用。通过双通道事件总线与 `ConfigManager`、`SessionDispatcher` 事件驱动解耦：
  1. `PluginRuntimeEvent` 通道 — `PluginManager` 发布插件生命周期事件（加载/卸载/崩溃）
  2. `ConfigEvent` 通道 — `ConfigManager` 发布配置变更事件
  3. 三层解耦：`PluginManager` → `PluginRuntimeEvent` → `ConfigManager`（监听并同步注册/解注册） → `ConfigEvent` → `SessionDispatcher`（监听并重建管道）
- `ConfigEntry` 通过 `builtin_plugin/config/` 中的纯配置组件在 `builtin_registry.rs` 中注册，内置组件的所有 Entry 类型统一在 `builtin_registry` 中管理

## 依赖方向

见 `.omp/AGENTS.md` 的顶层目录职责表。
