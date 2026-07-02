---
paths:
  - "**"
---

# 目录结构与文件放置规范

## Workspace 结构（crate 维度）

```
ZeroLaunch-rs/                          ← Cargo workspace 根
├── Cargo.toml                          ← [workspace] 定义 + [workspace.dependencies]
├── crates/
│   ├── plugin-api/                     ← zerolaunch-plugin-api
│   │   └── src/
│   │       ├── config/                 ← Configurable trait, SettingDefinition, ComponentType 等
│   │       ├── host/                   ← HostApiError, OpenTarget, CacheLevel, PluginHandle, PluginSdkConfig
│   │       ├── platform/               ← PlatformCapability, PlatformCapabilities
│   │       ├── plugin/                 ← Plugin trait, Query/QueryResponse, 组件 trait, CachedCandidateData
│   │       ├── services/               ← 能力域 trait（具体数量以代码为准；新增域时**必须**在此登记 + sdk.md 同步更新）
│   │       ├── common/                 ← DirUtils, ImageUtils
│   │       └── mock/                   ← Stub 实现 + mock_plugin_handle()（feature = "mock"）
│   ├── plugin-protocol/                ← zerolaunch-plugin-protocol
│   │   └── src/                        ← JSON-RPC 消息体, 方法名常量, manifest schema, 错误码, codec (LSP Content-Length 帧编解码)
│   ├── plugin-host/                    ← zerolaunch-plugin-host
│   │   └── src/                        ← 子进程管理, transport, JsonRpcClient, RemotePluginAdapter
│   ├── plugin-sdk-rust/                ← zerolaunch-plugin-sdk-rust
│   │   └── src/                        ← Rust 第三方插件 SDK (run() + HostProxy)
│   └── platform-windows/               ← zerolaunch-platform-windows
│       └── src/                        ← Windows 平台 trait 实现 + windows_capabilities()
├── zerolaunch-cli/                     ← zerolaunch-cli (独立 bin crate)
├── plugin-template/                    ← Rust 第三方插件项目模板（不在 workspace，显式 exclude）
└── src-tauri/                          ← zerolaunch-rs（主程序）
    └── src/
        ├── bootstrap.rs               ← 应用启动初始化（从 lib.rs 提取）
        ├── sdk.rs                      ← re-export 桥（类型本体在 plugin-api / platform-windows）
        ├── core/                       ← ConfigManager, ConfigStore, 核心配置组件
        ├── builtin_plugin/             ← 内置插件实现（具体数量以代码为准）
        ├── plugin_framework/           ← SessionRouter, Pipeline, Registry, PluginManager
        ├── tray/                       ← 系统托盘管理
        ├── window/                     ← 窗口位置工具函数
        ├── commands/                   ← IPC 命令薄代理
        ├── state/                      ← AppState
        └── utils/                      ← 通用工具
```

**依赖方向**: `plugin-api ← plugin-protocol ← plugin-host ← src-tauri`、`plugin-api ← platform-windows ← src-tauri`、`plugin-api ← plugin-sdk-rust` — 禁止反向依赖。

- **第三方插件作者**只依赖 `zerolaunch-plugin-api`，不需要 Tauri / Windows / 主程序源码
- **新增 SDK trait**：在 `crates/plugin-api/src/services/<domain>/` 定义
- **新增 Windows 实现**：在 `crates/platform-windows/src/` 实现对应的 trait
- **src-tauri 中的 sdk.rs** 现为 re-export 桥，类型本体已迁至 plugin-api

## 后端 (src-tauri/src/)

### 顶层目录职责

| 目录 | 职责 | 可引用 | 禁止 |
|------|------|--------|------|
| `sdk.rs` | re-export 桥（类型本体在 plugin-api / platform-windows） | 无外部依赖 | 引用 core/、builtin_plugin/、plugin_framework/ |
| `core/` | 业务核心：ConfigManager、Configurable trait、类型定义 | sdk.rs | 引用 builtin_plugin/、plugin_framework/ |
| `builtin_plugin/` | 内置插件实现：DataSource、Executor、SearchEngine 等 | sdk.rs、core/ | 引用 plugin_framework/ |
| `plugin_framework/` | 插件框架：SessionRouter、Pipeline、Registry、PluginManager | sdk.rs、core/、builtin_plugin/ | 被其他层反向引用 |
| `tray/` | 系统托盘管理 | state/ | 包含业务逻辑 |
| `window/` | 窗口位置工具函数 | 无外部依赖 | 包含业务逻辑 |
| `commands/` | IPC 命令：薄代理层，仅委托 | 全部 | 包含业务逻辑 |
| `state/` | AppState 定义 | core/、plugin_framework/ | 包含业务方法 |
| `utils/` | 通用工具（locale、font_database 等） | 无限制 | 包含业务逻辑 |

### 各目录详细说明

#### `sdk.rs` — re-export 桥

`sdk.rs` 是单文件的轻量 re-export 桥，类型本体已迁至 `crates/plugin-api/src/`：
- **trait + 数据类型** → `crates/plugin-api/src/services/<domain>/`
- **HostApi 错误/配置类型** → `crates/plugin-api/src/host/`
- **Windows 平台实现** → `crates/platform-windows/src/`

#### `core/` — 业务核心
```
core/
├── constants.rs         ← 应用常量
├── bridge_error.rs      ← BridgeError（IPC 错误）
├── config/              ← 配置系统
│   ├── manager.rs       ← ConfigManager 主调度器
│   ├── store.rs         ← ConfigStore（JSON 持久化）
│   ├── models.rs        ← 配置数据模型
│   ├── registry.rs      ← ConfigurableRegistry
│   ├── event.rs         ← ConfigEvent + PluginRuntimeEvent 广播
│   ├── setting_builders.rs ← SchemaBuilder API
│   └── mod.rs           ← 模块入口
```

- **核心配置组件**（`builtin_plugin/config/` 下的组件，如 AppearanceConfig、HotkeyConfig 等）：不属于任何插件的系统级配置
- **新增核心配置组件** 时放 `builtin_plugin/config/`。**新增插件** 时放 `builtin_plugin/` 对应子目录

#### `builtin_plugin/` — 内置插件实现
```
builtin_plugin/
├── config/               ← 核心配置组件（非插件系统级配置）
│   ├── appearance_config.rs
│   ├── general_config.rs
│   ├── hotkey_config.rs
│   ├── installation_monitor_config.rs
│   ├── storage_config.rs
│   └── window_behavior_config.rs
├── _template/            ← 内置插件模板（不被编译或 glob 扫描）
├── data_source/          ← 数据源（具体数量以代码为准）
├── executor/             ← 执行器（具体数量以代码为准）
├── keyword_optimizer/    ← 关键词优化器（具体数量以代码为准）
├── score_booster/        ← 分数增强器（具体数量以代码为准）
├── search_engine/        ← 搜索引擎（具体数量以代码为准）
└── triggerable/          ← 可触发插件（具体数量以代码为准）
```

- 每个插件实现 `Configurable` trait（配置）+ 对应的领域 trait（如 `DataSource`、`ActionExecutor`）。**必须** 通过 `PluginHandle` 访问平台能力（见 [plugin-system.md](plugin-system.md)）
- 新增插件在对应目录添加 .rs 文件 + `inventory::submit!` 块即自动注册，**无需** 修改 `lib.rs`

#### `plugin_framework/` — 插件框架
```
plugin_framework/
├── builtin_registry.rs   ← inventory 自动发现与注册编排器
├── builtin.rs            ← 内置插件定义
├── inspector.rs          ← Plugin Inspector 调试面板 (feature = "inspector")
├── session_router.rs     ← 搜索会话路由（核心调度器）
├── candidate_pipeline.rs ← 候选项采集管道
├── search_pipeline.rs    ← 搜索排序管道
├── executor_registry.rs  ← 执行器注册表
├── manager.rs            ← 第三方插件生命周期管理（单入口）
├── host_handler.rs       ← 子进程 Host 管理
├── plugin_installer.rs   ← 插件安装/卸载逻辑（从 manager.rs 提取）
├── plugin_info.rs        ← 插件信息类型
├── zlplugin_protocol.rs  ← zlplugin:// 自定义协议处理（从 manager.rs 提取）
├── service.rs            ← PluginService
├── registry.rs           ← PluginRegistry
└── mod.rs                ← 模块入口（含 re-export，类型定义在各自模块内，已消除冗余 types.rs shim）
```

- **SessionRouter** 是运行时的中枢。所有 bridge 命令通过它路由
- **builtin_registry** 通过 `inventory` 在编译期收集所有内置组件，启动时统一注册
- **manager.rs** 是第三方插件生命周期的唯一入口，通过 `PluginRuntimeEvent` 广播通道与 ConfigManager/SessionRouter 事件驱动解耦
- **禁止** 在此层定义配置 schema 或持久化逻辑（那属于 core/）

#### `tray/` — 系统托盘
```
tray/
└── mod.rs                ← TrayManager 实现
```

#### `window/` — 窗口工具函数
```
window/
└── mod.rs                ← 窗口位置计算、多显示器支持
```

#### `commands/` — IPC 命令层
```
commands/
├── bridge.rs     ← bridge_ 前缀（搜索/会话，具体命令数见 commands.md）
├── config_file.rs← config_ 前缀（配置管理，具体命令数见 commands.md）
├── resource.rs   ← resource_ 前缀（资源管理，具体命令数见 commands.md）
├── plugin.rs     ← plugin_ 前缀（第三方插件管理，具体命令数见 commands.md）
├── inspector.rs  ← inspector_ 前缀（插件检查器，具体命令数见 commands.md）
└── cli.rs        ← cli_ 前缀（CLI HTTP 服务器，具体命令数见 commands.md）
```

- 命令处理器是 **薄代理**：接收参数 → 委托给 SessionRouter/ConfigManager → 返回结果。详细规范见 [commands.md](commands.md)
- IPC 命令按前缀分散在 `commands/` 子文件中，前缀 → 文件对应关系见 [commands.md](commands.md)

### 前端 (src-ui/)

| 目录 | 职责 | 放置规则 |
|------|------|---------|
| `bridge/` | IPC 契约层 | 类型定义 + 命令封装 + 事件监听 |
| `stores/` | Pinia 状态管理 | 每个关注点一个 store |
| `composables/` | 可复用逻辑 Hook | 有副作用的逻辑封装 |
| `router/` | Vue Router 配置 | 路由定义 |
| `views/` | 页面级组件 | 每个窗口入口对应一个 View |
| `components/` | UI 组件 | 按功能域子目录组织 |
| `components/settings/fields/` | 设置字段渲染器 | 每种 SettingType 一个组件 |
| `components/settings/fields/array/` | 数组 UI 策略 | 每种 ArrayUiHint 一个组件 |
| `plugins/built-in/` | 内置前端插件 | `import.meta.glob` 自动发现，目录约定 `built-in/<id>/index.ts` |
| `plugins/third-party-host/` | 第三方插件宿主 | iframe 宿主 + PostMessage 通信桥 |
| `plugins/built-in/_template/` | 前端插件模板 | 参考实现，不被 glob 扫描 |
| `utils/` | 纯工具函数 | 无副作用的工具 |
| `styles/` | 全局样式 | CSS 变量定义（variables.css + transitions.css） |
| `i18n/` | 国际化 | 语言文件 |

### 新文件放置决策树

```
新增一个功能 →
├─ 需要平台 API？
│  └─ 是 → 在 crates/plugin-api/src/services/ 定义 trait
│         → 在 crates/platform-windows/src/ 实现
│         → 在 HostApi 添加方法，通过 PluginHandle 暴露
├─ 是系统级配置（非插件）？
│  └─ 是 → 放 builtin_plugin/config/
├─ 是新的数据源/执行器/搜索引擎？
│  └─ 是 → 放 builtin_plugin/ 对应子目录
├─ 需要新的 IPC 命令？
│  └─ 是 → 放 commands/ 对应文件（按前缀规则）
├─ 需要前端新页面？
│  └─ 是 → 放 views/
├─ 是可复用的前端逻辑？
│  └─ 是 → 放 composables/
└─ 其他
   └─ 放 utils/
```
