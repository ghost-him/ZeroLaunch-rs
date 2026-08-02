---
name: no-callsite-crate-path
description: 禁止在调用点书写 crate:: 完整路径 — 类型/函数/宏须在文件头 use 导入
condition:
  - '(?m)^(?![ \t]*(pub(\([^)]*\))?[ \t]+)?use[ \t]+).*\bcrate::[A-Za-z_][A-Za-z0-9_]*::'
scope:
  - tool:edit(**/*.rs)
  - tool:write(**/*.rs)
  - tool:ast_edit
---

# 禁止在调用点书写 crate:: 完整路径

## 约束

在 Rust 代码中，**禁止**在调用点（表达式、参数、返回值、impl 等位置）书写 `crate::...` 完整限定路径。
类型、函数、枚举及枚举变体、宏**必须**在文件开头的 `use` 语句中导入，调用点使用短名。

## 原因

- 调用点全限定路径与 `use` 导入重复，同一类型在文件内出现两种引用形态，读代码需两处对照
- 模块路径变更（重构、移动模块）时所有调用点必须同步修改，`use` 集中在一处则只需改文件头
- 文件头 `use` 集中展示文件的依赖面，可读性与可审查性更高

## 正确写法

```rust
use crate::plugin_framework::QueryChannel;

let response = session_router
    .route_query(&trace_id, &query, QueryChannel::Ui)
    .await;
```

## 错误写法

```rust
let response = session_router
    .route_query(&trace_id, &query, crate::plugin_framework::QueryChannel::Ui)
    .await;
```

## 例外与边界

- `use` / `pub use` 语句本身（含 `pub(crate) use` 等 re-export）不受此约束——它们正是导入的载体
- 同名类型消歧时在文件头使用 `use ... as ...` 别名，不在调用点写完整路径
- `super::` 前缀同理：跨模块引用一律文件头导入，不在调用点展开
