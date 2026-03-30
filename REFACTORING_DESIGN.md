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

```
配置层次结构：
├── LocalConfig（本地配置）
│   ├── storage_destination    # 存储目标
│   ├── webdav_save_config     # WebDAV 配置
│   └── save_to_local_per_update # 缓存策略
│
└── RuntimeConfig（运行时配置）
    ├── app_config             # 应用配置
    ├── ui_config              # UI 配置
    ├── shortcut_config        # 快捷键配置
    ├── program_manager_config # 程序管理配置
    ├── everything_config      # Everything 配置
    └── ...
```

**设计亮点**：
- Partial 模式支持增量更新
- Arc 包装支持共享访问
- RwLock 保护并发修改

**改进空间**：
- 缺乏统一的配置验证机制
- 配置变更通知机制不完善
- 版本迁移逻辑分散

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

#### 4.1.1 架构设计原则

参考 Wox 的设计，Wox 的 `PluginManager` 同时承担了插件生命周期管理和查询流程编排两个职责。这种设计在 Wox 中是合理的，因为 Wox 是一个插件驱动的启动器，查询流程本质上就是调用插件。

但对于 ZeroLaunch，如果计划将 `ProgramManager` 拆分成多个小插件，采用**职责分离**的设计会更加清晰：

```
┌─────────────────────────────────────────────────────────────┐
│                    职责分离架构                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────┐      ┌─────────────────────┐      │
│  │  PluginRegistry     │      │  QueryDispatcher    │      │
│  │  (插件注册中心)      │      │  (查询分发器)        │      │
│  │                     │      │                     │      │
│  │  • 插件注册/注销     │      │  • 查询分发         │      │
│  │  • 插件发现         │      │  • 并行查询         │      │
│  │  • 触发词映射       │      │  • 结果聚合         │      │
│  └─────────────────────┘      └─────────────────────┘      │
│           │                            │                    │
│           │ 注册信息                    │ 查询调用           │
│           ▼                            ▼                    │
│  ┌─────────────────────────────────────────────────┐       │
│  │                  Plugin Instances               │       │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐        │       │
│  │  │ AppSearch│ │Calculator│ │ WebSearch│  ...   │       │
│  │  │  Plugin  │ │  Plugin  │ │  Plugin  │        │       │
│  │  └──────────┘ └──────────┘ └──────────┘        │       │
│  └─────────────────────────────────────────────────┘       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**与 Wox 设计的对比**：

| 方面              | Wox 设计                | 本设计                            |
| ----------------- | ----------------------- | --------------------------------- |
| **PluginManager** | 生命周期 + 查询流程     | 拆分为 Registry + Dispatcher      |
| **查询入口**      | `PluginManager.Query()` | `QueryDispatcher.dispatch()`      |
| **插件发现**      | PluginManager 内部      | Registry 独立管理                 |
| **扩展性**        | 需要修改 PluginManager  | 可独立扩展 Registry 或 Dispatcher |
| **职责**          | 混合职责                | 单一职责                          |

### 4.2 插件接口设计

// 直接查看当前的代码仓库，了解最新的定义

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
use std::sync::Arc;

pub struct ProgramPlugin {
    metadata: PluginMetadata,
    program_manager: Arc<ProgramManager>,
    settings: DashMap<String, String>,
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
            settings: DashMap::new(),
        }
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

    async fn query(&self, _ctx: &PluginContext, query: &Query) -> Result<Vec<QueryResult>, PluginError> {
        let results = self.program_manager.update(&query.search_term, 10).await;

        Ok(results.into_iter().map(|(guid, name, path)| {
            QueryResult {
                id: guid.to_string(),
                title: name,
                subtitle: path,
                icon: None,
                score: 1.0,
                actions: vec![
                    ResultAction {
                        id: "launch".to_string(),
                        label: "Launch".to_string(),
                        is_default: true,
                    },
                    ResultAction {
                        id: "launch_admin".to_string(),
                        label: "Launch as Admin".to_string(),
                        is_default: false,
                    },
                ],
            }
        }).collect())
    }

    async fn execute_action(&self, _ctx: &PluginContext, action_id: &str) -> Result<(), PluginError> {
        // 实现动作执行逻辑
        Ok(())
    }

    async fn get_setting(&self, key: &str) -> Option<String> {
        self.settings.get(key).map(|e| e.value().clone())
    }

    async fn update_setting(&self, key: &str, value: &str) -> Result<(), PluginError> {
        self.settings.insert(key.to_string(), value.to_string());
        Ok(())
    }
}
```

#### 内置命令插件

```rust
// src/plugin/builtin/command_plugin.rs

use crate::plugin::*;
use async_trait::async_trait;

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

#[async_trait]
impl Plugin for BuiltinCommandPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&self, _ctx: &PluginContext, _api: Arc<dyn PluginAPI>) -> Result<(), PluginError> {
        Ok(())
    }

    async fn query(&self, _ctx: &PluginContext, query: &Query) -> Result<Vec<QueryResult>, PluginError> {
        let commands = vec![
            ("settings", "Open Settings", "Open the settings window"),
            ("refresh", "Refresh Database", "Reload program database"),
            ("exit", "Exit", "Close ZeroLaunch"),
        ];

        let search = query.search_term.to_lowercase();
        let results: Vec<QueryResult> = commands
            .into_iter()
            .filter(|(id, _, _)| id.contains(&search) || search.is_empty())
            .map(|(id, title, subtitle)| QueryResult {
                id: id.to_string(),
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                icon: None,
                score: 1.0,
                actions: vec![ResultAction {
                    id: "execute".to_string(),
                    label: "Execute".to_string(),
                    is_default: true,
                }],
            })
            .collect();

        Ok(results)
    }

    async fn execute_action(&self, ctx: &PluginContext, action_id: &str) -> Result<(), PluginError> {
        // 根据结果 ID 执行对应命令
        Ok(())
    }

    async fn get_setting(&self, _key: &str) -> Option<String> {
        None
    }

    async fn update_setting(&self, _key: &str, _value: &str) -> Result<(), PluginError> {
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
│   └── session_router.rs          # 会话路由器（新增）
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
│   ├── parameter/                 # 参数解析（核心组件）
│   │   ├── mod.rs
│   │   ├── resolver.rs
│   │   ├── template_parser.rs
│   │   ├── parameter_types.rs
│   │   └── providers.rs
│   │
│   ├── config/                    # 配置管理（核心组件）
│   │   ├── mod.rs
│   │   ├── manager.rs
│   │   └── models.rs
│   │
│   └── event/                     # 事件系统
│       ├── mod.rs
│       ├── bus.rs
│       └── events.rs
│
├── platform/                      # 平台适配层（新增）
│   ├── mod.rs
│   ├── traits.rs                  # 平台能力 trait 定义
│   ├── windows/                   # Windows 实现
│   │   ├── mod.rs
│   │   ├── launcher.rs
│   │   ├── icon.rs
│   │   └── shortcut.rs
│   ├── macos/                     # macOS 实现（预留）
│   │   └── mod.rs
│   └── linux/                     # Linux 实现（预留）
│       └── mod.rs
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

### 6.1 目录设计原则

| 原则         | 说明                                                            |
| ------------ | --------------------------------------------------------------- |
| **职责分离** | 每个目录有明确的职责边界                                        |
| **依赖方向** | `commands → plugin_system → plugin → core → platform`，单向依赖 |
| **插件优先** | 功能通过插件实现，核心只提供基础设施                            |
| **可测试性** | 每个模块可独立测试，依赖通过 trait 抽象                         |
| **跨平台**   | 平台相关代码隔离在 platform/ 目录                               |

### 6.2 模块依赖关系

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
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       plugin/                               │
│                    (插件实现)                               │
│                                                             │
│  data_source/    search_engine/    score_booster/          │
│  launcher/       triggerable/                                │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       core/                                 │
│                   (核心基础设施)                            │
│                                                             │
│  storage/    ai/    parameter/    config/    event/         │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      platform/                              │
│                   (平台适配层)                              │
│                                                             │
│  traits.rs    windows/    macos/    linux/                 │
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
   commands/          plugin_system/          core/
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

### 6.4 启动器设计

启动器（Launcher）负责启动候选项。`launch()` 方法内部会判断程序是否已运行：如果已运行则唤醒窗口，否则启动新进程。

```rust
pub trait Launcher: Send + Sync {
    fn launch(&self, candidate: &SearchCandidate);
}
```

不同启动方式由不同的 Launcher 实现处理：

| Launcher 实现   | 处理的 LaunchMethod              |
| --------------- | -------------------------------- |
| PathLauncher    | Path（可执行文件路径）           |
| UwpLauncher     | PackageFamilyName（UWP 应用）    |
| UrlLauncher     | Url（网址）                      |
| CommandLauncher | Command / BuiltinCommand（命令） |

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
    │ PluginService   │                  │    Launcher     │
    │ .execute_action │                  │    .launch()    │
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

## 七、注意事项与避坑指南

### 7.1 并发安全

```rust
// ❌ 避免：嵌套锁
let lock1 = state.lock1.write();
let lock2 = state.lock2.write(); // 可能死锁

// ✅ 推荐：使用单一锁或避免嵌套
struct AppState {
    inner: RwLock<AppStateInner>,  // 单一锁
}

// 或使用 DashMap 进行细粒度锁
struct AppState {
    data: DashMap<String, Value>,  // 细粒度锁
}
```

---

## 八、总结

### 8.1 核心改进点

1. **引入插件架构** - 统一功能扩展入口，为未来支持外部插件预留空间
2. **职责分离设计** - 插件注册中心与查询分发器分离，遵循单一职责原则
3. **混合存储策略** - JSON 存储用户配置，SQLite 存储运行时数据
4. **模块解耦** - 通过 trait 定义接口，降低模块间耦合
5. **统一事件系统** - 支持模块间松耦合通信

### 8.2 预期收益

| 方面         | 当前状态     | 重构后                   |
| ------------ | ------------ | ------------------------ |
| **可扩展性** | 功能硬编码   | 插件化扩展               |
| **可维护性** | 模块耦合     | 接口清晰                 |
| **可测试性** | 难以单元测试 | 模块可独立测试           |
| **配置管理** | 分散         | 统一管理                 |
| **数据安全** | 写入可能损坏 | 原子写入 + 事务          |
| **职责清晰** | 混合职责     | Registry/Dispatcher 分离 |

### 8.3 架构决策说明

本设计参考了 Wox 的插件系统，但采用了职责分离的架构：

| 组件              | 职责             | 说明                                      |
| ----------------- | ---------------- | ----------------------------------------- |
| `PluginRegistry`  | 插件生命周期管理 | 注册、注销、发现插件                      |
| `QueryDispatcher` | 查询流程编排     | 分发查询、并行执行、结果聚合              |
| `PluginService`   | 整合层           | 提供统一入口，协调 Registry 和 Dispatcher |

这种设计相比 Wox 的 `PluginManager` 混合职责方案，更适合 ZeroLaunch 将 `ProgramManager` 拆分为多个小插件的目标。
