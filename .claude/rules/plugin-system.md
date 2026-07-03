---
paths:
  - "src-tauri/src/builtin_plugin/**"
  - "src-tauri/src/plugin_framework/**"
  - "crates/plugin-api/src/plugin/**"
  - "crates/plugin-api/src/host/**"
---

# 插件系统规范

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
- IPC：`inspector_get_state` 返回已注册组件清单 + 最近查询日志；`inspector_simulate_query` 手动模拟查询返回原始 QueryResponse
- 前端：设置页 > 插件检查器 tab，包含组件清单表格、查询日志、模拟器

## Configurable 生命周期

配置变更的 5 步流水线及各方法的职责边界见 [config.md](config.md) 的 Configurable 生命周期规范。参照实现：`HotkeyConfigComponent`、`InstallationMonitorConfigComponent`。

## ExecutorRegistry

- `ExecutorRegistry::resolve(ctx, action_id)` 是动作执行器的 **唯一** 查找入口，返回 `Arc<dyn ActionExecutor>`
- `ExecutorRegistry::resolve_fallback(ctx, fallback_action)` 用于窗口唤醒失败时的回退执行器查找
- `ExecutorRegistry::get_actions(target_type)` 用于查询某 `TargetType` 的可用动作，仅用于查询，**禁止** 用于执行路由
- 调用方从 `resolve()` / `resolve_fallback()` 获取 executor 后，再调用 `executor.execute(ctx, action_id).await`
- 参照实现：`session_router.rs` 的 `route_confirm()` — 先 resolve 再 execute，含 fallback 处理

## PluginHandle 使用

- 插件 **必须** 通过 `PluginHandle`（从 `HostApi::register()` 获取）访问平台能力。可用方法列表见 `PluginHandle` 源码（`crates/plugin-api/src/host/plugin_handle.rs`）
- 如果某平台操作没有 `PluginHandle` 方法，**必须** 先添加到 `PluginHandle` 再使用

## 配置存储模式

所有 `Configurable` 实现 **必须** 使用强类型 `Settings` struct（带 `#[derive(Serialize, Deserialize)]` + 每个字段标注 `#[serde(rename, default)]`），通过 `RwLock<Settings>` 存储。详细规范见 [config.md](config.md) 的 Serde 默认值强制规范。

`apply_settings()` 中 **必须** 使用 `serde_json::from_value::<Settings>(settings).unwrap_or_default()` 反序列化，然后写入 `RwLock`。**禁止** 在 `apply_settings()` 中做解析、校验或副作用。

## 核心配置组件模式（builtin_plugin/config/ 中的组件）

- 核心配置组件（HotkeyConfig、AppearanceConfig 等）可使用更复杂的内部状态
- 可持有 `Arc<HostApi>` 用于在 `on_settings_changed()` 中调用平台服务
- `on_settings_changed()` 中 **允许** spawn async task 执行副作用（如注册热键、启动监控）
- **禁止** 在 `apply_settings()` 中 spawn async task。副作用 **必须** 在 `on_settings_changed()` 中

## PluginHandle 回调 ID 规范

- 通过 PluginHandle 注册回调时，ID 自动被 plugin_id 前缀化（如 `"core:search_bar_toggle"`），**必须** 避免手动添加前缀
- 回调注销 **必须** 在 `on_settings_changed()` 或组件 drop 时执行

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

见 [directory-map.md](directory-map.md) 的顶层目录职责表。

## Plugin Trait Init

- `Plugin::init()` 接收 `&PluginContext`（请求级上下文）和 `Arc<PluginHandle>`（插件服务句柄）
- `PluginHandle` 从 `HostApi::register(plugin_id, config)` 获取，绑定插件身份与配置
- 用 `handle` 参数执行平台操作。用 `ctx` 参数获取 trace_id、query_id 等请求级信息
- **禁止** 在插件内部状态中存储 `PluginHandle`；通过 `init` 参数或 `PluginContext` 访问
