---
description: 新建结构体时必须添加注释：职责 + 使用范围；结构体每个字段也必须写注释说明含义
condition: '(?m)^\s*(pub(\([^)]*\))?\s+)?struct\s+\w+'
scope: "tool:edit(**/*.rs), tool:write(**/*.rs)"
---

# 新建结构体必须添加注释

你在 `.rs` 文件中编写了结构体定义。每个新结构体必须附带注释，说明以下两点：

1. **职责/作用** — 这个结构体负责什么，承载什么数据
2. **使用范围** — 在哪些场景、模块中使用（防止被不该使用的模块误引用）

同时，结构体内的**每个字段**也必须附带注释，说明其含义、取值范围、用途等，防止字段意图不清晰。

## 字段注释格式

字段注释也可以使用 Rust doc 注释 `///` 或普通注释 `//`。描述应当覆盖：

1. **字段用途** — 这个字段存储什么数据
2. **取值范围/约束** — 如果有特殊边界（如非空、不可为负、单位等）应当说明
```rust
/// 应用执行器 — 通过 PluginHandle::launch_app() 启动系统应用。
///
/// 仅在 executor 管道中使用，由 SessionRouter 调度。
/// 数据源和其他模块不应直接持有此类型。
pub struct AppExecutor {
    /// 提供组件 ID、名称、类型等基础元数据。
    core: ComponentCore,
    /// 访问平台能力的桥接句柄。
    plugin_handle: Arc<PluginHandle>,
}

## 标准格式

使用 Rust doc 注释 `///`（推荐）或普通注释 `//`：

```rust
/// 应用执行器 — 通过 PluginHandle::launch_app() 启动系统应用。
///
/// 仅在 executor 管道中使用，由 SessionRouter 调度。
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

**不充分（缺少结构体使用范围和/或字段注释）：**

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

- **仅本文件使用的私有结构体**：注释写 `仅限本文件内使用` 即可
- **序列化/反序列化数据模型**（data transfer / config structs）：需说明数据来源、反序列化自哪个配置段
- **需要内部可变性的结构体**：额外注明使用了 `RwLock`/`Mutex` 的原因
- **需要内部可变性的字段**：`RwLock`/`Mutex` 字段需注明为何需要内部可变性，以及写入时机（如"仅在 apply_settings 时写入"）
- **`Option` 字段**：需说明 `None` 的语义（如"None 表示未执行过"、"None 表示使用全局默认值"）
