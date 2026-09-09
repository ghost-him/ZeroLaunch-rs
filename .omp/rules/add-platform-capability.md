---
description: 新增平台能力/平台 crate 流程 — trait 定义在 services/<capability>/，平台实现集中在 crates/platform-<os>/，宿主经 src-tauri/src/platform.rs 编译期别名注入，禁止直引平台 crate
condition: "services/|platform-|Arc<dyn|pub trait|HostApi|PluginHandle|platform.rs"
scope: "tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**), tool:edit(crates/platform-windows/src/**), tool:write(crates/platform-windows/src/**), tool:edit(crates/platform-macos/src/**), tool:write(crates/platform-macos/src/**), tool:edit(src-tauri/src/sdk.rs), tool:write(src-tauri/src/sdk.rs), tool:edit(src-tauri/src/platform.rs), tool:write(src-tauri/src/platform.rs)"
---

# 新增平台能力 / 新增平台 crate 的流程

**正确做法：**
1. 在 `crates/plugin-api/src/services/<capability>/` 中定义 trait。每个能力域包含 `mod.rs`（重新导出）、一个 trait 文件、以及按需的 `types.rs`（共享类型）
2. 在 `crates/platform-<os>/src/` 中实现（现有平台：`platform-windows/`、`platform-macos/`）
3. 在 `HostApi` 结构体中添加 `Arc<dyn Trait>` 字段
4. 通过 `HostApi` 方法暴露（如果是请求-响应模式，再通过 `PluginHandle` 暴露）

**错误做法：**
- 把平台特定代码放在 `core/`、`builtin_plugin/` 或 `plugin_framework/` 中（平台实现必须集中在 `crates/platform-<os>/`）
- 宿主（src-tauri）代码绕过 `src-tauri/src/platform.rs` 直接引用任何平台 crate 的具体类型（平台 crate 依赖方向为 `plugin-api ← platform-<os>`，src-tauri 只能经 platform.rs 别名消费）
- 从内置插件代码绕过 `PluginHandle` 直接调用平台 API（第三方插件在子进程，天然不可达，经 SDK `host/*` RPC）

## 新增 OS 平台 crate（第二路径）

当为新的操作系统新增平台 crate 时，**不要**为每个既有能力重新定义 trait，而是：

1. 在 workspace 根 `Cargo.toml` 注册新 crate（`members` + `[workspace.dependencies]` 别名）
2. 新 crate 直接实现 `plugin-api` 中既有的 services traits，模块按服务拆分（如 `hotkey.rs`、`icon.rs`），crate 头写 `#![cfg(target_os = "<os>")]` 使非目标平台编译为空 crate
3. 在 `src-tauri/src/platform.rs` 中为对应 `#[cfg(target_os = "<os>")]` 段添加该 crate 类型的 `Platform*` 别名与 `platform_capabilities()` 转发
4. `src-tauri/Cargo.toml` 中经 `[target.'cfg(target_os = "<os>")'.dependencies]` 声明依赖（保持非目标平台不解析）
5. 宿主启动链路（`build_platform_host_api_builder`、`bootstrap.rs`）只消费 `platform.rs` 的 `Platform*` 别名，不感知具体平台
6. 新 crate 的平台差异（能力位无法表达的降级/恒空实现、监听语义差异）必须用注释显式登记；能力粒度不足时宁可隐藏入口也不静默半注册

**平台 crate 自身**：实现可以是纯跨平台 Rust + CLI 调用（便于其他平台交叉编译验证），也可以是目标平台 FFI 绑定（如 macos 的 objc2 系），后者仅在目标平台编译。

## 新增方法决策

- 特权方法（仅核心调用）→ 只在 `HostApi` 上实现
- 通用方法（插件也需要）→ 只在 `PluginHandle` 上实现
- 如需新 trait 依赖，则在 `HostApi` 上添加 `Arc<dyn NewTrait>` 字段，再在 `register()` 中 clone 给 `PluginHandle`

## Mock 同步

- 新增能力域/`PluginHandle` 方法后，**必须** 同步 `crates/plugin-api/src/mock/stubs.rs` 与 `mock/helpers.rs`：`mock_plugin_handle()` 构造的桩需注入对应 `Stub*Service`，否则 mock 编译失败

## 验证

- `platform-<os>` crate 含 `#![cfg(target_os)]` 头时，非目标平台 `cargo check` 只验证空 crate；纯跨平台实现的 crate 可临时去掉该头在任意平台编译验证，验证后必须恢复
- 目标平台专用 FFI 绑定代码无法在非目标平台编译，改动后必须由目标平台 CI 或实机验证
