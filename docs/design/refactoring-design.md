# ZeroLaunch-rs 重构设计文档

本文档描述 ZeroLaunch-rs 的系统架构设计理念和核心概念。

---

## 一、架构总览

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

## 二、SDK 层设计理念

### 2.1 架构定位

SDK 层是**跨平台抽象层**，负责：
1. 定义跨平台能力 trait（契约）
2. 提供各平台的具体实现 (`sdk/platform/<os>/`)
3. 通过 `HostApi` 统一暴露给上层，上层无需关心平台差异

### 2.2 HostApi - 核心枢纽

`HostApi` 是 SDK 层的**中心结构体**，持有所有平台实现，并提供统一 API：

```
┌──────────────────────────────────────────────────────────────────┐
│  HostApi (跨平台 struct)                                          │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ register(plugin_id, config) → Arc<PluginHandle>            │  │
│  │ capabilities()                                             │  │
│  │                                                             │  │
│  │ icon_extractor: Arc<dyn IconExtractor> ← 平台注入           │  │
│  │ shell_executor: Arc<dyn ShellExecutor> ← 平台注入           │  │
│  │ window_manager: Arc<dyn WindowManager> ← 平台注入           │  │
│  │ app_enumerator: Arc<dyn AppEnumerator> ← 平台注入           │  │
│  │ app_launcher: Arc<dyn AppLauncher>     ← 平台注入           │  │
│  │ ...                                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### 2.3 PluginHandle - 插件服务句柄

插件通过 `HostApi::register()` 获取 `PluginHandle`，句柄绑定了插件身份与配置：

```
┌──────────────────────────────────────────────────────────────────┐
│  PluginHandle (跨平台 struct)                                     │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ plugin_id: "everything"                                    │  │
│  │ config: { icon_cache_level: SkipAll }                      │  │
│  │ icon_extractor: Arc<dyn IconExtractor>                     │  │
│  │ shell_executor: Arc<dyn ShellExecutor>                     │  │
│  │ ...                                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
│  get_icon() → icon_extractor.get_icon(cache,..)                 │  │
│  shell_open() → shell_executor.shell_open(..)                   │  │
│  activate_window() → window_manager.activate(..)                │  │
└──────────────────────────────────────────────────────────────────┘
```

### 2.4 Platform Capabilities - 平台能力查询

不同平台能力不对等是必然的：

| 能力                        | Windows                       | macOS              | Linux                     |
| --------------------------- | ----------------------------- | ------------------ | ------------------------- |
| 图标提取 (IconExtraction)   | 完整支持                      | 部分支持           | 部分支持                  |
| Shell 打开 (ShellOpen)      | ShellExecuteW                 | NSWorkspace        | xdg-open                  |
| 以管理员运行 (RunAsAdmin)   | runas                         | osascript          | pkexec                    |
| 应用枚举 (AppEnumeration)   | shell:AppsFolder              | Launch Services DB | .desktop + Flatpak + Snap |
| 应用启动 (AppLaunch)        | IApplicationActivationManager | LSOpenURLsWithRole | flatpak run / snap run    |
| 窗口激活 (WindowActivation) | Win32 API                     | NSWorkspace        | wmctrl                    |

### 2.5 推送式回调模式

Hotkey 和 InstallationMonitor 采用**推送式回调**模式：SDK 主动向程序发出函数调用。

```
插件/核心程序调用 SDK            InstallationMonitor (Push-based)
─────────────                       ─────────────
   Plugin                            File System
     │                                  │ 变化事件
     ▼                                  ▼
  PluginHandle.get_icon()         InstallationMonitor (SDK)
     │                                  │
     ▼                                  ▼ 依次调用
  SDK 平台实现                      Callback 1
                                   Callback 2
                                   Callback N
```

---

## 三、Core 层设计理念

### 3.1 Configurable Trait - 统一配置契约

所有可配置组件（插件、数据源、搜索引擎、核心组件等）都实现此 trait：

```
┌───────────────────┐
│   Configurable    │  ← 基础配置能力
└─────────┬─────────┘
          │
    ┌─────┴─────┬─────────────────┬───────────────┐
    ▼           ▼                 ▼               ▼
┌─────────┐ ┌───────────┐ ┌───────────────┐ ┌─────────────┐
│ Plugin  │ │ DataSource│ │ SearchEngine  │ │ ScoreBooster│
└─────────┘ └───────────┘ └───────────────┘ └─────────────┘
```

**设计原则**：
- `apply_settings(&self, ...)` 使用不可变引用，组件内部通过 `RwLock` 实现可变性
- 数值配置读取统一使用 `as_f64()` 而非 `as_i64()`，预防前端浮点数传入导致的解析失败

### 3.2 ConfigManager - 配置管理中枢

`ConfigManager` 统一管理所有可配置组件的注册、CRUD、持久化和事件发布：

```
组件注册 → ConfigurableRegistry
配置读写 → apply_settings(验证→应用→回调→事件→持久化)
持久化   → ConfigStore（本地JSON）+ 可选远程同步（WebDAV）
事件     → broadcast channel，SessionRouter 等订阅
```

### 3.3 配置组件目录组织规范

**核心原则**：所有**核心程序**（非插件）的 Configurable 组件属于上层业务配置，统一放在 `core/config/components/` 下，按能力维度组织。

**SDK 层与 Configurable 的职责边界**：
- `sdk/` 只提供**平台原语**（trait + 实现），如 `HotkeyManager`、`StorageService trait`。
- `core/config/components/` 提供**业务配置组件**（实现 `Configurable` trait）。

---

## 四、Plugin 层设计理念

### 4.1 模块结构

```
plugin/
├── data_source/        # 数据源实现
├── executor/           # 动作执行器实现
├── keyword_optimizer/  # 关键词优化器
├── score_booster/      # 分数提升器
├── search_engine/      # 搜索引擎实现
└── triggerable/        # 触发式插件
```

### 4.2 核心 Trait 继承关系

| Trait              | 职责                                                      | 继承           |
| :----------------- | :-------------------------------------------------------- | :------------- |
| `Configurable`     | **基础配置能力**（在 `core/types/` 定义，在此 re-export） | -              |
| `DataSource`       | **提供者**：产出原始搜索候选项                            | `Configurable` |
| `KeywordOptimizer` | **处理器**：对查询关键词进行清洗或扩展                    | `Configurable` |
| `SearchEngine`     | **匹配器**：计算候选项与关键词的匹配得分                  | `Configurable` |
| `ScoreBooster`     | **排序器**：基于用户习惯进行二次分值提升                  | `Configurable` |
| `ActionExecutor`   | **执行器**：定义如何执行动作                              | `Configurable` |
| `Plugin`           | **独立功能**：处理特定指令的完整闭环插件                  | `Configurable` |

### 4.3 插件分类：管道插件 vs 独立插件

插件分为两类，它们在程序中的角色和控制流截然不同。

**管道插件**（`DataSource`、`KeywordOptimizer`、`SearchEngine`、`ScoreBooster`、`ActionExecutor`）在核心程序固定的管道中运行。管道顺序不可变：

```
CandidatePipeline.collect()
  → DataSource[]      (采集候选)
  → KeywordOptimizer[](优化关键词)
SearchPipeline.search()
  → SearchEngine      (基础打分)
  → ScoreBooster[]    (个性化提分)
ExecutorRegistry.resolve(ctx, action_id) → executor
  → executor.execute(ctx, action_id).await   (执行动作)
```

管道插件是**可替换的处理步骤**：更换搜索引擎、增删数据源、调整排序策略，都不改变管道结构本身。它们始终在线，每次搜索都会经过完整管道。

**独立插件**（`Plugin`）通过触发关键词（如 `=` 触发计算器）激活。一旦触发，插件**绕过整个搜索管道**，直接接管本次会话的查询与执行：

```
用户输入 → 解析触发词 → 命中 → SessionMode::Plugin
  → plugin.query()       (插件自处理查询)
  → plugin.execute_action() (插件自处理执行)
  → 返回 CustomPanel     (插件自定 UI)
```

独立插件在被触发期间拥有本次会话的完整控制权，核心程序不做任何管道处理。会话结束后控制权归还核心程序。

| 维度 | 管道插件 | 独立插件 |
|------|---------|---------|
| **激活方式** | 始终运行 | 触发关键词前缀匹配 |
| **控制流** | 经过固定管道 | 绕过管道，自处理 |
| **结果类型** | `QueryResponse::List` | `QueryResponse::CustomPanel` |
| **SessionMode** | `Search` | `Plugin(plugin_id)` |
| **配置变更响应** | 重建候选缓存或搜索管道 | 仅记录日志 |

---

## 五、PluginSystem 层设计理念

### 5.1 职责分离架构

```
┌─────────────────────────────────────────────────────────────┐
│                    职责分离架构                              │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐      ┌─────────────────────┐      │
│  │  PluginRegistry     │      │  QueryDispatcher    │      │
│  │  (插件注册中心)      │      │  (查询分发器)        │      │
│  │                     │      │                     │      │
│  │  • 插件注册/注销     │      │  • 查询分发         │      │
│  │  • 插件发现         │      │  • 并行查询         │      │
│  │  • 触发词映射       │      │  • 结果聚合         │      │
│  └─────────────────────┘      └─────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

**设计优势**：
- **单一职责**：Registry 只管插件，Dispatcher 只管查询
- **可独立测试**：可以单独测试 Registry 或 Dispatcher
- **灵活扩展**：可以替换 Dispatcher 实现不同的查询策略

---

## 六、数据流向

### 6.1 搜索流程

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

### 6.2 执行流程

```
用户选择候选项 → SessionRouter.route_confirm()
  ├─ 插件模式 → Plugin.execute_action()
  └─ 搜索模式 → ExecutorRegistry.resolve(ctx, action_id)
      ├─ 得到 Arc<dyn ActionExecutor> → executor.execute(ctx, action_id).await
      ├─ ActivationFailed → ExecutorRegistry.resolve_fallback(ctx, fallback_action)
      │   └─ fallback_executor.execute(ctx, fallback_action).await
      └─ 成功 → SearchPipeline.record(candidate_id, query) → ScoreBooster.record()
```

### 6.3 配置变更流程

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
       ├─ SearchEngine/ScoreBooster → rebuild_search_pipeline()
       └─ ActionExecutor/Plugin/Core → 记录 debug 日志
```

---

## 七、搜索引擎设计理念

搜索引擎（SearchEngine）负责计算候选项的基础分数。传统搜索与语义搜索是**互斥的搜索策略**，而非串联的处理步骤。

```
┌─────────────────────────────────────────────────────────────┐
│                      SearchPipeline                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  SearchEngine (选择其一)                             │   │
│  │  ┌─────────────────┐    ┌─────────────────────────┐ │   │
│  │  │ TraditionalEngine│ OR │ SemanticEngine         │ │   │
│  │  │ 拼音/模糊匹配    │    │ 向量相似度计算          │ │   │
│  │  └─────────────────┘    └─────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────┘   │
│                           ↓                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  ScoreBooster[] (串联执行)                           │   │
│  │  ┌─────────────────┐    ┌─────────────────────────┐ │   │
│  │  │ HistoryBooster  │ →  │ QueryAffinityBooster    │ │   │
│  │  └─────────────────┘    └─────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 八、执行器设计理念

执行器（ActionExecutor）负责执行候选项的动作。`execute()` 方法内部会判断程序是否已运行：如果已运行则唤醒窗口，否则启动新进程。

不同目标类型由不同的 Executor 实现处理：

| Executor 实现          | 处理的 TargetType                |
| ---------------------- | -------------------------------- |
| PathExecutor           | Path（可执行文件路径）           |
| AppExecutor            | App（系统应用，跨平台统一）      |
| UrlExecutor            | Url（网址）                      |
| CommandExecutor        | Command / BuiltinCommand（命令） |
| WindowActivateExecutor | Path, App（窗口唤醒）            |

---

## 九、核心工程规范

### 9.1 代码风格
- 命名：Rust 蛇形命名（函数/变量）、驼峰命名（类型/trait）
- 函数头部**必须**包含简要功能描述、参数及返回值含义

### 9.2 配置处理安全
- JSON 数值配置统一使用 `as_f64()` 而非 `as_i64()`（预防前端浮点数传入导致解析失败）

### 9.3 最小化改动
- 优先在现有框架内解决问题，除非必要或用户明确要求，否则避免大面积重构

### 9.4 模块间调用设计
- **SDK 层定义能力契约，不定义调用逻辑**
- **Core 层 Config 组件定义调用逻辑（何时/怎么调用 HostApi）**
- **插件通过 PluginHandle 句柄访问服务，不直接操作平台实现**
- **推送式服务（Hotkey/InstallationMonitor）通过回调注册机制实现**
- **所有跨模块操作经过 HostApi 统一出口**

---

详细设计请参考：
- [plugin-sdk.md](plugin-sdk.md) - Plugin SDK 详细设计
- [config-system.md](config-system.md) - 配置系统详细设计
- [decisions.md](decisions.md) - 设计决策记录
