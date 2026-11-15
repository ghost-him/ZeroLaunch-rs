---
description: Plugin Trait Init — async init 接收 PluginContext 和 Option<Arc<PluginHandle>>，禁止在插件内部状态存储 PluginHandle；on_shutdown 为成对的退出钩子
condition: "impl Plugin for|fn init\\(|on_shutdown"
scope: "tool:edit(src-tauri/src/builtin_plugin/**), tool:write(src-tauri/src/builtin_plugin/**), tool:edit(crates/plugin-api/src/plugin/**), tool:write(crates/plugin-api/src/plugin/**), tool:edit(crates/plugin-api/src/host/**), tool:write(crates/plugin-api/src/host/**)"
---

# Plugin Trait Init

- `Plugin::init()` 是 async 方法：`async fn init(&self, ctx: &PluginContext, handle: Option<Arc<PluginHandle>>)`（请求级上下文 + 可选插件服务句柄）
- `PluginHandle` 从 `HostApi::register(plugin_id, config)` 获取，绑定插件身份与配置
- 用 `handle` 参数执行平台操作。用 `ctx` 参数获取 trace_id、query_id 等请求级信息
- **禁止** 在实现 `Plugin` trait 的第三方插件内部状态中存储 `PluginHandle`；平台能力仅经 `init` 参数传入的 `Arc<PluginHandle>` 访问（`PluginContext` 仅携带 trace_id/query_id 等请求级信息，**无** 句柄访问渠道）
- **内置插件**（进程内、生命周期与宿主一致）：允许在 `init()` 中保存 `Arc<PluginHandle>` 到自身状态，供 `query`/`execute_action` 经句柄访问平台能力（如剪贴板）。宿主 MUST 在启动注册完成后对全部内置插件调用 `init()`（bootstrap.rs 启动序列内联的 init 循环，遍历 `SessionDispatcher::plugin_registry().get_all()`），确保句柄发放
- **注意**：内置组件（DataSource/ActionExecutor/Configurable）通过构造函数 `new(handle: Arc<PluginHandle>)` 获取 PluginHandle 是允许的，不在此禁令范围内
- **生命周期成对**：`init()` 是启动钩子（内置在 bootstrap 注册完成后调用，远端经 `plugin/init` RPC，每次进程启动调用一次）；退出侧是 `async fn on_shutdown(&self, ctx: &PluginContext) -> Result<(), PluginError>`（默认空实现）。**两类插件共用同一入口**：应用退出时 src-tauri 遍历插件注册表对每个插件调用一次（远端适配器内部转 `plugin/shutdown` RPC）；卸载/重载/卸载安装由 plugin-host `teardown` 调用。进程关闭基础设施只负责关进程，不得重复发送钩子；崩溃时不调用
