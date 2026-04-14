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

// HostApi: 跨平台注册层 struct，管理插件句柄
struct HostApi {
    fn register(plugin_id, config) -> Arc<PluginHandle>;
    fn update_icon_cache_dir(...);
    fn capabilities() -> &PlatformCapabilities;
}

// PluginHandle: 跨平台服务层 struct，绑定插件身份与配置
struct PluginHandle {
    async fn get_icon(...);
    async fn get_icon_and_update_cache(...);
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

## 三、组件注入架构：HostApi + PluginHandle + 平台 Trait

### 3.1 架构设计

Plugin SDK 采用**跨平台 struct + 平台 trait 注入**的架构：

- **HostApi（跨平台 struct）**：宿主持有，管理插件注册表，通过 `new_windows()` 工厂方法注入平台组件
- **PluginHandle（跨平台 struct）**：插件持有，绑定插件身份与配置，所有服务方法委托给注入的平台 trait
- **平台 Trait（IconExtractor 等）**：定义平台原语 + 跨平台业务默认实现

核心设计原则：**平台抽象只有一层，且在组件注入点**。

```
┌──────────────────────────────────────────────────────┐
│  HostApi (跨平台 struct)                              │
│  ┌──────────────────────────────────────────────────┐ │
│  │ register(plugin_id, config) → Arc<PluginHandle>  │ │
│  │ update_icon_cache_dir(...)                       │ │
│  │ capabilities()                                   │ │
│  │                                                   │ │
│  │ icon_cache: Arc<IconCacheService>  ← 共享缓存    │ │
│  │ icon_extractor: Arc<dyn IconExtractor> ← 平台注入 │ │
│  └──────────────────────────────────────────────────┘ │
│         │ register() 注入共享组件                      │
│         ▼                                             │
│  ┌──────────────────────────────────────────────────┐ │
│  │ PluginHandle (跨平台 struct)                      │ │
│  │  ┌────────────────────────────────────────────┐  │ │
│  │  │ plugin_id: "everything"                    │  │ │
│  │  │ config: { icon_cache_level: SkipAll }      │  │ │
│  │  │ icon_extractor: Arc<dyn IconExtractor>     │  │ │
│  │  │ icon_cache: Arc<IconCacheService>           │  │ │
│  │  └────────────────────────────────────────────┘  │ │
│  │ get_icon() → icon_extractor.get_icon(cache,..)   │ │
│  │ shell_open() → shell_executor.shell_open(..)      │ │
│  └──────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
        ▲ 创建时注入                      ▲ 创建时注入
        │                                 │
┌───────┴──────────┐            ┌─────────┴──────────┐
│ WindowsIcon-     │            │ MacIcon-           │
│ Extractor        │            │ Extractor          │
│ (只实现原语)      │            │ (只实现原语)        │
└──────────────────┘            └────────────────────┘
```

### 3.2 设计理由

| 设计决策                       | 理由                                                                                     |
| ------------------------------ | ---------------------------------------------------------------------------------------- |
| **先注册后使用**               | 插件必须调用 `register()` 获取句柄，强制注册流程                                         |
| **句柄绑定配置**               | 每次服务调用自动应用该插件的配置，无需重复传递                                           |
| **全局操作留在 HostApi**       | `update_icon_cache_dir` 等宿主级操作不暴露给插件                                         |
| **可更新配置**                 | `update_config()` 允许插件运行时调整 SDK 配置                                            |
| **跨平台 struct + trait 注入** | HostApi/PluginHandle 为跨平台 struct，通过 Arc<dyn Trait> 注入平台代码，平台抽象只有一层 |
| **默认实现提供业务逻辑**       | IconExtractor trait 的默认实现提供跨平台缓存策略，平台只需实现原语                       |

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

### 4.4 三组件协作模型

图标服务由三个组件协作完成，职责正交：

| 组件                   | 职责                                          | 依赖                         |
| ---------------------- | --------------------------------------------- | ---------------------------- |
| `IconExtractor` trait  | 平台原语 + 跨平台业务逻辑（缓存策略、后处理） | IconCacheService, ImageUtils |
| `IconCacheService`     | 纯缓存工具（L1/L2 原语）                      | 无业务依赖                   |
| `WindowsIconExtractor` | Windows API 图标提取（只实现 6 个原语）       | Win32 API                    |

```rust
// IconExtractor trait — 平台原语 + 默认业务实现
#[async_trait]
pub trait IconExtractor: Send + Sync {
    // 平台原语（必须实现）
    async fn extract_from_path(&self, path: &str) -> Result<Vec<u8>, HostApiError>;
    async fn extract_from_url(&self, url: &str) -> Result<Vec<u8>, HostApiError>;
    async fn extract_from_extension(&self, ext: &str) -> Result<Vec<u8>, HostApiError>;
    fn default_app_icon_path(&self) -> &str;
    fn default_web_icon_path(&self) -> &str;
    fn is_network_available(&self) -> bool;

    // 跨平台业务逻辑（默认实现，可覆盖）
    async fn extract(&self, request: &IconRequest) -> Result<Vec<u8>, HostApiError>;
    async fn extract_and_process(&self, request: &IconRequest) -> Result<Vec<u8>, HostApiError>;
    async fn load_default_icon(&self, request: &IconRequest) -> Vec<u8>;
    async fn get_icon(&self, cache: &IconCacheService, request: &IconRequest, level: CacheLevel) -> Result<Vec<u8>, HostApiError>;
    async fn get_icon_and_update_cache(&self, cache: &IconCacheService, request: &IconRequest, level: CacheLevel) -> Result<Vec<u8>, HostApiError>;
}
```

```rust
// IconCacheService — 纯缓存工具，不知道提取
pub struct IconCacheService {
    memory_cache: DashMap<String, Vec<u8>>,
    cache_dir: RwLock<String>,
    cached_file_hashes: DashSet<String>,
}
```

核心方法：
| 方法                                                     | 说明                                    |
| -------------------------------------------------------- | --------------------------------------- |
| `get_l1(key)` / `set_l1(key, data)`                      | L1 内存缓存读写                         |
| `contains_l2(key)` / `get_l2(key)` / `set_l2(key, data)` | L2 文件缓存操作                         |
| `update_cache_dir(new_dir)`                              | 切换 L2 缓存目录，清空 L1，重新扫描 L2  |
| `init()`                                                 | 扫描 L2 缓存目录填充 cached_file_hashes |

图标提取由 `sdk/platform/windows/icon.rs` 中的 `WindowsIconExtractor` 直接实现，不再依赖 `core::image_processor`。

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
├── host_api.rs                # HostApi struct + PluginHandle struct + IconRequest + CacheLevel + PluginSdkConfig + 错误类型
├── common/
│   ├── mod.rs                 # 通用模块入口
│   └── image_utils.rs         # ImageUtils — 跨平台图片处理工具函数
├── icon/
│   ├── mod.rs                 # 图标模块入口
│   ├── icon_cache.rs          # IconCacheService — 纯缓存工具（L1/L2 原语）
│   └── icon_extractor.rs      # IconExtractor trait — 平台原语 + 跨平台默认实现
└── platform/
    ├── mod.rs                 # 条件编译选择平台实现
    ├── capabilities.rs        # PlatformCapabilities 定义
    └── windows/
        ├── mod.rs             # Windows 平台入口
        └── icon.rs            # WindowsIconExtractor — Windows API 图标提取实现
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

// 整合后：增加 HostApi 参数（HostApi 为 struct，不再需要 dyn）
async fn init(&self, ctx: &PluginContext, api: Arc<dyn PluginAPI>, host_api: Arc<HostApi>) -> Result<(), PluginError>;
```

插件在 `init()` 中注册并持有 PluginHandle：

```rust
async fn init(&self, ctx: &PluginContext, api: Arc<dyn PluginAPI>, host_api: Arc<HostApi>) -> Result<(), PluginError> {
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
    host_api: RwLock<Option<Arc<HostApi>>>,  // 新增（struct，非 trait）
}
```

### 8.3 初始化流程

在 lib.rs 的 init_plugin_system 中，创建 HostApi 实例并存入 AppState：

```rust
let host_api = Arc::new(HostApi::new_windows(icon_cache_dir, icon_extractor));
state.set_host_api(host_api);
```

---

## 九、依赖方向

```
plugin/          →  sdk/ (通过 PluginHandle struct)
plugin_system/   →  sdk/ (通过类型引用，如 ExecutionContext)
sdk/             →  sdk/platform/ (内部委托)
sdk/platform/    →  Windows API / macOS API / Linux API
sdk/common/      →  image/palette/kmeans_colors (跨平台图片处理)
```

整体依赖方向：
```
commands → plugin_system → plugin → sdk → platform
                                              ↓
                                       sdk/common (跨平台)
```

**注意**：`core/image_processor.rs` 中的图标提取代码已搬迁到 `sdk/platform/windows/icon.rs`，跨平台图片处理函数已搬迁到 `sdk/common/image_utils.rs`。SDK 不再依赖 `core/` 目录。

---

## 十、迁移路线图

```
阶段一：框架搭建 ✅
├── 定义 HostApi trait、PluginHandle trait
├── 定义 CacheLevel、PluginSdkConfig
├── 定义 PlatformCapabilities
├── 创建 WindowsHostApi + WindowsPluginHandle 骨架（todo!() 占位）
└── 验证编译通过

阶段二：图标服务迁移 ✅ (初版)
├── 新建 sdk/icon/icon_cache.rs（IconCacheService — L1 DashMap + L2 文件缓存）
├── 新建 sdk/platform/windows/icon.rs（WindowsIconExtractor — 委托 ImageProcessor）
├── 为 sdk::host_api::IconRequest 添加 blake3 hash 方法
├── PluginHandle trait 新增 get_icon_and_update_cache() 方法
├── WindowsPluginHandle 实现 get_icon() / get_icon_and_update_cache() — 根据 CacheLevel 委托 IconCacheService
├── WindowsHostApi 实现 update_icon_cache_dir() — 委托 IconCacheService
└── 验证编译通过

阶段二重构：架构升级 ✅
├── HostApi / PluginHandle 从 trait 重构为跨平台 struct
├── 新建 sdk/icon/icon_extractor.rs（IconExtractor trait — 平台原语 + 跨平台默认实现）
├── IconCacheService 精简为纯缓存工具（移除业务逻辑）
├── 缓存策略从 IconCacheService 迁移到 IconExtractor 默认实现
├── 新建 sdk/common/image_utils.rs（跨平台图片处理函数）
├── WindowsIconExtractor 搬迁 ImageProcessor 代码（不再依赖 core/）
├── 删除 sdk/platform/windows/host_api_impl.rs（逻辑合并到 host_api.rs）
├── 删除 ImageIdentity 枚举（被 IconRequest 替代）
└── 验证编译通过

阶段三：Shell 服务迁移
├── 定义 ShellExecutor trait（平台原语）
├── 将 shell_execute_open 等函数迁移到 sdk/platform/windows/shell.rs
├── PluginHandle::shell_open() 委托给 shell_executor
├── 修改 PathExecutor / UrlExecutor 使用 HostApi
└── 去除插件对 core::platform 的直接依赖

阶段四：窗口服务迁移
├── 定义 WindowManager trait（平台原语）
├── 将 core::platform::window 迁移到 sdk/platform/windows/window.rs
├── PluginHandle 窗口方法委托给 window_manager
├── 修改 WindowActivateExecutor 使用 HostApi
└── 删除 core::platform 目录

阶段五：Plugin::init() 整合
├── 修改 Plugin trait 签名增加 HostApi 参数
├── 插件在 init() 中注册获取 PluginHandle
├── 所有 Executor 通过 PluginHandle 调用平台能力
└── 彻底消除插件对 platform 的直接依赖
```
