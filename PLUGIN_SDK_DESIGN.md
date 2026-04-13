# Plugin SDK 设计文档

## 一、定位与目标

Plugin SDK 是**核心程序向插件提供统一服务**的接口层。它解决的问题是：插件需要调用平台相关能力（图标提取、shell 操作、窗口管理），但不应该直接依赖平台实现。

| 维度         | 说明                                               |
| ------------ | -------------------------------------------------- |
| **核心职责** | 定义宿主向插件暴露的平台能力契约                   |
| **设计原则** | 插件只关注「做什么」，SDK 负责平台差异的「怎么做」 |
| **消费者**   | plugin/ 目录下的所有插件实现                       |
| **提供者**   | sdk/platform/ 目录下的各平台实现                   |

---

## 二、HostApi vs PluginAPI

两者平行共存，职责不同：

| 维度           | PluginAPI                     | HostApi                                 |
| -------------- | ----------------------------- | --------------------------------------- |
| **定位**       | 平台无关的通用能力            | 平台相关的服务能力                      |
| **内容**       | 日志、通知、配置读写、UI 回调 | 图标提取、shell 操作、窗口管理          |
| **平台相关性** | 所有平台行为一致              | 能力不对等，需查询 PlatformCapabilities |
| **稳定性**     | 稳定，很少变                  | 随平台演进，可能新增能力                |
| **定义位置**   | plugin_system/types.rs        | sdk/host_api.rs                         |

```rust
// PluginAPI: 通用能力
trait PluginAPI {
    async fn log(...);
    async fn notify(...);
    async fn get_setting(...);
    async fn set_setting(...);
    async fn refresh_programs();
    async fn hide_window();
}

// HostApi: 注册层，管理插件句柄
trait HostApi {
    fn register(plugin_id, config) -> Arc<PluginHandle>;
    async fn update_icon_cache_dir(...);
    fn capabilities() -> &PlatformCapabilities;
}

// PluginHandle: 服务层，绑定插件身份与配置
trait PluginHandle {
    async fn get_icon(...);
    async fn shell_open(...);
    async fn shell_open_folder(...);
    async fn get_default_browser(...);
    async fn activate_window_by_process(...);
    async fn activate_window_by_title(...);
    fn update_config(config);
    fn capabilities() -> &PlatformCapabilities;
}
```

---

## 三、双层架构：HostApi + PluginHandle

### 3.1 架构设计

Plugin SDK 采用**注册层 + 服务层**的双层架构：

- **HostApi（注册层）**：宿主持有，管理插件注册表，提供全局管理操作
- **PluginHandle（服务层）**：插件持有，绑定插件身份与配置，提供所有服务方法

```
┌──────────────────────────────────────────────────┐
│  HostApi (注册层)                                 │
│  ┌─────────────────────────────────────────────┐ │
│  │ register(plugin_id, config) → PluginHandle  │ │
│  │ update_icon_cache_dir(...)                  │ │
│  │ capabilities()                              │ │
│  └─────────────────────────────────────────────┘ │
│         │                                        │
│         │ register() 返回 Arc<PluginHandle>       │
│         ▼                                        │
│  ┌─────────────────────────────────────────────┐ │
│  │ PluginHandle (服务层)                        │ │
│  │  ┌───────────────────────────────────────┐  │ │
│  │  │ plugin_id: "everything"               │  │ │
│  │  │ config: { icon_cache_level: SkipAll } │  │ │
│  │  └───────────────────────────────────────┘  │ │
│  │ get_icon(request) → 按 SkipAll 行为执行   │ │
│  │ shell_open(target)                          │ │
│  │ shell_open_folder(path)                     │ │
│  │ get_default_browser()                       │ │
│  │ activate_window_by_process(name)            │ │
│  │ activate_window_by_title(title)             │ │
│  │ update_config(config)                       │ │
│  │ capabilities()                              │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

### 3.2 设计理由

| 设计决策                 | 理由                                             |
| ------------------------ | ------------------------------------------------ |
| **先注册后使用**         | 插件必须调用 `register()` 获取句柄，强制注册流程 |
| **句柄绑定配置**         | 每次服务调用自动应用该插件的配置，无需重复传递   |
| **全局操作留在 HostApi** | `update_icon_cache_dir` 等宿主级操作不暴露给插件 |
| **可更新配置**           | `update_config()` 允许插件运行时调整 SDK 配置    |

### 3.3 调用流程

```rust
// 1. 插件初始化时注册
let handle = host_api.register("everything", PluginSdkConfig {
    icon_cache_level: Some(CacheLevel::SkipAll),
});

// 2. 查询时通过句柄调用，自动应用配置
let icon = handle.get_icon(IconRequest::Path("foo.exe")).await;
// 内部查配置 → SkipAll → 直接提取，不更新缓存

// 3. 普通插件使用默认配置
let handle = host_api.register("program-search", PluginSdkConfig::default());
let icon = handle.get_icon(IconRequest::Path("chrome.exe")).await;
// 内部查配置 → None → 默认 Full → 双层缓存

// 4. 运行时更新配置
handle.update_config(PluginSdkConfig {
    icon_cache_level: Some(CacheLevel::SkipMemory),
});
```

---

## 四、PluginSdkConfig 与 CacheLevel

### 4.1 缓存等级

```rust
/// 缓存等级枚举，控制图标服务的缓存策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheLevel {
    /// 双层缓存（L1 → L2 → 提取 → 更新 L1+L2）— 适用于图标被频繁提取的场景
    #[default]
    Full,
    /// 跳过内存缓存（L2 → 提取 → 更新 L2）— 适用于图标只在每次启动时提取的场景
    SkipMemory,
    /// 跳过所有缓存（直接提取）— 适用于图标在几天的时间内可能只被提取一次的场景
    SkipAll,
}
```

```
┌──────────────┬──────────────────┬──────────────────────────────────┐
│ CacheLevel   │ 缓存行为         │ 适用场景                          │
├──────────────┼──────────────────┼──────────────────────────────────┤
│ Full (默认)  │ L1 → L2 →        │ 适用于图标被频繁提取的场景          │
│              │ 提取→更新L1+L2   │ (程序列表、书签)                  │
├──────────────┼──────────────────┼──────────────────────────────────┤
│ SkipMemory   │ 跳过 L1          │ 适用于图标只在每次启动时提取的场景    │
│              │ L2 → 提取→       │                                  │
│              │ 更新 L2        │                                  │
├──────────────┼──────────────────┼──────────────────────────────────┤
│ SkipAll      │ 跳过全部缓存     │ 适用于图标在几天的时间内可能只被提取一次的场景│
│              │ 提取→不更新任何缓存  │                                │
└──────────────┴──────────────────┴──────────────────────────────────┘
```

### 4.2 插件 SDK 配置

```rust
/// 插件 SDK 配置，各字段可选，不需要配置的服务无需设置
#[derive(Debug, Clone, Default)]
pub struct PluginSdkConfig {
    /// 图标缓存等级。None 时使用默认值 CacheLevel::Full
    pub icon_cache_level: Option<CacheLevel>,
    // 未来扩展：
    // pub shell_config: Option<ShellConfig>,
}
```

### 4.3 缓存流程示例（CacheLevel::Full）

```
get_icon(IconRequest::Path("chrome.exe"))
    │
    ▼
┌─────────────────────────────────────────────┐
│  L1: 内存缓存 (DashMap<hash, Vec<u8>>)      │
│  命中 → 直接返回，零 IO                      │
└──────────────────┬──────────────────────────┘
                   │ 未命中
                   ▼
┌─────────────────────────────────────────────┐
│  L2: 文件缓存 (icon_cache_dir/hash.png)     │
│  命中 → 读取文件，写入 L1，返回              │
└──────────────────┬──────────────────────────┘
                   │ 未命中
                   ▼
┌─────────────────────────────────────────────┐
│  提取图标 (平台特定实现)                      │
│  成功 → 写入 L1 + L2，返回                   │
│  失败 → 返回默认图标                          │
└─────────────────────────────────────────────┘
```

| 设计要点        | 说明                                                     |
| --------------- | -------------------------------------------------------- |
| **框架级缓存**  | 由 SDK 统一管理，避免多个插件重复缓存同一图标            |
| **L1 内存缓存** | 启动后热数据驻留内存，零 IO 响应                         |
| **L2 文件缓存** | 跨重启持久化，首次启动后无需重新提取                     |
| **缓存键**      | IconRequest 的 blake3 哈希值，保证唯一性和一致性         |
| **插件无感知**  | 插件只需在注册时指定 CacheLevel，服务调用无需传参        |
| **权限隔离**    | 插件通过配置表达意图，无权修改全局缓存配置、无法清空缓存 |
| **配置可更新**  | 运行时通过 PluginHandle::update_config() 调整            |

---

## 五、HostApiError

```rust
pub enum HostApiError {
    /// 平台不支持该能力
    UnsupportedCapability(PlatformCapability),
    /// 插件未注册
    PluginNotRegistered(String),
    /// 图标提取失败
    IconExtractionFailed { request: String, reason: String },
    /// Shell 操作失败
    ShellOperationFailed { target: String, reason: String },
    /// 窗口操作失败
    WindowOperationFailed { detail: String },
    /// 通用执行失败
    ExecutionFailed { service: String, reason: String },
}
```

---

## 六、PlatformCapabilities

不同平台能力不对等是必然的：

| 能力                        | Windows       | macOS       | Linux        |
| --------------------------- | ------------- | ----------- | ------------ |
| 图标提取 (IconExtraction)   | 完整支持      | 部分支持    | 部分支持     |
| Shell 打开 (ShellOpen)      | ShellExecuteW | NSWorkspace | xdg-open     |
| 以管理员运行 (RunAsAdmin)   | runas         | osascript   | pkexec       |
| UWP 启动 (UwpLaunch)        | 专属 API      | 不存在      | 不存在       |
| 窗口激活 (WindowActivation) | Win32 API     | NSWorkspace | wmctrl       |
| 默认浏览器 (DefaultBrowser) | 注册表        | LSWorkspace | xdg-settings |

插件通过 `handle.capabilities()` 查询平台支持的能力，UI 层根据能力动态隐藏/禁用不可用功能。

```rust
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum PlatformCapability {
    IconExtraction,
    ShellOpen,
    RunAsAdmin,
    UwpLaunch,
    WindowActivation,
    DefaultBrowser,
}

pub struct PlatformCapabilities {
    capabilities: HashSet<PlatformCapability>,
}

impl PlatformCapabilities {
    pub fn new(capabilities: HashSet<PlatformCapability>) -> Self { ... }
    pub fn has(&self, capability: PlatformCapability) -> bool { ... }

    // 各平台构造函数，不使用 Default trait 避免平台语义混淆
    #[cfg(target_os = "windows")]
    pub fn windows() -> Self { ... }  // 包含所有能力
}
```

---

## 七、目录结构

```
src-tauri/src/sdk/
├── mod.rs                     # 模块入口，导出公共 API
├── host_api.rs                # HostApi + PluginHandle trait + CacheLevel + PluginSdkConfig + 错误类型
└── platform/
    ├── mod.rs                 # 条件编译选择平台实现
    ├── capabilities.rs        # PlatformCapabilities 定义
    └── windows/
        ├── mod.rs             # Windows 平台入口
        └── host_api_impl.rs   # WindowsHostApi + WindowsPluginHandle 实现
```

platform 放在 sdk/ 下的理由：
1. platform 的唯一消费者是 sdk（HostApi 的实现层）
2. 其他模块不应直接调用 platform 代码——这正是 SDK 存在的意义
3. 封装性：sdk 是公共接口，platform 是私有实现
4. 放在离使用者近的地方，符合"最小可见性"原则

---

## 八、与现有架构的整合

### 8.1 Plugin::init() 整合

当前 Plugin::init() 接收 `Arc<dyn PluginAPI>`。整合后：

```rust
// 当前
async fn init(&self, ctx: &PluginContext, api: Arc<dyn PluginAPI>) -> Result<(), PluginError>;

// 整合后：增加 HostApi 参数
async fn init(&self, ctx: &PluginContext, api: Arc<dyn PluginAPI>, host_api: Arc<dyn HostApi>) -> Result<(), PluginError>;
```

插件在 `init()` 中注册并持有 PluginHandle：

```rust
async fn init(&self, ctx: &PluginContext, api: Arc<dyn PluginAPI>, host_api: Arc<dyn HostApi>) -> Result<(), PluginError> {
    let handle = host_api.register("everything", PluginSdkConfig {
        icon_cache_level: Some(CacheLevel::SkipAll),
    });
    // 存储 handle 供后续 query() 使用
    Ok(())
}
```

### 8.2 AppState 整合

HostApi 实例将在 AppState 中初始化，供整个应用共享：

```rust
pub struct AppState {
    // ... 现有字段
    host_api: RwLock<Option<Arc<dyn HostApi>>>,  // 新增
}
```

### 8.3 初始化流程

在 lib.rs 的 init_plugin_system 中，创建 HostApi 实例并存入 AppState：

```rust
let host_api: Arc<dyn HostApi> = Arc::new(WindowsHostApi::new());
state.set_host_api(host_api);
```

---

## 九、依赖方向

```
plugin/          →  sdk/ (通过 PluginHandle trait)
plugin_system/   →  sdk/ (通过类型引用，如 ExecutionContext)
sdk/             →  sdk/platform/ (内部委托)
sdk/platform/    →  Windows API / macOS API / Linux API
```

整体依赖方向：
```
commands → plugin_system → plugin → sdk → platform
                                     ↑
                              core/ ─┘ (core 暂不依赖 sdk，迁移后逐渐改变)
```

---

## 十、迁移路线图

```
阶段一：框架搭建（当前）
├── 定义 HostApi trait、PluginHandle trait
├── 定义 CacheLevel、PluginSdkConfig
├── 定义 PlatformCapabilities
├── 创建 WindowsHostApi + WindowsPluginHandle 骨架（todo!() 占位）
└── 验证编译通过

阶段二：图标服务迁移
├── 将 IconManager 迁移到 sdk/icon/
├── 将 ImageProcessor 中平台相关代码拆到 sdk/platform/windows/icon.rs
├── WindowsPluginHandle::get_icon() 根据缓存等级调用对应逻辑
└── SearchCandidate.icon 从 String 改为 IconRequest

阶段三：Shell 服务迁移
├── 将 shell_execute_open 等函数迁移到 sdk/platform/windows/shell.rs
├── WindowsPluginHandle::shell_open() 委托给 platform 层
├── 修改 PathExecutor / UrlExecutor 使用 HostApi
└── 去除插件对 core::platform 的直接依赖

阶段四：窗口服务迁移
├── 将 core::platform::window 迁移到 sdk/platform/windows/window.rs
├── WindowsPluginHandle 窗口方法委托给 platform 层
├── 修改 WindowActivateExecutor 使用 HostApi
└── 删除 core::platform 目录

阶段五：Plugin::init() 整合
├── 修改 Plugin trait 签名增加 HostApi 参数
├── 插件在 init() 中注册获取 PluginHandle
├── 所有 Executor 通过 PluginHandle 调用平台能力
└── 彻底消除插件对 platform 的直接依赖
```
