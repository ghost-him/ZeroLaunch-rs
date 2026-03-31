# 配置系统架构设计文档

** 当前文档由ai生成，在重构的过程中也可能会出现文档与代码不一样的情况。如果出现了这种情况，则以代码为准**

## 一、核心问题分析

### 重构前的问题

1. `RuntimeConfig` 是一个巨大的配置容器，每添加新模块都要修改它
2. `Partial` 模式导致代码冗余（每个配置都需要两套结构体）
3. 配置与模块分离，耦合不紧密
4. 更新机制复杂，需要手动处理每个字段

### 新设计目标

1. 配置与组件紧密绑定
2. 统一的配置管理接口
3. 支持前端动态发现和渲染配置界面
4. 简化更新机制

---

## 二、新架构设计

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ConfigManager                                   │
│  - 统一管理所有可配置组件                                                      │
│  - 提供配置的 CRUD 操作                                                       │
│  - 负责配置的持久化                                                            │
│  - 向前端提供配置 schema                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ 管理所有 Configurable
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          ConfigurableRegistry                                │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  Map<component_id, Arc<dyn Configurable>>                           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  组件类型：                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Plugin       │  │ DataSource   │  │ SearchEngine │  │ ScoreBooster │    │
│  │ (Configurable)│  │ (Configurable)│  │ (Configurable)│  │ (Configurable)│    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 三、核心 Trait 设计

### Configurable Trait

```rust
pub trait Configurable: Send + Sync {
    // 必需方法
    fn component_id(&self) -> &str;           // 组件唯一标识
    fn component_name(&self) -> &str;         // 组件显示名称
    fn component_type(&self) -> ComponentType; // 组件类型

    // 配置定义与读写
    fn setting_schema(&self) -> Vec<SettingDefinition>;  // 配置项定义
    fn get_settings(&self) -> HashMap<String, String>;   // 获取所有配置值
    fn apply_settings(&mut self, settings: HashMap<String, String>) -> Result<(), ConfigError>;

    // 可选方法
    fn validate_settings(&self, settings: &HashMap<String, String>) -> Result<(), ConfigError>;
    fn on_settings_changed(&self);  // 配置变更回调
}
```

### Trait 继承关系

```
┌───────────────────┐
│   Configurable    │  ← 基础配置能力
└─────────┬─────────┘
          │
    ┌─────┴─────┬─────────────────┐
    ▼           ▼                 ▼
┌─────────┐ ┌───────────┐ ┌───────────────┐
│ Plugin  │ │ DataSource│ │ SearchEngine  │
└─────────┘ └───────────┘ └───────────────┘
```

---

## 四、SettingDefinition 设计

```rust
pub struct SettingDefinition {
    pub key: String,                    // 配置项键名
    pub label: String,                  // 显示标签
    pub description: String,            // 描述说明
    pub setting_type: SettingType,      // 输入类型
    pub default_value: String,          // 默认值
    pub group: Option<String>,          // 分组（用于UI分组显示）
    pub order: u32,                     // 排序权重
    pub visible: bool,                  // 是否在UI中显示
    pub editable: bool,                 // 是否可编辑
}

pub enum SettingType {
    Text,                         // 文本输入
    Number { min, max, step },    // 数字输入
    Boolean,                      // 开关
    Select { options },           // 下拉选择
    Path { mode },                // 路径选择
    PathList,                     // 路径列表（可添加/删除）
    Color,                        // 颜色选择
    Json,                         // JSON 编辑器
}
```

---

## 五、ConfigManager 设计

### 职责

1. 注册/注销可配置组件
2. 提供配置的统一访问接口
3. 配置的持久化（加载/保存）
4. 向前端提供配置 Schema

### 核心方法

```rust
impl ConfigManager {
    // 组件管理
    fn register(component: Arc<dyn Configurable>);
    fn unregister(component_id: &str);
    fn get_component(component_id: &str) -> Option<Arc<dyn Configurable>>;
    fn get_all_components() -> Vec<ComponentInfo>;
    fn get_components_by_type(type: ComponentType) -> Vec<ComponentInfo>;

    // 配置 Schema（供前端渲染配置界面）
    fn get_component_schema(component_id: &str) -> Option<ComponentSchema>;
    fn get_all_schemas() -> HashMap<String, ComponentSchema>;

    // 配置读写
    fn get_settings(component_id: &str) -> Option<HashMap<String, String>>;
    fn apply_settings(component_id: &str, settings: HashMap<String, String>);
    fn reset_to_default(component_id: &str);

    // 持久化
    fn load_from_storage();
    fn save_to_storage();
    fn export_all() -> ConfigExport;
    fn import_all(config: ConfigExport);
}
```

---

## 六、前端交互数据结构

### 1. 获取所有可配置组件列表

```
GET /api/config/components
→ Response: Vec<ComponentInfo>

struct ComponentInfo {
    id: String,                    // 组件ID
    name: String,                  // 显示名称
    component_type: ComponentType, // 组件类型
    enabled: bool,                 // 是否启用
    has_settings: bool,            // 是否有配置项
}
```

### 2. 获取单个组件的配置 Schema

```
GET /api/config/components/{id}/schema
→ Response: ComponentSchema

struct ComponentSchema {
    component_id: String,
    component_name: String,
    groups: Vec<SettingGroup>,     // 分组的配置项
}

struct SettingGroup {
    name: String,                  // 分组名称
    order: u32,                    // 排序
    settings: Vec<SettingDefinition>,
}
```

### 3. 获取组件的当前配置值

```
GET /api/config/components/{id}/settings
→ Response: HashMap<String, String>
```

### 4. 更新组件配置（整体替换，不再使用 Partial）

```
PUT /api/config/components/{id}/settings
Body: HashMap<String, String>
→ Response: Result<(), ConfigError>
```

### 5. 重置为默认配置

```
POST /api/config/components/{id}/reset
→ Response: HashMap<String, String>  // 返回默认值
```

---

## 七、配置持久化设计

### 配置文件结构

```json
{
  "version": "3",
  "components": {
    "program-source": {
      "enabled": true,
      "settings": {
        "target_paths": "[{\"root_path\":\"...\", ...}]",
        "scan_uwp": "true",
        "forbidden_paths": "[\"...\"]"
      }
    },
    "calculator-plugin": {
      "enabled": true,
      "settings": {
        "precision": "10",
        "history_size": "50"
      }
    },
    "app-config": {
      "enabled": true,
      "settings": {
        "language": "zh-CN",
        "auto_start": "true"
      }
    }
  }
}
```

### 特点

1. 扁平化结构，所有组件配置在同一层级
2. 统一的 enabled 字段管理启用状态
3. settings 内部是 String -> String 映射，具体解析由组件负责
4. 新增组件只需添加新条目，无需修改整体结构

---

## 八、完整架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                  前端                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         Settings Page                                │    │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │    │
│  │  │ Plugin List     │  │ Setting Form    │  │ Preview/Actions │     │    │
│  │  │ (动态渲染)       │  │ (动态渲染)       │  │                 │     │    │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ Tauri Commands
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Tauri Commands                                  │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  get_all_components() → Vec<ComponentInfo>                          │    │
│  │  get_component_schema(id) → ComponentSchema                         │    │
│  │  get_component_settings(id) → HashMap<String, String>               │    │
│  │  apply_component_settings(id, settings) → Result<()>                │    │
│  │  reset_component_settings(id) → Result<()>                          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            ConfigManager                                     │
│  ┌──────────────────────┐  ┌──────────────────────┐                        │
│  │ ConfigurableRegistry │  │   ConfigStorage      │                        │
│  │ (内存中的组件注册)    │  │ (文件持久化)          │                        │
│  └──────────────────────┘  └──────────────────────┘                        │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ 管理
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Configurable Components                              │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        Plugin (impl Configurable)                    │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │    │
│  │  │ Calculator  │  │ Translator  │  │ WebSearch   │  ...             │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      DataSource (impl Configurable)                  │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │    │
│  │  │ProgramSource│  │ BookmarkSrc │  │ FileSource  │  ...             │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      其他组件 (impl Configurable)                     │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │    │
│  │  │ AppConfig   │  │ UiConfig    │  │ SearchEngine│  ...             │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 九、与原系统对比

| 方面     | 原系统                              | 新系统                     |
| -------- | ----------------------------------- | -------------------------- |
| 配置结构 | `RuntimeConfig` 包含所有配置        | 扁平化，每个组件自管理     |
| 更新方式 | Partial 模式，增量更新              | 整体替换                   |
| 新增组件 | 修改 `RuntimeConfig` + 新建 Partial | 只需实现 `Configurable`    |
| 前端渲染 | 需要硬编码配置界面                  | 根据 Schema 动态渲染       |
| 配置验证 | 分散在各处                          | 统一在 `validate_settings` |
| 启用状态 | 无统一管理                          | 统一的 `enabled` 字段      |

---

## 十、配置变更通知机制

```
前端修改配置 → ConfigManager.apply_settings()
                        │
                        ▼
             组件.validate_settings()
                        │
                        ▼
             组件.apply_settings()
                        │
                        ▼
             组件.on_settings_changed()  ← 触发重新加载、刷新缓存等
                        │
                        ▼
             ConfigManager.save_to_storage()
                        │
                        ▼
             前端收到成功响应
```

---

# ConfigManager 与 SessionRouter 交互分析

## 一、重构前的交互模式

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        重构前的配置交互流程                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  前端修改配置                                                                 │
│       │                                                                      │
│       ▼                                                                      │
│  command_save_remote_config(partial_config)                                 │
│       │                                                                      │
│       ▼                                                                      │
│  RuntimeConfig.update(partial_config)  ← 更新中央配置                        │
│       │                                                                      │
│       ▼                                                                      │
│  save_config_to_file()                 ← 持久化                              │
│       │                                                                      │
│       ▼                                                                      │
│  [问题] 配置已保存，但各组件状态未同步！                                        │
│                                                                              │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                              │
│  需要手动调用 update_app_setting() 来同步所有组件：                            │
│       │                                                                      │
│       ├── apply_log_level()                                                 │
│       ├── apply_language_and_tray()                                         │
│       ├── reload_program_catalog()                                          │
│       │      ├── icon_manager.load_from_config()                            │
│       │      ├── everything_manager.load_from_config()                      │
│       │      ├── bookmark_loader.load_from_config()                         │
│       │      └── program_manager.load_from_config()                         │
│       ├── handle_auto_start()                                               │
│       ├── update_window_size_and_position()                                 │
│       ├── enable_window_effect()                                            │
│       └── update_shortcut_manager()                                         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**问题**：
1. 配置更新与组件同步是分离的，容易遗漏
2. `update_app_setting` 是一个巨大的函数，需要知道所有组件
3. 新增组件需要修改这个函数

---

## 二、新架构中的交互场景分析

根据重构前的代码，ConfigManager 与 SessionRouter（以及其他组件）在以下场景会交互：

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           交互场景分析                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  场景1: 程序启动时                                                            │
│  ────────────────────────────────────────────────────────────────────────   │
│  ConfigManager 加载配置 → SessionRouter 初始化各 Pipeline                     │
│                                                                              │
│  场景2: 用户修改 DataSource 配置（如 ProgramSource 的索引目录）                 │
│  ────────────────────────────────────────────────────────────────────────   │
│  ConfigManager.apply_settings() → DataSource.on_settings_changed()           │
│       → SessionRouter.refresh_candidates()  ← 需要刷新缓存                    │
│                                                                              │
│  场景3: 用户修改 SearchEngine 配置                                            │
│  ────────────────────────────────────────────────────────────────────────   │
│  ConfigManager.apply_settings() → SearchEngine.on_settings_changed()         │
│       → SessionRouter 需要重建 SearchPipeline                                │
│                                                                              │
│  场景4: 用户启用/禁用某个插件                                                  │
│  ────────────────────────────────────────────────────────────────────────   │
│  ConfigManager.set_enabled() → PluginRegistry.register/unregister            │
│       → SessionRouter 需要知道哪些插件可用                                    │
│                                                                              │
│  场景5: 用户修改 Plugin 配置                                                  │
│  ────────────────────────────────────────────────────────────────────────   │
│  ConfigManager.apply_settings() → Plugin.on_settings_changed()               │
│       → 可能影响 SessionRouter 的路由逻辑                                     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 三、新架构的交互设计

**核心思想：事件驱动 + 组件自响应**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        ConfigManager                                 │
│                                                                              │
│  apply_settings(component_id, settings) {                           │
│      1. component.validate_settings(settings)                       │
│      2. component.apply_settings(settings)                          │
│      3. component.on_settings_changed()  ← 组件自己处理变更          │
│      4. emit(ConfigChangedEvent { component_id })  ← 发送事件        │
│      5. save_to_storage()                                           │
│  }                                                                  │
│                                                                              │
│  set_enabled(component_id, enabled) {                               │
│      1. update_enabled_status(component_id, enabled)                │
│      2. emit(EnabledChangedEvent { component_id, enabled })         │
│      3. save_to_storage()                                           │
│  }                                                                  │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ 事件广播
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      EventBus (事件总线)                              │
│                                                                              │
│  Events:                                                             │
│  - ConfigChanged { component_id, settings }                         │
│  - EnabledChanged { component_id, enabled }                         │
│  - ComponentRegistered { component_id }                             │
│  - ComponentUnregistered { component_id }                           │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
              ┌───────────────────────┼───────────────────────┐
              ▼                       ▼                       ▼
┌───────────────────┐  ┌───────────────────┐  ┌───────────────────┐
│  SessionRouter    │  │  PluginRegistry   │  │  其他组件...       │
│  (订阅事件)        │  │  (订阅事件)        │  │                   │
└───────────────────┘  └───────────────────┘  └───────────────────┘
```

---

## 四、SessionRouter 的订阅逻辑

```rust
impl SessionRouter {
    fn on_config_changed(&self, event: ConfigChangedEvent) {
        match event.component_type {
            ComponentType::DataSource => {
                // DataSource 配置变更，需要刷新候选项缓存
                self.refresh_candidates();
            }
            ComponentType::SearchEngine => {
                // SearchEngine 配置变更，可能需要重建 Pipeline
                self.rebuild_search_pipeline();
            }
            ComponentType::ScoreBooster => {
                // ScoreBooster 配置变更，更新 Pipeline
                self.update_score_boosters();
            }
            ComponentType::Plugin => {
                // Plugin 配置变更，通常不需要 SessionRouter 处理
                // Plugin 自己会在 on_settings_changed 中处理
            }
        }
    }

    fn on_enabled_changed(&self, event: EnabledChangedEvent) {
        match event.component_type {
            ComponentType::DataSource => {
                if event.enabled {
                    self.candidate_pipeline.add_source(component);
                } else {
                    self.candidate_pipeline.remove_source(component_id);
                }
                self.refresh_candidates();
            }
            ComponentType::Plugin => {
                // 由 PluginRegistry 处理
            }
            // ... 其他类型
        }
    }
}
```

---

## 五、完整的交互流程图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      用户修改 ProgramSource 配置的完整流程                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  前端                                                                        │
│    │                                                                         │
│    │ PUT /api/config/components/program-source/settings                     │
│    │ Body: { "target_paths": [...], "forbidden_paths": [...] }             │
│    ▼                                                                         │
│  Tauri Command                                                               │
│    │                                                                         │
│    │ config_manager.apply_settings("program-source", settings)              │
│    ▼                                                                         │
│  ConfigManager                                                               │
│    │                                                                         │
│    ├─→ program_source.validate_settings(settings)     // 验证               │
│    │                                                                         │
│    ├─→ program_source.apply_settings(settings)        // 应用               │
│    │                                                                         │
│    ├─→ program_source.on_settings_changed()           // 组件自处理          │
│    │      │                                                                  │
│    │      └─→ 重新扫描目录，更新内部程序列表                                   │
│    │                                                                         │
│    ├─→ event_bus.emit(ConfigChanged { component_id: "program-source" })     │
│    │                                                                         │
│    └─→ save_to_storage()                              // 持久化              │
│                                                                              │
│  EventBus                                                                    │
│    │                                                                         │
│    │ 广播 ConfigChanged 事件                                                  │
│    ▼                                                                         │
│  SessionRouter (订阅者)                                                       │
│    │                                                                         │
│    ├─→ 检测到 DataSource 类型配置变更                                         │
│    │                                                                         │
│    └─→ refresh_candidates()                           // 刷新缓存            │
│           │                                                                  │
│           └─→ candidate_pipeline.collect()                                   │
│                  │                                                           │
│                  └─→ program_source.fetch_candidates()  // 获取最新数据       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 六、ConfigManager 与 SessionRouter 的关系定义

### ConfigManager

**职责**：
- 管理所有可配置组件的注册
- 提供配置的 CRUD 接口
- 配置的持久化
- 发送配置变更事件

**不负责**：
- 知道配置变更后具体要做什么
- 直接调用其他组件的方法来同步状态

### SessionRouter

**职责**：
- 路由查询请求到正确的处理器
- 管理 CandidatePipeline 和 SearchPipeline
- 维护候选项缓存
- 订阅配置变更事件，做出相应响应

**不负责**：
- 管理配置
- 知道配置的具体内容

### 关系：配置变更的发布-订阅模式

```
ConfigManager ──(发布事件)──▶ EventBus ──(广播)──▶ SessionRouter
```

---

## 七、初始化顺序

```
阶段1: 基础设施
────────────────────────────────────────────────────────────────────────────
1. 创建 EventBus
2. 创建 ConfigManager (依赖 EventBus)
3. 创建 PluginRegistry
4. 创建 PluginService (依赖 PluginRegistry)

阶段2: 核心组件
────────────────────────────────────────────────────────────────────────────
5. 创建 SessionRouter (依赖 PluginService)
   - 内部创建 CandidatePipeline
   - 内部创建 SearchPipeline
   - 订阅 ConfigManager 的事件

阶段3: 注册组件
────────────────────────────────────────────────────────────────────────────
6. 创建各个 DataSource/Plugin/SearchEngine 实例
7. 注册到 ConfigManager (实现 Configurable)
8. 注册到各自的 Registry (DataSource → CandidatePipeline, etc.)

阶段4: 加载配置
────────────────────────────────────────────────────────────────────────────
9. ConfigManager.load_from_storage()
   - 读取配置文件
   - 对每个组件调用 apply_settings()
   - 触发 on_settings_changed()

阶段5: 初始化完成
────────────────────────────────────────────────────────────────────────────
10. SessionRouter.refresh_candidates()
    - 收集所有 DataSource 的候选项
    - 构建初始缓存
```

---

## 八、关键设计要点

### 1. 组件自响应

每个组件在自己的 `on_settings_changed()` 中处理配置变更后的逻辑，不需要外部知道组件内部如何响应配置变更。

### 2. 事件驱动解耦

ConfigManager 不直接调用 SessionRouter，通过事件总线广播变更，感兴趣的组件自己订阅。

### 3. 启用状态统一管理

所有组件的 enabled 状态由 ConfigManager 统一管理，SessionRouter 根据启用状态决定是否使用某个组件。

### 4. 配置变更的原子性

`apply_settings()` 是原子操作：验证 → 应用 → 通知组件 → 发送事件 → 持久化，任何一步失败都会回滚。

### 5. 缓存刷新策略

- DataSource 配置变更 → 立即刷新缓存
- SearchEngine 配置变更 → 不刷新缓存，只影响下次搜索
- Plugin 配置变更 → 不影响缓存，只影响插件行为
