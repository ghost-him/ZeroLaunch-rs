---
paths:
  - "src-tauri/src/plugin/**"
  - "src-tauri/src/plugin_system/**"
---

# 插件系统规范

## Configurable 生命周期

配置变更的 5 步流水线及各方法的职责边界见 [config.md](config.md) 的 Configurable 生命周期规范。参照实现：`HotkeyConfigComponent`、`InstallationMonitorConfigComponent`。

## ExecutorRegistry

- `ExecutorRegistry::resolve(ctx, action_id)` 是动作执行器的 **唯一** 查找入口，返回 `Arc<dyn ActionExecutor>`
- `ExecutorRegistry::resolve_fallback(ctx, fallback_action)` 用于窗口唤醒失败时的回退执行器查找
- `ExecutorRegistry::get_actions(target_type)` 用于查询某 `TargetType` 的可用动作，仅用于查询，**禁止** 用于执行路由
- 调用方从 `resolve()` / `resolve_fallback()` 获取 executor 后，再调用 `executor.execute(ctx, action_id).await`
- 参照实现：`session_router.rs` 的 `route_confirm()` — 先 resolve 再 execute，含 fallback 处理

## PluginHandle 使用

- 插件 **必须** 通过 `PluginHandle`（从 `HostApi::register()` 获取）访问平台能力。可用方法列表见 `PluginHandle` 源码（`src-tauri/src/sdk/host_api.rs`）
- 如果某平台操作没有 `PluginHandle` 方法，**必须** 先添加到 `PluginHandle` 再使用

## 配置存储模式（RwLock\<Value\> 组件）

- 插件组件（DataSource、Executor 等）使用 `RwLock<serde_json::Value>` 存储配置
- `apply_settings()` 仅做一件事：`*self.settings.write() = settings;`
- 读取配置时，通过 helper 方法从 JSON 反序列化：
  ```rust
  fn parse_xxx(&self) -> Vec<XxxConfig> {
      self.settings.read().get("key")
          .and_then(|v| v.as_array())
          .map(|arr| arr.iter().filter_map(|item| serde_json::from_value(item.clone()).ok()).collect())
          .unwrap_or_default()
  }
  ```
- **禁止** 在 `apply_settings()` 中做任何解析、校验或副作用。仅存储 raw JSON

## 核心配置组件模式（core/config/components/ 中的组件）

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

## 依赖方向

见 [directory-map.md](directory-map.md) 的顶层目录职责表。

## Plugin Trait Init

- `Plugin::init()` 接收 `&PluginContext`（请求级上下文）和 `Arc<HostApi>`（平台服务）
- 用 `host_api` 参数执行平台操作。用 `ctx` 参数获取 trace_id、query_id 等请求级信息
- **禁止** 在插件内部状态中存储 `HostApi`；通过 `init` 参数或 `PluginContext` 访问
