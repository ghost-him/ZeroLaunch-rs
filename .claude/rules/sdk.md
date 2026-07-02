---
paths:
  - "src-tauri/src/sdk.rs"
  - "crates/plugin-api/src/**"
  - "crates/platform-windows/src/**"
---

# SDK / 平台抽象层规范

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
- **新增方法决策**：特权方法（仅核心调用）→ 只在 `HostApi` 上实现；通用方法（插件也需要）→ 只在 `PluginHandle` 上实现。如需新 trait 依赖，则在 `HostApi` 上添加 `Arc<dyn NewTrait>` 字段，再在 `register()` 中 clone 给 `PluginHandle`

## 新增平台能力的流程

**正确做法：**
1. 在 `crates/plugin-api/src/services/<capability>/` 中定义 trait
2. 在 `crates/platform-windows/src/` 中实现
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
- 消费平台服务的代码 **必须** 优雅处理 `UnsupportedCapability` 错误
- UI **必须** 基于平台能力隐藏/禁用功能

## 模块组织

- 能力域 trait 定义在 `crates/plugin-api/src/services/<domain>/`，平台实现在 `crates/platform-windows/src/`
- 每个能力域包含：`mod.rs`（重新导出）、一个 trait 文件、以及按需的 `types.rs`（共享类型）
- `src-tauri/src/sdk.rs` 现为 re-export 桥，类型本体在 plugin-api

## 当前已实现的能力域（以代码为准）

| 能力域 | Trait 位置 | Windows 实现 | HostApi 方法 |
|--------|-----------|-------------|-------------|
| 应用枚举 | `crates/plugin-api/src/services/app/` | `WindowsAppEnumerator` | `enumerate_apps()` |
| 应用启动 | `crates/plugin-api/src/services/app/` | `WindowsAppLauncher` | `launch_app()` |
| 开机自启 | `crates/plugin-api/src/services/autostart/` | `WindowsAutoStartManager` | `set_autostart()` / `is_autostart_enabled()` |
| 焦点监控 | `crates/plugin-api/src/services/focus_monitor/` | `WindowsFocusMonitor` | `register_focus_callback()` |
| 全局热键 | `crates/plugin-api/src/services/hotkey/` | `WindowsHotkeyManager` | `register_hotkey()` / `apply_hotkey_config()` |
| 图标提取 | `crates/plugin-api/src/services/icon/` | `WindowsIconExtractor` | `get_icon_or_default()` |
| 安装监控 | `crates/plugin-api/src/services/installation_monitor/` | `WindowsInstallationMonitor` | `start_installation_monitor()` |
| 参数解析 | `crates/plugin-api/src/services/parameter/` | `DefaultParameterResolver` | `resolve_parameters()` |
| 路径解析 | `crates/plugin-api/src/services/path/` | `WindowsPathResolver` | `resolve_path()` |
| Shell 执行 | `crates/plugin-api/src/services/shell/` | `WindowsShellExecutor` | `shell_open()` / `shell_execute_elevation()` |
| .lnk 解析 | `crates/plugin-api/src/services/shell/` | `WindowsLnkResolver` | `resolve_lnk()` |
| 资源加载 (trait) | `crates/plugin-api/src/services/resource/` | `WindowsResourceLoader` (`AppResourceService`) | `get_icon_path()` |
| 资源加载 (shell) | `crates/plugin-api/src/services/shell/` | `WindowsResourceLoader` (`ResourceLoader`) | `parse_localized_names_from_dir()` |
| 存储服务 | `crates/plugin-api/src/services/storage/` | `LocalStorageService` / `WebDavStorageService` | `storage()` |
| 定时器 | `crates/plugin-api/src/services/timer/` | `TokioTimerManager` | `set_interval()` / `set_timeout()` |
| 窗口管理 | `crates/plugin-api/src/services/window/` | `WindowsWindowManager` | `show_window()` / `hide_window()` |

- 新增能力 **必须** 在此表中登记
- 表中的方法名是简化表示，以 `HostApi` 源码和 `PluginHandle` 源码为准

## Crate 边界规范（workspace 拆分后）

SDK 已拆分为多 crate workspace：

| Crate | 路径 | 内容 |
|-------|------|------|
| `zerolaunch-plugin-api` | `crates/plugin-api/` | traits、数据类型、HostApi error types、PluginHandle、Plugin trait |
| `zerolaunch-platform-windows` | `crates/platform-windows/` | Windows 平台实现（14 个 trait impl） |
| `zerolaunch-rs` | `src-tauri/` | 主程序：ConfigManager、SessionRouter、内置插件、IPC 命令 |

- **新增 SDK trait**：在 `crates/plugin-api/src/services/<domain>/` 定义，在 `crates/platform-windows/src/` 实现
- **插件作者只依赖** `zerolaunch-plugin-api`，不需要 Tauri / Windows / 主程序源码
- **src-tauri 中的 sdk.rs** 现为 re-export 桥，类型本体在 plugin-api
- **共享编解码器**：LSP `Content-Length` 帧编解码在 `crates/plugin-protocol/src/codec.rs` 中定义，由 `plugin-host` 和 `plugin-sdk-rust` 共享，避免重复实现
