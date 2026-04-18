# ZeroLaunch-rs 重构设计文档

本文档基于对当前 ZeroLaunch-rs 代码库的深度分析，结合 Wox 插件系统架构的学习成果，为软件重构提供详细的设计方案和实施路径。

重构的过程中不会严格按照文档的说明来走。很有可能会出现文档与代码不一致的情况。如果出现，则以代码为准。

---

## 一、当前架构分析

### 1.1 项目整体结构

```
ZeroLaunch-rs/
├── src-tauri/                     # Rust 后端核心
│   ├── src/
│   │   ├── commands/              # Tauri 命令层（API 入口）
│   │   ├── core/                  # 核心功能模块
│   │   │   ├── ai/                # AI 相关（语义搜索）
│   │   │   └── storage/           # 存储管理（WebDAV/本地）
│   │   ├── modules/               # 业务模块
│   │   │   ├── config/            # 配置管理
│   │   │   ├── program_manager/   # 程序管理核心
│   │   │   ├── icon_manager/      # 图标管理
│   │   │   ├── shortcut_manager/  # 快捷键管理
│   │   │   ├── bookmark_loader/   # 书签加载
│   │   │   ├── everything/        # Everything 集成
│   │   │   ├── refresh_scheduler/ # 刷新调度
│   │   │   ├── parameter_resolver/# 参数解析
│   │   │   ├── ui_controller/     # UI 控制器
│   │   │   └── version_checker/   # 版本检查
│   │   ├── state/                 # 应用状态
│   │   ├── utils/                 # 工具函数
│   │   └── lib.rs                 # 入口点
│   └── Cargo.toml
├── src-ui/                        # Vue 前端
│   ├── api/                       # API 定义
│   ├── composables/               # Vue 组合式函数
│   ├── stores/                    # Pinia 状态管理
│   ├── windows/                   # 窗口组件
│   └── i18n/                      # 国际化
└── xtask/                         # 构建工具
```

### 1.2 当前架构特点

#### 优点

| 方面           | 描述                                                                     |
| -------------- | ------------------------------------------------------------------------ |
| **模块化设计** | `modules/` 目录下各模块职责清晰，如 `program_manager`、`icon_manager` 等 |
| **配置分离**   | 本地配置（存储设置）与远程配置（应用设置）分离                           |
| **并发安全**   | 使用 `parking_lot::RwLock`、`DashMap` 等实现线程安全                     |
| **异步架构**   | 基于 Tokio 的异步运行时，支持高并发                                      |
| **多存储后端** | 支持本地存储、WebDAV 等多种存储方式                                      |

#### 问题与改进空间

| 问题             | 当前状态                                                      | 影响               |
| ---------------- | ------------------------------------------------------------- | ------------------ |
| **模块耦合度高** | `ProgramManager` 直接依赖 `IconManager`、`SemanticManager` 等 | 难以独立测试和替换 |
| **配置系统分散** | 配置逻辑分布在多个文件中                                      | 维护困难，扩展性差 |
| **缺乏插件抽象** | 功能硬编码在模块中                                            | 无法动态扩展功能   |
| **状态管理复杂** | `AppState` 包含大量 RwLock 字段                               | 容易产生死锁风险   |
| **命令层过重**   | `commands/` 包含大量业务逻辑                                  | 违反分层原则       |

### 1.3 核心模块分析

#### ProgramManager（程序管理器）

```
ProgramManager
├── program_registry: Vec<Arc<Program>>     # 程序注册表
├── program_locater: DashMap<u64, usize>    # GUID 定位器
├── program_loader: ProgramLoader           # 程序加载器
├── program_launcher: ProgramLauncher       # 程序启动器
├── program_ranker: ProgramRanker           # 程序排序器
├── search_engine: Arc<dyn SearchEngine>    # 搜索引擎（策略模式）
├── semantic_manager: SemanticManager       # 语义管理器
└── parameter_resolver: ParameterResolver   # 参数解析器
```

**设计亮点**：
- 使用策略模式 (`SearchEngine` trait) 支持多种搜索算法
- 分离加载、排序、启动职责
- 支持语义搜索和传统搜索切换

**改进空间**：
- 依赖过多，应拆分为更小的组件
- 缺乏事件机制，状态变更难以通知外部

#### 配置系统

**新架构设计**：

采用扁平化配置结构，每个组件自管理配置，通过 `Configurable` trait 提供统一的配置能力：

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
│                         Configurable Components                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Plugin      │  │ DataSource  │  │ SearchEngine│  │ ScoreBooster│        │
│  │(Configurable)│  │(Configurable)│  │(Configurable)│  │(Configurable)│        │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────────────────────┘
```

**配置文件结构**：

```json
{
  "version": "3",
  "components": {
    "program-source": {
      "enabled": true,
      "settings": {
        "target_paths": "[{\"root_path\":\"...\", ...}]",
        "scan_uwp": "true"
      }
    },
    "calculator-plugin": {
      "enabled": true,
      "settings": {
        "precision": "10"
      }
    }
  }
}
```

**设计优势**：
- 扁平化结构，所有组件配置在同一层级
- 统一的 `enabled` 字段管理启用状态
- 新增组件只需添加新条目，无需修改整体结构
- 前端根据 Schema 动态渲染配置界面

**与原系统对比**：

| 方面     | 原系统                              | 新系统                  |
| -------- | ----------------------------------- | ----------------------- |
| 配置结构 | `RuntimeConfig` 包含所有配置        | 扁平化，每个组件自管理  |
| 更新方式 | Partial 模式，增量更新              | 整体替换                |
| 新增组件 | 修改 `RuntimeConfig` + 新建 Partial | 只需实现 `Configurable` |
| 前端渲染 | 需要硬编码配置界面                  | 根据 Schema 动态渲染    |
| 启用状态 | 无统一管理                          | 统一的 `enabled` 字段   |

---

## 二、与 Wox 架构对比

### 2.1 架构差异

| 维度         | ZeroLaunch-rs    | Wox                          |
| ------------ | ---------------- | ---------------------------- |
| **后端语言** | Rust             | Go                           |
| **前端框架** | Vue + Tauri      | Flutter                      |
| **插件系统** | 无（功能硬编码） | 多语言插件（Python/Node.js） |
| **配置存储** | JSON 文件        | SQLite                       |
| **通信方式** | Tauri IPC        | WebSocket + JSON-RPC         |
| **进程隔离** | 单进程           | 插件独立进程                 |

### 2.2 可借鉴的 Wox 设计

#### 2.2.1 插件架构

Wox 的三层插件架构：

```
┌─────────────────────────────────────┐
│          Plugin Manager             │  ← 插件生命周期管理
├─────────────────────────────────────┤
│          Plugin Host                │  ← 运行时隔离
├─────────────────────────────────────┤
│          Plugin SDK                 │  ← 开发接口
└─────────────────────────────────────┘
```

**对 ZeroLaunch 的启示**：
- 将内置功能抽象为"内置插件"
- 定义统一的插件接口
- 为未来扩展预留空间

#### 2.2.2 设置系统

Wox 的声明式设置：

```python
SettingDefinitionItem(
    type=PluginSettingDefinitionType.TEXTBOX,
    value=PluginSettingValueTextbox(
        key="api_key",
        label="API Key",
        value="",
    )
)
```

前端根据定义自动渲染，无需为每个设置项编写 UI 代码。

#### 2.2.3 查询流程

```
用户输入 → 查询解析 → 插件匹配 → 并行查询 → 结果聚合 → UI 渲染
```

ZeroLaunch 当前流程类似，但缺乏插件匹配和并行查询的抽象。

---

## 四、插件系统设计（Rust 内置插件）

### 4.1 设计目标

由于你目前只考虑 Rust 内置插件（不涉及多语言支持），设计可以大大简化：

| 目标             | 说明                                     |
| ---------------- | ---------------------------------------- |
| **统一接口**     | 所有功能模块实现相同的插件接口           |
| **职责分离**     | 插件管理与查询分发分离，遵循单一职责原则 |
| **动态注册**     | 支持运行时注册和发现插件                 |
| **配置隔离**     | 每个插件有独立的配置命名空间             |
| **生命周期管理** | 统一的初始化、查询、销毁流程             |
| **可扩展性**     | 为未来支持外部插件预留接口               |

### 4.2 插件接口设计

// 直接查看当前的代码仓库，了解最新的定义

### 4.2.1 Configurable Trait（配置能力基础）

所有可配置组件都需实现 `Configurable` trait，它提供了统一的配置管理能力：

```rust
pub trait Configurable: Send + Sync {
    fn component_id(&self) -> &str;
    fn component_name(&self) -> &str;
    fn component_type(&self) -> ComponentType;

    fn setting_schema(&self) -> Vec<SettingDefinition> { vec![] }
    fn get_settings(&self) -> serde_json::Value { serde_json::Value::Object(serde_json::Map::new()) }
    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let _ = settings;
        Ok(())
    }
    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> { Ok(()) }
    fn get_default_settings(&self) -> serde_json::Value { ... }
    fn on_settings_changed(&self) {}
    fn config_actions(&self) -> Vec<ConfigActionDef> { vec![] }
    fn execute_config_action(&self, action: &str) -> Result<serde_json::Value, String> { ... }
    fn enabled(&self) -> bool { true }
}
```

**设计说明**：
- 使用 `serde_json::Value` 作为配置传递介质，支持复杂嵌套结构
- 保留类型信息（数字、布尔、数组、对象），与前端 JSON 交互更自然
- 可直接使用 `serde_json::from_value` 反序列化到结构体

**Trait 继承关系**：

```
┌───────────────────┐
│   Configurable    │  ← 基础配置能力
└─────────┬─────────┘
          │
    ┌─────┴─────┬─────────────────┬───────────────┐
    ▼           ▼                 ▼               ▼
┌─────────┐ ┌───────────┐ ┌───────────────┐ ┌─────────────┐
│ Plugin  │ │ DataSource│ │ SearchEngine  │ │ ScoreBooster│
│         │ │           │ │               │ │             │
└─────────┘ └───────────┘ └───────────────┘ └─────────────┘
     ▲
     │
┌─────────────────┐ ┌───────────┐
│ActionExecutor   │ │KeywordOpt. │
│                 │ │           │
└─────────────────┘ └───────────┘
```

**设计优势**：
- 配置与组件紧密绑定，每个组件自己定义和管理配置
- 新增组件只需实现 `Configurable` trait，无需修改核心代码
- 前端可根据 Schema 动态渲染配置界面，无需硬编码

### 4.3 插件注册中心设计

// 直接看当前的代码仓库，了解最新的定义

### 4.4 查询分发器设计

// 直接看当前的代码仓库，了解最新的定义

### 4.5 插件服务整合

// 直接看当前的代码仓库，了解最新的定义

**设计优势**：

| 优势           | 说明                                   |
| -------------- | -------------------------------------- |
| **单一职责**   | Registry 只管插件，Dispatcher 只管查询 |
| **可独立测试** | 可以单独测试 Registry 或 Dispatcher    |
| **灵活扩展**   | 可以替换 Dispatcher 实现不同的查询策略 |
| **清晰的依赖** | Dispatcher 依赖 Registry，方向明确     |


### 4.6 内置插件示例

#### 程序搜索插件

```rust
// src/plugin/builtin/program_plugin.rs

use crate::plugin::*;
use crate::modules::program_manager::ProgramManager;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ProgramPlugin {
    metadata: PluginMetadata,
    program_manager: Arc<ProgramManager>,
    settings: HashMap<String, String>,
}

impl ProgramPlugin {
    pub fn new(program_manager: Arc<ProgramManager>) -> Self {
        Self {
            metadata: PluginMetadata {
                id: "builtin.program".to_string(),
                name: "Program Search".to_string(),
                version: "1.0.0".to_string(),
                description: "Search and launch installed programs".to_string(),
                author: "ZeroLaunch".to_string(),
                trigger_keywords: vec![],
                supported_os: vec!["windows".to_string()],
                priority: 100,
            },
            program_manager,
            settings: HashMap::new(),
        }
    }
}

impl Configurable for ProgramPlugin {
    fn component_id(&self) -> &str {
        &self.metadata.id
    }

    fn component_name(&self) -> &str {
        &self.metadata.name
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Plugin
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
'            SettingDefinition {
                field: FieldDefinition {
                    key: "max_results".to_string(),
                    label: "最大结果数".to_string(),
                    description: "搜索返回的最大结果数量".to_string(),
                    setting_type: SettingType::Number { min: Some(1.0), max: Some(50.0), step: Some(1.0) },
                    default_value: serde_json::json!(10),
                    visible: true,
                    editable: true,
                },
                group: Some("搜索设置".to_string()),
                order: 0,
            },
        ]
    }

    fn get_settings(&self) -> HashMap<String, String> {
        self.settings.clone()
    }

    fn apply_settings(&mut self, settings: HashMap<String, String>) -> Result<(), ConfigError> {
        self.settings = settings;
        Ok(())
    }

    fn on_settings_changed(&self) {
        // 配置变更后的处理逻辑
    }
}

#[async_trait]
impl Plugin for ProgramPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&self, _ctx: &PluginContext, _api: Arc<dyn PluginAPI>) -> Result<(), PluginError> {
        Ok(())
    }

    async fn query(&self, _ctx: &PluginContext, query: &Query) -> Result<QueryResponse, PluginError> {
        let results = self.program_manager.update(&query.search_term, 10).await;

        let items: Vec<ListItem> = results.into_iter().map(|(guid, name, path)| {
            ListItem {
                id: guid as u64,
                title: name,
                subtitle: path,
                icon: String::new(),
                score: 1.0,
                actions: vec![
                    ResultAction {
                        id: "launch".to_string(),
                        label: "打开".to_string(),
                        icon: String::new(),
                        is_default: true,
                        shortcut_key: String::new(),
                    },
                    ResultAction {
                        id: "launch_admin".to_string(),
                        label: "以管理员身份运行".to_string(),
                        icon: String::new(),
                        is_default: false,
                        shortcut_key: "Ctrl+Enter".to_string(),
                    },
                ],
            }
        }).collect();

        Ok(QueryResponse::List { results: items })
    }

    async fn execute_action(&self, _ctx: &PluginContext, action_id: &str, _payload: serde_json::Value) -> Result<(), PluginError> {
        // 实现动作执行逻辑
        Ok(())
    }
}
```

#### 内置命令插件

```rust
// src/plugin/builtin/command_plugin.rs

use crate::plugin::*;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct BuiltinCommandPlugin {
    metadata: PluginMetadata,
}

impl BuiltinCommandPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "builtin.command".to_string(),
                name: "Built-in Commands".to_string(),
                version: "1.0.0".to_string(),
                description: "System commands and utilities".to_string(),
                author: "ZeroLaunch".to_string(),
                trigger_keywords: vec![">".to_string()],
                supported_os: vec!["windows".to_string()],
                priority: 200,
            },
        }
    }
}

impl Configurable for BuiltinCommandPlugin {
    fn component_id(&self) -> &str {
        &self.metadata.id
    }

    fn component_name(&self) -> &str {
        &self.metadata.name
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Plugin
    }
}

#[async_trait]
impl Plugin for BuiltinCommandPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&self, _ctx: &PluginContext, _api: Arc<dyn PluginAPI>) -> Result<(), PluginError> {
        Ok(())
    }

    async fn query(&self, _ctx: &PluginContext, query: &Query) -> Result<QueryResponse, PluginError> {
        let commands = vec![
            ("settings", "打开设置", "打开设置窗口"),
            ("refresh", "刷新数据库", "重新加载程序数据库"),
            ("exit", "退出", "关闭 ZeroLaunch"),
        ];

        let search = query.search_term.to_lowercase();
        let items: Vec<ListItem> = commands
            .into_iter()
            .filter(|(id, _, _)| id.contains(&search) || search.is_empty())
            .map(|(id, title, subtitle)| ListItem {
                id: id.as_bytes().iter().map(|&b| b as u64).sum(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                icon: String::new(),
                score: 1.0,
                actions: vec![ResultAction {
                    id: "execute".to_string(),
                    label: "执行".to_string(),
                    icon: String::new(),
                    is_default: true,
                }],
            })
            .collect();

        Ok(QueryResponse::List { results: items })
    }

    async fn execute_action(&self, _ctx: &PluginContext, _action_id: &str, _payload: serde_json::Value) -> Result<(), PluginError> {
        // 根据结果 ID 执行对应命令
        Ok(())
    }
}
```

### 4.7 插件注册流程

```rust
// src/lib.rs

fn register_plugins(app_state: &Arc<AppState>) {
    let plugin_service = PluginService::new(app_state.clone());

    plugin_service.register(Arc::new(ProgramPlugin::new(
        app_state.get_program_manager()
    )));

    plugin_service.register(Arc::new(BuiltinCommandPlugin::new()));

    plugin_service.register(Arc::new(BookmarkPlugin::new(
        app_state.get_bookmark_loader()
    )));

    plugin_service.register(Arc::new(EverythingPlugin::new(
        app_state.get_everything_manager()
    )));

    app_state.set_plugin_service(Arc::new(plugin_service));
}
```

---

### 4.8 Plugin SDK 与跨平台架构

#### 4.8.1 架构分层

```
┌────────────────────────────────────────────────────────────────┐
│                        应用层 (Application)                     │
│                    前端 UI / 业务逻辑                           │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                      插件层 (Plugins)                           │
│     FileExecutor / UrlExecutor / PathExecutor / ...           │
│                    只依赖 Plugin SDK 接口                       │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     Plugin SDK (抽象接口)                       │
│     ┌─────────────────────────────────────────────────────┐   │
│     │  trait HostApi:                                      │   │
│     │    - shell_open(path) -> Result                     │   │
│     │    - shell_open_folder(path) -> Result              │   │
│     │    - ...                                             │   │
│     └─────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                   平台实现层 (Platform Impl)                    │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐ │
│  │   WindowsApi     │  │     MacApi       │  │   LinuxApi   │ │
│  │  ShellExecuteW   │  │   NSWorkspace    │  │   xdg-open   │ │
│  └──────────────────┘  └──────────────────┘  └──────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

#### 4.8.2 重构顺序

```
阶段一：插件化重构（当前）
├── 定义 Plugin SDK 接口 (HostApi trait)
├── 将现有功能迁移为插件
├── 验证接口设计的合理性
└── 此时平台实现只有 Windows

阶段二：跨平台实现（后续）
├── SDK 接口已稳定（经过阶段一验证）
├── 插件代码无需改动
├── 只需新增 macOS / Linux 平台实现
└── 核心逻辑零修改
```

#### 4.8.3 与现有架构的整合

Plugin SDK 作为独立模块 `sdk/`，与 `plugin_system/`、`core/` 同级。HostApi trait 定义在 `sdk/host_api.rs`，平台实现层 `platform/` 放置在 `sdk/` 下（因为 platform 的唯一消费者是 sdk）。

详细设计请参考 `PLUGIN_SDK_DESIGN.md`。

```
sdk/
├── mod.rs                     # 模块入口，导出公共 API
├── host_api.rs                # HostApi trait + HostApiError + OpenTarget + IconRequest
└── platform/
    ├── mod.rs                 # 条件编译选择平台实现
    ├── capabilities.rs        # PlatformCapabilities 定义
    └── windows/
        ├── mod.rs             # Windows 平台入口
        └── host_api_impl.rs   # WindowsHostApi 实现
```

HostApi 与 PluginAPI 平行共存：
- **PluginAPI** (`plugin_system/types.rs`) = 平台无关通用能力（日志、通知、配置读写）
- **HostApi** (`sdk/host_api.rs`) = 平台相关服务能力（图标提取、shell 操作、窗口管理）

插件通过 `ActionExecutor` trait 的上下文参数获取 `HostApi`：

```rust
pub trait ActionExecutor: Configurable {
    fn execute(&self, ctx: &ExecutionContext, action_id: &str, host: &dyn HostApi)
        -> Result<(), ExecutionError>;
}
```

---

## 六、重构后的目录结构

```
src-tauri/src/
│
├── lib.rs                         # 应用入口点
├── main.rs                        # 主函数
├── logging.rs                     # 日志系统
├── tray.rs                        # 系统托盘（核心功能）
├── window_effect.rs               # 窗口效果（核心功能）
├── window_position.rs             # 窗口位置（核心功能）
│
├── plugin_system/                 # 插件系统框架（已完成）
│   ├── mod.rs
│   ├── types.rs                   # 核心类型定义（含 trait）
│   ├── registry.rs                # 插件注册中心
│   ├── dispatcher.rs              # 查询分发器
│   ├── search_pipeline.rs         # 搜索管道
│   ├── service.rs                 # 插件服务整合层
│   ├── api.rs                     # 插件 API 实现
│   ├── session_router.rs          # 会话路由器（新增）
│   ├── cached_candidate.rs        # 候选项缓存
│   ├── candidate_pipeline.rs      # 候选项处理管道
│   └── executor_registry.rs       # 执行器注册中心
│
├── plugin/                        # 插件实现（按 trait 分类）
│   ├── mod.rs
│   │
│   ├── data_source/               # 数据源插件
│   │   ├── mod.rs
│   │   ├── program_source.rs      # 程序数据源
│   │   ├── bookmark_source.rs     # 书签数据源
│   │   └── url_source.rs          # URL 数据源
│   │
│   ├── keyword_optimizer/         # 关键字优化器
│   │   ├── mod.rs
│   │   ├── pinyin_converter.rs    # 拼音转换
│   │   ├── version_number_remover.rs # 版本号移除
│   │   └── ...                    # 其他优化器
│   │
│   ├── search_engine/             # 搜索引擎插件
│   │   ├── mod.rs
│   │   ├── traditional.rs         # 传统搜索
│   │   └── semantic.rs            # 语义搜索
│   │
│   ├── score_booster/             # 分数增强器
│   │   ├── mod.rs
│   │   ├── history_booster.rs     # 历史记录增强
│   │   └── query_affinity.rs      # 查询亲和度增强
│   │
│   ├── launcher/                  # 启动器插件
│   │   ├── mod.rs
│   │   ├── path_launcher.rs       # 路径启动
│   │   ├── uwp_launcher.rs        # UWP 启动
│   │   ├── url_launcher.rs        # URL 启动
│   │   └── command_launcher.rs    # 命令启动
│   │
│   └── triggerable/               # 可触发插件（独立功能）
│       ├── mod.rs
│       ├── everything_plugin.rs   # Everything 搜索
│       ├── calculator_plugin.rs   # 计算器
│       ├── command_plugin.rs      # 内置命令 (>)
│       └── websearch_plugin.rs    # 网页搜索
│
├── core/                          # 核心模块
│   ├── mod.rs
│   │
│   ├── types/                     # 基础类型层
│   │   ├── mod.rs
│   │   ├── configurable.rs        # Configurable trait
│   │   ├── component_type.rs      # ComponentType 枚举
│   │   ├── setting_def.rs         # SettingDefinition 等
│   │   ├── config_error.rs        # ConfigError 枚举
│   │   └── config_action.rs       # ConfigActionDef
│   │
│   ├── config/                    # 配置管理层
│   │   ├── mod.rs
│   │   ├── manager.rs             # ConfigManager 主结构
│   │   ├── registry.rs            # ConfigurableRegistry
│   │   ├── store.rs               # ConfigStore
│   │   ├── event.rs               # ConfigEvent
│   │   └── models.rs              # 数据模型
│   │
│   ├── storage/                   # 存储后端
│   │   ├── mod.rs
│   │   ├── local_save.rs
│   │   ├── webdav.rs
│   │   └── config.rs
│   │
│   ├── ai/                        # AI 功能
│   │   ├── mod.rs
│   │   ├── embedding/             # 嵌入模型
│   │   └── semantic/              # 语义处理
│   │
│   ├── image_processor.rs        # 图像处理器
│   │
│   ├── parameter/                 # 参数解析（核心组件）
│   │   ├── mod.rs
│   │   ├── resolver.rs
│   │   ├── template_parser.rs
│   │   ├── parameter_types.rs
│   │   └── providers.rs
│
├── sdk/                           # Plugin SDK（核心向插件提供统一服务）
│   ├── mod.rs                     # 模块入口，导出公共 API
│   ├── host_api.rs                # HostApi struct + PluginHandle struct + IconRequest + CacheLevel + PluginSdkConfig + 错误类型
│   ├── common/
│   │   ├── mod.rs                 # 通用模块入口
│   │   └── image_utils.rs         # ImageUtils — 跨平台图片处理工具函数
│   ├── icon/
│   │   ├── mod.rs                 # 图标模块入口
│   │   ├── icon_cache.rs          # IconCacheService — 纯缓存工具
│   │   └── icon_extractor.rs      # IconExtractor trait — 平台原语 + 跨平台默认实现
│   ├── shell/
│   │   ├── mod.rs                 # Shell 模块入口
│   │   ├── shell_executor.rs      # ShellExecutor trait — 平台原语
│   │   ├── lnk_resolver.rs        # LnkResolver trait — Lnk 快捷方式解析
│   │   └── resource_loader.rs      # ResourceLoader trait — 本地化字符串资源加载
│   ├── window/
│   │   ├── mod.rs                 # 窗口模块入口
│   │   └── window_manager.rs      # WindowManager trait — 平台原语
│   ├── path/
│   │   ├── mod.rs                 # 路径模块入口
│   │   └── path_resolver.rs       # PathResolver trait — 平台原语
│   ├── app/
│   │   ├── mod.rs                 # 应用模块入口
│   │   ├── app_enumerator.rs      # AppEnumerator trait
│   │   └── app_launcher.rs        # AppLauncher trait
│   └── platform/
│       ├── mod.rs                 # 条件编译选择平台实现
│       ├── capabilities.rs        # PlatformCapabilities 定义
│       └── windows/
│           ├── mod.rs             # Windows 平台入口
│           ├── icon.rs            # WindowsIconExtractor
│           ├── shell.rs           # WindowsShellExecutor
│           ├── lnk_resolver.rs    # WindowsLnkResolver
│           ├── resource_loader.rs # WindowsResourceLoader
│           ├── window.rs          # WindowsWindowManager
│           ├── path_resolver.rs   # WindowsPathResolver
│           ├── app_enumerator.rs  # WindowsAppEnumerator
│           └── app_launcher.rs    # WindowsAppLauncher
│
├── state/                         # 应用状态（核心组件）
│   ├── mod.rs
│   └── app_state.rs
│
├── commands/                      # Tauri 命令入口
│   ├── mod.rs
│   ├── query.rs                   # 查询相关命令
│   ├── config.rs                  # 配置相关命令
│   └── system.rs                  # 系统相关命令
│
└── utils/                         # 全局工具箱
    ├── mod.rs
    ├── i18n.rs
    ├── notify.rs
    ├── defer.rs
    └── ...
```

### 6.1 模块依赖关系

```
┌─────────────────────────────────────────────────────────────┐
│                      commands/                              │
│                   (Tauri 命令入口)                          │
│                   query() / confirm()                       │
└─────────────────────────────┬───────────────────────────────┘
                              │ 调用
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    plugin_system/                           │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              SessionRouter                          │   │
│  │         (会话路由器 - 顶层协调)                       │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                  │
│          ┌───────────────┴───────────────┐                 │
│          ▼                               ▼                 │
│  ┌─────────────────┐            ┌─────────────────────┐    │
│  │  PluginService  │            │   SearchPipeline    │    │
│  │  (插件协调)      │            │   (搜索管道)        │    │
│  └────────┬────────┘            └──────────┬──────────┘    │
│           │                                │                │
│           ▼                                ▼                │
│  ┌─────────────────┐            ┌─────────────────────┐    │
│  │ QueryDispatcher │            │ SearchEngine        │    │
│  │ PluginRegistry  │            │ ScoreBooster        │    │
│  └─────────────────┘            └─────────────────────┘    │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              api.rs (PluginAPI)                    │   │
│  │         PluginAPI trait - 平台无关通用能力           │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                              │ 通过 HostApi trait
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       plugin/                               │
│                    (插件实现)                               │
│                                                             │
│  data_source/    search_engine/    score_booster/          │
│  executor/       triggerable/      (依赖 HostApi 接口)     │
└─────────────────────────────┬───────────────────────────────┘
                              │ HostApi trait 实现
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       sdk/                                   │
│                 (Plugin SDK - 宿主服务接口)                  │
│                                                             │
│  HostApi trait    PlatformCapabilities    HostApiError     │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │              platform/ (平台实现层)                     │ │
│  │  WindowsHostApi    MacHostApi(预留)    LinuxHostApi   │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       core/                                 │
│                   (核心基础设施)                            │
│                                                             │
│  storage/    ai/    parameter/    config/    event/         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      utils/                                 │
│                   (全局工具箱)                              │
└─────────────────────────────────────────────────────────────┘

                    ┌─────────────────┐
                    │      state/     │
                    │  (应用状态容器)  │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
   commands/          plugin_system/          sdk/
```

### 6.3 搜索引擎设计

搜索引擎（SearchEngine）负责计算候选项的基础分数。传统搜索与语义搜索是**互斥的搜索策略**，而非串联的处理步骤。

```
┌─────────────────────────────────────────────────────────────┐
│                      SearchPipeline                         │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  SearchEngine (选择其一)                             │   │
│  │  ┌─────────────────┐    ┌─────────────────────────┐ │   │
│  │  │ TraditionalEngine│ OR │ SemanticEngine         │ │   │
│  │  │                 │    │ (内部维护embedding缓存) │ │   │
│  │  │ 拼音/模糊匹配    │    │ 向量相似度计算          │ │   │
│  │  └─────────────────┘    └─────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────┘   │
│                           ↓                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  ScoreBooster[] (串联执行)                           │   │
│  │  ┌─────────────────┐    ┌─────────────────────────┐ │   │
│  │  │ HistoryBooster  │ →  │ QueryAffinityBooster    │ │   │
│  │  │ 启动频率增强     │    │ 查询亲和度增强           │ │   │
│  │  └─────────────────┘    └─────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

**SemanticEngine 的 embedding 管理**：SemanticSearchEngine 内部维护 embedding 缓存（`candidate_id → embedding`），数据源在加载程序时调用其方法注册 embedding，SearchCandidate 本身不需要存储 embedding 数据。

### 6.4 执行器设计

执行器（ActionExecutor）负责执行候选项的动作。`execute()` 方法内部会判断程序是否已运行：如果已运行则唤醒窗口，否则启动新进程。

```rust
pub trait ActionExecutor: Configurable {
    fn supported_target_types(&self) -> Vec<TargetType>;
    fn supported_actions(&self) -> Vec<ResultAction> { ... }
    fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError>;
}
```

不同目标类型由不同的 Executor 实现处理：

| Executor 实现          | 处理的 TargetType                |
| ---------------------- | -------------------------------- |
| PathExecutor           | Path（可执行文件路径）           |
| AppExecutor            | App（系统应用，跨平台统一）      |
| UrlExecutor            | Url（网址）                      |
| CommandExecutor        | Command / BuiltinCommand（命令） |
| WindowActivateExecutor | Path, App（窗口唤醒）            |

### 6.5 模块之间的数据流动

```
                              用户输入
                                 │
                                 ▼
              ┌──────────────────────────────────────┐
              │           SessionRouter              │
              │         route_query(input)           │
              └──────────────────┬───────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │   parse_trigger(input)  │
                    └────────────┬────────────┘
                                 │
              ┌──────────────────┴──────────────────┐
              │                                     │
              ▼                                     ▼
       有触发词匹配                          无触发词匹配
              │                                     │
              ▼                                     ▼
    ┌─────────────────┐                  ┌─────────────────┐
    │   插件模式       │                  │   搜索模式       │
    │                 │                  │                 │
    │ PluginService   │                  │ SearchPipeline  │
    │   .query()      │                  │   .search()     │
    └────────┬────────┘                  └────────┬────────┘
             │                                    │
             ▼                                    ▼
    ┌─────────────────┐                  ┌─────────────────┐
    │ Plugin.query()  │                  │ DataSource      │
    │                 │                  │   ↓             │
    │ 插件自定义逻辑   │                  │ SearchEngine    │
    │                 │                  │   ↓             │
    │                 │                  │ ScoreBooster    │
    └────────┬────────┘                  └────────┬────────┘
             │                                    │
             └────────────────┬───────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │   返回结果列表   │
                    │  Vec<QueryResult>│
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   前端显示结果   │
                    └────────┬────────┘
                             │
                             ▼
                       用户确认选择
                             │
                             ▼
              ┌──────────────────────────────────────┐
              │           SessionRouter              │
              │    route_confirm(result, action)     │
              └──────────────────┬───────────────────┘
                                 │
              ┌──────────────────┴──────────────────┐
              │                                     │
              ▼                                     ▼
        current_mode ==                      current_mode ==
           Plugin                              Search
              │                                     │
              ▼                                     ▼
    ┌─────────────────┐                  ┌─────────────────┐
    │ PluginService   │                  │  ActionExecutor  │
    │ .execute_action │                  │    .execute()    │
    └────────┬────────┘                  └────────┬────────┘
             │                                    │
             ▼                                    ▼
    ┌─────────────────┐                  ┌─────────────────┐
    │ Plugin          │                  │ 判断是否已运行   │
    │ .execute_action │                  │   ↓             │
    │                 │                  │ 已运行→唤醒窗口  │
    │ 插件自定义逻辑   │                  │ 未运行→启动进程  │
    └────────┬────────┘                  └────────┬────────┘
             │                                    │
             └────────────────┬───────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │   会话结束       │
                    │ reset_session() │
                    └─────────────────┘
```

route_query:
  1. 生成 QueryResponse 时，生成唯一的 result_id
  2. 将 ExecutionContext 存入缓存
  3. 返回 QueryResponse（不包含 ExecutionContext）

route_confirm:
  1. 通过 result_id 查缓存获取 ExecutionContext
  2. 执行对应逻辑



搜索的完整的流程示例

```
用户搜索 "chrome"
        ↓
后端返回 QueryResult：
┌─────────────────────────────────────────────────────────────────────┐
│ id: 123 (候选项ID)                                                  │
│ title: "Google Chrome"                                              │
│ subtitle: "C:\...\chrome.exe"                                       │
│ actions: [                                                          │
│   { id: "launch",        label: "打开",      is_default: true  },   │
│   { id: "launch_admin",  label: "管理员运行", is_default: false },   │
│   { id: "open_location", label: "打开位置",   is_default: false },   │
│ ]                                                                   │
└─────────────────────────────────────────────────────────────────────┘
        ↓
前端显示：
┌─────────────────────────┐
│ 🔵 Google Chrome        │
│    C:\...\chrome.exe    │
│    [打开] [管理员运行] [打开位置] │
└─────────────────────────┘
        ↓
用户点击 "管理员运行" 或选中后按快捷键
        ↓
前端调用：route_confirm(trace_id, result_id=123, action_id="launch_admin")
        ↓
后端：找到候选项123 → 根据 action_id="launch_admin" → 以管理员权限启动
```

插件相关的

```

┌─────────────────────────────────────────────────────────┐
│                    插件前端加载策略                      │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────┐      ┌─────────────────┐          │
│  │   内置插件       │      │   外部插件       │          │
│  │   (编译时)       │      │   (运行时)       │          │
│  └────────┬────────┘      └────────┬────────┘          │
│           │                        │                    │
│           ▼                        ▼                    │
│  ┌─────────────────┐      ┌─────────────────┐          │
│  │  CustomPanel    │      │  WebView        │          │
│  │  (原生组件)      │      │  (iframe)       │          │
│  │  高性能、集成好   │      │  灵活、隔离      │          │
│  └─────────────────┘      └─────────────────┘          │
│                                                         │
└─────────────────────────────────────────────────────────┘


┌─────────────────────────────────────────────────────────┐
│                    插件开发流程                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│   1. 后端：实现 Plugin trait                            │
│              ↓                                          │
│   2. 后端：返回 CustomPanel { panel_type: "notebook" }  │
│              ↓                                          │
│   3. 前端：注册组件 registerPanel("notebook", Notebook) │
│              ↓                                          │
│   4. 运行时：后端返回 panel_type → 前端找到对应组件渲染  │
│                                                         │
└─────────────────────────────────────────────────────────┘

用户输入 "note"
       ↓
后端 NotebookPlugin.query() 被触发
       ↓
返回 QueryResult {
    response_type: CustomPanel {
        panel_type: "notebook",  ← 字符串标识
        data: { notes: [...] }   ← 传递给组件的数据
    }
}
       ↓
前端收到响应
       ↓
检测到 response_type 是 "custom-panel"
       ↓
查找注册表：panelRegistry["notebook"] → NotebookPanel 组件
       ↓
渲染：<NotebookPanel :data="{ notes: [...] }" />
```

---

## 八、总结

### 8.1 核心改进点

1. **引入插件架构** - 统一功能扩展入口，为未来支持外部插件预留空间
2. **职责分离设计** - 插件注册中心与查询分发器分离，遵循单一职责原则
3. **Plugin SDK 层** - 通过 HostApi trait 抽象平台能力，插件与平台实现解耦
4. **跨平台架构** - 平台相关代码隔离在 platform/ 目录，支持渐进式跨平台开发
5. **混合存储策略** - JSON 存储用户配置，SQLite 存储运行时数据
6. **模块解耦** - 通过 trait 定义接口，降低模块间耦合
7. **统一事件系统** - 支持模块间松耦合通信

### 8.2 预期收益

| 方面         | 当前状态     | 重构后                     |
| ------------ | ------------ | -------------------------- |
| **可扩展性** | 功能硬编码   | 插件化扩展                 |
| **可维护性** | 模块耦合     | 接口清晰                   |
| **可测试性** | 难以单元测试 | 模块可独立测试（Mock SDK） |
| **配置管理** | 分散         | 统一管理                   |
| **数据安全** | 写入可能损坏 | 原子写入 + 事务            |
| **职责清晰** | 混合职责     | Registry/Dispatcher 分离   |
| **跨平台**   | 仅 Windows   | 架构支持多平台             |

### 8.3 重构阶段规划

```
阶段一：插件化重构 + SDK 基础设施
├── 定义核心 trait（Configurable, DataSource, ActionExecutor 等）
├── 实现 Plugin SDK（HostApi + PluginHandle + 平台 trait 注入）
├── 定义 PathResolver trait（路径解析接口）
├── 实现 WindowsPathResolver（SHGetKnownFolderPath）
├── 扩展 HostApi / PluginHandle（注入 PathResolver）
├── 修复 ProgramSource 硬编码路径（委托 PluginHandle::resolve_path()）
├── 将现有功能迁移为插件
└── 验证接口设计合理性

阶段二：应用枚举与启动抽象
├── 定义 AppEnumerator trait（应用枚举接口）
├── 定义 AppLauncher trait（应用启动接口）
├── 定义 AppInfo 统一数据结构
├── 实现 WindowsAppEnumerator（迁移 AppSource 的 Win32 调用）
├── 实现 WindowsAppLauncher（迁移 AppExecutor 的 Win32 调用）
├── 扩展 PlatformCapability（新增 AppEnumeration、AppLaunch）
├── 扩展 HostApi / PluginHandle（注入新组件，暴露新方法）
├── 重命名 UwpExecutor → AppExecutor（委托 PluginHandle::launch_app()）
├── 重命名 UwpSource → AppSource（委托 WindowsAppEnumerator）
└── 重命名 TargetType::PackageFamilyName → TargetType::App
└── 验证 UWP 应用枚举和启动功能

阶段三：ShellExecutor 扩展
├── 扩展 ShellExecutor trait（新增 execute_command 方法）
├── 实现 Windows execute_command（封装 CommandExt::creation_flags）
├── 重构 CommandExecutor（委托 PluginHandle::execute_command()）
└── 验证命令执行功能

阶段四：LnkResolver 快捷方式解析
├── 定义 LnkResolver trait（Lnk 解析接口）
├── 实现 WindowsLnkResolver（lnk crate + 双编码回退）
├── 扩展 HostApi / PluginHandle（注入 LnkResolver）
├── 迁移 WindowActivateExecutor（委托 PluginHandle::resolve_lnk_target()）
└── 验证 .lnk 文件解析功能

阶段五：ResourceLoader 本地化字符串加载
├── 定义 ResourceLoader trait（资源加载接口）
├── 实现 WindowsResourceLoader（LoadLibraryExW + LoadStringW + desktop.ini 解析）
├── 扩展 HostApi / PluginHandle（注入 ResourceLoader）
├── 迁移 ProgramSource（委托 PluginHandle::parse_localized_names_from_dir()）
└── 验证本地化名称显示功能

阶段六：Plugin::init() 整合
├── 修改 Plugin::init() 签名（增加 host_api: Arc<HostApi> 参数）
├── 更新所有 Plugin::init() 实现
└── 验证 Plugin 可通过 init 访问平台能力

阶段七：跨平台实现
├── SDK 接口已稳定
├── 实现 macOS 平台适配（Launch Services 集成）
├── 实现 Linux 平台适配（Flatpak/Snap 集成）
└── 插件代码零修改
```

### 8.4 当前进度

| 模块                      | 状态   | 说明                                                        |
| ------------------------- | ------ | ----------------------------------------------------------- |
| **Plugin SDK 设计**       | ✅ 完成 | `PLUGIN_SDK_DESIGN.md` v2.2                                 |
| **PathResolver trait**    | ✅ 完成 | 阶段一：路径解析接口 + Windows 实现                         |
| **WindowsPathResolver**   | ✅ 完成 | 阶段一：SHGetKnownFolderPath                                |
| **AppEnumerator trait**   | ✅ 完成 | 阶段二：应用枚举统一接口（async）                           |
| **AppLauncher trait**     | ✅ 完成 | 阶段二：应用启动统一接口                                    |
| **WindowsAppEnumerator**  | ✅ 完成 | 阶段二：迁移 UwpSource Win32 调用                           |
| **WindowsAppLauncher**    | ✅ 完成 | 阶段二：迁移 UwpExecutor Win32 调用                         |
| **TargetType::App**       | ✅ 完成 | 阶段二：PackageFamilyName → App 重命名                      |
| **AppSource/AppExecutor** | ✅ 完成 | 阶段二：UwpSource→AppSource, UwpExecutor→AppExecutor        |
| **ShellExecutor 扩展**    | ✅ 完成 | 阶段三：`shell_execute_command` 方法 + CommandExecutor 迁移 |
| **LnkResolver trait**     | ✅ 完成 | 阶段四：Lnk 快捷方式解析接口 + Windows 实现（lnk crate）    |
| **WindowsLnkResolver**    | ✅ 完成 | 阶段四：双编码回退（GB18030 → UTF-16LE）                    |
| **ResourceLoader trait**  | ✅ 完成 | 阶段五：本地化字符串资源加载接口 + Windows 实现             |
| **WindowsResourceLoader** | ✅ 完成 | 阶段五：LoadLibraryExW + LoadStringW + desktop.ini 解析     |
| **Plugin::init() 整合**   | ✅ 完成 | 阶段六：Plugin::init() 接收 Arc<HostApi> 参数               |

详细设计请参考 [PLUGIN_SDK_DESIGN.md](file:///c:/Users/ghost/ZeroLaunch/ZeroLaunch-rs/PLUGIN_SDK_DESIGN.md)
