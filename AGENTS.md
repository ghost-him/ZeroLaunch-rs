# AGENTS.md - AI 协作指南

本文档旨在引导 AI 助手快速理解 **ZeroLaunch-rs** 项目新系统架构，并明确协作中的工程规范。

## 一、 项目概览

**ZeroLaunch-rs** 是基于 Tauri + Vue 3 的 Windows 快捷启动器。

- **后端 (Rust/Tauri)**: `src-tauri/` - SDK 平台抽象层、核心逻辑、插件系统。
- **前端 (Vue 3/TS)**: `src-ui/` - 用户界面与交互（待重构，不参与后端逻辑）。
- **核心能力**: 多源搜索（UWP/文件）、Everything 集成、多端同步。

## 二、 新系统架构总览

新系统采用**三层架构**：

```
┌────────────────────────────────────────────────┐
│               Plugin/PluginSystem               │  ← 业务层：插件实现与编排
│  (plugin/, plugin_system/)                      │
├────────────────────────────────────────────────┤
│           Core（核心逻辑与配置系统）               │  ← 中间层：业务无关的核心服务
│  (core/config/, core/types/, core/storage/)     │
├────────────────────────────────────────────────┤
│          SDK（平台抽象与宿主 API）                 │  ← 底层：屏蔽平台差异，暴露统一接口
│  (sdk/ - HostApi, traits, platform impls)       │
└────────────────────────────────────────────────┘
```

- **SDK 层** (`sdk/`)：定义跨平台 trait 接口，提供平台实现（Windows/macOS/Linux），通过 `HostApi` 统一暴露给上层。
- **Core 层** (`core/`)：包含配置系统 (`ConfigManager`)、核心类型定义、存储服务。
- **Plugin 层** (`plugin/`)：插件具体实现，实现 `plugin_system/types.rs` 中定义的 traits。
- **PluginSystem 层** (`plugin_system/`)：插件框架，管理插件注册、查询分发、执行路由、管道编排。

### 依赖规则

```
lib.rs → sdk/（构建 HostApi）
lib.rs → core/（构建 ConfigManager）
lib.rs → plugin/（构建具体插件实例）
lib.rs → plugin_system/（注册插件、构建管道）
core/   → sdk/HostApi（Configurable 组件通过 HostApi 操作平台能力）
plugin/ → sdk/PluginHandle（插件通过 PluginHandle 访问平台能力）
```

**禁止反向依赖**：sdk/ 不得引用 core/ 或 plugin/；core/ 不得引用 plugin/ 或 plugin_system/。

---

## 三、 SDK 层 (`src-tauri/src/sdk/`)

### 3.1 架构定位

SDK 层是**跨平台抽象层**，负责：
1. 定义跨平台能力 trait（契约）
2. 提供各平台的具体实现 (`sdk/platform/<os>/`)
3. 通过 `HostApi` 统一暴露给上层，上层无需关心平台差异

### 3.2 模块结构

```
sdk/
├── host_api.rs           # HostApi（核心枢纽）+ PluginHandle + Builder
├── mod.rs                # 模块导出
├── app/                  # 应用枚举 & 启动
│   ├── app_enumerator.rs # AppEnumerator trait
│   ├── app_launcher.rs   # AppLauncher trait
│   └── mod.rs            # AppInfo 类型
├── autostart/            # 自启动管理
│   └── autostart_manager.rs  # AutoStartManager trait
├── common/               # 公共工具
├── hotkey/               # 全局快捷键
│   ├── hotkey_manager.rs # HotkeyManager trait
│   ├── types.rs          # Hotkey, HotkeyEvent, HotkeyCallback 等类型
│   └── mod.rs
├── icon/                 # 图标提取 & 缓存
├── installation_monitor/ # 安装监控（新增，推送式回调）
│   ├── types.rs          # InstallationEvent, InstallationCallback
│   ├── installation_monitor.rs  # InstallationMonitor trait
│   └── mod.rs
├── parameter/            # 系统参数解析
├── path/                 # 已知路径解析
├── platform/             # 平台实现
│   ├── capabilities.rs   # PlatformCapability 枚举 + PlatformCapabilities
│   ├── mod.rs            # 按 cfg 条件导出平台实现
│   └── windows/          # Windows 平台实现
├── shell/                # Shell 操作
├── storage/              # 本地 & WebDAV 存储
└── window/               # 窗口管理
```

### 3.3 HostApi - 核心枢纽

`HostApi` 是 SDK 层的**中心结构体**，持有所有平台实现，并提供统一 API：

```rust
pub struct HostApi {
    handles: DashMap<String, Arc<PluginHandle>>,  // 已注册的插件句柄
    capabilities: PlatformCapabilities,            // 平台能力集合
    icon_extractor: Arc<dyn IconExtractor>,        // 平台注入
    shell_executor: Arc<dyn ShellExecutor>,
    window_manager: Arc<dyn WindowManager>,
    hotkey_manager: Arc<dyn HotkeyManager>,
    installation_monitor: Arc<dyn InstallationMonitor>,
    // ... 其他平台服务
}
```

**构建方式**：使用 Builder 模式，平台层在 `lib.rs` 初始化时依次注入各平台实现。

### 3.4 PluginHandle - 插件服务句柄

插件通过 `HostApi::register()` 获取 `PluginHandle`，句柄绑定了插件身份与配置：

```rust
pub struct PluginHandle {
    plugin_id: String,
    config: RwLock<PluginSdkConfig>,
    icon_extractor: Arc<dyn IconExtractor>,
    shell_executor: Arc<dyn ShellExecutor>,
    // ...
}
```

`PluginHandle` 提供的方法包括：
- 图标服务：`get_icon()`, `get_icon_and_update_cache()`
- Shell 服务：`shell_open()`, `shell_open_folder()`, `shell_execute_elevation()`, `shell_execute_command()`
- 窗口服务：`activate_window_by_process()`, `activate_window_by_title()`
- 路径服务：`resolve_path()`
- 应用服务：`enumerate_apps()`, `launch_app()`
- 参数解析：`resolve_parameters()`, `count_user_parameters()`, `has_system_parameters()`

### 3.5 Platform Capabilities - 平台能力查询

每个平台声明其支持的能力集合，插件可查询：

```rust
pub enum PlatformCapability {
    IconExtraction, ShellOpen, RunAsAdmin,
    AppEnumeration, AppLaunch, WindowActivation,
    AutoStart, HotkeyListening, InstallationMonitoring,
}
```

### 3.6 推送式回调模式

Hotkey 和 InstallationMonitor 采用**推送式回调**模式：SDK 主动向程序发出函数调用。

**回调注册模式**：支持多个回调同时注册，事件发生时依次调用。

以 InstallationMonitor 为例：
```rust
pub trait InstallationMonitor: Send + Sync {
    fn register_callback(&self, id: &str, callback: InstallationCallback);
    fn unregister_callback(&self, id: &str);
    async fn start_watching(&self) -> Result<(), HostApiError>;
    async fn stop_watching(&self) -> Result<(), HostApiError>;
    fn is_watching(&self) -> bool;
    fn update_watch_paths(&self, paths: Vec<String>);
}
```

---

## 四、 Core 层 (`src-tauri/src/core/`)

### 4.1 模块结构

```
core/
├── config/              # 配置系统
│   ├── manager.rs       # ConfigManager - 配置管理中枢
│   ├── registry.rs      # ConfigurableRegistry - 组件注册中心
│   ├── store.rs         # ConfigStore - 文件持久化
│   ├── event.rs         # ConfigEvent - 配置变更事件总线
│   ├── models.rs        # 持久化数据模型
│   └── components/      # 可配置组件实现
│       ├── mod.rs
│       ├── hotkey_config.rs
│       ├── storage_config.rs
│       └── installation_monitor_config.rs
├── types/               # 核心类型定义
│   ├── component_type.rs  # ComponentType 枚举
│   ├── config_action.rs   # ConfigActionDef
│   ├── config_error.rs    # ConfigError
│   ├── configurable.rs    # Configurable trait
│   ├── setting_def.rs     # SettingDefinition 系列类型
│   └── mod.rs
├── storage/             # 旧系统存储管理器（新系统将不再使用该模块）
└── mod.rs
```

### 4.2 Configurable trait - 统一配置契约

所有可配置组件（插件、数据源、搜索引擎、核心组件等）都实现此 trait：

```rust
pub trait Configurable: Send + Sync {
    fn component_id(&self) -> &str;
    fn component_name(&self) -> &str;
    fn component_type(&self) -> ComponentType;

    fn setting_schema(&self) -> Vec<SettingDefinition> { vec![] }
    fn get_settings(&self) -> serde_json::Value;
    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError>;
    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> { Ok(()) }
    fn get_default_settings(&self) -> serde_json::Value;
    fn on_settings_changed(&self) {}

    // 配置辅助动作（如自动检测浏览器）
    fn config_actions(&self) -> Vec<ConfigActionDef> { vec![] }
    fn execute_config_action(&self, action: &str) -> Result<serde_json::Value, String>;

    fn default_enabled(&self) -> bool { true }
}
```

**关键设计**：
- `apply_settings(&self, ...)` 使用不可变引用，组件内部通过 `RwLock` 实现可变性
- 数值配置读取统一使用 `as_f64()` 而非 `as_i64()`，预防前端浮点数传入导致的解析失败

### 4.3 ConfigManager - 配置管理中枢

`ConfigManager` 统一管理所有可配置组件的注册、CRUD、持久化和事件发布：

```
组件注册 → ConfigurableRegistry
配置读写 → apply_settings(验证→应用→回调→事件→持久化)
持久化   → ConfigStore（本地JSON）+ 可选远程同步（WebDAV）
事件     → broadcast channel，SessionRouter 等订阅
```

### 4.4 Configurable 生命周期契约（铁律）

`ConfigManager::apply_settings()` 的执行顺序是固定的：

```
1. component.validate_settings(&settings)?   ← 纯校验，无副作用
2. component.apply_settings(settings.clone())? ← 仅更新组件内部状态（写入 RwLock）
3. component.on_settings_changed()           ← 执行副作用（重建服务、注册回调等）
4. 发布 ConfigEvent
5. save_to_storage()
```

**这条流水线是不可妥协的契约，所有 Configurable 实现者必须遵守：**

| 方法 | 职责 | 禁止 |
|------|------|------|
| `validate_settings` | 纯校验（枚举值范围、格式检查等） | 修改状态、执行 I/O、操作平台能力 |
| `apply_settings` | 仅将 settings 写入内部 RwLock | 重建外部服务、调用 HostApi、执行任何副作用 |
| `on_settings_changed` | 响应配置变更的副作用：重建 storage service、启动/停止监听器、注册/注销热键等 | 修改配置值（此时配置已生效） |

**参考实现**：`HotkeyConfigComponent` 和 `InstallationMonitorConfigComponent` 已遵循此契约。

**反例**：不要把校验逻辑写在 `apply_settings` 里（应写 `validate_settings`）；不要把副作用写在 `apply_settings` 里（应写 `on_settings_changed`）。

### 4.5 ConfigAction — 保存前的连通性测试

如果某个副作用必须影响"配置是否保存"的判断（例如 WebDAV 连通性测试），不要在 `apply_settings` 或 `on_settings_changed` 中强行执行复杂副作用。使用 `config_actions` 机制：

```rust
fn config_actions(&self) -> Vec<ConfigActionDef> { ... }
fn execute_config_action(&self, action: &str) -> Result<serde_json::Value, String>;
```

前端可以提供一个"测试连接"按钮，触发 `execute_config_action`，这是一个独立的用户操作，与保存配置解耦。

### 4.6 配置组件粒度

**禁止**创建"大杂烩"式配置组件（如 `AppConfigComponent`、`UIConfigComponent` 囊括所有设置）。按功能域拆分，每个组件职责单一：

- `AppearanceConfigComponent` — 主题、语言
- `StorageConfigComponent` — 存储后端、路径
- `HotkeyConfigComponent` — 全局快捷键
- `InstallationMonitorConfigComponent` — 安装监控
- 各 plugin 自己的配置项

好处：独立加载/保存/校验、前端按需获取、减少合并冲突。

### 4.7 Configurable 组件实现原则

`core/config/components/` 下的组件负责：
1. 定义配置 Schema（前端据此渲染设置界面）
2. 配置变更时通过 `on_settings_changed()` 调用 HostApi 操作平台能力
3. 组件 **关心"怎么调用"平台能力**，而不关心平台能力的具体实现

---

## 五、 Plugin 层 (`src-tauri/src/plugin/`)

### 5.1 模块结构

```
plugin/
├── mod.rs
├── data_source/        # 数据源实现
│   ├── app_source.rs   # AppSource - UWP/Sandbox 应用数据源
│   └── program_source.rs # ProgramSource - 传统程序数据源
├── executor/           # 动作执行器实现
│   ├── app_executor.rs
│   ├── command_executor.rs
│   ├── file_executor.rs
│   ├── path_executor.rs
│   ├── url_executor.rs
│   └── window_activate_executor.rs
├── keyword_optimizer/  # 关键词优化器（9个实现）
├── score_booster/      # 分数提升器
│   └── history_booster.rs # 基于历史启动次数的分数提升
├── search_engine/      # 搜索引擎实现
│   ├── launchy_search_model.rs
│   ├── skim_search_model.rs
│   └── standard_search_model.rs
└── triggerable/        # 触发式插件（待实现，骨架文件）
    ├── calculator_plugin.rs
    └── everything_plugin.rs
```

---

## 六、 PluginSystem 层 (`src-tauri/src/plugin_system/`)

### 6.1 核心类型 (`types.rs`)

| Trait              | 职责                                                      | 继承           |
| :----------------- | :-------------------------------------------------------- | :------------- |
| `Configurable`     | **基础配置能力**（在 `core/types/` 定义，在此 re-export） | -              |
| `DataSource`       | **提供者**：产出原始搜索候选项                            | `Configurable` |
| `KeywordOptimizer` | **处理器**：对查询关键词进行清洗或扩展                    | `Configurable` |
| `SearchEngine`     | **匹配器**：计算候选项与关键词的匹配得分                  | `Configurable` |
| `ScoreBooster`     | **排序器**：基于用户习惯进行二次分值提升                  | `Configurable` |
| `ActionExecutor`   | **执行器**：定义如何执行动作                              | `Configurable` |
| `Plugin`           | **独立功能**：处理特定指令的完整闭环插件                  | `Configurable` |

### 6.2 Plugin trait - 完整插件契约

```rust
#[async_trait]
pub trait Plugin: Configurable {
    fn metadata(&self) -> &PluginMetadata;

    async fn init(
        &self,
        ctx: &PluginContext,
        api: Arc<dyn PluginAPI>,        // 宿主暴露的横切能力
        host_api: Arc<HostApi>,         // SDK 平台能力
    ) -> Result<(), PluginError>;

    async fn query(&self, ctx: &PluginContext, query: &Query)
        -> Result<QueryResponse, PluginError>;

    async fn execute_action(
        &self,
        ctx: &PluginContext,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), PluginError>;
}
```

### 6.3 PluginAPI - 宿主向插件暴露的横切能力

```rust
pub trait PluginAPI: Send + Sync {
    async fn log(&self, ctx: &PluginContext, level: LogLevel, message: &str);
    async fn notify(&self, ctx: &PluginContext, title: &str, message: &str);
    async fn get_setting(&self, plugin_id: &str, key: &str) -> Option<String>;
    async fn set_setting(&self, plugin_id: &str, key: &str, value: &str);
    async fn refresh_programs(&self);
    async fn hide_window(&self);
}
```

### 6.5 ActionExecutor 异步契约

`ActionExecutor::execute` 是 `async fn`（通过 `#[async_trait]`）。所有执行器**必须**使用 `.await` 调用异步 SDK 方法。

**绝对禁止**：在 executor 或任何 async context 中使用 `tauri::async_runtime::block_on`。这会阻塞 tokio worker thread，导致性能退化乃至死锁。

| 模式 | 做法 |
|------|------|
| 调用 SDK 异步方法 | 直接 `.await`，不要 `block_on` |
| 调用可能失败的操作 | 用 `?` 或 `.map_err()` 传播错误，不要 `tokio::spawn` 静默吞错 |

### 6.6 parking_lot RwLock 守卫生命周期规则

`parking_lot::RwLock*Guard` 是 `!Send`。在 `#[tauri::command]`（要求 `Future: Send`）的 async context 中：

- **任何 `RwLock*Guard` 都不能存活过 `.await` 点**，否则整个 future 变为 `!Send`。
- 正确做法：在作用域块中锁定 → 提取/克隆需要的数据 → 释放守卫 → 然后 `.await`。

```rust
// 正确：守卫在 .await 前释放
let data = {
    let guard = self.some_lock.read();
    guard.get_data().clone()  // 克隆出需要的数据
};  // guard 在此释放
something_async().await;

// 错误：守卫存活过 .await
let guard = self.some_lock.read();
something_async().await;  // guard 仍存活 → future !Send
```

**此规则适用于 SessionRouter、ConfigManager 以及所有持有 `RwLock`/`Mutex` 的 async 代码路径。**

### 6.7 ExecutorRegistry API 设计

`ExecutorRegistry` 的公共 API 保持最小化：
- **`execute()`**（pub async）：唯一的动作执行入口，内部处理正常执行 + `ActivationFailed` 回退链
- **`resolve()` / `resolve_fallback()`**（private）：仅供内部使用的同步查找方法
- **`get_actions()`**（pub）：查询某 TargetType 下所有可用动作
- 外部调用方只通过 `execute()` 执行动作，不直接操作 executor 查找逻辑

```
plugin_system/
├── types.rs              # 核心 trait 和类型定义
├── mod.rs                # 模块导出
├── registry.rs           # PluginRegistry - 插件注册中心（含触发词索引）
├── dispatcher.rs         # QueryDispatcher - 查询分发器
├── session_router.rs     # SessionRouter - 会话路由中枢
├── service.rs            # PluginService - 插件服务层
├── executor_registry.rs  # ExecutorRegistry - 执行器注册中心
├── api.rs                # DefaultPluginAPI - PluginAPI 默认实现
├── candidate_pipeline.rs # CandidatePipeline - 候选管道（收集+优化）
├── search_pipeline.rs    # SearchPipeline - 搜索管道（评分+排序）
├── cached_candidate.rs   # CachedCandidateData - 候选项缓存
```

---

## 七、 数据流向

### 7.1 搜索流程

```
用户输入 → Tauri Command → SessionRouter.route_query()
  ├─ 查询所有 Plugin → 命中触发器 → 返回 Plugin 结果 → 进入插件模式
  └─ 未命中触发器 → 进入搜索模式
      ├─ CandidatePipeline.collect()          ← 只在程序初始化或刷新数据库时调用
      │   ├─ DataSource.fetch_candidates()    ← 各数据源产出候选
      │   └─ KeywordOptimizer.optimize()      ← 按优先级链式优化关键词
      ├─ SearchPipeline.search(candidates, query)
      │   ├─ SearchEngine.calculate_scores()  ← 基础评分
      │   └─ ScoreBooster.boost()             ← 个性化提升
      └─ 返回排序后的 ListItem → 前端渲染
```

### 7.2 执行流程

```
用户选择候选项 → SessionRouter.route_confirm()
  ├─ 插件模式 → Plugin.execute_action()
  └─ 搜索模式 → ExecutorRegistry.execute(ctx, action_id)
      ├─ 通过 (TargetType, action_id) 定位 Executor
      ├─ 窗口唤醒失败 → 执行 Executor 声明的回退策略
      └─ 成功 → ScoreBooster.record() ← 记录用户行为
```

### 7.3 配置变更流程

```
用户在设置面板修改配置
  → Tauri Command → ConfigManager.apply_settings(component_id, settings)
    → 1. validate_settings()
    → 2. apply_settings()
    → 3. on_settings_changed()  ← 组件在此响应（如调用 HostApi）
    → 4. 发布 ConfigEvent（broadcast channel）
    → 5. save_to_storage()（本地 + 远程同步）
       │
       ▼ SessionRouter 收到 ConfigEvent
       ├─ DataSource/KeywordOptimizer → refresh_candidates()
       └─ 其他类型 → 记录日志或 TODO
```

### 7.4 系统参数快照流程

```
搜索栏唤醒 → SessionRouter.on_search_bar_wake()
  → HostApi.capture_parameter_snapshot()
    → 各 SystemParameterProvider.get_value()
      → 剪贴板提供者 / 窗口句柄提供者 / 选中文本提供者
  → 存储到 parameter_snapshot
  → route_confirm 时由 ExecutionContext 引用
```

### 7.5 安装监控回调流程（推送式）

```
用户配置启用 → InstallationMonitorConfigComponent
  → HostApi.start_installation_monitor()
    → WindowsInstallationMonitor.start_watching()
      → notify crate 监控文件系统
        └─ 文件变化 → 遍历所有注册回调 → 依次调用
```

---

## 八、 核心工程规范

### 8.1 代码风格
- 命名：Rust 蛇形命名（函数/变量）、驼峰命名（类型/trait）
- 函数头部**必须**包含简要功能描述、参数及返回值含义

### 8.2 配置处理安全
- JSON 数值配置统一使用 `as_f64()` 而非 `as_i64()`（预防前端浮点数传入导致解析失败）

### 8.3 死代码清理纪律
- **不保留备份文件**：重构完成后立即删除旧代码（如 `lib copy.rs`、未注册的模块文件）。Git 历史是唯一的备份。
- **不保留临时文件**：标注 "临时/待重构" 的代码在功能被新代码覆盖后立即删除（如 `new_search.rs` 被 `bridge.rs` 覆盖后删除）。
- **不在 mod.rs 中保留已删除模块的声明**：删除文件时同步清理 `mod.rs`。
- **死代码信号**：文件中出现 `// temp`、`// 临时`、`// 待重构`、`// TODO: remove` 但长时间未动作 → 立即处理。

### 8.4 核心路径冒烟测试

每次涉及架构层的更改后，**必须**验证以下核心路径：

| # | 路径 | 验证点 |
|---|------|--------|
| 1 | 启动应用 | Tauri app 正常启动，无 panic |
| 2 | 唤醒搜索框 | 热键唤起搜索栏，参数快照正确捕获 |
| 3 | 搜索程序 | 输入关键词，候选项正确返回 |
| 4 | 启动程序 | 选中候选项执行，目标程序正常启动 |
| 5 | 打开设置 | 设置面板正常渲染 |
| 6 | 修改配置 | 修改主题/语言/快捷键等，即时生效 |
| 7 | 保存配置 | 配置持久化到磁盘 |
| 8 | 重启后配置生效 | 重启应用，之前的配置仍然正确加载 |
| 9 | 刷新候选栏 | 安装/卸载程序后候选项更新 |
| 10 | 托盘退出 | 托盘菜单退出，进程正常结束 |

### 8.5 最小化改动
- 优先在现有框架内解决问题，除非必要或用户明确要求，否则避免大面积重构

### 8.6 模块间调用设计
- **SDK 层定义能力契约，不定义调用逻辑**
- **Core 层 Config 组件定义调用逻辑（何时/怎么调用 HostApi）**
- **插件通过 PluginHandle 句柄访问服务，不直接操作平台实现**
- **推送式服务（Hotkey/InstallationMonitor）通过回调注册机制实现**
- **所有跨模块操作经过 HostApi 统一出口**
