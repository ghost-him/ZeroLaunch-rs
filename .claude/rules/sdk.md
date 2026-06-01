---
paths:
  - "src-tauri/src/sdk/**"
---

# SDK / 平台抽象层规范

## 平台抽象

- SDK 层 **只** 提供平台能力（trait 定义 + 平台实现），**不** 关心用户配置。用户配置的读取、校验、持久化由 Configurable 组件（`core/config/components/` 或 `plugin/`）负责
- SDK 定义 **能力契约**（trait），调用逻辑（何时/如何调用平台服务）属于 `core/config/components/`（业务配置组件）
- 每个平台能力包含三部分：
  1. `sdk/<capability>/` 中的 trait（如 `IconExtractor`、`ShellExecutor`、`HotkeyManager`）
  2. `sdk/platform/<os>/` 中的平台实现（如 `WindowsIconExtractor`）
  3. `HostApi` 中的 `Arc<dyn Trait>` 字段

## HostApi — 唯一跨模块出口

- `HostApi` 是所有平台操作的 **唯一** 出口。`sdk/` 之外所有需要平台服务的代码都通过 `HostApi`
- `HostApi` 方法体 **必须** 委托给注入的 `Arc<dyn Trait>` 实现
- 新增 `HostApi` 方法时：**必须** 先加到 `HostApi` 结构体，再考虑 `PluginHandle` 是否也要暴露
- 核心程序也通过 `PluginHandle` 调用 `HostApi`（插件名字为 `core`），`HostApi` 只暴露只对核心程序开放的方法。对于插件与核心程序都可以使用的方法，**必须** 只在 `PluginHandle` 上暴露，不需要在 `HostApi` 上实现，以减少重复的代码。

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
- 消费平台服务的代码 **必须** 优雅处理 `UnsupportedCapability` 错误
- UI **必须** 基于平台能力隐藏/禁用功能

## 模块组织

- `sdk/` 模块按能力域组织。当前支持的能力域请直接阅读 `sdk/` 目录结构，以代码为准
- 每个能力域包含：`mod.rs`（重新导出）、一个 trait 文件、以及按需的 `types.rs`（共享类型）
- 平台实现放入 `sdk/platform/<os>/`，按所实现的 trait 命名

## 当前已实现的能力域（以代码为准）

| 能力域 | Trait 位置 | Windows 实现 | HostApi 方法 |
|--------|-----------|-------------|-------------|
| 应用枚举 | `sdk/app/app_enumerator.rs` | `WindowsAppEnumerator` | `enumerate_apps()` |
| 应用启动 | `sdk/app/app_launcher.rs` | `WindowsAppLauncher` | `launch_app()` |
| 开机自启 | `sdk/autostart/autostart_manager.rs` | `WindowsAutoStartManager` | `set_autostart()` / `is_autostart_enabled()` |
| 焦点监控 | `sdk/focus_monitor/monitor.rs` | `WindowsFocusMonitor` | `register_focus_callback()` |
| 全局热键 | `sdk/hotkey/hotkey_manager.rs` | `WindowsHotkeyManager` | `register_hotkey()` / `apply_hotkey_config()` |
| 图标提取 | `sdk/icon/icon_extractor.rs` | `WindowsIconExtractor` | `get_icon_or_default()` |
| 安装监控 | `sdk/installation_monitor/monitor.rs` | `WindowsInstallationMonitor` | `start_installation_monitor()` |
| 参数解析 | `sdk/parameter/resolver.rs` | `DefaultParameterResolver` | `resolve_parameters()` |
| 路径解析 | `sdk/path/path_resolver.rs` | `WindowsPathResolver` | `resolve_path()` |
| Shell 执行 | `sdk/shell/shell_executor.rs` | `WindowsShellExecutor` | `shell_open()` / `shell_execute_elevation()` |
| .lnk 解析 | `sdk/shell/lnk_resolver.rs` | `WindowsLnkResolver` | `resolve_lnk()` |
| 资源加载 | `sdk/shell/resource_loader.rs` | `WindowsResourceLoader` | `parse_localized_names_from_dir()` |
| 存储服务 | `sdk/storage/storage_service.rs` | `LocalStorageService` / `WebDavStorageService` | `storage()` |
| 定时器 | `sdk/timer/timer_manager.rs` | `TokioTimerManager` | `set_interval()` / `set_timeout()` |
| 窗口管理 | `sdk/window/window_manager.rs` | `WindowsWindowManager` | `show_window()` / `hide_window()` |

- 新增能力 **必须** 在此表中登记
- 表中的方法名是简化表示，以 `HostApi` 源码和 `PluginHandle` 源码为准

## Crate 边界规范（workspace 拆分后）

SDK 已拆分为多 crate workspace：

| Crate | 路径 | 内容 |
|-------|------|------|
| `zerolaunch-plugin-api` | `crates/plugin-api/` | traits、数据类型、HostApi error types、PluginHandle、Plugin trait |
| `zerolaunch-platform-windows` | `crates/platform-windows/` | Windows 平台实现 + `build_windows_host_api_builder()` |
| `zerolaunch-rs` | `src-tauri/` | 主程序：ConfigManager、SessionRouter、内置插件、IPC 命令 |

- **新增 SDK trait**：在 `crates/plugin-api/src/services/<domain>/` 定义，在 `crates/platform-windows/src/` 实现
- **插件作者只依赖** `zerolaunch-plugin-api`，不需要 Tauri / Windows / 主程序源码
- **src-tauri 中的 sdk/** 现为 re-export 桥，类型本体在 plugin-api
