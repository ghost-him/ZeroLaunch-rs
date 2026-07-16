# 架构原则

> 本文件是 code-review skill **Agent 4a（架构结构）与 4b（架构行为）**的核心审查依据。
> 所有原则均从既有代码实证提炼（**固化既有**），而非理想化设计。
> 当原则与代码冲突时：原则仍合理 → 改代码（A 类）；原则已过时 → 更新本文件（B 类）。

## P1 职责驱动的代码放置

**规则**：代码该放在哪个模块，由它承担的职责决定。每个模块/目录有明确的职责边界，跨边界放置即为违规。

### 项目放置约定（实证提炼）

| 代码类别 | 必须放在 | 不可放在 | 依据 |
|---------|---------|---------|------|
| `#[tauri::command]` 函数 | `commands/*.rs`（按前缀分文件） | core/、plugin_framework/ | commands.md 前缀表 |
| IPC 请求/响应 struct | `commands/` 内（如 `bridge.rs`） | core/、plugin_framework/、builtin_plugin/ | 实证：BridgeQueryResponse 等均在 commands/bridge.rs |
| IPC 边界错误类型 | `commands/` 内 | 根目录、core/ | bridge_error.rs:9 注释已声明职责 |
| 配置类型（Settings、Schema） | `core/config/` | commands/、plugin_framework/ | config.md |
| 插件框架类型（SessionRouter、PluginManager、Registry） | `plugin_framework/` | commands/、core/ | .omp/AGENTS.md |
| 内置插件实现 | `builtin_plugin/` | plugin_framework/（框架不含具体实现） | .omp/AGENTS.md |
| 平台操作（shell、窗口、托盘） | `tray/`、`window/`、`sdk.rs` | commands/、前端 | 前端是薄展示层 |
| 通用工具函数 | `utils/` | 任何业务模块 | .omp/AGENTS.md |

**检查方式**：LLM 按上表逐条核对变更文件的放置；新增 struct/函数必须归类到对应模块。

## P2 类型的职责边界 = 使用范围

**原则**：一个类型的**定义位置编码了它的职责**，职责决定了**使用范围**。类型不可越出其职责对应的范围被使用。

### 判定方法论

对变更中涉及的**每个**类型（struct / enum / trait），按以下四步判断。不能因为某类型未被脚本监控就认为没有违规——方法论适用于所有类型，包括新增的。

**第一步：确定类型的职责**

- 它定义在哪个模块/层级？（定义位置 = 职责的编码）
- 它的命名暗示什么职责？（`Bridge*` / `*Payload` / `*Response` 暗示边界类型；`*Manager` / `*Router` 暗示领域类型）
- 它如何被使用？（`#[derive(Serialize)]` 且服务于 IPC = 边界；内部逻辑传递 = 领域）

**第二步：分类**

- **边界类型** — 存在目的是跨越系统边界（IPC 通信、JSON-RPC 协议、HTTP API 响应、文件格式）
  - 应定义在：边界层（`commands/`、`crates/plugin-protocol/`、`cli_server/`）
  - 允许使用：仅在该边界层内
  - 禁止：泄漏到内部模块（被 `core/`、`plugin_framework/`、`builtin_plugin/`、`state/` 引用或存入字段）

- **领域类型** — 代表内部业务逻辑与状态
  - 应定义在：领域层（`core/`、`plugin_framework/`）
  - 允许使用：同层或更高层的内部模块
  - 禁止：直接序列化跨边界传输而不经边界层转换为 DTO（如把 `ConfigManager` 直接 `Serialize` 给前端）

- **共享/工具类型** — 跨层复用的基础类型（错误类型、配置值、通用结构）
  - 应定义在：需要它的最低层（`utils/`、`core/`）
  - 允许使用：所有更高层
  - 禁止：定义在高层却被低层引用（同时也是 P3 违规）

**第三步：检查实际使用是否越出范围**

- 边界类型出现在内部模块 → **向内泄漏**（违规）
- 领域类型直接跨边界传输未经转换 → **向外泄漏**（违规）
- 类型定义在高层却用于低层 → **反向依赖**（同时是 P3 违规）

**第四步：错误类型的特殊处理**

- 内部错误留在内部，边界错误留在边界
- 转换发生在边界层（通过 `From` impl）
- 内部模块不可依赖边界错误类型（如 `BridgeError` 不可被 `core/` 使用——`bridge_error.rs:9` 已声明此约束）

### 确定性脚本兜底（已知边界类型）

`check-type-scope.sh` 自动监控以下**已知**边界类型是否泄漏到内部模块：

- `commands/*.rs` 内所有 `pub struct`（IPC DTO）
- `BridgeError`（边界错误）

脚本覆盖的是已知案例。对于变更中**新增**的类型，LLM 必须按上述方法论独立判断。

### 既有实例参考（方法论的应用样本）

| 类型 | 职责 | 分类 | 允许范围 | 违规示例 |
|------|------|------|---------|---------|
| `BridgeQueryResponse` 等 | IPC 响应 | 边界类型 | `commands/` | 被 `core/` use 引用 |
| `BridgeError` | IPC 错误 | 边界类型 | `commands/` | 被 `plugin_framework/` 当错误用 |
| `SessionRouterError` 等 | 各域内部错误 | 领域类型 | 各域内部 | 泄漏到其他职责域作通用错误 |
| `ConfigManager` 等 | 内部领域模型 | 领域类型 | L3+ | 被 `utils/` 引用；或直接 Serialize 给前端 |
| `CliToken` | CLI 鉴权令牌 | 共享类型 | 应在 `core/`（L2） | 定义在 `cli_server/`（L5）被 `state/`（L4）引用 = 反向依赖 |

**检查方式**：`check-type-scope.sh` 确定性检测已知边界类型泄漏；LLM 按方法论对变更中所有类型（含新增）独立判断。

## P3 编译期层级与依赖方向

**规则**：`use crate::X` 中 X 的层级必须 **≤** 调用方层级。高层可用低层，低层不可用高层。

### 内部模块层级表（实证提炼自 `use crate::` 依赖图）

| 层级 | 模块 | 职责 | 实证依据（依赖项） |
|------|------|------|------------------|
| L0 | `utils`、`logging` | 通用基础 | 零 `use crate::` |
| L1 | `sdk.rs` | 平台抽象 re-export | 零 `use crate::`，仅依赖外部 crate |
| L2 | `core` | 领域核心（ConfigManager、Configurable、AppCommand） | 零跨模块依赖，完全自包含 |
| L3 | `plugin_framework` | 框架层（SessionRouter、PluginManager、Pipeline、Registry） | → core, sdk |
| L3 | `tray`、`window` | 平台相关服务 | → core, sdk |
| L4 | `builtin_plugin` | 内置插件实现 | → core, plugin_framework, utils |
| L4 | `state` | 组合根（聚合所有 manager 实例的 AppState） | → core, plugin_framework, sdk, tray |
| L5 | `commands`、`cli_server` | 入口层（IPC / HTTP） | → state + 下层 |
| L6 | `bootstrap`、`lib`、`main` | 编排层（启动装配） | → core, plugin_framework, state, tray, window |

### 层级判定原理

**层级 = 你依赖的最高层模块 + 1。** 不是由"做多少逻辑"决定，而是由"必须知道哪些类型"决定。`state` 持有 `Arc<SessionRouter>`（L3），就必须 `use` L3 类型，所以 `state` ≥ L4——它不能再低，否则 L3 反向依赖它就是违规。

### 特例

- **`state`（L4）作为组合根**，允许引用 L3 的多个不同模块——这是它的职责（聚合）。但 L3 模块不可反向引用 `state`。
- **`bootstrap`（L6）作为编排层**，允许引用各层做装配。但被装配的模块不可反向引用 `bootstrap`。

### 已知既有违规（脚本会报，列为"既有问题"）

| 位置 | 违规 | 根因 | 修复方向 |
|------|------|------|---------|
| `state/app_state.rs:1` `use crate::cli_server::token::CliToken` | L4 → L5 反向 | `CliToken` 共享类型被定义在入口层 `cli_server/` | 将 `CliToken` 下沉到 `core/` 或独立共享类型模块 |

**检查方式**：`check-deps-direction.sh` 扩展内部模块层级检查（确定性）。

## P4 运行时职责域解耦

**规则**：三个**同级职责域**之间，**类型级 `use` 允许向下**（如 session_router 导入 ConfigEvent 类型），但**运行时直接方法调用禁止**，必须走事件通道 / 管道 / 回调。

> 注意：P3 管的是**编译期类型依赖方向**；P4 管的是**运行时行为耦合**。两者是不同维度，不可混为一谈。

### 三大核心职责域

| 职责域 | 所在模块 | 职责 | 运行时交互方式 |
|--------|---------|------|---------------|
| **ConfigManager**（配置生命周期） | `core/config` | 校验→写入→副作用→广播 ConfigEvent→持久化 | 发布 `ConfigEvent`；监听 `PluginRuntimeEvent` |
| **SessionRouter**（查询路由） | `plugin_framework/session_router` | 搜索/插件模式分发、候选缓存、执行路由 | 监听 `ConfigEvent` 重建管道；**不直接调** ConfigManager 方法 |
| **PluginManager**（插件进程生命周期） | `plugin_framework/manager` | 加载/卸载/崩溃重启第三方插件 | 发布 `PluginRuntimeEvent`；**不持有** ConfigManager 引用 |

### 解耦链路（实证，见 plugin-system.md）

```
PluginManager ──PluginRuntimeEvent──▶ ConfigManager ──ConfigEvent──▶ SessionRouter
   (发布)                              (监听+同步注册)               (监听+重建管道)
```

三个域**两两不直接方法调用**。`CandidatePipeline` / `SearchPipeline` 是由这三个域**动态生成**的管道对象，不算独立职责域。

### 允许 vs 禁止

| 场景 | 判定 |
|------|------|
| `session_router.rs` 内 `use crate::core::config::ConfigEvent`（导入事件类型） | ✅ 允许（类型级向下） |
| `session_router.rs` 内 `config_manager.apply_settings(...)`（直接调方法） | ❌ 禁止（运行时直连职责域） |
| `plugin_framework/manager.rs` 持有 `Arc<ConfigManager>` 字段 | ❌ 禁止（plugin-system.md 已声明"不再持有直接引用"） |
| `commands/` 内 `state.get_config_manager().apply_settings(...)` | ✅ 允许（入口层编排，不是职责域互调） |

**检查方式**：部分脚本化——grep `config_manager\.` / `session_router\.` / `plugin_manager\.` 在彼此职责域模块内的方法调用；LLM 复核调用语义区分"类型导入"与"运行时调用"。

## P5 通信方式随关系而定

**规则**：模块间的关系决定允许的通信方式，不可错用。

| 关系 | 允许的通信方式 | 禁止 |
|------|--------------|------|
| 跨层（高 → 低） | 直接方法调用、类型导入 | — |
| 同职责域内（如 core/config 内部子模块） | 直接调用 | — |
| 同级不同职责域（ConfigManager ↔ SessionRouter） | 事件通道、管道、回调 | 直接方法调用（见 P4） |
| 跨进程（主程序 ↔ 第三方插件） | JSON-RPC（plugin-protocol 定义） | 直接函数调用、共享内存 |
| 前后端（Rust ↔ TypeScript） | Tauri IPC（invoke / emit-listen） | 前端直接做业务逻辑/平台操作 |

**检查方式**：LLM 按关系表判断变更中引入的调用方式是否匹配关系类型。

## P6 接口复用优先于新增

**规则**：新功能应优先由一批设计良好的**既有接口组合**完成，而非每个功能新增一个并行接口。新增抽象必须有充分理由。

### 既有核心抽象（必须优先沿用）

| 抽象 | 位置 | 用途 |
|------|------|------|
| `PluginHandle` | `crates/plugin-api` | 插件访问宿主能力的统一入口 |
| `ExecutorRegistry` | `plugin_framework` | 按 (TargetType, action_id) 路由执行器 |
| `CandidatePipeline` | `plugin_framework` | 候选采集 + 关键词优化链 |
| `SearchPipeline` | `plugin_framework` | 搜索引擎 + score booster 链 |
| `Configurable` trait | `core` | 配置生命周期（priority、apply_settings、on_settings_changed） |
| `inventory::submit!` | `builtin_plugin` | 内置组件编译期自动注册 |
| 事件通道（PluginRuntimeEvent / ConfigEvent） | `core`/`plugin_framework` | 职责域解耦 |

### 违规信号

- 新增一个与 `ExecutorRegistry` 平行的执行器分发机制
- 新增一个与 `CandidatePipeline` 平行的候选采集路径
- 新增 trait 仅有一个实现者且无明确扩展计划
- 绕过 `PluginHandle` 直接暴露宿主内部 API 给插件

**检查方式**：LLM 判断——变更是否新增了与上表平行的抽象？现有抽象是否能容纳该需求？

## 审查优先级

当多个原则同时触发时，按以下优先级报告。P1/P2/P3 由 Agent 4a 主审，P4/P5/P6 由 Agent 4b 主审：

1. **P3 编译期层级违规**（反向依赖）— 阻塞，脚本确定性（4a）
2. **P2 类型职责边界违规**（向内泄漏 / 向外泄漏 / 错误类型越界）— 阻塞，脚本兜底已知类型 + LLM 方法论判断所有类型（4a）
3. **P4 运行时职责域直连** — 阻塞，脚本+LLM（4b）
4. **P1 代码放置错误** — 高，LLM（4a）
5. **P6 绕过既有抽象** — 高，LLM（4b）
6. **P5 通信方式错配** — 中，LLM（4b）

P3 完全由脚本确定性覆盖。P2 由脚本兜底已知边界类型，LLM 须按方法论独立判断所有类型（含新增）。脚本结论优先于 LLM 判断——LLM 不得覆盖脚本的违规报告，只能补充语义分析；但 LLM 可独立发现脚本未覆盖的新增类型违规。
