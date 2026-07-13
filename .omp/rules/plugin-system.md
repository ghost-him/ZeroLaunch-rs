---
description: 插件系统总览：inventory 自动注册、Configurable 生命周期、CandidatePipeline、SearchPipeline、事件驱动解耦
condition: ".*"
scope: "tool:read(src-tauri/src/builtin_plugin/**), tool:edit(src-tauri/src/builtin_plugin/**), tool:write(src-tauri/src/builtin_plugin/**), tool:read(src-tauri/src/plugin_framework/**), tool:edit(src-tauri/src/plugin_framework/**), tool:write(src-tauri/src/plugin_framework/**), tool:read(crates/plugin-api/src/plugin/**), tool:edit(crates/plugin-api/src/plugin/**), tool:write(crates/plugin-api/src/plugin/**), tool:read(crates/plugin-api/src/host/**), tool:edit(crates/plugin-api/src/host/**), tool:write(crates/plugin-api/src/host/**)"
interruptMode: never
---

# 插件系统总览

## 内置组件自动注册 (inventory)

- 内置组件通过 `inventory` crate 实现编译期自动发现。新增组件 **无需** 修改 `lib.rs`
- 每个组件文件底部通过 `::inventory::submit!` 块注册。`lib.rs` 的 `init_plugin_system` 在启动时通过 `builtin_registry::register_all_builtin_components()` 遍历所有条目并统一注册
- 7 种 Entry 类型对应 7 种组件类别：`ExecutorEntry`、`DataSourceEntry`、`KeywordOptimizerEntry`、`SearchEngineEntry`、`ScoreBoosterEntry`、`PluginEntry`、`ConfigEntry`
- `InventoryContext` 负责懒创建并缓存 `PluginHandle`，相同 `handle_key` 的组件共享同一个 handle
- `ConfigEntry` 配合 `ConfigComponentFactory` 用于纯配置组件（仅实现 `Configurable`，无其他领域 trait）
- 新增组件的步骤：在对应目录创建 .rs 文件 → 实现 trait → 添加 `inventory::submit!` 块 → `cargo build` 即自动生效

## Plugin Inspector (feature = "inspector")

- 开发用调试面板，**仅** 在 `cargo build --features inspector` 时启用
- 后端：`plugin_framework/inspector.rs` 维护 ring buffer (容量 200)，记录每次 `bridge_query` 的 trace_id、raw_query、mode、耗时
IPC：`inspector_get_state` 返回已注册组件清单 + 最近查询日志；`debug_simulate_query` 手动模拟查询返回原始 QueryResponse
- 前端：设置页 > 插件检查器 tab，包含组件清单表格、查询日志、模拟器

## Configurable 生命周期

配置变更遵循 5 步流水线：校验（validate_settings）→ 写入（apply_settings）→ 副作用（on_settings_changed）→ 广播 ConfigEvent → 持久化。

## 候选项管道（CandidatePipeline）

- `CandidatePipeline::collect()` 是异步方法，按注册顺序调用每个 DataSource 的 `fetch_candidates()`
- 采集后的候选项经过 KeywordOptimizer 链处理关键词
- 候选项缓存在 `SessionRouter` 中，通过 `bridge_refresh_candidates` 命令触发重新采集。搜索 **必须** 使用缓存数据

## SearchPipeline

- `SearchPipeline::search()` 接收查询和缓存候选项，返回排序后的 top_k 结果
- 当前搜索引擎通过 `config_set_enabled` 切换。**仅** 允许一个搜索引擎同时启用
- ScoreBooster 在搜索引擎打分后追加分数修正（历史频率、查询亲和度）

## 事件驱动解耦

- `PluginManager`（`plugin_framework/manager.rs`）不再持有 `ConfigManager` 直接引用。通过双通道事件总线与 `ConfigManager`、`SessionRouter` 事件驱动解耦：
  1. `PluginRuntimeEvent` 通道 — `PluginManager` 发布插件生命周期事件（加载/卸载/崩溃）
  2. `ConfigEvent` 通道 — `ConfigManager` 发布配置变更事件
  3. 三层解耦：`PluginManager` → `PluginRuntimeEvent` → `ConfigManager`（监听并同步注册/解注册） → `ConfigEvent` → `SessionRouter`（监听并重建管道）
- `PluginRuntimeEvent` 替代了已删除的 `AdapterRegistrar`（`plugin_framework/adapter_registrar.rs`）
- `ConfigEntry` 通过 `builtin_plugin/config/` 中的纯配置组件在 `builtin_registry.rs` 中注册（无需单独的 `core_registry.rs`），内置组件的所有 Entry 类型统一在 `builtin_registry` 中管理，消除循环依赖

## 依赖方向

见 `.omp/AGENTS.md` 的顶层目录职责表。
