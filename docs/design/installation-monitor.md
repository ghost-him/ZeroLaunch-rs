# InstallationMonitor SDK 设计文档

## 一、背景与目标

### 关键差异：Push-based 服务

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

## 二、调用逻辑

### 新架构调用链

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

### 核心设计原则

| 层级            | 职责                                                  |
| --------------- | ----------------------------------------------------- |
| **Config 组件** | 决定「是否启用」「监控什么」「debounce 时间」         |
| **HostApi**     | 统一入口，管理回调注册/注销，委托给平台 trait         |
| **SDK trait**   | 定义平台无关的能力契约                                |
| **平台实现**    | 如何调用 OS API（`notify` crate），如何分发事件给回调 |

---

## 三、SDK 层设计

### 目录结构

```
sdk/installation_monitor/
├── mod.rs                     # 模块入口
├── types.rs                   # InstallationEvent, InstallationCallback 等类型
└── installation_monitor.rs    # InstallationMonitor trait
```

### 类型定义

**InstallationEvent**：安装监控事件，表示监控目录中发生了文件系统变化。

**InstallationEventKind**：文件系统变化类型（Created、Modified、Removed、Other）。

**InstallationCallback**：安装监控回调函数类型，当监控目录发生变化时，所有已注册的回调将被依次调用。

**设计说明**：
- `InstallationEvent` 携带 `changed_paths` 而非单个路径，因为 `notify` 可能在一个事件中报告多个路径变化
- `InstallationEventKind` 简化自 `notify::EventKind`，提供跨平台统一视图
- 回调不设事件过滤器（与 `HotkeyManager` 不同），因为文件变化事件语义简单，调用方可自行判断

### Trait 定义

**InstallationMonitor trait**：安装监控器 trait，定义平台无关的文件系统监控能力契约。各平台实现此 trait，处理平台特定的文件监控逻辑。支持多个回调注册，当监控目录发生变化时依次调用所有回调。

**设计说明**：
- 不提供事件过滤器：文件监控事件语义简单，回调自行判断是否关注
- `start_watching` / `stop_watching` 管理监听生命周期
- `register_callback` / `unregister_callback` 管理回调注册（同步方法，与 HotkeyManager 一致）

---

## 四、平台实现层设计

### 文件位置

```
sdk/platform/windows/installation_monitor.rs    # WindowsInstallationMonitor
```

### Windows 平台实现要点

1. **监控路径**：使用 `PathResolver` 获取已知路径（开始菜单），避免硬编码
   - 公共开始菜单：`C:\ProgramData\Microsoft\Windows\Start Menu`
   - 用户开始菜单：`%APPDATA%\Microsoft\Windows\Start Menu`

2. **事件转换**：将 `notify::Event` 转换为 `InstallationEvent`

3. **回调分发**：在监控线程中收到事件后，遍历 `callbacks` 依次调用

4. **线程安全**：
   - `callbacks: Arc<DashMap<...>>` — 细粒度锁，回调注册/注销与事件分发可并发
   - `is_watching: AtomicBool` — 原子状态标志
   - `watcher: Mutex<Option<RecommendedWatcher>>` — 生命周期互斥

---

## 五、HostApi 集成

### 新增字段

在 `host_api.rs` 的 `HostApi` struct 中添加：

```
installation_monitor: Arc<dyn InstallationMonitor>
```

### 新增方法

| 方法                                      | 说明                     |
| ----------------------------------------- | ------------------------ |
| `register_installation_callback(id, cb)`  | 注册安装事件回调         |
| `unregister_installation_callback(id)`    | 注销安装事件回调         |
| `start_installation_monitor()`            | 启动安装监控             |
| `stop_installation_monitor()`             | 停止安装监控             |
| `is_installation_monitor_running()`       | 检查安装监控是否正在运行 |

### PluginHandle 不暴露

`PluginHandle` 不暴露安装监控回调注册方法。原因：
- `PluginHandle` 的设计是请求-响应式的（pull-based），而安装监控是事件推送式的（push-based）
- 与 `HotkeyManager` 的一致性：Hotkey 回调通过 `HostApi` 注册，而非 `PluginHandle`
- 插件如需监听安装事件，可通过 `Plugin::init()` 中的 `Arc<HostApi>` 参数直接注册

---

## 六、配置组件设计

### 文件位置

```
core/config/components/installation_monitor_config.rs
```

### 配置项

| Key                           | 类型    | 默认值 | 说明                                 |
| ----------------------------- | ------- | ------ | ------------------------------------ |
| `enable_installation_monitor` | Boolean | false  | 是否启用安装监控                     |
| `monitor_debounce_secs`       | Number  | 5      | 文件变化后的去抖等待时间（秒）       |
| `monitor_watch_paths`         | Array   | []     | 自定义监控路径（空数组使用平台默认） |

**平台默认监控路径**：
- **Windows**：公共开始菜单 + 用户开始菜单
- **macOS**（未来）：`/Applications` + `~/Applications`
- **Linux**（未来）：`/usr/share/applications` + `~/.local/share/applications`

### Configurable 实现要点

**`on_settings_changed()` 行为**：配置变更时自动启动/停止 HostApi 的安装监控服务。

**初始化时注册刷新回调**：在 `lib.rs` 中的应用启动流程中注册安装监控回调，检测到新程序时刷新数据库。

---

## 七、设计要点

### 监控路径是否可配置？

当前旧代码硬编码 Windows 开始菜单路径。设计方案中提供了 `monitor_watch_paths` 配置项（空数组使用平台默认）。

**倾向**：保留配置项，但初始版本可先使用平台默认路径（不暴露给前端 UI），后续按需开放。

### Debounce 逻辑放在哪里？

旧代码中 debounce 由 `RefreshScheduler` 的调度线程处理（在收到事件后等待 debounce 时间再触发回调）。

**倾向**：Debounce 应放在**平台实现层**（`WindowsInstallationMonitor`），因为：
1. 它是「如何监控」的实现细节，而非「何时监控」的配置逻辑
2. `notify` crate 本身就支持 debounced watcher，可直接利用
3. 与 HotkeyManager 的双击 Ctrl 检测逻辑在平台实现层一致
