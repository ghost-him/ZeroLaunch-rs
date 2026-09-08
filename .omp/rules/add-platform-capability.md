---
description: 新增平台能力流程 — trait 定义在 services/<capability>/，平台实现在 platform-windows/，经 PlatformServices 端口注入宿主，HostApi impl PluginHost 转发
condition: "services/|platform-windows|Arc<dyn|pub trait|PluginHost|PluginHandle|PlatformServices"
scope: "tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**), tool:edit(crates/platform-windows/src/**), tool:write(crates/platform-windows/src/**), tool:edit(src-tauri/src/sdk.rs), tool:write(src-tauri/src/sdk.rs)"
---

# 新增平台能力的流程（三层架构）

**分层职责**：
- **插件接口层**（plugin-api `host/plugin_handle.rs`）：`PluginHandle` 只保存插件身份/配置/能力集，公开方法全部委托宿主契约（`host/plugin_host.rs` 的 `PluginHost`）
- **宿主层**（src-tauri `sdk.rs`）：`HostApi` 实现 `PluginHost` 契约 —— 唯一执行节点，直接处理宿主级服务（model/storage/timer/参数/资源与缓存规约）或转发平台面操作
- **平台层**（platform-windows + plugin-api `platform/services.rs`）：`PlatformServices` 是平台实现的统一集合（平台端口），能力集与实现同源存放；组装权在平台 crate 的统一工厂 `windows_platform_services()`

**正确做法（新增能力）**：
1. 在 `crates/plugin-api/src/services/<capability>/` 中定义 trait。每个能力域包含 `mod.rs`（重新导出）、一个 trait 文件、以及按需的 `types.rs`（共享类型）
2. 在 `crates/platform-windows/src/` 中实现
3. 若为 OS 特定服务：在 `crates/plugin-api/src/platform/services.rs` 的 `PlatformServices` 添加 `Arc<dyn Trait>` 字段，并在 `platform-windows/src/platform_services.rs` 工厂中装配
4. 若插件需要该操作：在 `PluginHost` 契约添加方法（作用域参数如 plugin_id/缓存等级由委托方显式传入），`HostApi` 实现中决定「直接处理宿主级服务」或「转发 `self.platform.<service>`」；`PluginHandle` 增加委托方法（保持公开方法面与调用点稳定）

**错误做法**：
- 把平台特定代码放在 `core/`、`builtin_plugin/` 或 `plugin_framework/` 中（平台实现必须集中在 `platform-windows/`）
- 从内置插件代码绕过 `PluginHandle` 直接调用平台 API（第三方插件在子进程，天然不可达，经 SDK `host/*` RPC）
- 在 `PluginHandle` 中保存服务实现/平台 Arc 字段（职责混入宿主层）；在 `PlatformServices` 中放宿主级服务（model/storage/timer/parameter_resolver/app_resource 与 OS 无关，不属于平台面）

## 新增方法决策

- 特权方法（仅核心调用）→ 只在 `HostApi` 上实现（inherent 方法，不进 `PluginHost` 契约）
- 通用方法（插件也需要）→ 加 `PluginHost` 契约 + `HostApi` impl（转发 `self.platform` 或直接处理宿主字段）+ `PluginHandle` 委托方法

## Mock 同步

- 新增能力域/`PluginHost` 方法后，**必须** 同步：
  - `crates/plugin-api/src/mock/stubs.rs` 增加 `Stub*Service`；
  - `mock/helpers.rs` 的 `mock_platform_services()`（平台面字段）与 `MockPluginHost`（`PluginHost` 实现）；
  否则 mock 编译失败
