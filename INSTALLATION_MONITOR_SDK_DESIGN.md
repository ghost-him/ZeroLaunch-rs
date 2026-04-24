# InstallationMonitor SDK 化重构设计方案

## 一、背景与目标

### 1.1 当前状态

`InstallationMonitor` 位于 `modules/refresh_scheduler/installation_monitor.rs`，是一个独立的旧架构模块。它使用 `notify` crate 监控 Windows 开始菜单的 `.lnk` 文件变化，通过 `Condvar` 向 `RefreshScheduler` 发送信号。`RefreshScheduler` 只有一个单一回调（`Arc<dyn Fn(RefreshTrigger)>`），无法扩展。

现有问题：
- **耦合度高**：与 `RefreshScheduler` 强绑定，不可独立使用
- **不可扩展**：只支持单一回调，插件无法注册自己的监听
- **不符合新架构**：未遵循 SDK/trait/平台注入的架构范式
- **跨平台不支持**：硬编码 Windows 路径

### 1.2 重构目标

将 `InstallationMonitor` 封装为 Plugin SDK 的 HostApi 服务，遵循 `HotkeyManager` 的多回调注册模式：

1. **SDK 层**：定义跨平台 `InstallationMonitor` trait + 类型
2. **平台层**：Windows 实现（`notify` crate），未来可扩展 macOS/Linux
3. **HostApi 集成**：统一注册入口，管理回调生命周期
4. **配置组件**：在 `core/config/components/` 中新增 `InstallationMonitorConfigComponent`，负责「何时启用、监控参数」，不关心底层实现

### 1.3 关键差异：Push-based 服务

与其他 SDK 服务（图标提取、Shell 打开 → 插件调用 SDK）不同，`InstallationMonitor` 是 **Push-based** 服务：

```
插件/核心程序调用 SDK            InstallationMonitor (Push-based)
─────────────                       ─────────────
   Plugin                            File System
     │                                  │ 变化事件
     ▼                                  ▼
  PluginHandle.get_icon()         InstallationMonitor (SDK)
     │                                  │
     ▼                                  ▼ 依次调用
  SDK 平台实现                      Callback 1
                                   Callback 2
                                   Callback N
```

这与 `HotkeyManager` 的按键事件推送模式一致。

---

## 二、调用逻辑确认

### 2.1 新架构调用链

经过对现有代码库（`PLUGIN_SDK_DESIGN.md`、`REFACTORING_DESIGN.md`、`HotkeyManager` 实现）的深度分析，确认新架构的调用逻辑与用户描述一致：

```
┌──────────────────────────────────────────────────────────────────┐
│                      配置层 (Config Component)                    │
│  InstallationMonitorConfigComponent                              │
│  - 持有 Arc<HostApi>                                             │
│  - 决定「是否启用」「debounce 时间」等                              │
│  - 配置变更时调用 host_api.start/stop_installation_monitor()      │
│  - 初始化时通过 host_api.register_installation_callback() 注册回调 │
└──────────────────────────────┬───────────────────────────────────┘
                               │ 调用 HostApi 方法
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│                      HostApi (跨平台 struct)                       │
│  - register_installation_callback(id, callback)                  │
│  - unregister_installation_callback(id)                          │
│  - start_installation_monitor()                                  │
│  - stop_installation_monitor()                                   │
│  - is_installation_monitor_running()                             │
│                                                                  │
│  installation_monitor: Arc<dyn InstallationMonitor>  ← 平台注入   │
└──────────────────────────────┬───────────────────────────────────┘
                               │ 委托给 trait
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│              InstallationMonitor trait (SDK 抽象)                 │
│  - start_watching() / stop_watching()                            │
│  - is_watching()                                                 │
│  - register_callback(id, callback) / unregister_callback(id)     │
└──────────────────────────────┬───────────────────────────────────┘
                               │ 平台实现
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│              WindowsInstallationMonitor (平台实现)                 │
│  - notify crate 监控开始菜单 .lnk 文件变化                         │
│  - DashMap<String, CallbackRegistration> 管理多个回调              │
│  - 文件变化时依次调用所有已注册回调                                 │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 核心设计原则

| 层级            | 职责                                                  |
| --------------- | ----------------------------------------------------- |
| **Config 组件** | 决定「是否启用」「监控什么」「debounce 时间」         |
| **HostApi**     | 统一入口，管理回调注册/注销，委托给平台 trait         |
| **SDK trait**   | 定义平台无关的能力契约                                |
| **平台实现**    | 如何调用 OS API（`notify` crate），如何分发事件给回调 |

---

## 三、SDK 层设计

### 3.1 目录结构

```
sdk/installation_monitor/
├── mod.rs                     # 模块入口
├── types.rs                   # InstallationEvent, InstallationCallback 等类型
└── installation_monitor.rs    # InstallationMonitor trait
```

### 3.2 类型定义 (`types.rs`)

```rust
/// 安装监控事件，表示监控目录中发生了文件系统变化。
#[derive(Debug, Clone)]
pub struct InstallationEvent {
    /// 发生变化的文件路径列表
    pub changed_paths: Vec<String>,
    /// 变化类型
    pub kind: InstallationEventKind,
}

/// 文件系统变化类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationEventKind {
    /// 文件/目录被创建（可能表示新程序安装）
    Created,
    /// 文件/目录被修改
    Modified,
    /// 文件/目录被删除（可能表示程序卸载）
    Removed,
    /// 其他或混合变化
    Other,
}

/// 安装监控回调函数类型。
/// 当监控目录发生变化时，所有已注册的回调将被依次调用。
pub type InstallationCallback = Arc<dyn Fn(InstallationEvent) + Send + Sync>;

/// 回调注册信息（内部使用）。
pub(crate) struct CallbackRegistration {
    /// 回调 ID（用于注销）
    pub id: String,
    /// 回调函数
    pub callback: InstallationCallback,
}
```

**设计说明**：
- `InstallationEvent` 携带 `changed_paths` 而非单个路径，因为 `notify` 可能在一个事件中报告多个路径变化
- `InstallationEventKind` 简化自 `notify::EventKind`，提供跨平台统一视图
- 回调不设事件过滤器（与 `HotkeyManager` 不同），因为文件变化事件语义简单，调用方可自行判断

### 3.3 Trait 定义 (`installation_monitor.rs`)

```rust
use crate::sdk::host_api::HostApiError;
use crate::sdk::installation_monitor::types::InstallationCallback;
use async_trait::async_trait;

/// 安装监控器 trait，定义平台无关的文件系统监控能力契约。
/// 各平台实现此 trait，处理平台特定的文件监控逻辑。
/// 支持多个回调注册，当监控目录发生变化时依次调用所有回调。
#[async_trait]
pub trait InstallationMonitor: Send + Sync {
    /// 开始监控文件系统变化。
    /// 启动后，当监控目录发生变化时，将依次调用所有已注册的回调。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn start_watching(&self) -> Result<(), HostApiError>;

    /// 停止监控文件系统变化。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn stop_watching(&self) -> Result<(), HostApiError>;

    /// 检查是否正在监控。
    /// 返回：正在监控返回 true，否则返回 false。
    fn is_watching(&self) -> bool;

    /// 注册安装事件回调。
    /// 参数：id - 回调标识（用于注销）；callback - 回调函数。
    fn register_callback(&self, id: &str, callback: InstallationCallback);

    /// 注销安装事件回调。
    /// 参数：id - 回调标识。
    fn unregister_callback(&self, id: &str);
}
```

**设计说明**：
- 不提供事件过滤器：文件监控事件语义简单，回调自行判断是否关注
- `start_watching` / `stop_watching` 管理监听生命周期
- `register_callback` / `unregister_callback` 管理回调注册（同步方法，与 HotkeyManager 一致）

---

## 四、平台实现层设计

### 4.1 文件位置

```
sdk/platform/windows/installation_monitor.rs    # WindowsInstallationMonitor
```

### 4.2 Windows 平台实现 (`WindowsInstallationMonitor`)

```rust
use crate::sdk::host_api::HostApiError;
use crate::sdk::installation_monitor::{InstallationMonitor, InstallationCallback};
use crate::sdk::installation_monitor::types::{
    CallbackRegistration, InstallationEvent, InstallationEventKind,
};
use async_trait::async_trait;
use dashmap::DashMap;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

/// Windows 平台安装监控器实现。
/// 使用 `notify` crate 监控开始菜单 .lnk 文件变化，
/// 通过 DashMap 管理多个回调，事件发生时依次调用。
pub struct WindowsInstallationMonitor {
    /// 文件系统监控器
    watcher: Mutex<Option<RecommendedWatcher>>,
    /// 是否正在监控
    is_watching: AtomicBool,
    /// 回调注册表
    callbacks: Arc<DashMap<String, CallbackRegistration>>,
}
```

**实现要点**：

1. **监控路径**：使用 `PathResolver` 获取已知路径（开始菜单），避免硬编码
   - 公共开始菜单：`C:\ProgramData\Microsoft\Windows\Start Menu`
   - 用户开始菜单：`%APPDATA%\Microsoft\Windows\Start Menu`

2. **事件转换**：将 `notify::Event` 转换为 `InstallationEvent`
   ```rust
   fn convert_event(event: notify::Event) -> InstallationEvent {
       let kind = match event.kind {
           EventKind::Create(_) => InstallationEventKind::Created,
           EventKind::Modify(_) => InstallationEventKind::Modified,
           EventKind::Remove(_) => InstallationEventKind::Removed,
           _ => InstallationEventKind::Other,
       };
       InstallationEvent {
           changed_paths: event.paths.iter()
               .map(|p| p.to_string_lossy().to_string())
               .collect(),
           kind,
       }
   }
   ```

3. **回调分发**：在监控线程中收到事件后，遍历 `callbacks` 依次调用
   ```rust
   fn dispatch_event(&self, event: InstallationEvent) {
       for entry in self.callbacks.iter() {
           (entry.value().callback)(event.clone());
       }
   }
   ```

4. **线程安全**：
   - `callbacks: Arc<DashMap<...>>` — 细粒度锁，回调注册/注销与事件分发可并发
   - `is_watching: AtomicBool` — 原子状态标志
   - `watcher: Mutex<Option<RecommendedWatcher>>` — 生命周期互斥

---

## 五、HostApi 集成

### 5.1 新增字段

在 `host_api.rs` 的 `HostApi` struct 中添加：

```rust
pub struct HostApi {
    // ... 现有字段 ...
    /// 安装监控器（平台实现）
    installation_monitor: Arc<dyn InstallationMonitor>,
}
```

### 5.2 新增方法

```rust
impl HostApi {
    // ===== 安装监控服务 =====

    /// 注册安装事件回调。
    /// 参数：id - 回调标识；callback - 回调函数。
    pub fn register_installation_callback(
        &self,
        id: &str,
        callback: InstallationCallback,
    ) -> Result<(), HostApiError> {
        self.installation_monitor.register_callback(id, callback);
        Ok(())
    }

    /// 注销安装事件回调。
    /// 参数：id - 回调标识。
    pub fn unregister_installation_callback(&self, id: &str) -> Result<(), HostApiError> {
        self.installation_monitor.unregister_callback(id);
        Ok(())
    }

    /// 启动安装监控。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn start_installation_monitor(&self) -> Result<(), HostApiError> {
        self.installation_monitor.start_watching().await
    }

    /// 停止安装监控。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn stop_installation_monitor(&self) -> Result<(), HostApiError> {
        self.installation_monitor.stop_watching().await
    }

    /// 检查安装监控是否正在运行。
    pub fn is_installation_monitor_running(&self) -> bool {
        self.installation_monitor.is_watching()
    }
}
```

### 5.3 HostApiBuilder 新增

```rust
pub struct HostApiBuilder {
    // ... 现有字段 ...
    installation_monitor: Option<Arc<dyn InstallationMonitor>>,
}

impl HostApiBuilder {
    /// 设置安装监控器。
    pub fn installation_monitor(
        mut self,
        installation_monitor: Arc<dyn InstallationMonitor>,
    ) -> Self {
        self.installation_monitor = Some(installation_monitor);
        self
    }

    // build() 中添加：
    // installation_monitor: self.installation_monitor.expect("missing installation_monitor"),
}
```

### 5.4 PluginHandle 不暴露

`PluginHandle` 不暴露安装监控回调注册方法。原因：
- `PluginHandle` 的设计是请求-响应式的（pull-based），而安装监控是事件推送式的（push-based）
- 与 `HotkeyManager` 的一致性：Hotkey 回调通过 `HostApi` 注册，而非 `PluginHandle`
- 插件如需监听安装事件，可通过 `Plugin::init()` 中的 `Arc<HostApi>` 参数直接注册

---

## 六、配置组件设计

### 6.1 文件位置

```
core/config/components/installation_monitor_config.rs
```

### 6.2 结构定义

```rust
/// 安装监控配置组件。
/// 管理安装监控的启用/禁用及 debounce 时间。
/// 配置变更时自动启动/停止 HostApi 的安装监控服务。
pub struct InstallationMonitorConfigComponent {
    /// HostApi 引用，用于控制安装监控服务
    host_api: Arc<HostApi>,
    /// 当前配置状态
    settings: RwLock<serde_json::Value>,
}
```

### 6.3 配置项

| Key                           | 类型    | 默认值 | 说明                                 |
| ----------------------------- | ------- | ------ | ------------------------------------ |
| `enable_installation_monitor` | Boolean | false  | 是否启用安装监控                     |
| `monitor_debounce_secs`       | Number  | 5      | 文件变化后的去抖等待时间（秒）       |
| `monitor_watch_paths`         | Array   | []     | 自定义监控路径（空数组使用平台默认） |

**平台默认监控路径**：
- **Windows**：公共开始菜单 + 用户开始菜单
- **macOS**（未来）：`/Applications` + `~/Applications`
- **Linux**（未来）：`/usr/share/applications` + `~/.local/share/applications`

### 6.4 Configurable 实现要点

```rust
impl Configurable for InstallationMonitorConfigComponent {
    fn component_id(&self) -> &str { "installation-monitor-config" }
    fn component_name(&self) -> &str { "安装监控配置" }
    fn component_type(&self) -> ComponentType { ComponentType::Core }
    // ... setting_schema, get_settings, apply_settings, etc.
}
```

**`on_settings_changed()` 行为**：

```rust
fn on_settings_changed(&self) {
    let settings = self.settings.read().clone();
    let enabled = settings.get("enable_installation_monitor")
        .and_then(|v| v.as_bool()).unwrap_or(false);

    let host_api = self.host_api.clone();
    tokio::spawn(async move {
        if enabled {
            // 启动监控（已启动则忽略）
            if !host_api.is_installation_monitor_running() {
                if let Err(e) = host_api.start_installation_monitor().await {
                    warn!("启动安装监控失败: {}", e);
                }
            }
        } else {
            // 停止监控
            if host_api.is_installation_monitor_running() {
                if let Err(e) = host_api.stop_installation_monitor().await {
                    warn!("停止安装监控失败: {}", e);
                }
            }
        }
    });
}
```

**初始化时注册刷新回调**（在 `lib.rs` 中的应用启动流程中）：

```rust
// 注册安装监控回调：检测到新程序时刷新数据库
host_api.register_installation_callback(
    "program-refresh",
    Arc::new(move |event| {
        info!("安装监控检测到变化: {:?} 个路径, 类型: {:?}",
            event.changed_paths.len(), event.kind);
        // 触发程序数据库刷新
        tauri::async_runtime::spawn(async {
            update_app_setting().await;
        });
    }),
);
```

### 6.5 modules 注册

在 `core/config/components/mod.rs` 中添加：

```rust
pub mod installation_monitor_config;
```

---

## 七、旧代码迁移

### 7.1 替换关系

| 旧代码                                                                                            | 新代码                                                                       |
| ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `modules/refresh_scheduler/installation_monitor.rs`                                               | `sdk/installation_monitor/` + `sdk/platform/windows/installation_monitor.rs` |
| `modules/refresh_scheduler/config.rs` 中的 `enable_installation_monitor`、`monitor_debounce_secs` | `core/config/components/installation_monitor_config.rs`                      |
| `RefreshScheduler` 中单一回调 `set_callback()`                                                    | `HostApi::register_installation_callback()` 多回调                           |

### 7.2 保留不变

- `RefreshScheduler` 的**定时刷新**和**手动触发**逻辑保持不变（或另行重构）
- `refresh_scheduler` 不再直接持有 `InstallationMonitor`

---

## 八、待确认问题

### Q1：监控路径是否可配置？

当前旧代码硬编码 Windows 开始菜单路径。设计方案中提供了 `monitor_watch_paths` 配置项（空数组使用平台默认）。

**倾向**：保留配置项，但初始版本可先使用平台默认路径（不暴露给前端 UI），后续按需开放。

### Q2：Debounce 逻辑放在哪里？

旧代码中 debounce 由 `RefreshScheduler` 的调度线程处理（在收到事件后等待 debounce 时间再触发回调）。

**倾向**：Debounce 应放在**平台实现层**（`WindowsInstallationMonitor`），因为：
1. 它是「如何监控」的实现细节，而非「何时监控」的配置逻辑
2. `notify` crate 本身就支持 debounced watcher，可直接利用
3. 与 HotkeyManager 的双击 Ctrl 检测逻辑在平台实现层一致

### Q3：是否需要事件过滤器？

`HotkeyManager` 支持按 `HotkeyEventFilter` 过滤回调。安装监控是否也需要？

**倾向**：**不需要**。文件监控事件相对简单，回调自行判断即可。如果未来有复杂过滤需求（如「只关注 .lnk 文件」「只关注创建事件」），可在回调中处理或后续添加。

---

## 九、实施步骤

| 步骤 | 内容                                                    | 涉及文件                                                |
| ---- | ------------------------------------------------------- | ------------------------------------------------------- |
| 1    | 创建 `sdk/installation_monitor/` 模块（types + trait）  | `types.rs`, `installation_monitor.rs`, `mod.rs`         |
| 2    | 实现 `WindowsInstallationMonitor`                       | `sdk/platform/windows/installation_monitor.rs`          |
| 3    | 注册到 Windows 平台 mod                                 | `sdk/platform/windows/mod.rs`                           |
| 4    | 集成到 HostApi + HostApiBuilder                         | `sdk/host_api.rs`                                       |
| 5    | 在 `sdk/mod.rs` 中导出                                  | `sdk/mod.rs`                                            |
| 6    | 创建 `InstallationMonitorConfigComponent`               | `core/config/components/installation_monitor_config.rs` |
| 7    | 注册到 components mod                                   | `core/config/components/mod.rs`                         |
| 8    | 在 `lib.rs` 中初始化并注册回调                          | `lib.rs`                                                |
| 9    | 移除旧 `installation_monitor.rs`，清理 RefreshScheduler | `modules/refresh_scheduler/`                            |
| 10   | 编译验证 + 功能测试                                     | -                                                       |

---

## 十、补充说明

### 10.1 关于「知道新安装了哪个程序」

`notify` crate 只能报告「某个目录下的某个文件发生了变化」，无法直接判断「程序 X 已安装」。在当前旧代码中也是同样的逻辑——只通知「有变化」，由上层决定是否全量刷新。

新架构下，如果需要识别具体程序，可由回调中的业务逻辑实现（例如记录变化前后的文件列表 diff）。这属于**上层业务逻辑**，不应放在 SDK 层。SDK 层只负责「通知有变化」。

### 10.2 关于跨平台默认路径

`WindowsInstallationMonitor` 内部使用 `crate::sdk::path::path_resolver::KnownPath` 获取开始菜单路径（而非硬编码），这为未来 macOS/Linux 实现提供了统一的路径获取机制。

### 10.3 关于与 HotkeyManager 的对齐

| 维度         | HotkeyManager                         | InstallationMonitor                   |
| ------------ | ------------------------------------- | ------------------------------------- |
| 事件源       | 按键事件（rdev）                      | 文件系统事件（notify）                |
| 回调注册方式 | `register_callback(id, filter, cb)`   | `register_callback(id, cb)`           |
| 是否有过滤器 | ✅ HotkeyEventFilter                   | ❌ 无（初始版本）                      |
| 事件分发     | 遍历 callbacks，按 filter 匹配        | 遍历 callbacks，全部调用              |
| 启动方式     | `start_listening()`                   | `start_watching()`                    |
| 生命周期管理 | `stop_listening()` / `is_listening()` | `stop_watching()` / `is_watching()`   |
| HostApi 方法 | `register_hotkey_callback()` 等       | `register_installation_callback()` 等 |

---

*文档版本: v1.0 | 创建日期: 2026-04-24 | 状态: 待确认*
