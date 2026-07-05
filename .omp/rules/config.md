---
description: 配置系统规范：ConfigManager save/load 流水线、ConfigAction、组件粒度、命名规范、Serde 默认值强制规范
globs:
  - "src-tauri/src/core/config/**"
  - "src-tauri/src/commands/config_file.rs"
  - "crates/plugin-api/src/config/**"
---

# Config 系统规范

## ConfigManager 的 save/load 流水线

`ConfigManager::apply_settings()` 按以下固定顺序执行，不可重排、不可跳过、不可合并：

1. `validate_settings(&settings)?` — 纯校验，不得修改状态
   - **禁止**：修改 `self` 内部状态、执行文件/网络 I/O、调用 HostApi、注册/注销回调、spawn async task
   - **允许**：检查字段值范围、验证必填项、检查字段间约束、返回 `Err(ConfigError)` 拒绝无效输入
2. `apply_settings(settings)` — 仅写入内部 `RwLock`
   - **禁止**：重建外部服务、启动/停止监听器、注册热键、调用 HostApi、spawn async task、执行 I/O
   - **允许**：反序列化 settings JSON、写入 `self.settings`（或等价的内部 RwLock）
3. `on_settings_changed()` — 副作用（重建服务、注册回调）
   - **禁止**：修改配置值（此时配置已生效，再改会造成不一致）
   - **允许**：spawn async task 注册热键、重启监听器、发送通知等
4. 通过广播通道发布 `ConfigEvent`
5. `save_to_storage()` — 持久化到本地 JSON，然后可选远程同步

双通道事件总线：`ConfigManager` 同时是 `PluginRuntimeEvent` 的监听端，接收来自 `PluginManager` 的插件生命周期事件（加载/卸载），自动同步注册或解注册对应组件配置，再通过 `ConfigEvent` 通知 `SessionRouter` 重建管道。三层解耦：`PluginManager → PluginRuntimeEvent → ConfigManager → ConfigEvent → SessionRouter`。

如果校验（步骤 1）失败，后续步骤 **不得** 执行。
广播事件（步骤 4）**无论** 持久化成功与否都会触发。

## ConfigAction 用于保存前测试

- 如果副作用必须决定配置能否保存（如 WebDAV 连通性测试）：**必须** 使用 `ConfigAction`。通过 `config_actions() → Vec<ConfigActionDef>` 声明，在 `execute_config_action(&self, action: &str) → Result<serde_json::Value, String>` 中实现
- 前端 **必须** 将 `config_execute_action` 作为单独的用户触发操作调用，与保存流程解耦

## 组件粒度

- 按 **功能域** 拆分配置组件。每个组件处理 **一个** 关注点
- **正确**：`AppearanceConfigComponent`（主题、语言）、`StorageConfigComponent`（后端、路径）、`HotkeyConfigComponent`（快捷键）、`InstallationMonitorConfigComponent`（监控设置）
- **错误**：`AppConfigComponent` 或 `UIConfigComponent` 将所有设置打包成一个巨无霸组件
- 每个组件拥有自己的 `component_id`、自己的 settings JSON、自己的 schema。**禁止** 跨组件共享 settings 对象

## 组件位置

- 核心配置组件（非插件）和插件专属配置的放置规则见 `.omp/AGENTS.md` 的新文件放置决策树
- 配置组件实现 `Configurable` trait，可持有 `Arc<HostApi>` 以在 `on_settings_changed()` 中调用平台服务

## 命名规范

- `component_id`：**必须** 使用 kebab-case，**必须** 以 `-config` 后缀结尾（如 `"hotkey-config"`、`"window-behavior-config"`、`"appearance-config"`）。此后缀用于标识该组件为配置组件，与 `-source`（DataSource）、`-executor`（ActionExecutor）等命名惯例保持一致
- Setting JSON key：**必须** 使用 snake_case（如 `"open_search_bar"`、`"is_esc_hide_window_priority"`）
- 前端通过 `configStore.settings[component_id][setting_key]` 读取，key 名与后端一致
- 前后端 key 名 **必须** 保持一致。新增/重命名 key 时，前后端 **同一 commit** 中同步修改

## 存储分离

- `ConfigStore` 处理本地 JSON 文件持久化。远程同步（WebDAV）是可选的，单独处理
- 本地持久化 **必须** 独立于远程同步而成功

## ConfigAction 参数传递

- `execute_config_action(&self, action: &str, params: &serde_json::Value)` 签名支持参数
- 无参数的动作（如 `detect_browsers`）：前端不传 params，后端收到 `Value::Null`
- 有参数的动作（如 `read_bookmarks`）：前端传 `{ paramKey: value }`，后端从 params 中提取
- **禁止** 在 `execute_config_action` 中修改组件内部状态。它是 **纯查询/计算** 操作
- 动作返回值格式：
  - 填充某个字段 → 返回 `{ "fieldKey": [...] }` 或 `{ "fieldKey": value }`
  - 返回预览数据 → 返回数组 `[{ ... }, { ... }]`
  - **禁止** 返回需要前端特殊解析的非 JSON 数据

## Serde 默认值强制规范

所有被 `serde_json::from_str` 或 `serde_json::from_value` 反序列化的 struct，**必须** 在反序列化方向上进行缺失字段保护。

### 规则

- **必须** 给 struct 的每个字段标注 `#[serde(default)]` 或 `#[serde(default = "fn_name")]`
- `bool` 字段用 `#[serde(default)]`（默认 `false`）。如果业务默认值不是 `false`，用 `#[serde(default = "fn")]` 指定
- `f64` / `u32` / `i32` 等数值字段 **必须** 用 `#[serde(default = "default_xxx")]` 指定业务默认值（`#[serde(default)]` 会得到 0.0 / 0，破坏业务语义）
- `String` 字段：空字符串是合法默认值时用 `#[serde(default)]`，否则用 `#[serde(default = "default_xxx")]`
- `Vec<T>` / `HashMap<K,V>` / `Option<T>` 字段：用 `#[serde(default)]`
- `serde_json::Value` 字段：用 `#[serde(default)]`（默认 `Value::Null`）

### 原因

老用户的配置文件是持久化的。新版本新增字段时，老 JSON 中缺失该字段 → `serde_json::from_str` 直接失败 → ConfigManager 中的 `.unwrap_or_default()` 把**所有**用户设置静默重置为出厂值。`#[serde(default)]` 让反序列化对缺失字段宽容，单个字段回退到默认值而非整体失败。

### 反序列化防护

- **必须** 在 struct 定义处（反序列化点）做缺失字段保护，而非依赖调用处的 `.unwrap_or_default()`（调用处的兜底在 `from_str` 整体失败时才触发，此时 **所有** 字段都丢失了）
- 所有 `Configurable` impl **必须** 定义带 `#[derive(Serialize, Deserialize)]` 的强类型 `Settings` struct，每个字段标注 `#[serde(rename = "...", default)]`，用 `RwLock<Settings>` 存储
- `apply_settings()` 中 **必须** 使用 `serde_json::from_value::<Settings>(settings).unwrap_or_default()` 反序列化。`get_settings()` 中 **必须** 使用 `serde_json::to_value(&*self.settings.read()).unwrap_or_default()` 序列化
- 所有字段访问 **必须** 通过强类型 struct 的字段
- keyword_optimizer / score_booster / triggerable 等插件如果使用自定义 inner struct（如 `RwLock<FooInner>`），**必须** 改为 `RwLock<FooSettings>`（带 `#[serde(rename, default)]` 的 serde struct），inner struct 仅保留运行时状态（非配置字段）
