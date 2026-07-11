# 设计决策记录

本文档记录 ZeroLaunch-rs 项目中的重要架构设计决策及其理由。

---

## 1. Configurable Trait 的两种配置存储模式

### 背景

项目中所有插件组件都实现了 `Configurable` trait，但在配置存储方式上存在两种不同的设计模式。

### 两种模式对比

#### 模式 A: RwLock<Inner> (类型化存储)

**特点**：解析时转换，存储类型化数据。

**使用场景**: `KeywordOptimizer` 系列组件

#### 模式 B: RwLock<Value> (原始 JSON 存储)

**特点**：直接存储原始 JSON，按需解析。

**使用场景**: `DataSource` 系列组件

### 多维度分析

| 维度             | 模式 A (Inner)                    | 模式 B (Value)         |
| ---------------- | --------------------------------- | ---------------------- |
| **类型安全**     | ✅ 编译时检查，字段访问有 IDE 支持 | ❌ 运行时解析，可能失败 |
| **性能**         | ✅ 解析一次，后续 O(1) 访问        | ❌ 每次使用都需解析     |
| **配置验证时机** | ✅ apply_settings 时立即发现错误   | ❌ 使用时才发现错误     |
| **代码简洁性**   | ❌ 需定义 Inner + 构造 JSON        | ✅ 直接存储，无需转换   |
| **数据一致性**   | ❌ 存在两份数据表示                | ✅ 单一数据源           |
| **灵活性**       | ❌ 新增字段需改结构体              | ✅ JSON 结构灵活        |

### 决策：根据组件特性选择不同模式

**不强制统一**，而是根据组件的配置复杂度和使用频率选择合适的模式：

| 组件类型         | 配置复杂度           | 使用频率                    | 采用模式 |
| ---------------- | -------------------- | --------------------------- | -------- |
| KeywordOptimizer | 简单 (2-3个字段)     | 极高 (每次 optimize 调用)   | Inner    |
| DataSource       | 复杂 (嵌套数组/对象) | 低 (仅 fetch_candidates 时) | Value    |
| ScoreBooster     | 中等                 | 高                          | Inner    |
| ActionExecutor   | 无配置               | -                           | 无状态   |
| SearchEngine     | 无配置               | -                           | 无状态   |

### 选择理由

#### KeywordOptimizer 选择 Inner 模式

1. **高频访问**: `priority` 和 `uses_context` 在每次 `optimize()`、`uses_context()`、`get_priority()` 调用时都要访问
2. **性能敏感**: 如果用 Value 模式，每次调用都要解析 JSON，性能开销不可接受
3. **配置简单**: 字段固定且少（通常 2-3 个），定义 Inner 结构体成本低
4. **类型安全**: 编译期保证字段类型正确

#### DataSource 选择 Value 模式

1. **配置复杂**: 如 `ProgramSource` 的 `directories` 配置是嵌套数组，定义完整 Inner 结构体繁琐
2. **低频使用**: 仅在 `fetch_candidates` 时解析一次，性能影响小
3. **灵活性**: 保持原始 JSON 便于扩展和迁移
4. **代码简洁**: 无需维护 Inner 与 JSON 的双向转换

---

## 2. Inner 模式的逻辑委托规范

### 背景

对于采用 `RwLock<Inner>` 模式的组件，存在两种代码组织方式：

1. **逻辑分散型**: 外壳定义辅助方法，Inner 仅存储数据
2. **逻辑委托型**: 外壳仅做委托，Inner 包含所有业务逻辑

### 决策：统一采用逻辑委托型

**外壳只做委托**：外壳方法签名与 inner 相同，方法体只有一行委托调用。

**Inner 包含完整逻辑**：所有业务逻辑集中在 inner 中。

### 验收标准

1. 外壳方法签名与 inner 相同
2. 外壳方法体只有一行：`self.inner.read().xxx(...)` 或 `self.inner.write().xxx(...)`
3. 所有业务逻辑集中在 inner 中

### 理由

1. **职责清晰**: 外壳负责并发控制，Inner 负责业务逻辑
2. **代码组织**: 避免逻辑分散在多个 impl 块中
3. **可测试性**: Inner 可独立测试（如果需要）

---

## 3. 插件系统职责分离设计

### 背景

参考 Wox 的设计，Wox 的 `PluginManager` 同时承担了插件生命周期管理和查询流程编排两个职责。这种设计在 Wox 中是合理的，因为 Wox 是一个插件驱动的启动器，查询流程本质上就是调用插件。

但对于 ZeroLaunch，如果计划将 `ProgramManager` 拆分成多个小插件，采用**职责分离**的设计会更加清晰。

### 决策：拆分为 Registry + Dispatcher

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

### 与 Wox 设计的对比

| 方面              | Wox 设计                | 本设计                            |
| ----------------- | ----------------------- | --------------------------------- |
| **PluginManager** | 生命周期 + 查询流程     | 拆分为 Registry + Dispatcher      |
| **查询入口**      | `PluginManager.Query()` | `QueryDispatcher.dispatch()`      |
| **插件发现**      | PluginManager 内部      | Registry 独立管理                 |
| **扩展性**        | 需要修改 PluginManager  | 可独立扩展 Registry 或 Dispatcher |
| **职责**          | 混合职责                | 单一职责                          |

### 选择理由

1. **单一职责**: Registry 只管插件，Dispatcher 只管查询
2. **可独立测试**: 可以单独测试 Registry 或 Dispatcher
3. **灵活扩展**: 可以替换 Dispatcher 实现不同的查询策略
4. **清晰的依赖**: Dispatcher 依赖 Registry，方向明确

---

## 4. Plugin SDK 层设计

### 背景

在当前架构中，插件（如 `FileLauncher`、`UrlLauncher`）直接调用框架层的平台相关函数，存在以下问题：

| 问题                 | 影响                         |
| -------------------- | ---------------------------- |
| 插件与框架隐式耦合   | 框架函数变更会波及所有插件   |
| 平台差异泄露到插件层 | 插件代码包含平台特定逻辑     |
| 难以独立测试         | 需要 mock 框架层才能测试插件 |

为解决这些问题，并支持未来的跨平台开发（macOS、Linux），引入 **Plugin SDK** 层。

### 架构分层

```
┌────────────────────────────────────────────────────────────────┐
│                        应用层 (Application)                     │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                      插件层 (Plugins)                           │
│              只依赖 HostApi 接口（sdk/host_api.rs）             │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     sdk/ (Plugin SDK)                           │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  HostApi trait (抽象接口)                                │ │
│  │  图标服务 / Shell 服务 / 窗口服务 / PlatformCapabilities │ │
│  └──────────────────────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  platform/ (平台实现层)                                   │ │
│  │  WindowsHostApi    MacHostApi(预留)    LinuxHostApi(预留) │ │
│  └──────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

### 核心设计原则

| 原则           | 说明                                               |
| -------------- | -------------------------------------------------- |
| **关注点分离** | 插件只关注「做什么」，SDK 负责平台差异的「怎么做」 |
| **接口抽象**   | 通过 trait 定义能力契约，平台实现可替换            |
| **渐进式演进** | 先完善 SDK 接口设计，再补充各平台实现              |
| **测试友好**   | 可注入 Mock 实现进行单元测试                       |

### 接口设计要点

#### 1. 接口粒度控制

- **太细**：每个操作一个方法 → 每个平台都要实现多个方法，且逻辑相似
- **太粗**：一个通用执行方法 → 插件需要自己拼命令，平台差异泄露到插件层
- **合适**：语义化接口 + 枚举参数 → 平台层统一处理差异，插件层语义清晰

#### 2. 平台能力差异处理

不同平台能力不对等是必然的。

设计建议：
- SDK 接口返回 `Result<Option<...>>` 或定义 `Capability` 查询
- 插件通过 `host.capabilities()` 查询平台支持的能力
- UI 层根据能力动态隐藏/禁用不可用功能

---

## 5. 目录结构设计原则

| 原则         | 说明                                                            |
| ------------ | --------------------------------------------------------------- |
| **职责分离** | 每个目录有明确的职责边界                                        |
| **依赖方向** | `commands → plugin_system → plugin → core → platform`，单向依赖 |
| **插件优先** | 功能通过插件实现，核心只提供基础设施                            |
| **可测试性** | 每个模块可独立测试，依赖通过 trait 抽象                         |
| **跨平台**   | 平台相关代码隔离在 platform/ 目录                               |

---

## 6. 并发安全设计

### 问题

嵌套锁可能导致死锁。

### 解决方案

- 使用单一锁或避免嵌套
- 使用 `DashMap` 进行细粒度锁


## 7. 远程适配器统一为 RemoteComponent（取消多 adapter 委托）

### 背景

第三方插件以子进程 + JSON-RPC 方式运行。构造插件组件时，当前实现会为一个逻辑组件生成**两个 struct 实例**：

```
RemoteConfigurableAdapter  → 处理 Configurable（身份、配置项、Schema）
RemoteDataSourceAdapter    → 处理 DataSource（fetch_candidates）
                             ↑ 内含 Arc<RemoteConfigurableAdapter>，手动委托所有 Configurable 方法
```

`RemoteExecutorAdapter`、`RemotePluginAdapter` 同理，各含一个 `Arc<RemoteConfigurableAdapter>` 并逐一手写委托。

### 问题

同一个逻辑组件的 identity 被分散在多个 struct 中：

- `component_id` 在 `RemoteConfigurableAdapter` 和 `RemoteDataSourceAdapter` 中各存一份（值完全相同）
- 向 `Configurable` trait 新增一个方法，需要同步修改 4 个 adapter 文件，且每个 domain adapter 都要加一行机械委托
- 上次 commit `4bd1838` 仅新增一个 `component_description` 字段就触达 **46 个文件**，其中 Remote*Adapter 的委托是纯样板代码

更根本的矛盾：**内置插件是「一个 struct 实现多个 trait」**（同一个 `Arc<AppSource>` 可同时作为 `Arc<dyn Configurable>` 和 `Arc<dyn DataSource>`），而远程插件被迫用**多 struct 委托**来模拟同一行为，与内置架构不一致。

### 决策：用单一 `RemoteComponent` 取代分散的 Remote*Adapter

**核心思想**：让远程插件组件回归内置插件的模式——一个 struct 同时实现 `Configurable`、`DataSource`、`ActionExecutor`、`Plugin`。

#### 7.1 基础模型

远程组件由通用身份/通信字段 + `kind` enum 组成。种类专属数据应放在 enum 变体中，而不是摊平成 struct 上的 `Option<T>` 或空 Vec。

```rust
/// 远程插件的单个组件。无论 ComponentKind 是什么，都由同一个 struct 承载。
pub struct RemoteComponent {
    // ── 身份（Configurable）──
    pub component_id: String,
    pub component_name: String,
    pub component_description: String,
    pub component_type: ComponentType,
    pub priority: u32,

    // ── 通信 ──
    pub client: Arc<JsonRpcClient>,

    // ── 私有缓存 ──
    cached_schema: RwLock<Vec<SettingDefinition>>,
    cached_settings: RwLock<serde_json::Value>,
    /// 配置动作缓存。config_actions() 是 Configurable trait 的通用方法，
    /// 所有组件类型（DataSource / ActionExecutor / Plugin）均可能使用
    /// （例：BookmarkSource 提供了"自动检测浏览器"和"读取书签"两个动作），
    /// 因此放在 struct 层面而非 kind variant 内部。
    cached_actions: RwLock<Vec<ConfigActionDef>>,

    // ── 种类与专属数据 ──
    pub kind: RemoteComponentKind,
}

pub enum RemoteComponentKind {
    DataSource,
    ActionExecutor {
        target_types: Vec<TargetType>,
        result_actions: Vec<ResultAction>,
    },
    Plugin {
        metadata: PluginMetadata,
    },
}

impl Configurable for RemoteComponent { /* 从通用字段/缓存读取 */ }
impl DataSource for RemoteComponent { /* match DataSource => RPC */ }
impl ActionExecutor for RemoteComponent { /* match ActionExecutor { ... } */ }
impl Plugin for RemoteComponent { /* match Plugin { ... } */ }
```

设计要点：

1. **用 enum payload 表达种类差异**：`RemoteComponentKind` 的每个变体携带该种类**专属**的数据。所有种类共享的数据（如 `cached_actions`）保留在 struct 层面。判断标准：字段只有一个变体需要→入 variant；所有变体都需要→入 struct。
2. **缓存字段私有**：`cached_schema`、`cached_settings`、`cached_actions` 是内部优化，不是组件的公开契约。`Configurable` 通过方法暴露，外部不直接读写缓存。
3. **kind 不匹配时返回错误，不 panic**：各 trait 实现中先 `match` `self.kind`，若种类错误则返回 `PluginError::InvalidComponentKind`，保证运行时安全。

#### 7.2 构造与注册

`build_adapters()` 从两步（先建 ConfigurableAdapter、再包一层 DomainAdapter）简化为一步：

```rust
fn build_components(
    init_result: &InitResult,
    client: Arc<JsonRpcClient>,
) -> Vec<Arc<RemoteComponent>> {
    init_result
        .components
        .iter()
        .map(|comp| {
            Arc::new(RemoteComponent {
                component_id: comp.component_id.clone(),
                component_name: comp.component_name.clone(),
                component_description: comp.component_description.clone(),
                component_type: comp.component_type,
                priority: comp.priority,
                client: client.clone(),
                cached_schema: RwLock::new(comp.setting_schema.clone()),
                cached_settings: RwLock::new(comp.default_settings.clone()),
                cached_actions: RwLock::new(comp.config_actions.clone()),
                kind: comp.kind.clone(),
            })
        })
        .collect()
}
```

`PluginRegistration` 从四个独立 Vec 简化为一个：

```rust
pub struct PluginRegistration {
    pub plugin_id: String,
    pub manifest: Manifest,
    pub components: Vec<Arc<RemoteComponent>>,
}
```

消费者通过显式转换方法注册，避免 `as` 散落在业务代码中：

```rust
impl RemoteComponent {
    pub fn as_data_source(self: Arc<Self>) -> Option<Arc<dyn DataSource>> {
        matches!(self.kind, RemoteComponentKind::DataSource)
            .then(|| self as Arc<dyn DataSource>)
    }

    pub fn as_action_executor(self: Arc<Self>) -> Option<Arc<dyn ActionExecutor>> {
        matches!(self.kind, RemoteComponentKind::ActionExecutor { .. })
            .then(|| self as Arc<dyn ActionExecutor>)
    }

    pub fn as_plugin(self: Arc<Self>) -> Option<Arc<dyn Plugin>> {
        matches!(self.kind, RemoteComponentKind::Plugin { .. })
            .then(|| self as Arc<dyn Plugin>)
    }
}

for comp in &registration.components {
    if let Some(ds) = comp.clone().as_data_source() {
        router.register_data_source(ds).await;
    }
    if let Some(ex) = comp.clone().as_action_executor() {
        router.register_executor(ex);
    }
    if let Some(pl) = comp.clone().as_plugin() {
        router.register_remote_plugin(pl);
    }
}
```

#### 7.3 可选进阶：统一内置/远程的 `ComponentCore`

上述模型已经能让远程插件与内置插件采用同一模式（一个 struct 多 trait）。如果想进一步统一两者的**身份与配置存储**，可以抽出一个共用的 `ComponentCore`：

```rust
pub struct ComponentCore {
    pub component_id: String,
    pub component_name: String,
    pub component_description: String,
    pub component_type: ComponentType,
    pub priority: u32,
    cached_settings: RwLock<serde_json::Value>,
}
```

- 远程：`RemoteComponent { core: ComponentCore, client, kind }`
- 内置：`AppSource { core: ComponentCore, plugin_handle }` 等

**影响范围**：需要修改 20+ 内置组件 struct 的定义、`Configurable` impl 以及工厂函数。改动机械但文件多，建议作为第二阶段独立进行，不要与远程重构混在一起。

**建议**：

- **第一阶段**：只做远程端的 `RemoteComponent` + `RemoteComponentKind` 重构，解决当前痛点；
- **第二阶段**：评估是否值得抽 `ComponentCore`。如果目标是「内置/远程代码完全同构」，则做；否则方向 1 已足够。

### 收益

| 维度                   | 当前                                         | 优化后                                             |
| ---------------------- | -------------------------------------------- | -------------------------------------------------- |
| 新增 Configurable 方法 | 改 trait + 4 个 adapter 各加委托             | 改 trait + 1 个 `RemoteComponent` impl             |
| component_id 存储      | 两处（configurable + domain adapter 各一份） | 一处                                               |
| 种类专属数据表达       | 摊平成 struct 上的 `Option`/空 Vec           | 收拢到 enum 变体                                   |
| 架构一致性             | 内置/远程模式不同                            | **相同模式**（一个 struct 多 trait）               |
| 可删除文件             | —                                            | 4 个 adapter 文件合并为 1 个 `remote_component.rs` |

### 此前讨论过的其他方案

1. **宏消除委托样板**：写一个 `delegate_configurable!(self.configurable)` 宏，每个 domain adapter 一行。最轻量，但不解决多 struct 的架构不一致问题。
2. **`HasConfigurable` + blanket impl**：分离 `Configurable` 与领域 trait 的继承关系，用 blanket impl 自动生成委托。改动较大，会波及所有调用点。
3. **彻底拆开继承关系**：让 `DataSource` 等不再继承 `Configurable`，消费者通过 registry 分别获取两个 trait object。与方案 2 类似但更彻底。

上述方案都是在**修补**现有的多 struct 架构，而本方案是**消除**多 struct 本身，从根源上让远程架构与内置架构统一。
