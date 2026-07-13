---
description: SDK/平台抽象层总览：平台能力模型、HostApi/PluginHandle 权限隔离、当前已实现能力域、Crate 边界
condition: ".*"
scope: "tool:read(src-tauri/src/sdk.rs), tool:edit(src-tauri/src/sdk.rs), tool:write(src-tauri/src/sdk.rs), tool:read(crates/plugin-api/src/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**), tool:read(crates/platform-windows/src/**), tool:edit(crates/platform-windows/src/**), tool:write(crates/platform-windows/src/**)"
interruptMode: never
---

# SDK / 平台抽象层总览

## 平台抽象

- SDK 层 **只** 提供平台能力（trait 定义 + 平台实现），**不** 关心用户配置。用户配置的读取、校验、持久化由 Configurable 组件（`builtin_plugin/config/` 或 `builtin_plugin/`）负责
- SDK 定义 **能力契约**（trait），调用逻辑（何时/如何调用平台服务）属于 `builtin_plugin/config/`（业务配置组件）
- 每个平台能力包含三部分：
  1. `crates/plugin-api/src/services/<capability>/` 中的 trait（如 `IconExtractor`、`ShellExecutor`、`HotkeyManager`）
  2. `crates/platform-windows/src/` 中的平台实现（如 `WindowsIconExtractor`）
  3. `HostApi` 中的 `Arc<dyn Trait>` 字段

## HostApi / PluginHandle — 权限隔离模型

`HostApi` 和 `PluginHandle` 是**两个独立对象**，各自持有 `Arc<dyn Trait>` 引用。权限隔离在编译期由 Rust 类型系统保证，无运行时检查。

| | HostApi（特权对象） | PluginHandle（通用句柄） |
|---|---|---|
| **位置** | `src-tauri/src/sdk.rs` | `crates/plugin-api/src/host/plugin_handle.rs` |
| **可见性** | 宿主内部，插件不可见 | 通过 `HostApi::register()` 获取 |
| **职责** | 窗口控制、全局生命周期管理 | 图标、shell、路径、回调注册等通用服务 |
| **典型方法** | `hide_window`, `show_window`, `compute_window_position`, `update_icon_cache_dir`, `capture_parameter_snapshot`, `apply_autostart_setting` | `get_icon`, `shell_open`, `resolve_path`, `enumerate_apps`, `register_hotkey_callback` |

- `HostApi` 方法体 **必须** 委托给注入的 `Arc<dyn Trait>` 实现
- 核心程序在 `lib.rs` 中同时持有两者：以 `"core"` 为 ID 注册 `PluginHandle` 复用通用服务，特权操作直接调 `HostApi`

## 平台能力

- 每个平台通过 `PlatformCapability` 枚举声明其支持的能力。用 `capabilities()` 查询
- 消费平台服务的代码 **必须** 优雅处理 `UnsupportedCapability` 错误
- UI **必须** 基于平台能力隐藏/禁用功能

## 当前已实现的能力域

所有能力域遵循 `crates/plugin-api/src/services/<domain>/` 模式，平台实现在 `crates/platform-windows/src/`。能力域名与 HostApi/PluginHandle 方法名的对应以源码为准。新增能力 **必须** 在 services 目录下创建对应子目录。

## Crate 边界规范

Crate 结构与依赖方向详见 `.omp/AGENTS.md` 的 Workspace 结构段。
- **共享编解码器**：LSP `Content-Length` 帧编解码在 `crates/plugin-protocol/src/codec.rs` 中定义，由 `plugin-host` 和 `plugin-sdk-rust` 共享，避免重复实现
