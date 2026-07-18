---
description: 新增平台能力流程 — trait 定义在 services/<capability>/，平台实现在 platform-windows/，通过 HostApi Arc<dyn Trait> 暴露
condition: "services/|platform-windows|Arc<dyn|pub trait|HostApi|PluginHandle"
scope: "tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**), tool:edit(crates/platform-windows/src/**), tool:write(crates/platform-windows/src/**), tool:edit(src-tauri/src/sdk.rs), tool:write(src-tauri/src/sdk.rs)"
---

# 新增平台能力的流程

**正确做法：**
1. 在 `crates/plugin-api/src/services/<capability>/` 中定义 trait。每个能力域包含 `mod.rs`（重新导出）、一个 trait 文件、以及按需的 `types.rs`（共享类型）
2. 在 `crates/platform-windows/src/` 中实现
3. 在 `HostApi` 结构体中添加 `Arc<dyn Trait>` 字段
4. 通过 `HostApi` 方法暴露（如果是请求-响应模式，再通过 `PluginHandle` 暴露）

**错误做法：**
- 把平台特定代码放在 `core/` 或 `plugin/` 中
- 从插件代码直接调用平台 API

## 新增方法决策

- 特权方法（仅核心调用）→ 只在 `HostApi` 上实现
- 通用方法（插件也需要）→ 只在 `PluginHandle` 上实现
- 如需新 trait 依赖，则在 `HostApi` 上添加 `Arc<dyn NewTrait>` 字段，再在 `register()` 中 clone 给 `PluginHandle`
