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

```
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

### 4.2 缓存流程示例（CacheLevel::Full）

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

### 4.3 三组件协作模型

图标服务由三个组件协作完成，职责正交：

| 组件                   | 职责                                          | 依赖                         |
| ---------------------- | --------------------------------------------- | ---------------------------- |
| `IconExtractor` trait  | 平台原语 + 跨平台业务逻辑（缓存策略、后处理） | IconCacheService, ImageUtils |
| `IconCacheService`     | 纯缓存工具（L1/L2 原语）                      | 无业务依赖                   |
| `WindowsIconExtractor` | Windows API 图标提取（只实现 6 个原语）       | Win32 API                    |

---

## 五、HostApiError

错误类型设计：

| 错误类型                     | 说明                     |
| ---------------------------- | ------------------------ |
| `UnsupportedCapability`      | 平台不支持该能力         |
| `PluginNotRegistered`        | 插件未注册               |
| `IconExtractionFailed`       | 图标提取失败             |
| `ShellOperationFailed`       | Shell 操作失败           |
| `WindowOperationFailed`      | 窗口操作失败             |
| `AppEnumerationFailed`       | 应用枚举失败             |
| `AppLaunchFailed`            | 应用启动失败             |
| `ExecutionFailed`            | 通用执行失败             |
| `LnkResolutionFailed`        | Lnk 快捷方式解析失败     |

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

### 各平台专属应用生态

| 维度             | Windows                         | macOS                         | Linux                      |
| ---------------- | ------------------------------- | ----------------------------- | -------------------------- |
| **专属应用类型** | UWP、MSIX、开始菜单固定项       | App Bundle (.app)、DMG 安装项 | Flatpak、Snap、AppImage    |
| **发现机制**     | `shell:AppsFolder` 虚拟目录     | Launch Services 数据库        | .desktop 文件 + 包管理器   |
| **启动方式**     | IApplicationActivationManager   | LSOpenURLsWithRole            | flatpak run / snap run     |
| **权限模型**     | AppContainer 沙盒               | entitlements + Hardened Runtime | portals + namespaces     |

---

## 七、设计原则总结

| 原则           | 说明                                               |
| -------------- | -------------------------------------------------- |
| **关注点分离** | 插件只关注「做什么」，SDK 负责平台差异的「怎么做」 |
| **接口抽象**   | 通过 trait 定义能力契约，平台实现可替换            |
| **渐进式演进** | 先完善 SDK 接口设计，再补充各平台实现              |
| **测试友好**   | 可注入 Mock 实现进行单元测试                       |
