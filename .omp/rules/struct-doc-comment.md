---
description: 新建结构体/枚举时必须添加注释：职责 + 使用范围；结构体每个字段、枚举每个变体也必须写注释说明含义
condition: '(?m)^\s*(pub(\([^)]*\))?\s+)?(struct|enum)\s+\w+'
scope: "tool:edit(**/*.rs), tool:write(**/*.rs)"
---

# 新建结构体与枚举必须添加注释

你在 `.rs` 文件中编写了结构体或枚举定义。每个新结构体和每个新枚举必须附带注释，说明以下两点：

1. **职责/作用** — 这个类型负责什么，承载什么数据
2. **使用范围** — 在哪些场景、模块中使用（防止被不该使用的模块误引用）

## 结构体字段注释

结构体内的**每个字段**必须附带注释，说明其含义、取值范围、用途等，防止字段意图不清晰。

字段注释也可以使用 Rust doc 注释 `///` 或普通注释 `//`。描述应当覆盖：

1. **字段用途** — 这个字段存储什么数据
2. **取值范围/约束** — 如果有特殊边界（如非空、不可为负、单位等）应当说明

## 枚举变体注释

枚举的**每个变体**必须附带注释（单元变体同样需要），说明：

1. **变体语义** — 该变体代表什么状态/结果/形态
2. **产生时机与消费方** — 什么场景产生该变体、由谁（前端/流程/插件）消费
3. **变体内字段** — 与结构体字段同等要求，逐字段说明含义与约束

```rust
/// 插件查询响应 —— 一次查询的展示结果契约（跨 IPC 序列化）。
///
/// 由 Plugin::query 返回，经 SessionDispatcher 路由后下发前端；
/// 四种变体对应前端不同的展示形态。
pub enum QueryResponse {
    /// 候选列表结果 —— 默认搜索与插件均可返回，前端按列表渲染。
    List {
        /// 排序后的候选项列表（含动作、占位符统计等展示元数据）。
        results: Vec<ListItem>,
    },
    /// 空结果 —— 无任何展示内容。前端映射 mode "search" + 空数组。
    Empty,
}
```

**不充分（缺少枚举本体注释或变体注释）：**

```rust
pub enum QueryResponse {
    /// 候选列表结果
    List { results: Vec<ListItem> },
    Empty,
}
```

## 标准格式

使用 Rust doc 注释 `///`（推荐）或普通注释 `//`：

```rust
/// 应用执行器 — 通过 PluginHandle::launch_app() 启动系统应用。
///
/// 仅在 executor 管道中使用，由 SessionDispatcher 调度。
/// 数据源和其他模块不应直接持有此类型。
pub struct AppExecutor {
    /// 提供组件 ID、名称、类型等基础元数据。
    core: ComponentCore,
    /// 访问平台能力的桥接句柄。
    plugin_handle: Arc<PluginHandle>,
}

## 示例

**正确：**

```rust
/// 书签数据源 — 从 Chrome/Edge 浏览器解析书签文件产出候选项。
///
/// 仅在 candidate_pipeline 的 DataSource 阶段使用，
/// 由 `bookmark_source` 模块内部构造，外部通过 PluginHandle 访问。
pub struct BookmarkSource {
    /// 组件 ID、名称、类型等元数据。
    core: ComponentCore,
    /// 通过 RwLock 提供内部可变性，仅在 apply_settings 时写入。
    settings: RwLock<BookmarkSourceSettings>,
}

```rust
/// 内部辅助类型：单条内置命令定义。
///
/// 仅限本文件 (`builtin_command_source.rs`) 内使用。不导出到模块外部。
struct BuiltinCommandDef {
    /// 命令名称，用于匹配用户输入。
    name: &'static str,
    /// 命令执行内容，传递到后台执行。
    command: &'static str,
}
```

**不充分（缺少类型使用范围注释）：**

```rust
/// 书签数据源
pub struct BookmarkSource {
    ...
}
```

```rust
// 临时用一下
struct TempData {
    ...
}
```

## 特殊情况

- **仅本文件使用的私有结构体/枚举**：注释写 `仅限本文件内使用` 即可
- **序列化/反序列化数据模型**（data transfer / config structs）：需说明数据来源、反序列化自哪个配置段
- **需要内部可变性的结构体**：额外注明使用了 `RwLock`/`Mutex` 的原因
- **需要内部可变性的字段**：`RwLock`/`Mutex` 字段需注明为何需要内部可变性，以及写入时机（如"仅在 apply_settings 时写入"）
- **`Option` 字段**：需说明 `None` 的语义（如"None 表示未执行过"、"None 表示使用全局默认值"）
- **跨 IPC 序列化的枚举**：变体注释需注明对应的前端 mode 词表 / 序列化键名（如 `serde(rename)` 后的名称），防止前后端契约漂移
