---
description: PluginHandle 使用 — 插件必须通过 PluginHandle 访问宿主能力（接口层），实现全部在宿主 HostApi（impl PluginHost）与平台实现
condition: "PluginHandle|HostApi::register|PluginHost"
scope: "tool:edit(src-tauri/src/**), tool:write(src-tauri/src/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**)"
---

# PluginHandle 使用（三层架构）

- **内置插件**（进程内）**必须** 通过 `PluginHandle`（从 `HostApi::register()` 获取）访问宿主能力。`PluginHandle` 是插件接口层（薄视图）：只含插件身份/配置/能力集，方法全部委托 `PluginHost` 契约（`crates/plugin-api/src/host/plugin_host.rs`）——实现方为宿主 `HostApi`（`src-tauri/src/sdk.rs`），宿主内部直接处理宿主级服务或转发 `PlatformServices` 平台端口
- **禁止** 内置插件绕过 `PluginHandle` 直接调用平台实现或宿主字段；需要新操作时按 `add-platform-capability` 规则补全契约（`PluginHost` 方法 + `HostApi` impl + `PluginHandle` 委托）
- 第三方插件（子进程）无法直接接触平台 API，经 SDK `host/*` RPC 访问宿主能力（宿主侧 `TauriHostCallHandler` 同样经 `PluginHost` 契约委托宿主执行）——「经 PluginHandle」约束内置插件
- 宿主自身使用能力（窗口控制、通知、热键配置、自启动等宿主级操作）直接调用 `HostApi` inherent 方法，**不经** `PluginHost` 契约/插件中转
