---
paths:
  - "src-tauri/src/plugin/**"
  - "src-tauri/src/plugin_system/**"
---

# 插件系统规范

## Configurable 生命周期铁律

配置变更的 5 步流水线顺序见 [config.md](config.md) 的 `ConfigManager 的 save/load 流水线`。此处从组件实现者视角补充各方法的职责边界：

### validate_settings

- **只做**：纯校验 — 检查枚举范围、格式正确性、必要字段
- **禁止**：修改状态、执行 I/O、调用 `HostApi`、注册回调

### apply_settings

- **只做**：将配置写入组件的内部 `RwLock`。不做任何其他副作用
- **禁止**：重建外部服务、启动/停止监听器、注册热键、调用 `HostApi`

### on_settings_changed

- **只做**：执行副作用 — 重建服务、注册/取消注册回调、启动/停止监听器
- **禁止**：修改配置值（此时配置已生效）

参照实现：`HotkeyConfigComponent`、`InstallationMonitorConfigComponent`。

### 反模式

- **禁止** 把校验逻辑放在 `apply_settings` 中。放入 `validate_settings`
- **禁止** 把副作用放在 `apply_settings` 中。放入 `on_settings_changed`

## ExecutorRegistry

- `ExecutorRegistry::resolve(ctx, action_id)` 是动作执行器的 **唯一** 查找入口，返回 `Arc<dyn ActionExecutor>`
- `ExecutorRegistry::resolve_fallback(ctx, fallback_action)` 用于窗口唤醒失败时的回退执行器查找
- `ExecutorRegistry::get_actions(target_type)` 用于查询某 `TargetType` 的可用动作，仅用于查询，**禁止** 用于执行路由
- 调用方从 `resolve()` / `resolve_fallback()` 获取 executor 后，再调用 `executor.execute(ctx, action_id).await`
- 参照实现：`session_router.rs` 的 `route_confirm()` — 先 resolve 再 execute，含 fallback 处理

## PluginHandle 使用

- 插件通过 `PluginHandle`（从 `HostApi::register()` 获取）访问平台能力。**禁止** 从 `plugin/` 代码直接 import 或调用 `sdk/platform/*`
- 如果某平台操作没有 `PluginHandle` 方法：先添加到 `PluginHandle`，再使用。**禁止** 绕过 `PluginHandle`
- 可用的方法列表请直接阅读 `PluginHandle` 源码（`src-tauri/src/sdk/host_api.rs`），以代码为准，避免文档滞后

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

- 通过 PluginHandle 注册回调时，ID 自动被 plugin_id 前缀化（如 `"core:search_bar_toggle"`）
- **禁止** 在回调 ID 中手动添加 plugin_id 前缀
- 回调注销 **必须** 在 `on_settings_changed()` 或组件 drop 时执行。**禁止** 泄漏注册的回调

## 候选项管道（CandidatePipeline）

- `CandidatePipeline::collect()` 是异步方法，按注册顺序调用每个 DataSource 的 `fetch_candidates()`
- 采集后的候选项经过 KeywordOptimizer 链处理关键词
- 候选项缓存在 `SessionRouter` 中，通过 `bridge_refresh_candidates` 命令触发重新采集
- **禁止** 在搜索路径（`route_query`）中调用 `collect()`。搜索使用缓存数据

## SearchPipeline

- `SearchPipeline::search()` 接收查询和缓存候选项，返回排序后的 top_k 结果
- 当前搜索引擎通过 `config_set_enabled` 切换。**仅** 允许一个搜索引擎同时启用
- ScoreBooster 在搜索引擎打分后追加分数修正（历史频率、查询亲和度）

## 依赖方向

- `sdk/` → **禁止** 引用 `core/`、`plugin/`、`plugin_system/`
- `core/` → **禁止** 引用 `plugin/`、`plugin_system/`
- `plugin/` → 可引用 `core/` 和 `sdk/`
- `plugin_system/` → 可引用 `plugin/`、`core/`、`sdk/`
- **禁止** 添加反向依赖引用

## Plugin Trait Init

- `Plugin::init()` 接收 `&PluginContext`（请求级上下文）和 `Arc<HostApi>`（平台服务）
- 用 `host_api` 参数执行平台操作。用 `ctx` 参数获取 trace_id、query_id 等请求级信息
- **禁止** 在插件内部状态中存储 `HostApi`；通过 `init` 参数或 `PluginContext` 访问
