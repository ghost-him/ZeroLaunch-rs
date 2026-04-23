# Plugin SDK 设计文档

## 一、定位与目标

Plugin SDK 是**核心程序向插件提供统一服务**的接口层。它解决的问题是：插件需要调用平台相关能力（图标提取、shell 操作、窗口管理、应用枚举与启动），但不应该直接依赖平台实现。

| 维度         | 说明                                               |
| ------------ | -------------------------------------------------- |
| **核心职责** | 定义宿主向插件暴露的平台能力契约                   |
| **设计原则** | 插件只关注「做什么」，SDK 负责平台差异的「怎么做」 |
| **消费者**   | plugin/ 目录下的所有插件实现                       |
| **提供者**   | sdk/platform/ 目录下的各平台实现                   |

---

## 二、HostApi vs PluginAPI

两者平行共存，职责不同：

| 维度           | PluginAPI                     | HostApi                                        |
| -------------- | ----------------------------- | ---------------------------------------------- |
| **定位**       | 平台无关的通用能力            | 平台相关的服务能力                             |
| **内容**       | 日志、通知、配置读写、UI 回调 | 图标提取、shell 操作、窗口管理、应用枚举与启动 |
| **平台相关性** | 所有平台行为一致              | 能力不对等，需查询 PlatformCapabilities        |
| **稳定性**     | 稳定，很少变                  | 随平台演进，可能新增能力                       |
| **定义位置**   | plugin_system/types.rs        | sdk/host_api.rs                                |

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
    async fn shell_execute_elevation(...);
    async fn shell_execute_command(...);
    async fn activate_window_by_process(...);
    async fn activate_window_by_title(...);
    async fn enumerate_apps(...);
    async fn launch_app(...);
    fn resolve_lnk_target(...) -> Option<String>;
    fn parse_localized_names_from_dir(...) -> HashMap<String, String>;
    fn resolve_path(...) -> Result<String, HostApiError>;
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
┌──────────────────────────────────────────────────────────────────┐
│  HostApi (跨平台 struct)                                          │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ register(plugin_id, config) → Arc<PluginHandle>            │  │
│  │ update_icon_cache_dir(...)                                 │  │
│  │ capabilities()                                             │  │
│  │                                                             │  │
│  │ icon_cache: Arc<IconCacheService>     ← 共享缓存            │  │
│  │ icon_extractor: Arc<dyn IconExtractor> ← 平台注入           │  │
│  │ shell_executor: Arc<dyn ShellExecutor> ← 平台注入           │  │
│  │ window_manager: Arc<dyn WindowManager> ← 平台注入           │  │
│  │ app_enumerator: Arc<dyn AppEnumerator> ← 平台注入           │  │
│  │ app_launcher: Arc<dyn AppLauncher>     ← 平台注入           │  │
│  │ lnk_resolver: Arc<dyn LnkResolver>     ← 平台注入           │  │
│  │ resource_loader: Arc<dyn ResourceLoader> ← 平台注入         │  │
│  └────────────────────────────────────────────────────────────┘  │
│         │ register() 注入共享组件                                 │
│         ▼                                                         │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ PluginHandle (跨平台 struct)                                │  │
│  │  ┌──────────────────────────────────────────────────────┐  │  │
│  │  │ plugin_id: "everything"                              │  │  │
│  │  │ config: { icon_cache_level: SkipAll }                │  │  │
│  │  │ icon_extractor: Arc<dyn IconExtractor>               │  │  │
│  │  │ icon_cache: Arc<IconCacheService>                     │  │  │
│  │  │ shell_executor: Arc<dyn ShellExecutor>               │  │  │
│  │  │ window_manager: Arc<dyn WindowManager>               │  │  │
│  │  │ app_enumerator: Arc<dyn AppEnumerator>               │  │  │
│  │  │ app_launcher: Arc<dyn AppLauncher>                   │  │  │
│  │  │ lnk_resolver: Arc<dyn LnkResolver>                   │  │  │
│  │  │ resource_loader: Arc<dyn ResourceLoader>             │  │  │
│  │  └──────────────────────────────────────────────────────┘  │  │
│  │ get_icon() → icon_extractor.get_icon(cache,..)             │  │
│  │ shell_open() → shell_executor.shell_open(..)               │  │
│  │ activate_window() → window_manager.activate(..)            │  │
│  │ enumerate_apps() → app_enumerator.enumerate_apps()         │  │
│  │ launch_app() → app_launcher.launch_app(..)                 │  │
│  │ resolve_lnk_target() → lnk_resolver.resolve_lnk_target(..) │  │
│  │ parse_localized_names_from_dir() → resource_loader        │  │
│  │ resolve_path() → path_resolver.resolve_path(..)           │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
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
    /// 应用枚举失败
    AppEnumerationFailed { reason: String },
    /// 应用启动失败
    AppLaunchFailed { app_id: String, reason: String },
    /// 通用执行失败
    ExecutionFailed { service: String, reason: String },
    /// Lnk 快捷方式解析失败
    LnkResolutionFailed { path: String, reason: String },
}
```

---

## 六、PlatformCapabilities

不同平台能力不对等是必然的。

每个平台都有"传统文件系统搜索找不到"的专属应用：

| 能力                        | Windows                       | macOS              | Linux                     |
| --------------------------- | ----------------------------- | ------------------ | ------------------------- |
| 图标提取 (IconExtraction)   | 完整支持                      | 部分支持           | 部分支持                  |
| Shell 打开 (ShellOpen)      | ShellExecuteW                 | NSWorkspace        | xdg-open                  |
| 以管理员运行 (RunAsAdmin)   | runas                         | osascript          | pkexec                    |
| 应用枚举 (AppEnumeration)   | shell:AppsFolder              | Launch Services DB | .desktop + Flatpak + Snap |
| 应用启动 (AppLaunch)        | IApplicationActivationManager | LSOpenURLsWithRole | flatpak run / snap run    |
| 窗口激活 (WindowActivation) | Win32 API                     | NSWorkspace        | wmctrl                    |

### 6.1 各平台专属应用生态

| 维度             | Windows                         | macOS                         | Linux                      |
| ---------------- | ------------------------------- | ----------------------------- | -------------------------- |
| **传统应用**     | `.exe`/`.lnk` 文件系统扫描      | `.app` 包扫描 `/Applications` | `.desktop` 文件扫描        |
| **系统级注册库** | `shell:AppsFolder` (UWP)        | **Launch Services 数据库**    | **Flatpak/Snap 注册库**    |
| **专属 API**     | `IApplicationActivationManager` | `LSFindApplicationForInfo`    | `libflatpak` / `snapd API` |
| **启动方式**     | `ActivateApplication()`         | `LSOpenURLsWithRole()`        | `flatpak run` / `snap run` |
| **沙箱特性**     | AppContainer                    | App Sandbox                   | Flatpak Sandbox            |

### 6.2 PlatformCapability 定义

插件通过 `handle.capabilities()` 查询平台支持的能力，UI 层根据能力动态隐藏/禁用不可用功能。

```rust
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum PlatformCapability {
    /// 图标提取：从文件/网址/扩展名中提取图标
    IconExtraction,
    /// Shell 打开：使用系统默认方式打开文件/网址/文件夹
    ShellOpen,
    /// 以管理员身份运行
    RunAsAdmin,
    /// 应用枚举：发现系统中已安装的应用（含沙箱/容器应用）
    AppEnumeration,
    /// 应用启动：通过平台专属 API 启动应用
    AppLaunch,
    /// 窗口激活：根据进程名或标题激活已存在的窗口
    WindowActivation,
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

## 七、应用枚举与启动：AppEnumerator + AppLauncher

### 7.1 设计动机

每个平台都存在"传统文件系统搜索找不到"的应用，需要通过平台专属 API 发现和启动：

- **Windows**: UWP 应用安装在 `C:\Program Files\WindowsApps\`（无权限访问），只能通过 `shell:AppsFolder` 枚举
- **macOS**: Launch Services 数据库记录所有已注册应用，包括 Mac App Store 安装的应用
- **Linux**: Flatpak 安装在 `~/.local/share/flatpak/`，Snap 安装在 `/snap/`，需通过专属 API 查询

统一抽象为 `AppEnumerator`（发现）和 `AppLauncher`（启动）两个 trait，实现跨平台一致性。

### 7.2 统一数据结构

```rust
/// 应用信息，跨平台统一结构
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// 应用唯一标识
    /// - Windows: AppUserModelID (UWP) 或 exe 路径
    /// - macOS: Bundle ID
    /// - Linux: .desktop 文件名或 Flatpak app-id
    pub app_id: String,

    /// 显示名称
    pub display_name: String,

    /// 图标路径或图标标识符
    pub icon: String,

    /// 安装路径（某些平台可能为空，如 UWP 沙箱应用）
    pub install_path: Option<String>,
}
```

**设计说明**：
- 不需要 `AppType` / `SandboxType` 枚举，所有应用统一用 `AppInfo` 表示
- 不包含 `launch_command` 字段：启动方式是 `AppLauncher` 的实现细节，不应编码进数据结构。`AppLauncher` 根据 `app_id` 的格式自行决定启动方式（如 Windows UWP 用 COM API，Linux Flatpak 内部构造 `flatpak run`）

### 7.3 AppEnumerator trait

```rust
/// 应用枚举器 - 发现系统中已安装的应用
#[async_trait]
pub trait AppEnumerator: Send + Sync {
    /// 枚举所有已安装应用
    async fn enumerate_apps(&self) -> Vec<AppInfo>;
}
```

**设计说明**：
- `enumerate_apps` 为 async，与 SDK 其他 trait（IconExtractor、ShellExecutor、WindowManager）风格统一
- 不提供 `get_app_info(app_id)` 方法：当前无按 ID 单独查询的场景（YAGNI），如需可在 AppSource 层做缓存

### 7.4 AppLauncher trait

```rust
/// 应用启动器 - 启动已安装的应用
#[async_trait]
pub trait AppLauncher: Send + Sync {
    /// 启动应用
    /// 参数：app_id - 应用唯一标识；args - 启动参数（可选）
    /// 返回：成功返回 Ok(pid)，失败返回 HostApiError
    async fn launch_app(&self, app_id: &str, args: Option<&[String]>) -> Result<u32, HostApiError>;
}
```

### 7.5 Windows 平台实现

**文件位置**: `sdk/platform/windows/app_enumerator.rs`

```rust
/// Windows 应用枚举器实现
/// 通过 shell:AppsFolder 枚举 UWP 应用，通过文件系统扫描传统应用
pub struct WindowsAppEnumerator {
    // COM 初始化状态管理
    // IShellItem / IPropertyStore 缓存
}

impl AppEnumerator for WindowsAppEnumerator {
    async fn enumerate_apps(&self) -> Vec<AppInfo> {
        // 1. 枚举 shell:AppsFolder 获取 UWP 应用
        // 2. 通过 IPropertyStore 读取属性：
        //    - System.AppUserModel.ID
        //    - System.AppUserModel.PackageInstallPath
        //    - System.Tile.SmallLogoPath
        //    - System.Launcher.AppState
        // 3. 验证并选择最佳分辨率图标
    }
}
```

**文件位置**: `sdk/platform/windows/app_launcher.rs`

```rust
/// Windows 应用启动器实现
/// 通过 IApplicationActivationManager 激活 UWP 应用
pub struct WindowsAppLauncher;

impl AppLauncher for WindowsAppLauncher {
    async fn launch_app(&self, app_id: &str, args: Option<&[String]>) -> Result<u32, HostApiError> {
        // 1. CoInitialize
        // 2. CoCreateInstance(&ApplicationActivationManager)
        // 3. IApplicationActivationManager::ActivateApplication
        // 4. CoUninitialize
    }
}
```

### 7.6 调用链示例

```
AppSource::fetch_candidates()
    → WindowsAppEnumerator::enumerate_apps()  // SDK 层
        → shell:AppsFolder + IPropertyStore   // 平台实现

AppExecutor::execute()
    → PluginHandle::launch_app(app_id)        // SDK 层
        → WindowsAppLauncher::launch_app()    // 平台实现
            → IApplicationActivationManager   // Win32 API
```

---

## 八、ShellExecutor 扩展

### 8.1 新增方法

```rust
#[async_trait]
pub trait ShellExecutor: Send + Sync {
    // ... 现有方法

    /// 执行命令字符串（后台运行，无窗口）
    /// 参数：command - 要执行的命令字符串
    /// 返回：成功返回 Ok(())，失败返回 HostApiError
    async fn execute_command(&self, command: &str) -> Result<(), HostApiError>;
}
```

### 8.2 Windows 实现

```rust
impl ShellExecutor for WindowsShellExecutor {
    async fn execute_command(&self, command: &str) -> Result<(), HostApiError> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        std::process::Command::new("cmd")
            .args(["/D", "/S", "/C"])
            .raw_arg(command)
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn()
            .map(|_| ())
            .map_err(|e| HostApiError::ShellOperationFailed {
                target: command.to_string(),
                reason: format!("命令执行失败: {}", e),
            })
    }
}
```

---

## 八-A、LnkResolver：Lnk 快捷方式解析

### 8A.1 设计动机

Windows 系统中，开始菜单和桌面的程序入口通常是 `.lnk` 快捷方式文件。插件在判断程序的实际目标路径时（例如窗口唤醒需要通过 .lnk 找到 exe 进程名），需要解析 `.lnk` 文件获取其指向的真实路径。

这是一个平台特定的操作（Windows .lnk 二进制格式），因此抽象为 trait 注入 SDK。

### 8A.2 LnkResolver trait

```rust
/// Lnk 快捷方式解析器 trait，定义平台原语。
/// 各平台实现通过系统 API 解析 .lnk 快捷方式文件的目标路径，
/// 插件通过 PluginHandle 委托调用。
pub trait LnkResolver: Send + Sync {
    /// 解析 .lnk 快捷方式文件的目标路径。
    /// 参数：lnk_path - .lnk 文件的路径。
    /// 返回：解析成功返回目标路径，失败返回 None。
    fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String>;
}
```

**设计说明**：
- 只有 1 个方法，职责单一
- 返回 `Option<String>` 而非 `Result`：解析失败是常见场景（文件损坏、格式异常），调用方通常需要 fallback 而非中断
- 同步方法：`.lnk` 文件读取是本地 IO，无需异步

### 8A.3 Windows 平台实现

**文件位置**: `sdk/platform/windows/lnk_resolver.rs`

```rust
/// Windows 平台 Lnk 快捷方式解析器实现。
/// 使用 lnk crate 解析 .lnk 快捷方式文件，优先使用 GB18030 编码，失败后回退 UTF-16LE。
pub struct WindowsLnkResolver;

impl LnkResolver for WindowsLnkResolver {
    fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String> {
        // 1. 优先使用 GB18030 编码打开 .lnk 文件
        // 2. GB18030 失败时回退 UTF-16LE 编码
        // 3. 从 link_info 中提取 local_base_path
    }
}
```

**设计说明**：
- 使用 `lnk` crate 而非 Win32 `IShellLink` COM 接口，避免 COM 初始化开销
- 双编码策略：中文 Windows 系统的 .lnk 文件主要使用 GB18030 编码，但部分文件可能使用 UTF-16LE
- 回退策略确保兼容性

### 8A.4 调用链示例

```
WindowActivateExecutor::execute()
    → PluginHandle::resolve_lnk_target(lnk_path)   // SDK 层
        → WindowsLnkResolver::resolve_lnk_target()  // 平台实现
            → lnk::ShellLink::open() + link_info     // lnk crate
```

---

## 八-B、ResourceLoader：本地化字符串资源加载

### 8B.1 设计动机

Windows 系统中，开始菜单文件夹的显示名称可能通过 `desktop.ini` 文件引用 PE 资源（如 `@C:\Windows\System32\shell32.dll,-12345`），指向 DLL/EXE 中的本地化字符串资源。程序数据源在枚举程序时需要将这些资源引用解析为用户可见的本地化名称。

这涉及两个平台特定操作：
1. **资源字符串解析**：从 PE 文件加载本地化字符串（`LoadLibraryExW` + `LoadStringW`）
2. **desktop.ini 解析**：读取 INI 文件并提取 `[LocalizedFileNames]` 部分

### 8B.2 ResourceLoader trait

```rust
/// 平台资源加载器 trait，定义平台原语。
/// 各平台实现通过系统 API 加载本地化字符串资源，
/// 插件通过 PluginHandle 委托调用。
pub trait ResourceLoader: Send + Sync {
    /// 解析指定目录下的 desktop.ini 文件，提取 [LocalizedFileNames] 部分。
    /// 参数：dir_path - 要解析的目录路径。
    /// 返回：从原始文件名到本地化名称的映射。
    fn parse_localized_names_from_dir(&self, dir_path: &Path) -> HashMap<String, String>;
}
```

**设计说明**：
- `parse_localized_names_from_dir`：读取目录下的 `desktop.ini`，自动解析其中的 DLL 资源引用
- 两个方法都是同步的：文件读取和 PE 资源加载都是本地操作
- 返回 `Option` / `HashMap`：解析失败是常见场景，调用方需要 fallback

### 8B.3 Windows 平台实现

**文件位置**: `sdk/platform/windows/resource_loader.rs`

```rust
/// Windows 平台资源加载器实现。
/// 通过 Windows API 加载 PE 文件中的本地化字符串资源，并解析 desktop.ini 文件。
pub struct WindowsResourceLoader;

impl ResourceLoader for WindowsResourceLoader {

    fn parse_localized_names_from_dir(&self, dir_path: &Path) -> HashMap<String, String> {
        // 1. 读取 desktop.ini 文件
        // 2. 支持 UTF-16LE（BOM 标记）和 UTF-8 编码
        // 3. 解析 [LocalizedFileNames] section
        // 4. 自动调用 resolve_resource_string 处理 DLL 资源引用
    }
}
```

**设计说明**：
- `LoadLibraryExW` 使用 `LOAD_LIBRARY_AS_DATAFILE` 标志，仅加载资源不执行 DllMain
- `desktop.ini` 支持 UTF-16LE 和 UTF-8 两种编码（Windows 系统文件可能使用 UTF-16LE）
- 环境变量展开：`%SystemRoot%\System32\shell32.dll` 需要展开为实际路径
- 资源 ID 支持负值：Windows 资源 ID 可以是负数，使用 `unsigned_abs()` 转换

### 8B.4 调用链示例

```
ProgramSource::fetch_candidates()
    → PluginHandle::parse_localized_names_from_dir(dir_path)  // SDK 层
        → WindowsResourceLoader::parse_localized_names_from_dir()
            → desktop.ini 读取 + resolve_resource_string()
                → LoadLibraryExW + LoadStringW              // Win32 API

resolve_resource_string("@shell32.dll,-12345")
    → WindowsResourceLoader::resolve_resource_string()
        → LoadLibraryExW + LoadStringW + FreeLibrary         // Win32 API
```

---

## 九、目录结构

```
src-tauri/src/sdk/
├── mod.rs                     # 模块入口，导出公共 API（不导出 Configurable 组件）
├── host_api.rs                # HostApi struct + PluginHandle struct + IconRequest + CacheLevel + PluginSdkConfig + 错误类型
├── common/
│   ├── mod.rs                 # 通用模块入口
│   └── image_utils.rs         # ImageUtils — 跨平台图片处理工具函数（无系统调用，纯跨平台 crate）
├── icon/
│   ├── mod.rs                 # 图标模块入口
│   ├── icon_cache.rs          # IconCacheService — 纯缓存工具（L1/L2 原语）
│   └── icon_extractor.rs      # IconExtractor trait — 平台原语 + 跨平台默认实现
├── shell/
│   ├── mod.rs                 # Shell 模块入口
│   ├── shell_executor.rs      # ShellExecutor trait — 平台原语
│   ├── lnk_resolver.rs        # LnkResolver trait — 平台原语
│   ├── resource_loader.rs      # ResourceLoader trait — 平台原语
├── window/
│   ├── mod.rs                 # 窗口模块入口
│   └── window_manager.rs      # WindowManager trait — 平台原语
├── path/
│   ├── mod.rs                 # [新增] 路径模块入口
│   └── path_resolver.rs       # [新增] PathResolver trait — 平台原语
├── app/
│   ├── mod.rs                 # 应用模块入口
│   ├── app_enumerator.rs      # AppEnumerator trait
│   └── app_launcher.rs        # AppLauncher trait
├── storage/
│   ├── mod.rs                 # 存储模块入口
│   ├── storage_service.rs     # StorageService trait — 存储操作抽象接口（跨平台）
│   ├── local_storage.rs       # LocalStorageService — 本地文件存储实现（trait 实现）
│   └── webdav_storage.rs      # WebDAVStorageService — WebDAV 远程存储实现（trait 实现）
├── parameter/
│   ├── mod.rs                 # 参数模块入口（子模块必须 pub）
│   ├── parameter_resolver.rs  # ParameterResolver trait + 错误定义 + 结构体（核心入口）
│   ├── parser.rs              # 解析器 — 环境变量和路径占位符展开逻辑（pub）
│   └── provider.rs            # 系统参数提供者 — 时间、用户、系统信息（pub）
└── hotkey/
    ├── mod.rs                 # 快捷键模块入口（仅导出平台原语）
    ├── hotkey_manager.rs      # HotkeyManager — 快捷键注册与监听管理（平台原语）
    └── types.rs               # Hotkey、HotkeyConfig、HotkeyEvent 等类型定义（平台原语）
└── platform/
    ├── mod.rs                 # 条件编译选择平台实现
    ├── capabilities.rs        # PlatformCapabilities 定义
    └── windows/
        ├── mod.rs             # Windows 平台入口
        ├── icon.rs            # WindowsIconExtractor — Windows API 图标提取实现
        ├── shell.rs           # WindowsShellExecutor — Windows API Shell 操作实现
│       ├── lnk_resolver.rs   # WindowsLnkResolver — Windows Lnk 解析实现
│       ├── resource_loader.rs # WindowsResourceLoader — Windows 资源加载实现
        ├── window.rs          # WindowsWindowManager — Windows API 窗口管理实现
        ├── path_resolver.rs   # [新增] WindowsPathResolver — Windows API 路径解析实现
        ├── app_enumerator.rs  # WindowsAppEnumerator — Windows 应用枚举实现
        └── app_launcher.rs    # WindowsAppLauncher — Windows 应用启动实现
```

**SDK 层职责边界**：
- SDK 只提供**平台原语**（trait + 实现），不包含任何 Configurable 业务逻辑。
- SDK 不导出 Configurable 组件（如 `HotkeyConfigComponent`），这些属于上层配置管理，放在 `core/config/components/`。

platform 放在 sdk/ 下的理由：
1. platform 的唯一消费者是 sdk（HostApi 的实现层）
2. 其他模块不应直接调用 platform 代码——这正是 SDK 存在的意义
3. 封装性：sdk 是公共接口，platform 是私有实现
4. 放在离使用者近的地方，符合"最小可见性"原则

---

## 十、与现有架构的整合

### 10.1 Plugin::init() 整合 — ✅ 已落地

Plugin::init() 接收 `Arc<HostApi>` 参数，Plugin 类型的插件可在 init 中调用 `host_api.register()` 获取 `PluginHandle`，从而访问平台能力。

```rust
// 已落地的签名
async fn init(
    &self,
    ctx: &PluginContext,
    api: Arc<dyn PluginAPI>,
    host_api: Arc<HostApi>,  // 新增：访问平台能力
) -> Result<(), PluginError>;

// 使用示例：在 init 中注册获取 PluginHandle
async fn init(&self, ctx: &PluginContext, api: Arc<dyn PluginAPI>, host_api: Arc<HostApi>) -> Result<(), PluginError> {
    let handle = host_api.register("my-plugin", Default::default());
    // 存储 handle 供后续使用
    Ok(())
}
```

### 10.2 Executor 迁移 — ✅ 已完成

| 执行器                            | 实现方式                                                                   | 状态 |
| --------------------------------- | -------------------------------------------------------------------------- | ---- |
| `PathExecutor`                    | 使用 `PluginHandle::shell_open()`                                          | ✅    |
| `FileExecutor`                    | 使用 `PluginHandle::shell_open()`                                          | ✅    |
| `UrlExecutor`                     | 使用 `PluginHandle::shell_open()`                                          | ✅    |
| `WindowActivateExecutor`          | 使用 `PluginHandle::activate_window_by_process()` + `resolve_lnk_target()` | ✅    |
| `AppExecutor`（原 `UwpExecutor`） | 使用 `PluginHandle::launch_app()`                                          | ✅    |
| `CommandExecutor`                 | 使用 `PluginHandle::shell_execute_command()`                               | ✅    |

所有执行器均不再直接调用 Win32 API，完全委托 PluginHandle。

### 10.3 DataSource 迁移 — ✅ 已完成

| 数据源                        | 实现方式                                                                 | 状态 |
| ----------------------------- | ------------------------------------------------------------------------ | ---- |
| `ProgramSource`               | 使用 `PluginHandle::resolve_path()` + `parse_localized_names_from_dir()` | ✅    |
| `AppSource`（原 `UwpSource`） | 使用 `PluginHandle::enumerate_apps()`                                    | ✅    |

所有数据源均不再直接调用 Win32 API，完全委托 PluginHandle。

---

## 十一、迁移路线图

### 阶段一：基础设施（优先级：高）— ✅ 已完成

| 任务                            | 文件                                    | 说明                                             | 状态 |
| ------------------------------- | --------------------------------------- | ------------------------------------------------ | ---- |
| 定义 `PathResolver` trait       | `sdk/path/path_resolver.rs`             | 路径解析接口（StartMenu、Desktop、AppData）      | ✅    |
| 实现 `WindowsPathResolver`      | `sdk/platform/windows/path_resolver.rs` | Windows 路径解析（SHGetKnownFolderPath）         | ✅    |
| 扩展 `HostApi` / `PluginHandle` | `sdk/host_api.rs`                       | 注入 `PathResolver`，暴露 `resolve_path()` 方法  | ✅    |
| 修复 `ProgramSource` 硬编码路径 | `plugin/data_source/program_source.rs`  | 委托 `PluginHandle::resolve_path()` 动态生成配置 | ✅    |

**验收标准**：
- `program_source.rs` 中不再出现硬编码的用户路径
- 默认配置能正确获取当前用户的 StartMenu 和 Desktop 路径

### 阶段二：应用枚举与启动（优先级：高）— ✅ 已完成

| 任务                                 | 文件                                     | 说明                                         | 状态 |
| ------------------------------------ | ---------------------------------------- | -------------------------------------------- | ---- |
| 定义 `AppEnumerator` trait           | `sdk/app/app_enumerator.rs`              | 应用枚举接口（async）                        | ✅    |
| 定义 `AppLauncher` trait             | `sdk/app/app_launcher.rs`                | 应用启动接口                                 | ✅    |
| 定义 `AppInfo`                       | `sdk/app/mod.rs`                         | 统一数据结构（无 launch_command）            | ✅    |
| 实现 `WindowsAppEnumerator`          | `sdk/platform/windows/app_enumerator.rs` | 迁移 `UwpSource` 的 Win32 调用               | ✅    |
| 实现 `WindowsAppLauncher`            | `sdk/platform/windows/app_launcher.rs`   | 迁移 `UwpExecutor` 的 Win32 调用             | ✅    |
| 扩展 `PlatformCapability`            | `sdk/platform/capabilities.rs`           | `UwpLaunch` → `AppEnumeration` + `AppLaunch` | ✅    |
| 扩展 `HostApi` / `PluginHandle`      | `sdk/host_api.rs`                        | 注入新组件，暴露新方法                       | ✅    |
| 重命名 `UwpExecutor` → `AppExecutor` | `plugin/executor/app_executor.rs`        | 委托 `PluginHandle::launch_app()`            | ✅    |
| 重命名 `UwpSource` → `AppSource`     | `plugin/data_source/app_source.rs`       | 委托 `PluginHandle::enumerate_apps()`        | ✅    |
| 重命名 `PackageFamilyName` → `App`   | `plugin_system/types.rs`                 | TargetType + ExecutionTarget 统一命名        | ✅    |
| 注册 AppSource 到新插件系统          | `lib.rs`                                 | ConfigManager + CandidatePipeline            | ✅    |

**验收标准**：
- `AppExecutor` 不再直接调用 `windows::Win32` API
- `AppSource` 不再直接调用 `windows::Win32` API
- 系统应用枚举和启动功能正常工作

### 阶段三：ShellExecutor 扩展（优先级：中）— ✅ 已完成

| 任务                                 | 文件                                  | 说明                                         | 状态 |
| ------------------------------------ | ------------------------------------- | -------------------------------------------- | ---- |
| 扩展 `ShellExecutor` trait           | `sdk/shell/shell_executor.rs`         | 新增 `shell_execute_command` 方法            | ✅    |
| 实现 Windows `shell_execute_command` | `sdk/platform/windows/shell.rs`       | 封装 `CommandExt::creation_flags` + 空校验   | ✅    |
| 扩展 `PluginHandle`                  | `sdk/host_api.rs`                     | 新增 `shell_execute_command` 委托方法        | ✅    |
| 重构 `CommandExecutor`               | `plugin/executor/command_executor.rs` | 委托 `PluginHandle::shell_execute_command()` | ✅    |
| 更新注册处                           | `lib.rs`                              | 注册 `command-executor` PluginHandle         | ✅    |

**验收标准**：
- `CommandExecutor` 不再直接调用 `std::os::windows::process::CommandExt`
- `CommandExecutor` 不再包含任何 Windows 特定的 API 调用
- 命令执行功能正常工作（cmd /D /S /C 后台运行）
- ShellExecutor trait 的 4 个方法命名风格统一（`shell_*` 前缀）

### 阶段四：LnkResolver 快捷方式解析（优先级：高）— ✅ 已完成

| 任务                            | 文件                                   | 说明                                            | 状态 |
| ------------------------------- | -------------------------------------- | ----------------------------------------------- | ---- |
| 定义 `LnkResolver` trait        | `sdk/shell/lnk_resolver.rs`            | Lnk 解析接口                                    | ✅    |
| 实现 `WindowsLnkResolver`       | `sdk/platform/windows/lnk_resolver.rs` | Windows Lnk 解析（lnk crate + 双编码回退）      | ✅    |
| 扩展 `HostApi` / `PluginHandle` | `sdk/host_api.rs`                      | 注入 `LnkResolver`，暴露 `resolve_lnk_target()` | ✅    |
| 迁移 `WindowActivateExecutor`   | `plugin/executor/window_activate.rs`   | 委托 `PluginHandle::resolve_lnk_target()`       | ✅    |

**验收标准**：
- `WindowActivateExecutor` 不再直接调用 `core::storage::utils::get_lnk_target_path()`
- .lnk 文件解析功能正常工作

### 阶段五：ResourceLoader 本地化字符串加载（优先级：中）— ✅ 已完成

| 任务                            | 文件                                      | 说明                                                              | 状态 |
| ------------------------------- | ----------------------------------------- | ----------------------------------------------------------------- | ---- |
| 定义 `ResourceLoader` trait     | `sdk/shell/resource_loader.rs`            | 资源加载接口（parse_localized）                                   | ✅    |
| 实现 `WindowsResourceLoader`    | `sdk/platform/windows/resource_loader.rs` | Windows 资源加载（LoadLibraryExW + LoadStringW + INI 解析）       | ✅    |
| 扩展 `HostApi` / `PluginHandle` | `sdk/host_api.rs`                         | 注入 `ResourceLoader`，暴露 `parse_localized_names_from_dir()` 等 | ✅    |
| 迁移 `ProgramSource`            | `plugin/data_source/program_source.rs`    | 委托 `PluginHandle::parse_localized_names_from_dir()`             | ✅    |

**验收标准**：
- `ProgramSource` 不再直接调用 `LoadLibraryExW` / `LoadStringW`
- 开始菜单文件夹的本地化名称正确显示

### 阶段六：Plugin::init() 整合（优先级：高）— ✅ 已完成

| 任务                           | 文件                     | 说明                               | 状态 |
| ------------------------------ | ------------------------ | ---------------------------------- | ---- |
| 修改 `Plugin::init()` 签名     | `plugin_system/types.rs` | 增加 `host_api: Arc<HostApi>` 参数 | ✅    |
| 更新所有 `Plugin::init()` 实现 | `plugin/` 目录           | 适配新签名                         | ✅    |

**验收标准**：
- `Plugin::init()` 可接收 `Arc<HostApi>` 参数
- Plugin 类型的插件可在 init 中注册获取 PluginHandle

### 阶段七：文档与测试（优先级：低）

| 任务                         | 说明                  |
| ---------------------------- | --------------------- |
| 更新 `PLUGIN_SDK_DESIGN.md`  | 本文档                |
| 更新 `REFACTORING_DESIGN.md` | 同步迁移进度          |
| 编写单元测试                 | 各 trait 的 mock 测试 |

---

## 十二、设计决策记录

### 12.1 为什么统一为 AppEnumerator + AppLauncher？

**问题**: Windows 有 UWP，macOS 有 Launch Services，Linux 有 Flatpak/Snap，是否需要各自定义 trait？

**决策**: 统一抽象为 `AppEnumerator` + `AppLauncher`。

**理由**:
1. 三个平台的模式高度一致：都有"传统搜索找不到"的应用，都需要专属 API 启动
2. 统一接口降低插件开发成本，插件无需关心平台差异
3. 未来新增平台（如 BSD）只需实现 trait 即可
4. 符合 Plugin SDK 的设计原则：插件只关注「做什么」，SDK 负责「怎么做」

### 12.2 为什么重命名 UwpSource → AppSource、UwpExecutor → AppExecutor？

**问题**: `UwpSource` 和 `UwpExecutor` 是 Windows 专属命名，是否需要保留？

**决策**: 重命名为 `AppSource` 和 `AppExecutor`。

**理由**:
1. 这两个组件的本质是"通过平台专属 API 枚举和启动系统应用"，不局限于 UWP
2. 跨平台视角下，macOS 有 Launch Services，Linux 有 Flatpak/Snap，都是同一概念
3. 命名去 Windows 化，符合 Plugin SDK 的跨平台设计原则
4. 配合 `TargetType::App` / `ExecutionTarget::App`（原 `PackageFamilyName`）统一命名体系

### 12.3 为什么 TargetType::PackageFamilyName 改为 TargetType::App？

**问题**: `PackageFamilyName` 是 Windows UWP 专属概念，是否适合跨平台？

**决策**: 重命名为 `TargetType::App`，`ExecutionTarget::App(String)`。

**理由**:
1. `PackageFamilyName` 是 Windows 专属术语，macOS/Linux 开发者无法理解
2. `App` 是跨平台通用概念，语义清晰
3. 字符串 payload 的含义由各平台实现决定（Windows: AppUserModelID，macOS: Bundle ID，Linux: app-id）
4. 符合软件工程命名规范：用通用概念而非平台特定术语

### 12.4 PathResolver 为什么用 trait 注入而非条件编译？

**问题**: `PathResolver` 获取用户目录（StartMenu、Desktop、AppData）是平台特定的，应该放在 `common` 用条件编译，还是像 `IconExtractor` 一样用 trait 注入？

**决策**: 采用 trait 注入模式，放在 `sdk/path/path_resolver.rs` 定义 trait，平台实现放在 `sdk/platform/{platform}/path_resolver.rs`。

**理由**:
1. `PathResolver` 必须调用系统 API（Windows: `SHGetKnownFolderPath`，macOS: `NSHomeDirectory`，Linux: `xdg-user-dir`），与 `IconExtractor`、`ShellExecutor` 性质相同
2. `ImageUtils` 能放在 `common` 是因为它完全依赖跨平台 crate（`image`、`usvg`），没有任何系统调用或条件编译
3. 统一架构风格：所有平台相关能力都通过 trait 注入，避免在 `common` 中堆积条件编译代码
4. 新增平台时只需实现 trait，无需修改 `common` 中的条件编译分支

### 12.5 为什么删除 AppInfo.launch_command？

**问题**: `AppInfo` 是否需要 `launch_command` 字段来区分不同启动方式？

**决策**: 删除 `launch_command` 字段。

**理由**:
1. Windows 上 `launch_command` 与 `app_id` 完全重复（都是 AppUserModelID）
2. Linux 上的 shell 命令格式（如 `flatpak run xxx`）是 `AppLauncher` 的实现细节，不应由 `AppEnumerator` 编码进数据结构
3. 违背 SDK 设计原则：插件只关注「做什么」，SDK 负责「怎么做」。`launch_command` 把启动方式编码进数据结构，启动方式变化时数据结构也要跟着变
4. `AppLauncher` 可自行根据 `app_id` 格式决定启动方式，无需外部传入

### 12.6 为什么删除 AppEnumerator::get_app_info？

**问题**: `AppEnumerator` 是否需要按 ID 单独查询的方法？

**决策**: 删除 `get_app_info`，trait 只保留 `enumerate_apps` 一个方法。

**理由**:
1. 唯一的消费者 `AppSource::fetch_candidates()` 调用 `enumerate_apps()` 后遍历全量结果，没有任何按 ID 单独查询的场景
2. 实现 `get_app_info` 需要 `WindowsAppEnumerator` 维护内部索引表，引入隐式的「先 enumerate 再 get」调用顺序依赖
3. 即使未来需要单条查询，也可在 `AppSource` 层做缓存解决
4. trait 方法越少，平台实现者负担越小，越容易跨平台

### 12.7 为什么 enumerate_apps 是 async？

**问题**: Windows COM 枚举是同步操作，`enumerate_apps` 是否应该保持 sync？

**决策**: 使用 async。

**理由**:
1. 与 SDK 其他 trait（`IconExtractor`、`ShellExecutor`、`WindowManager`）风格统一
2. 未来 macOS/Linux 的枚举可能需要异步（如 IPC 调用 Flatpak/Snap）
3. 统一 async 风格降低未来跨平台适配成本

### 12.8 为什么 LnkResolver 用 lnk crate 而非 IShellLink COM 接口？

**问题**: Windows 原生提供了 `IShellLink` COM 接口来解析 .lnk 文件，是否应该使用原生 API？

**决策**: 使用 `lnk` crate。

**理由**:
1. `IShellLink` 需要 COM 初始化（`CoInitialize`），而 `lnk` crate 是纯 Rust 实现，无此依赖
2. 避免在每次解析时都进行 COM 初始化/反初始化的开销
3. `lnk` crate 直接解析 .lnk 二进制格式，性能更优
4. 跨平台潜力：未来如果其他平台也支持 .lnk 格式（如 Wine），可直接复用

### 12.9 为什么 LnkResolver 使用双编码回退策略？

**问题**: .lnk 文件应该使用什么编码解析？

**决策**: 优先 GB18030，失败后回退 UTF-16LE。

**理由**:
1. 中文 Windows 系统的 .lnk 文件主要使用 GB18030 编码
2. 部分系统创建的 .lnk 文件可能使用 UTF-16LE 编码
3. 双编码回退确保最大兼容性，不会因编码问题导致解析失败
4. 解析失败时记录 warn 日志，便于排查问题

### 12.10 为什么 ResourceLoader 返回 Option 而非 Result？

**问题**: 资源字符串解析可能因多种原因失败（文件不存在、资源 ID 无效、DLL 加载失败），是否应该返回 `Result` 提供详细错误信息？

**决策**: 返回 `Option<String>`。

**理由**:
1. 资源解析失败是常见场景（用户自定义文件夹不需要本地化、资源 ID 引用的 DLL 不存在等）
2. 调用方通常只需要 fallback 到原始字符串，不需要根据错误类型做不同处理
3. 失败原因已通过 `warn!` 日志记录，排查时可直接查看日志
4. 与 `LnkResolver::resolve_lnk_target()` 返回类型一致，API 风格统一

### 12.11 为什么 parse_localized_names_from_dir 放在 ResourceLoader 而非独立 trait？

**问题**: `parse_localized_names_from_dir` 读取 INI 文件，是否应该放在独立的 trait 中？

**决策**: 放在 `ResourceLoader` 中。

**理由**:
1. `desktop.ini` 的 `[LocalizedFileNames]` 中的值通常是 PE 资源引用（`@dll,-id`），需要调用 `resolve_resource_string` 解析
2. 两个方法紧密协作：`parse_localized_names_from_dir` 内部自动调用 `resolve_resource_string`
3. 拆分为独立 trait 会导致调用方需要同时持有两个 trait 的引用，增加复杂度
4. 从概念上，“加载本地化资源”包含“从 desktop.ini 读取映射”和“从 PE 文件解析字符串”两个步骤

---

*文档版本: v2.2 | 最后更新: 2026-04-18*
