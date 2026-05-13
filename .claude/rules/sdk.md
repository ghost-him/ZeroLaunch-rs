---
paths:
  - "src-tauri/src/sdk/**"
---

# SDK / 平台抽象层规范

## 平台抽象

- SDK 层定义 **能力契约**（trait）。SDK 层 **不** 定义调用逻辑（何时/如何调用平台服务）
- 调用逻辑属于 `core/config/components/`（业务配置组件）
- 每个平台能力包含三部分：
  1. `sdk/<capability>/` 中的 trait（如 `IconExtractor`、`ShellExecutor`、`HotkeyManager`）
  2. `sdk/platform/<os>/` 中的平台实现（如 `WindowsIconExtractor`）
  3. `HostApi` 中的 `Arc<dyn Trait>` 字段

## HostApi — 唯一跨模块出口

- `HostApi` 是所有平台操作的 **唯一** 出口。`sdk/` 之外所有需要平台服务的代码都通过 `HostApi`
- `HostApi` 方法体委托给注入的 `Arc<dyn Trait>` 实现。**禁止** 在 `HostApi` 方法中重复平台逻辑 — 委托给 trait
- 新增 `HostApi` 方法时：先加到 `HostApi` 结构体，再考虑 `PluginHandle` 是否也要暴露
- **禁止** 从 `plugin/` 或 `core/` 直接调用平台 API

## 新增平台能力的流程

**正确做法：**
1. 在 `sdk/<capability>/<name>.rs` 中定义 trait
2. 在 `sdk/platform/<os>/<name>.rs` 中实现
3. 在 `HostApi` 结构体中添加 `Arc<dyn Trait>` 字段
4. 通过 `HostApi` 方法暴露（如果是请求-响应模式，再通过 `PluginHandle` 暴露）

**错误做法：**
- 把平台特定代码放在 `core/` 或 `plugin/` 中
- 从插件代码直接调用平台 API

## 推送式回调模式

对于向应用推送事件的服务（`HotkeyManager`、`InstallationMonitor`、`FocusMonitor`、`TimerManager`）：
- **正确**：通过 `register_callback(id, callback)` 注册回调，通过 `unregister_callback(id)` 取消注册
- **正确**：将回调存储在线程安全集合（`DashMap`）中，事件发生时依次调用所有回调
- **正确**：通过 `start_watching()` / `stop_watching()` 管理生命周期
- **正确**：回调注册/注销可以通过 `PluginHandle` 暴露，插件通过句柄注册自己的回调
- **正确**：`PluginHandle` 上的回调注册方法内部用 `plugin_id` 前缀化 callback ID，避免不同插件间的 ID 冲突
- **全局生命周期管理**（`start_listening`、`stop_listening`、`start_watching`、`stop_watching`）保留在 `HostApi` 上，插件只能注册/注销自己的回调，不能启停全局服务

## 平台能力

- 每个平台通过 `PlatformCapability` 枚举声明其支持的能力。用 `capabilities()` 查询
- 消费平台服务的代码 **必须** 优雅处理 `UnsupportedCapability` 错误。**禁止** 假设任何能力在所有平台都可用
- UI **必须** 基于平台能力隐藏/禁用功能，而不对不支持的功能显示错误

## 模块组织

- `sdk/` 模块按能力域组织。当前支持的能力域请直接阅读 `sdk/` 目录结构，以代码为准
- 每个能力域包含：`mod.rs`（重新导出）、一个 trait 文件、以及按需的 `types.rs`（共享类型）
- 平台实现放入 `sdk/platform/<os>/`，按所实现的 trait 命名
