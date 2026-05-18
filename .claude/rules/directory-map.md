---
paths:
  - "**"
---

# 目录结构与文件放置规范

## 后端 (src-tauri/src/)

### 顶层目录职责

| 目录 | 职责 | 可引用 | 禁止 |
|------|------|--------|------|
| `sdk/` | 平台抽象层：trait 定义 + 平台实现 | 无外部依赖 | 引用 core/、plugin/、plugin_system/ |
| `core/` | 业务核心：ConfigManager、Configurable trait、类型定义 | sdk/ | 引用 plugin/、plugin_system/ |
| `plugin/` | 插件实现：DataSource、Executor、SearchEngine 等 | sdk/、core/ | 引用 plugin_system/ |
| `plugin_system/` | 插件框架：SessionRouter、Pipeline、Registry | sdk/、core/、plugin/ | 被其他层反向引用 |
| `commands/` | IPC 命令：薄代理层，仅委托 | 全部 | 包含业务逻辑 |
| `state/` | AppState 定义 | core/、plugin_system/ | 包含业务方法 |
| `utils/` | 通用工具（service_locator 等） | 无限制 | 包含业务逻辑 |

### 各目录详细说明

#### `sdk/` — 平台抽象层
```
sdk/
├── host_api.rs          ← HostApi 结构体（唯一出口）
├── app/                 ← 应用枚举与启动能力
├── autostart/           ← 开机自启管理
├── common/              ← 通用工具（image_utils, dir_utils）
├── focus_monitor/       ← 窗口焦点监控
├── hotkey/              ← 全局热键管理
├── icon/                ← 图标提取与缓存
├── installation_monitor/← 软件安装检测
├── parameter/           ← 参数解析（剪贴板、选中文本等）
├── path/                ← 路径解析（KnownPath）
├── platform/            ← 平台特定实现（windows/）
├── resource/            ← 应用内置资源
├── shell/               ← Shell 执行与 .lnk 解析
├── storage/             ← 存储后端（本地 + WebDAV）
├── timer/               ← 定时器管理
└── window/              ← 窗口管理
```

- 每个能力域包含：`mod.rs`（导出）、一个 trait 文件、可选 `types.rs`
- 平台实现放入 `sdk/platform/<os>/`
- **禁止** 在 sdk/ 中定义调用逻辑（何时调用）— 那属于 core/

#### `core/` — 业务核心
```
core/
├── constants.rs         ← 应用常量
├── tray/                ← 系统托盘管理
├── config/              ← 配置系统
│   ├── manager.rs       ← ConfigManager 主调度器
│   ├── store.rs         ← ConfigStore（JSON 持久化）
│   ├── models.rs        ← 配置数据模型
│   ├── registry.rs      ← ConfigurableRegistry
│   ├── event.rs         ← ConfigEvent 广播
│   ├── setting_builders.rs ← SchemaBuilder API
│   └── components/      ← 核心配置组件（非插件）
│       ├── appearance_config.rs
│       ├── hotkey_config.rs
│       ├── installation_monitor_config.rs
│       └── storage_config.rs
└── types/               ← 核心类型定义
    ├── configurable.rs  ← Configurable trait
    ├── setting_def.rs   ← SettingType/FieldDefinition/SettingDefinition
    ├── bridge_error.rs  ← BridgeError（IPC 错误）
    ├── config_action.rs ← ConfigActionDef
    ├── config_error.rs  ← ConfigError
    └── component_type.rs← ComponentType 枚举
```

- **核心配置组件**（core/config/components/）：不属于任何插件的系统级配置
- **新增核心配置组件** 时放这里。**新增插件** 时放 plugin/

#### `plugin/` — 插件实现
```
plugin/
├── data_source/         ← 数据源（5个）
├── executor/            ← 执行器（6个）
├── keyword_optimizer/   ← 关键词优化器（8个）
├── score_booster/       ← 分数增强器（2个）
├── search_engine/       ← 搜索引擎（3个）
└── triggerable/         ← 可触发插件（计算器等）
```

- 每个插件实现 `Configurable` trait（配置）+ 对应的领域 trait（如 `DataSource`、`ActionExecutor`）
- **禁止** 从 plugin/ 直接调用 `sdk/platform/*`。通过 `PluginHandle` 访问

#### `plugin_system/` — 插件框架
```
plugin_system/
├── session_router.rs    ← 搜索会话路由（核心调度器）
├── candidate_pipeline.rs← 候选项采集管道
├── search_pipeline.rs   ← 搜索排序管道
├── executor_registry.rs ← 执行器注册表
├── dispatcher.rs        ← Plugin 触发分发
├── service.rs           ← PluginService
├── registry.rs          ← PluginRegistry
├── cached_candidate.rs  ← 缓存候选项
└── types.rs             ← 所有运行时类型定义
```

- **SessionRouter** 是运行时的中枢。所有 bridge 命令通过它路由
- **禁止** 在此层定义配置 schema 或持久化逻辑（那属于 core/）

#### `commands/` — IPC 命令层
```
commands/
├── bridge.rs     ← bridge_ 前缀（搜索/会话）7个命令
├── config_file.rs← config_ 前缀（配置管理）8个命令
└── resource.rs   ← resource_ 前缀（资源管理）2个命令
```

- 命令处理器是 **薄代理**：接收参数 → 委托给 SessionRouter/ConfigManager → 返回结果
- **禁止** 在命令处理器中包含业务逻辑
- **禁止** 在命令处理器中直接访问 plugin/ 或 sdk/ 的类型

### 前端 (src-ui-new/)

| 目录 | 职责 | 放置规则 |
|------|------|---------|
| `bridge/` | IPC 契约层 | 类型定义 + 命令封装 + 事件监听 |
| `stores/` | Pinia 状态管理 | 每个关注点一个 store |
| `composables/` | 可复用逻辑 Hook | 有副作用的逻辑封装 |
| `views/` | 页面级组件 | 每个窗口入口对应一个 View |
| `components/` | UI 组件 | 按功能域子目录组织 |
| `components/settings/fields/` | 设置字段渲染器 | 每种 SettingType 一个组件 |
| `components/settings/fields/array/` | 数组 UI 策略 | 每种 ArrayUiHint 一个组件 |
| `plugins/` | 前端插件系统 | FrontendPlugin 接口实现 |
| `utils/` | 纯工具函数 | 无副作用的工具 |
| `styles/` | 全局样式 | CSS 变量定义 |
| `i18n/` | 国际化 | 语言文件 |

### 新文件放置决策树

```
新增一个功能 →
├─ 需要平台 API？
│  └─ 是 → 在 sdk/ 定义 trait，在 sdk/platform/windows/ 实现
│         → 在 HostApi 添加方法，通过 PluginHandle 暴露
├─ 是系统级配置（非插件）？
│  └─ 是 → 放 core/config/components/
├─ 是新的数据源/执行器/搜索引擎？
│  └─ 是 → 放 plugin/ 对应子目录
├─ 需要新的 IPC 命令？
│  └─ 是 → 放 commands/ 对应文件（按前缀规则）
├─ 需要前端新页面？
│  └─ 是 → 放 views/
├─ 是可复用的前端逻辑？
│  └─ 是 → 放 composables/
└─ 其他
   └─ 放 utils/
```
