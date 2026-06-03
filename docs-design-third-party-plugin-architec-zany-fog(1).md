# ZeroLaunch-rs SDK 多 Crate 化实施计划（里程碑 1）

> **本文档面向执行者**：一个不熟悉本项目的 AI 助手将按本计划逐步执行。本文档力求"无需自由发挥"——所有决策、文件路径、验证方式、陷阱都已写明。执行者**禁止**做计划之外的"优化"或"重构"。如果某步骤遇到本文档未覆盖的情况，**停下来报告**，不要自行决策。

---

## 0. 执行须知（必读，不要跳过）

### 0.1 必读文件（按顺序读完再开始任何修改）

执行者**必须**先按此顺序通读以下文件，建立项目认知。每个文件后写明"为什么读"。

| 序 | 路径 | 必读理由 |
|----|------|---------|
| 1 | `D:\code\ZeroLaunch-rs\CLAUDE.md` | 项目顶层架构、IPC 命令清单、关键文件位置 |
| 2 | `D:\code\ZeroLaunch-rs\.claude\rules\general.md` | **通用纪律：async 契约、RwLock 守卫规则、死代码纪律、JSON 数值安全、文件命名约定、日志规范** — 任何修改都必须遵守 |
| 3 | `D:\code\ZeroLaunch-rs\.claude\rules\directory-map.md` | 目录职责划分、依赖方向规则（**sdk/ ← core/ ← plugin/ ← plugin_system/**） |
| 4 | `D:\code\ZeroLaunch-rs\.claude\rules\sdk.md` | SDK 层规范、HostApi 出口规则、推送式回调模式 |
| 5 | `D:\code\ZeroLaunch-rs\.claude\rules\plugin-system.md` | 插件系统规范、Configurable 生命周期、PluginHandle 使用 |
| 6 | `D:\code\ZeroLaunch-rs\.claude\rules\config.md` | 配置存储模式、Serde 默认值规范 |
| 7 | `D:\code\ZeroLaunch-rs\.claude\rules\data-flow.md` | 插件生命周期、动作分发机制、关键数据流 |
| 8 | `D:\code\ZeroLaunch-rs\.claude\rules\commands.md` | IPC 命令层规范（如该文件存在） |
| 9 | `D:\code\ZeroLaunch-rs\docs\design\plugin-sdk.md` | Plugin SDK 设计哲学、HostApi vs PluginAPI、PluginHandle 架构 |
| 10 | `D:\code\ZeroLaunch-rs\docs\design\third-party-plugin-architecture.md` | 第三方插件架构设想（最终目标） |
| 11 | `D:\code\ZeroLaunch-rs\src-tauri\Cargo.toml` | 当前所有依赖与 features，迁移参考 |
| 12 | `D:\code\ZeroLaunch-rs\src-tauri\src\sdk\mod.rs` | SDK 总导出，了解 16 个能力域 |
| 13 | `D:\code\ZeroLaunch-rs\src-tauri\src\sdk\host_api.rs` | **整文件读完**（约 1246 行）—— Step 9 的核心改造对象 |
| 14 | `D:\code\ZeroLaunch-rs\src-tauri\src\plugin_system\types.rs` | Plugin trait + 所有 plugin 数据类型 — Step 5/10 核心 |
| 15 | `D:\code\ZeroLaunch-rs\src-tauri\src\core\types\configurable.rs` | Configurable trait — Step 4 核心 |
| 16 | `D:\code\ZeroLaunch-rs\src-tauri\src\plugin\triggerable\calculator_plugin.rs` | 唯一完整的 Plugin trait 实现样例 — Step 10 改造目标 |

读完后执行者应当能回答以下问题（如果不能，回头再读）：
1. SDK 层的 16 个能力域是哪些？
2. `HostApi` 与 `PluginHandle` 的职责分别是什么？为什么要分开？
3. `Plugin` trait 现在的 `init` 签名为什么不合规？应改成什么？
4. `Configurable` trait 与 `Plugin` trait 的关系？
5. 项目的依赖方向规则是什么？违反会有什么后果？
6. RwLock 守卫为什么不能跨 `.await`？
7. 现有 23 个插件分为哪 5 类（DataSource / SearchEngine / ...）？

### 0.2 项目核心概念扫盲（执行者必须理解）

#### 概念 1：三层架构（依赖方向不可逆）
```
sdk/         ← 平台抽象（trait + Windows 实现）— 零反向依赖
core/        ← ConfigManager + Configurable trait
plugin/      ← 23 个内置插件实现（数据源、执行器、搜索引擎等）
plugin_system/ ← 框架层（SessionRouter、Pipeline、Registry）
commands/    ← IPC 薄代理
```
**`sdk/` 不能引用 `core/`、`plugin/`、`plugin_system/`** —— 已经验证（grep 无 `use crate::(core|plugin|plugin_system)` 出现在 sdk/）。本计划保持这一规则。

#### 概念 2：`HostApi` vs `PluginHandle`
- **`HostApi`**：宿主持有，平台能力总入口。包含 17 个 `Arc<dyn Trait>`（IconExtractor、ShellExecutor 等）+ 5 个宿主级回调（hide_window_callback 等）+ 注册表（DashMap<plugin_id, PluginHandle>）。宿主级方法（apply_hotkey_config、reconfigure_storage、update_icon_cache_dir）只在 HostApi 上。
- **`PluginHandle`**：插件持有，从 `HostApi::register(plugin_id, config)` 拿到。绑定 plugin_id + 插件配置。所有插件可见的服务方法（get_icon、shell_open、enumerate_apps 等）都在 PluginHandle 上。回调注册自动以 `plugin_id:` 前缀化。

**红线**：插件**永远只用** `PluginHandle`，**不用** `HostApi`。本计划 Step 10 修正了一处历史遗留（`Plugin::init` 当前接收 `Arc<HostApi>`，要改为 `Arc<PluginHandle>`）。

#### 概念 3：23 个内置插件分类
| 类别 | 个数 | trait | 用途 |
|------|------|-------|------|
| 触发式插件 | 2 | `Plugin` | calculator（=1+2）、everything（搜索文件） |
| 数据源 | 5 | `DataSource` | 应用列表、URL、书签、命令等候选项采集 |
| 执行器 | 6 | `ActionExecutor` | 启动、管理员启动、打开文件夹等 |
| 关键词优化器 | 8 | `KeywordOptimizer` | 拼音、首字母、空格处理等 |
| 搜索引擎 | 3 | `SearchEngine` | 标准、Launchy、Skim 三种打分算法 |
| 分数增强器 | 2 | `ScoreBooster` | 历史频率、查询亲和度 |

执行者要明白：**只有 2 个**插件实现 `Plugin` trait（calculator + everything），其余 21 个是其他 5 个组件 trait。Step 10 修改 init 签名只影响这 2 个。

#### 概念 4：类型恒等（迁移技巧的核心）
**关键技巧**：每次把类型从 A crate 搬到 B crate 时，A crate 的原文件改为 `pub use B::Type;`。这样所有引用 `crate::a::Type` 的代码**类型签名不变**，编译器视为同一个类型，无需修改任何引用方代码。这就是"每步可独立 cargo build 通过"的关键。

**反例**（错误做法）：把类型搬走后直接删掉原文件，会导致大量 `use crate::a::Type` 编译失败，每步改动几十处文件，难以排错。

#### 概念 5：workspace 路径依赖
workspace 中 crate 间用 `path = "../foo"` 或 `path = "crates/foo"` 互相引用。同一 workspace 的 crate 共享 `target/` 目录。`workspace.dependencies` 表把所有依赖版本统一管理，子 crate 用 `dep.workspace = true` 引用。

### 0.3 通用陷阱（执行者必须避免）

| 陷阱 | 错误示例 | 正确做法 |
|------|---------|---------|
| RwLock 守卫跨 .await | `let g = lock.read(); foo().await; g.field.x` | `let x = { let g = lock.read(); g.field.x.clone() }; foo().await;` |
| 跳过类型恒等阶段 | 搬走 Type 后删原文件 → 全工程修改引用 | 搬走后原文件 `pub use new_crate::Type;`，引用方零修改 |
| 一步迁太多 | 把 sdk/ 全部一次性搬完 | 每个能力域独立一步，每步 cargo build 通过 |
| 修改无关代码 | "顺手"重构计算器逻辑、修改注释、调整格式 | **禁止**。本计划只做迁移，不做重构 |
| 错改 init 签名时机 | Step 5 同时改 Plugin trait | Step 5 保持 Plugin trait 在原位，只迁数据类型；Step 10 单独改 |
| 误删 platform/windows | Step 7 时把 Windows 实现也搬到 plugin-api | plugin-api **绝不**含平台实现；Windows 代码只去 platform-windows |
| 跨 .await 持有 RwLockGuard | parking_lot 守卫 !Send，async fn 里跨 await 编译失败 | clone 数据出来再 await |
| 遗忘 mod 声明 | 新建文件后忘记加 `pub mod xxx;` | 新建文件后**立即**改对应 mod.rs |
| 新增依赖未登记 | 新加 use 但 Cargo.toml 未声明 | 修改 use 前先确认 Cargo.toml 已有该依赖 |
| 修改公开 API 行为 | 把 `&str` 改 `String`、把 `Vec<X>` 改 `&[X]` | **禁止**。所有 trait 方法签名 1:1 迁移 |
| 用 `find/grep/cat` Bash | `Bash: find . -name "*.rs"` | 使用 Glob/Grep/Read 工具 |

### 0.4 通用工程纪律（来自 .claude/rules/general.md）

执行者**必须**严格遵守：
1. **Async 契约**：`ActionExecutor::execute` 是 `async fn`，必须 `.await`，错误用 `?` 传播
2. **RwLock 守卫**：parking_lot 守卫 !Send，必须在任何 .await 前释放
3. **死代码纪律**：删除模块时同步删除 mod.rs 中的声明；禁止留下 .bak/.copy/.old 文件
4. **变更纪律**：优先扩展现有抽象，禁止引入新模块/trait/层（除非本计划明确要求）
5. **JSON 数值安全**：从 serde_json::Value 读数字用 `as_f64()`，不用 `as_i64()`
6. **前后端职责**：本计划全是后端 Rust，不涉及前端
7. **冒烟测试**：每步后 `cargo check` 零错误，本计划要求 `cargo build`（更严格）
8. **文件命名**：Rust 文件 snake_case
9. **日志规范**：用 `tracing` crate；不输出敏感信息

### 0.5 红线（绝对禁止）

1. ❌ **禁止**修改 trait 方法签名（除 §3.3 明确指出的 Plugin::init）
2. ❌ **禁止**修改 struct 字段名（影响 serde 序列化、跨语言契约）
3. ❌ **禁止**修改任何业务逻辑（计算器、搜索算法、配置加载等）
4. ❌ **禁止**"顺手"调整格式、注释、命名（除非本计划要求）
5. ❌ **禁止**把 Windows 平台代码放进 plugin-api
6. ❌ **禁止**让 plugin-api 依赖 tauri / windows / winreg / uiautomation / rdev / arboard / whoami
7. ❌ **禁止**让 plugin-api 引用 src-tauri 内的任何代码
8. ❌ **禁止**用 `git push --force`、`git reset --hard`、`git commit --no-verify`
9. ❌ **禁止**跳过 cargo build 验证直接进入下一步
10. ❌ **禁止**遇到不确定的情况自行决策——停下来报告

---

## 1. Context（为什么做这件事）

### 1.1 背景
当前 ZeroLaunch-rs 是单一 cargo crate，所有代码（sdk/ + core/ + plugin/ + plugin_system/ + commands/）都编译进同一二进制。为支持"第三方插件可独立定义、热插拔"的长期目标（见 `docs/design/third-party-plugin-architecture.md`），第一步必须把 SDK 抽离为独立 crate。

### 1.2 用户已确认的设计方向
1. 拆多个 crate（cargo workspace，单仓库）
2. **Plugin trait + QueryResponse + PluginMetadata 一同迁入** plugin-api crate
3. **核心抽象 + 平台实现分离**：plugin-api 含 trait/数据类型；platform-windows 含具体实现
4. **插件作者只依赖 plugin-api**（含内置 mock feature，dev-dep 打开）
5. **宿主统一管理运行时实例**：插件不能自己实例化 HostApi/PluginHandle
6. **里程碑 1 不解决运行时动态加载**：插件仍通过源码 path 依赖集成

### 1.3 范围边界
- ✅ **本计划包含**：workspace 拆分、plugin-api 抽离、platform-windows 分离、内置 mock 支持、Plugin::init 签名修正、文档更新
- ❌ **本计划不包含**：dlopen/wasm/IPC 动态加载、插件市场、前端插件 UI 动态加载、权限模型、macOS/Linux 实现、HostApi 暴露面缩减、Tauri 打包脚本调整、crates.io 发布

---

## 2. crate 划分

最终形态为 **3 个 crate** 的 cargo workspace：

```
ZeroLaunch-rs/                      ← workspace 根（仓库根）
├── Cargo.toml                      ← workspace 定义 + workspace.dependencies
├── crates/
│   ├── plugin-api/                 ← 第三方插件作者唯一依赖
│   │   ├── Cargo.toml
│   │   └── src/
│   └── platform-windows/           ← Windows 平台实现 + 工厂函数
│       ├── Cargo.toml
│       └── src/
└── src-tauri/                      ← 主程序（zerolaunch-rs）
    ├── Cargo.toml                  ← 改为 workspace member
    └── src/                        ← 保留宿主调度 + 内置插件
```

**为什么是 3 个而不是 4+ 个**：把 `core/config/`（ConfigManager、ConfigStore）独立为 crate 会强制它与 `plugin_system/`（SessionRouter/Pipeline）跨 crate 协作。两者都依赖大量内置组件类型，强行拆会引入循环依赖风险。当前粒度（plugin-api 提供契约，src-tauri 提供宿主全部实现）最干净。

### 2.1 `zerolaunch-plugin-api`（第三方插件唯一依赖）

**目标**：第三方插件作者唯一直接依赖的 crate。零平台代码、零 Tauri 依赖、零业务逻辑。

**包含内容**（按当前 src-tauri/src/ 子树粒度，**精确到文件**）：

| 来源（src-tauri/src/ 下） | 去向（crates/plugin-api/src/ 下） |
|---------|---------|
| `core/types/configurable.rs` | `config/configurable.rs` |
| `core/types/component_type.rs` | `config/component_type.rs` |
| `core/types/setting_def.rs` | `config/setting_def.rs` |
| `core/types/config_error.rs` | `config/error.rs` |
| `core/types/config_action.rs` | `config/action.rs` |
| `core/types/bridge_error.rs` | **Step 4 grep 决定**：若被 Configurable 引用则迁入 `bridge_error.rs`，否则留主程序 |
| `plugin_system/types.rs` 中除 `Plugin` trait 外的全部内容（含 5 个组件 trait） | `plugin/types.rs` + `plugin/component_traits.rs`（按数据类型 vs trait 拆分） |
| `plugin_system/types.rs` 中的 `Plugin` trait | `plugin/plugin_trait.rs`（init 签名 Step 10 改） |
| `plugin_system/cached_candidate.rs` | `plugin/cached_candidate.rs` |
| `sdk/host_api.rs` 全部 1246 行 | `host/{error.rs, open_target.rs, cache_level.rs, sdk_config.rs, builder.rs, host_api.rs, plugin_handle.rs}` 拆 7 文件 |
| `sdk/platform/capabilities.rs` 中 `PlatformCapability` 枚举 + `PlatformCapabilities` 结构（**不含** `::windows()` 工厂方法） | `platform/capabilities.rs` |
| `sdk/icon/` 全部（icon_request、icon_extractor、icon_cache） | `services/icon/{request.rs, extractor.rs, cache.rs}` |
| `sdk/shell/` 全部 | `services/shell/{executor.rs, lnk.rs, resource.rs}` |
| `sdk/window/` 全部 | `services/window/{manager.rs, positioner.rs}` + 数据类型 |
| `sdk/path/` 全部 | `services/path_resolver.rs` |
| `sdk/app/` 全部 | `services/app/{enumerator.rs, launcher.rs}` + AppInfo |
| `sdk/autostart/` 全部 | `services/autostart.rs` |
| `sdk/hotkey/` 全部 | `services/hotkey/` |
| `sdk/installation_monitor/` 全部 | `services/installation_monitor/` |
| `sdk/focus_monitor/` 全部 | `services/focus_monitor/` |
| `sdk/parameter/` 全部（含纯 Rust 的 default_resolver、template_parser） | `services/parameter/` |
| `sdk/timer/` 全部（含 TokioTimerManager 纯 tokio 实现） | `services/timer/` |
| `sdk/storage/` 全部（含 LocalStorage / WebDAVStorage 纯 Rust 实现） | `services/storage/` |
| `sdk/resource/` 全部（AppResourceService） | `services/app_resource.rs` |
| `sdk/common/`（dir_utils、image_utils、com_guard） | `common/`（com_guard 用 `#[cfg(target_os = "windows")]`） |

**features**：
- `default = ["webdav"]`（保持现状不破坏宿主）
- `webdav` — gated `reqwest_dav` 与 webdav storage 实现
- `mock` — gated `mockall` + 提供 `mock` 子模块

**依赖红线**：
- ✅ 允许：serde、tokio、async-trait、anyhow、thiserror、tracing、parking_lot、dashmap、blake3、chrono、reqwest、reqwest_dav（gated）、image、resvg、usvg、tiny-skia、fontdb、encoding_rs、url、walkdir、zip、regex、ini、fnv、lru、once_cell、uuid、tokio-util、mockall（gated）
- ❌ 禁止：windows、windows-core、winreg、uiautomation、rdev、arboard、whoami、lnk、tauri、tauri-plugin-*、scraper、widestring

### 2.2 `zerolaunch-platform-windows`

**目标**：Windows 平台所有 trait 的具体实现 + 一键构造已注入 Windows 实现的 `HostApiBuilder`。

**包含内容**：

| 来源（src-tauri/src/ 下） | 去向（crates/platform-windows/src/ 下） |
|---------|---------|
| `sdk/platform/windows/app_enumerator.rs` | `app_enumerator.rs` |
| `sdk/platform/windows/app_launcher.rs` | `app_launcher.rs` |
| `sdk/platform/windows/autostart.rs` | `autostart.rs` |
| `sdk/platform/windows/focus_monitor.rs` | `focus_monitor.rs` |
| `sdk/platform/windows/hotkey.rs` | `hotkey.rs` |
| `sdk/platform/windows/icon.rs` | `icon.rs` |
| `sdk/platform/windows/installation_monitor.rs` | `installation_monitor.rs` |
| `sdk/platform/windows/lnk_resolver.rs` | `lnk_resolver.rs` |
| `sdk/platform/windows/parameter_providers.rs` | `parameter_providers.rs` |
| `sdk/platform/windows/path_resolver.rs` | `path_resolver.rs` |
| `sdk/platform/windows/resource_loader.rs` | `resource_loader.rs` |
| `sdk/platform/windows/shell.rs` | `shell.rs` |
| `sdk/platform/windows/window.rs` | `window.rs` |
| `sdk/platform/windows/window_positioner.rs` | `window_positioner.rs` |
| `sdk/platform/windows/task_template.xml` | `assets/task_template.xml` |
| `sdk/platform/capabilities.rs::impl PlatformCapabilities { fn windows() }` 工厂方法 | `capabilities.rs::windows_capabilities() -> PlatformCapabilities` |
| **新增** `build_windows_host_api_builder(icon_cache_dir) -> HostApiBuilder` | `lib.rs`（详见 §3.1） |

**对外 API**：所有 `WindowsXxx` 类型 + `windows_capabilities()` + `build_windows_host_api_builder()`。

**依赖**：`zerolaunch-plugin-api`（path） + windows / windows-core / winreg / uiautomation / rdev / arboard / whoami / lnk / widestring / scraper / backtrace。整个 crate 顶部加 `#![cfg(target_os = "windows")]`。

**features**：
- `default = ["everything"]`
- `everything` — gated everything-rs / everything-sys-bindgen

### 2.3 `zerolaunch-rs`（src-tauri/，主程序）

**保留不动的内容**（**禁止**修改这些目录的代码逻辑）：
- `commands/`（IPC 命令薄代理）
- `state/`（AppState）
- `core/config/`（ConfigManager、ConfigStore、setting_builders、registry、event、components/、models）
- `core/tray/`、`core/constants.rs`
- `plugin_system/{session_router, candidate_pipeline, search_pipeline, executor_registry, dispatcher, service, registry}.rs`（保留宿主调度器）
- `plugin/`（23 个内置插件实现，仅修改 import 与 init 签名）
- `lib.rs`（仅修改 HostApi 构造代码）

**改造内容**：
- `core/types/`：迁出后改为 `pub use zerolaunch_plugin_api::{...};`，Step 12 删除整个目录
- `plugin_system/types.rs` + `cached_candidate.rs`：同上
- `sdk/`：整个目录迁出后改为 re-export 桥，Step 12 删除
- `lib.rs::build_app()` 中 HostApi 构造代码改为调用 `zerolaunch_platform_windows::build_windows_host_api_builder(...)`

---

## 3. 关键技术问题的解法

### 3.1 `HostApi::build()` 的平台耦合（Step 9 处理）

**当前问题**（`src-tauri/src/sdk/host_api.rs:1168`）：
```rust
#[cfg(target_os = "windows")]
pub fn build(self) -> HostApi {
    // ...
    capabilities: PlatformCapabilities::windows(),  // 平台耦合
    icon_extractor: self.icon_extractor.expect("missing icon_extractor"),  // panic
    // ...
}
```

**目标**（plugin-api 中改造后）：
```rust
// 不再有 #[cfg]
pub fn build(self) -> Result<HostApi, HostApiBuildError> {
    let capabilities = self.capabilities
        .ok_or(HostApiBuildError::MissingComponent("capabilities"))?;
    let icon_extractor = self.icon_extractor
        .ok_or(HostApiBuildError::MissingComponent("icon_extractor"))?;
    // ...
    Ok(HostApi { capabilities, icon_extractor, /* ... */ })
}
```

**新增字段与 setter**：
- `HostApiBuilder` 新增 `capabilities: Option<PlatformCapabilities>` 字段
- 新增 `pub fn capabilities(mut self, caps: PlatformCapabilities) -> Self` setter

**新增错误类型**：
```rust
#[derive(Debug, thiserror::Error)]
pub enum HostApiBuildError {
    #[error("missing component: {0}")]
    MissingComponent(&'static str),
}
```

**why 改为 Result**：
1. 插件作者用 mock feature 时频繁触发 builder（`mock_plugin_handle()` 内部走 builder），清晰错误比 panic 更友好
2. plugin-api 不应包含 `panic!` 这种"hard fail"在公开 API 上
3. 宿主端 `lib.rs::build_app()` 用 `.expect("HostApi 构造失败")` 即可保留原来的失败语义

**platform-windows 提供的工厂函数**（新增到 `crates/platform-windows/src/lib.rs`）：
```rust
use std::sync::Arc;
use zerolaunch_plugin_api::{HostApi, HostApiBuilder, PlatformCapabilities};

/// 返回一个已注入 Windows 14 个平台实现 + DefaultParameterResolver + TokioTimerManager + Windows capabilities 的 HostApiBuilder。
/// 宿主只需补充：storage_service、app_resource、5 个宿主级回调，然后调用 build()。
pub fn build_windows_host_api_builder(icon_cache_dir: String) -> HostApiBuilder {
    HostApi::builder(icon_cache_dir)
        .capabilities(windows_capabilities())
        .icon_extractor(Arc::new(WindowsIconExtractor::new()))
        .shell_executor(Arc::new(WindowsShellExecutor::new()))
        .window_manager(Arc::new(WindowsWindowManager::new()))
        .window_positioner(Arc::new(WindowsWindowPositioner::new()))
        .path_resolver(Arc::new(WindowsPathResolver::new()))
        .app_enumerator(Arc::new(WindowsAppEnumerator::new()))
        .app_launcher(Arc::new(WindowsAppLauncher::new()))
        .lnk_resolver(Arc::new(WindowsLnkResolver::new()))
        .resource_loader(Arc::new(WindowsResourceLoader::new()))
        .parameter_resolver(Arc::new(zerolaunch_plugin_api::DefaultParameterResolver::new()))
        .parameter_providers(
            Arc::new(WindowsClipboardProvider::new()),
            Arc::new(WindowsWindowHandleProvider::new()),
            Arc::new(WindowsSelectionProvider::new()),
        )
        .autostart_manager(Arc::new(WindowsAutoStartManager::new()))
        .hotkey_manager(Arc::new(WindowsHotkeyManager::new()))
        .installation_monitor(Arc::new(WindowsInstallationMonitor::new()))
        .focus_monitor(Arc::new(WindowsFocusMonitor::new()))
        .timer_manager(Arc::new(zerolaunch_plugin_api::TokioTimerManager::new()))
}
```

**宿主端 `lib.rs::build_app()` 的最终代码**（仅展示构造部分）：
```rust
use zerolaunch_platform_windows::build_windows_host_api_builder;

let host_api = build_windows_host_api_builder(icon_cache_dir)
    .storage_service(/* ConfigManager 决定 Local 或 WebDAV */)
    .app_resource(app_resource)
    .notify_callback(notify_cb)
    .hide_window_callback(hide_cb)
    .show_window_callback(show_cb)
    .is_window_visible_callback(visible_cb)
    .set_window_position_callback(set_pos_cb)
    .build()
    .expect("HostApi 构造失败");
```

### 3.2 `PlatformCapabilities::windows()` 工厂归属

**决策**：
- `PlatformCapability` 枚举 → plugin-api
- `PlatformCapabilities` 结构（含 `new()`、`supports()`、查询方法）→ plugin-api
- `PlatformCapabilities::windows()` 工厂方法 → **删除**，改为 platform-windows 顶层函数 `windows_capabilities() -> PlatformCapabilities`

**why**：plugin-api 不能"知道"任何平台名称。即使用 `#[cfg(target_os = "windows")]` 守住也是反向耦合。

### 3.3 `Plugin::init` 改成接收 `Arc<PluginHandle>`（Step 10 处理）

**当前签名**（`src-tauri/src/plugin_system/types.rs:417`）：
```rust
async fn init(
    &self,
    ctx: &PluginContext,
    host_api: Arc<crate::sdk::HostApi>,
) -> Result<(), PluginError>;
```

**目标签名**：
```rust
async fn init(
    &self,
    ctx: &PluginContext,
    handle: Arc<PluginHandle>,
) -> Result<(), PluginError>;
```

**所有现有 init 实现位置**：
- `src-tauri/src/plugin/triggerable/calculator_plugin.rs:111`（参数名 `_host_api: Arc<HostApi>`，**未实际使用**）
- `src-tauri/src/plugin/triggerable/everything_plugin.rs`（**Step 10 执行时 grep 确认行号**，使用方式相同）
- `src-tauri/src/plugin/readme.md:251`（文档示例，需同步）

**唯一调用点**：`src-tauri/src/plugin_system/service.rs::init_all`（约 :39），调用形如：
```rust
plugin.init(&ctx, host_api.clone()).await
```

**Step 10 改造方案**：
1. plugin-api 中 `Plugin::init` 签名改为接收 `Arc<PluginHandle>`
2. 修改 calculator + everything 的 init 实现：参数类型 `Arc<HostApi>` → `Arc<PluginHandle>`（参数名 `_host_api` 改为 `_handle`，零行为变更）
3. 修改 `service.rs::init_all`：保留 `host_api: Arc<HostApi>` 参数；遍历每个插件时 `let handle = host_api.register(plugin.metadata().id.clone(), PluginSdkConfig::default()); plugin.init(&ctx, handle).await`
4. 更新 `plugin/readme.md:251` 示例

**why 安全**：
- 唯一行为变更是 init 拿到 PluginHandle 而非 HostApi
- 现有 init 实现都不使用参数（`_host_api`），零行为变更
- 此修改顺手修正了 rule `sdk.md` 已要求的"插件用 PluginHandle，不用 HostApi"

### 3.4 `mock` feature 实现策略（Step 11 处理）

**决策**：**手写 stub 默认实现 + 可选 mockall**。

**plugin-api/src/mock/ 目录结构**：
```
mock/
├── mod.rs                ← pub fn mock_plugin_handle()
├── icon.rs               ← StubIconExtractor
├── shell.rs              ← StubShellExecutor + StubLnkResolver + StubResourceLoader
├── window.rs             ← StubWindowManager + StubWindowPositioner
├── path.rs               ← StubPathResolver
├── app.rs                ← StubAppEnumerator + StubAppLauncher
├── autostart.rs          ← StubAutoStartManager
├── hotkey.rs             ← StubHotkeyManager
├── installation_monitor.rs ← StubInstallationMonitor
├── focus_monitor.rs      ← StubFocusMonitor
├── parameter.rs          ← StubParameterResolver + 3 个 StubSystemParameterProvider
├── timer.rs              ← StubTimerManager（或直接用 TokioTimerManager）
└── storage.rs            ← StubStorageService
```

**核心入口**：
```rust
// crates/plugin-api/src/mock/mod.rs
#[cfg(feature = "mock")]
pub mod mock {
    use std::sync::Arc;
    use crate::*;

    /// 一站式构造一个所有依赖都注入了 stub 实现的 PluginHandle，用于插件单元测试。
    /// 默认行为：所有方法返回 Ok(Default::default()) 或空集合。
    pub fn mock_plugin_handle() -> Arc<PluginHandle> {
        let host_api = HostApi::builder("mock_cache".to_string())
            .capabilities(PlatformCapabilities::new(Default::default()))
            .icon_extractor(Arc::new(StubIconExtractor::default()))
            .shell_executor(Arc::new(StubShellExecutor::default()))
            .window_manager(Arc::new(StubWindowManager::default()))
            .window_positioner(Arc::new(StubWindowPositioner::default()))
            .path_resolver(Arc::new(StubPathResolver::default()))
            .app_enumerator(Arc::new(StubAppEnumerator::default()))
            .app_launcher(Arc::new(StubAppLauncher::default()))
            .lnk_resolver(Arc::new(StubLnkResolver::default()))
            .resource_loader(Arc::new(StubResourceLoader::default()))
            .parameter_resolver(Arc::new(StubParameterResolver::default()))
            .parameter_providers(
                Arc::new(StubSystemParameterProvider::default()),
                Arc::new(StubSystemParameterProvider::default()),
                Arc::new(StubSystemParameterProvider::default()),
            )
            .autostart_manager(Arc::new(StubAutoStartManager::default()))
            .hotkey_manager(Arc::new(StubHotkeyManager::default()))
            .installation_monitor(Arc::new(StubInstallationMonitor::default()))
            .focus_monitor(Arc::new(StubFocusMonitor::default()))
            .timer_manager(Arc::new(TokioTimerManager::new()))
            .storage_service(Arc::new(StubStorageService::default()))
            .app_resource(Arc::new(AppResourceService::new()))
            .notify_callback(|_, _| {})
            .hide_window_callback(|| {})
            .show_window_callback(|| {})
            .is_window_visible_callback(|| false)
            .set_window_position_callback(|_, _| {})
            .build()
            .expect("mock host_api build");
        host_api.register("__mock__", PluginSdkConfig::default())
    }
}
```

**ShellExecutor stub 范例**：
```rust
// crates/plugin-api/src/mock/shell.rs
use crate::*;
use async_trait::async_trait;
use parking_lot::Mutex;

#[derive(Default)]
pub struct StubShellExecutor {
    /// 记录所有 shell_open 调用，便于断言
    pub opens: Mutex<Vec<OpenTarget>>,
}

#[async_trait]
impl ShellExecutor for StubShellExecutor {
    async fn shell_open(&self, target: &OpenTarget) -> Result<(), HostApiError> {
        self.opens.lock().push(target.clone());
        Ok(())
    }
    async fn shell_open_folder(&self, _path: &str) -> Result<(), HostApiError> { Ok(()) }
    async fn shell_execute_elevation(&self, _path: &str) -> Result<(), HostApiError> { Ok(()) }
    async fn shell_execute_command(&self, _cmd: &str) -> Result<(), HostApiError> { Ok(()) }
}
```

**why 不全用 mockall**：mockall 的 `automock` 与 `#[async_trait]` 配合需要 `mockall_double` + 额外宏，复杂度高。手写 stub 的"默认 Ok"语义对 90% 单测场景已够用。插件作者若需要复杂 expectation，可在自己的 dev-dependencies 加 mockall。

### 3.5 `webdav` feature 默认开关
保持 plugin-api `default = ["webdav"]`。**why**：避免破坏宿主依赖。未来发布到 crates.io 时再考虑改 default。

### 3.6 `BridgeError` 归属（Step 4 grep 决定）

`BridgeError` 当前在 `core/types/bridge_error.rs`，是 IPC 命令层错误，被 `commands/` 使用。

**Step 4 执行前必须 grep 确认**：
```bash
# 在 src-tauri/ 下
grep -rn "BridgeError" src-tauri/src/core/types/
grep -rn "BridgeError" src-tauri/src/core/config/
grep -rn "use.*BridgeError" src-tauri/src/sdk/
grep -rn "use.*BridgeError" src-tauri/src/plugin_system/
grep -rn "use.*BridgeError" src-tauri/src/plugin/
```

**判断规则**：
- 若 BridgeError 仅在 commands/ 中使用 → **留在主程序**，不进 plugin-api
- 若 Configurable / SettingDefinition / ConfigError 中有 `From<BridgeError>` 或方法签名带 BridgeError → **一同迁入** plugin-api

执行者把 grep 结果记录在执行日志，再做决定。

---

## 4. 迁移步骤（每步独立可验证）

### Step 模板说明

每个 Step 都按以下模板组织。执行者**严格按模板执行**，不要省略任何环节：

```
### Step N — <名字>
#### 前置阅读
<本步骤需要先读的文件>
#### 前置 grep 检查
<本步骤要执行的 grep 命令，记录输出>
#### 精确指令
<逐条要做的事，含文件路径>
#### 改前 / 改后对比
<关键文件的改动示例>
#### 验证命令
<必须运行的验证命令>
#### 常见陷阱
<本步骤特有的陷阱与正确做法>
#### 不能做什么
<本步骤的红线>
```

---

### Step 0 — 准备工作

#### 前置阅读
读完 §0、§1、§2 全部内容。

#### 精确指令
1. `cd D:\code\ZeroLaunch-rs`
2. 创建分支：`git checkout -b feature/workspace-split`
3. 创建目录：`mkdir crates`
4. 验证当前 main 分支可构建（基线）

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build  # 必须通过
cargo build --release  # 必须通过
```

#### 常见陷阱
- 如 `cargo build` 当前主分支就失败，立即停下报告。本计划假设基线可构建。

#### 不能做什么
- 不要在执行步骤前提交任何代码

---

### Step 1 — 引入 workspace（src-tauri 仍是单 member） ✅

#### 前置阅读
- `D:\code\ZeroLaunch-rs\src-tauri\Cargo.toml`（学习当前依赖列表）

#### 前置 grep 检查
无（首次创建文件）。

#### 精确指令
1. 在仓库根 `D:\code\ZeroLaunch-rs\Cargo.toml` 创建文件，内容用 §6.1 的完整模板
2. **不修改** `src-tauri/Cargo.toml`
3. 不创建任何代码文件

#### 改前 / 改后对比
**改前**：仓库根无 Cargo.toml
**改后**：仓库根有 workspace Cargo.toml，src-tauri 是其 member

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build  # 必须通过
cargo build --release  # 必须通过
```

#### 常见陷阱
- workspace Cargo.toml 的 `members = ["src-tauri"]` 路径必须正确
- 如执行 `cargo build` 报 workspace 错误，检查 `[workspace]` 段格式

#### 不能做什么
- 不要修改 src-tauri/Cargo.toml 的依赖
- 不要用 `[workspace.dependencies]` 强制 src-tauri 立即采用——这一步只引入 workspace 框架

---

### Step 2 — 创建空 plugin-api crate ✅

#### 前置阅读
- §1.3、§2.1 全部
- `D:\code\ZeroLaunch-rs\.claude\rules\sdk.md` 复习一遍

#### 前置 grep 检查
无。

#### 精确指令
1. `cd D:\code\ZeroLaunch-rs\crates`
2. `cargo new --lib plugin-api`
3. 修改 `crates/plugin-api/Cargo.toml`：
   - 改 `name = "zerolaunch-plugin-api"`
   - 加 `version.workspace = true`、`edition.workspace = true`、`license.workspace = true`
   - 加最小 dependencies：仅 `serde`、`serde_json`、`thiserror`、`tracing`（先空架子，后续步骤按需加）
4. 修改 `crates/plugin-api/src/lib.rs`：清空内容，改为：
   ```rust
   //! ZeroLaunch plugin SDK — traits, data types, host API surface.
   ```
5. 修改 workspace 根 Cargo.toml，把 `crates/plugin-api` 加入 members
6. 修改 `src-tauri/Cargo.toml`：在 `[dependencies]` 加一行 `zerolaunch-plugin-api = { workspace = true }`，但**代码不引用**

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build  # 必须通过；plugin-api 是空架子，src-tauri 仅声明依赖未使用
```

#### 常见陷阱
- 如 cargo build 报 `unused dependency: zerolaunch-plugin-api`，加 `#[allow(unused_imports)]` 或先在 src-tauri 某处写 `use zerolaunch_plugin_api as _;`（占位，后续删除）

#### 不能做什么
- 不要往 plugin-api 加任何代码（这一步只是建空架子）
- 不要现在就开始 re-export 类型（留给后续步骤）

---

### Step 3 — 创建空 platform-windows crate ✅

#### 精确指令
1. `cd D:\code\ZeroLaunch-rs\crates`
2. `cargo new --lib platform-windows`
3. 修改 `crates/platform-windows/Cargo.toml`：
   - 改 `name = "zerolaunch-platform-windows"`
   - 加 `version.workspace = true` 等
   - 加依赖 `zerolaunch-plugin-api = { workspace = true }`
   - 加 `tokio.workspace = true`、`async-trait.workspace = true`、`tracing.workspace = true`、`anyhow.workspace = true`、`thiserror.workspace = true`、`parking_lot.workspace = true`、`once_cell.workspace = true`、`backtrace.workspace = true`、`widestring.workspace = true`、`scraper.workspace = true`
   - **不要**先加 `windows`、`winreg` 等大依赖，后续 Step 8 再加
4. 修改 `crates/platform-windows/src/lib.rs`：
   ```rust
   #![cfg(target_os = "windows")]
   //! Windows platform implementations of zerolaunch-plugin-api traits.
   ```
5. 修改 workspace 根 Cargo.toml，把 `crates/platform-windows` 加入 members

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build  # 必须通过
```

#### 不能做什么
- 不要让 src-tauri 现在就依赖 platform-windows（Step 9 再加）

---

### Step 4 — 迁移 `core/types/`（最干净的纯类型迁移） ✅

#### 前置阅读
- `D:\code\ZeroLaunch-rs\src-tauri\src\core\types\` 整目录每个文件（**全部读完**）
- `D:\code\ZeroLaunch-rs\.claude\rules\config.md` 复习一遍（如该文件存在）

#### 前置 grep 检查
**必做**（结果记录在执行日志）：
```bash
grep -rn "BridgeError" src-tauri/src/core/types/
grep -rn "BridgeError" src-tauri/src/core/config/
grep -rn "use.*BridgeError" src-tauri/src/
```
**根据结果决定**（见 §3.6）：
- 如 Configurable 不依赖 BridgeError → **不**迁移 bridge_error.rs（留在主程序）
- 如 Configurable 间接依赖 BridgeError → 一同迁移

#### 精确指令
1. 在 `crates/plugin-api/src/` 下创建 `config/` 子目录
2. 创建 `crates/plugin-api/src/config/mod.rs`：
   ```rust
   pub mod configurable;
   pub mod component_type;
   pub mod setting_def;
   pub mod error;
   pub mod action;
   pub use configurable::Configurable;
   pub use component_type::ComponentType;
   pub use setting_def::*;  // 看实际类型再调
   pub use error::ConfigError;
   pub use action::*;
   ```
3. 把 `src-tauri/src/core/types/configurable.rs` 全部内容**复制**到 `crates/plugin-api/src/config/configurable.rs`，调整 `use super::...` 为同级或父级模块路径
4. 同样处理 component_type.rs / setting_def.rs / config_error.rs（→ error.rs）/ config_action.rs（→ action.rs）
5. 如前置 grep 决定迁移 bridge_error.rs：复制到 `crates/plugin-api/src/bridge_error.rs`
6. 修改 `crates/plugin-api/src/lib.rs`：
   ```rust
   pub mod config;
   pub use config::*;
   #[cfg(/* 决定迁移 BridgeError 时 */)]
   pub mod bridge_error;
   ```
7. 在 `crates/plugin-api/Cargo.toml` 加必要依赖（serde、serde_json、thiserror 等，按 config/ 文件实际 use 决定）
8. **关键**：修改 `src-tauri/src/core/types/mod.rs`：
   - 删除原 `pub mod configurable;` 等模块声明
   - 改为：
     ```rust
     // 从 plugin-api re-export，保持类型恒等，所有引用 crate::core::types::Configurable 的代码无需修改
     pub use zerolaunch_plugin_api::{
         Configurable, ComponentType, ConfigError, ConfigActionDef,
         SettingDefinition, FieldDefinition, ArrayItem, ArrayUiHint,
         PathMode, PrimitiveType, SettingType, DetailActionDef,
         // 实际类型清单以 plugin-api 实际导出为准
     };
     #[cfg(/* 如迁移了 */)]
     pub use zerolaunch_plugin_api::{BridgeError, ErrorCode};
     ```
9. **删除**原 `src-tauri/src/core/types/{configurable,component_type,setting_def,config_error,config_action}.rs` 文件

#### 改前 / 改后对比
**改前**（src-tauri/src/core/types/mod.rs）：
```rust
pub mod configurable;
pub mod component_type;
pub mod setting_def;
pub mod config_error;
pub mod config_action;
pub mod bridge_error;

pub use configurable::*;
pub use component_type::*;
// ...
```

**改后**：
```rust
pub use zerolaunch_plugin_api::{Configurable, ComponentType, ConfigError, /* ... */};
```

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build           # 必须通过
cargo test            # 必须通过
cargo clippy --workspace --all-targets  # 不增加新 warning
```

#### 常见陷阱
- 复制文件时遗漏 `use super::...` 修改 → 编译失败
- 类型恒等不保持（如改了字段名）→ 大量编译错误
- 遗漏某个类型的 re-export → `crate::core::types::SomeType not found`

#### 不能做什么
- 不要修改 trait 方法签名
- 不要修改 struct 字段
- 不要修改 derive macro 列表
- 不要"顺手"改 doc comment

---

### Step 5 — 迁移 plugin_system 的纯数据类型与 5 个组件 trait ✅

#### 前置阅读
- `src-tauri/src/plugin_system/types.rs` 全文 450 行通读
- `src-tauri/src/plugin_system/cached_candidate.rs` 全文

#### 前置 grep 检查
```bash
grep -rn "use crate::plugin_system::types::" src-tauri/src/  # 看哪些地方引用
grep -rn "use.*plugin_system::cached_candidate" src-tauri/src/
```

#### 精确指令
1. 在 `crates/plugin-api/src/` 下创建 `plugin/` 子目录
2. 创建 `plugin/types.rs`：放 Query、QueryResponse、ListItem、ResultAction、PluginMetadata、PluginContext、PluginError、SearchCandidate、ScoredCandidate、ScoreDetail、ExecutionContext、ExecutionTarget、ExecutionError、TargetType、CandidateId、ConfirmResult、RegistrationError
3. 创建 `plugin/component_traits.rs`：放 DataSource、SearchEngine、ScoreBooster、KeywordOptimizer、ActionExecutor 五个 trait
4. **保留** Plugin trait 在原位 `src-tauri/src/plugin_system/types.rs`（Step 10 再迁）
5. 创建 `plugin/cached_candidate.rs`：复制 cached_candidate.rs 内容
6. 修改 `crates/plugin-api/src/lib.rs`：加 `pub mod plugin;` 和 `pub use plugin::*;`
7. **关键**：修改 `src-tauri/src/plugin_system/types.rs`：
   - 删除已迁移的所有类型定义
   - 顶部加 `pub use zerolaunch_plugin_api::{Query, QueryResponse, ListItem, ...};`
   - **保留** Plugin trait 定义（含原 init 签名）
8. **关键**：修改 `src-tauri/src/plugin_system/cached_candidate.rs`：
   - 删除 CachedCandidateData 定义
   - 改为 `pub use zerolaunch_plugin_api::CachedCandidateData;`

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build           # 必须通过
cargo test            # 必须通过
```

#### 常见陷阱
- 误把 Plugin trait 也搬到 plugin-api（**禁止**，留给 Step 10）
- 漏迁移 RegistrationError 等小类型 → 编译失败
- 5 个组件 trait（DataSource 等）依赖 Configurable，确保 Step 4 已完成

#### 不能做什么
- 不要在本步骤改 Plugin trait 的签名
- 不要修改 plugin_system/ 中的其他文件（service.rs / registry.rs 等）

---

### Step 6 — 迁移 IconRequest（被广泛引用的纯数据类型） ✅

#### 前置阅读
- `src-tauri/src/sdk/icon/icon_request.rs`

#### 前置 grep 检查
```bash
grep -rn "use crate::sdk::IconRequest" src-tauri/src/
grep -rn "use crate::sdk::icon::IconRequest" src-tauri/src/
grep -rn "IconRequest" crates/plugin-api/src/  # 已经间接引用？
```

#### 精确指令
1. 复制 `src-tauri/src/sdk/icon/icon_request.rs` 到 `crates/plugin-api/src/services/icon_request.rs`
2. 在 `crates/plugin-api/src/lib.rs` 加 `pub mod services;` 和 services 子模块
3. 创建 `crates/plugin-api/src/services/mod.rs`：暂时只 `pub mod icon_request; pub use icon_request::IconRequest;`
4. **关键**：修改 `src-tauri/src/sdk/icon/icon_request.rs`：
   - 删除原内容
   - 改为 `pub use zerolaunch_plugin_api::IconRequest;`
5. 检查 `src-tauri/src/sdk/icon/mod.rs` 和 `src-tauri/src/sdk/mod.rs`，确认 IconRequest re-export 链路畅通

#### 验证命令
```bash
cargo build  # 必须通过
```

---

### Step 7 — 迁移所有 sdk/ 子模块的 trait 与数据类型（不含 Windows 实现） ✅

**这是体量最大的步骤**，按"叶子优先 / 每能力域一组"顺序，**每子步骤后必须 cargo build 通过**。

#### 通用迁移模板（每个能力域都按此执行）

对于能力域 `<capability>`：
1. **前置阅读**：通读 `src-tauri/src/sdk/<capability>/` 下所有文件
2. **前置 grep**：`grep -rn "use crate::sdk::<capability>" src-tauri/src/`
3. **精确指令**：
   - 复制 `src-tauri/src/sdk/<capability>/` 下的 trait 与数据类型文件到 `crates/plugin-api/src/services/<capability>/`
   - 调整 `use super::...` 为新位置的相对路径
   - 调整 `use crate::sdk::...` 为 `use crate::services::...` 或 `use crate::...`
   - 在 `crates/plugin-api/src/services/mod.rs` 加 `pub mod <capability>;`
   - 在 `crates/plugin-api/src/lib.rs` 顶层 `pub use services::<capability>::*;`（或具体类型）
   - **关键**：原 `src-tauri/src/sdk/<capability>/mod.rs` 中删除模块声明，改为 `pub use zerolaunch_plugin_api::*;`（或具体类型）；原 `src-tauri/src/sdk/<capability>/<file>.rs` 文件内容改为 `pub use zerolaunch_plugin_api::*;` 或直接删除（仅当 mod.rs 已 re-export 时）
4. **新增依赖**：在 `crates/plugin-api/Cargo.toml` 按需加入新依赖（参考 §6.2 完整列表）
5. **验证**：`cargo build` 必须通过
6. **不能做什么**：不要把 `sdk/platform/windows/` 下的 Windows 实现搬过来（platform-windows 的活）

#### 子步骤顺序（按依赖深度从浅到深）

> 每个子步骤后**必须** `cargo build` 通过再进入下一步。如某步失败，**回滚该步**而不是继续下一步。

| 子步 | 能力域 | 包含文件 | 备注 |
|------|--------|---------|------|
| 7.1 | shell | shell_executor.rs / lnk_resolver.rs / resource_loader.rs / mod.rs | 三个 trait + types |
| 7.2 | window | window_manager.rs / window_positioner.rs / 数据类型 | WindowPosition / PositionRequest |
| 7.3 | path | path_resolver.rs（含 KnownPath 枚举） | |
| 7.4 | app | app_enumerator.rs / app_launcher.rs / mod.rs（AppInfo） | |
| 7.5 | autostart | autostart_manager.rs | |
| 7.6 | hotkey | types.rs / hotkey_manager.rs | 含 HotkeyConfig / Hotkey / HotkeyEvent / HotkeyRegistration |
| 7.7 | installation_monitor | types.rs / monitor.rs | |
| 7.8 | focus_monitor | types.rs / monitor.rs | |
| 7.9 | parameter | 全部（含纯 Rust default_resolver、template_parser） | DefaultParameterResolver 是纯 Rust，可一同迁入 |
| 7.10 | timer | types.rs / timer_manager.rs / tokio_timer_manager.rs | TokioTimerManager 是纯 tokio，一同迁入 |
| 7.11 | storage | storage_error.rs / storage_service.rs / local_storage.rs / webdav_storage.rs | LocalStorage / WebDAVStorage 是纯 Rust，一同迁入 |
| 7.12 | resource | app_resource.rs（AppResourceService） | |
| 7.13 | icon | icon_extractor.rs / icon_cache.rs（IconRequest 已在 Step 6 迁完） | trait 默认实现含跨平台逻辑 |
| 7.14 | platform/capabilities | capabilities.rs（**仅枚举与结构，不含 windows() 工厂**） | windows() 工厂留 src-tauri 暂不动，Step 8 时迁到 platform-windows |
| 7.15 | common | dir_utils.rs / image_utils.rs / com_guard.rs | com_guard 用 `#[cfg(target_os = "windows")]` 守住 |

#### 关键依赖追加（plugin-api/Cargo.toml）

随着每个子步骤完成，按需向 plugin-api Cargo.toml 添加：
- 7.9（parameter）：可能需要 `regex`
- 7.10（timer）：`tokio` 已有
- 7.11（storage）：`reqwest`、`reqwest_dav`（gated webdav）、`url`、`tokio` 文件 IO
- 7.12（resource）：可能需要 `walkdir`
- 7.13（icon）：`image`、`resvg`、`usvg`、`tiny-skia`、`fontdb`、`encoding_rs`、`blake3`（缓存键哈希）、`ini`（如果 desktop.ini 解析在这）、`reqwest`（网络图标）
- 7.15（common）：`image`、`fnv`、`lru`、`once_cell`

#### 常见陷阱
- 某能力域同时有 trait 与 Windows 实现时，**只迁 trait**，Windows 实现等 Step 8
- platform/capabilities 中的 `PlatformCapabilities::windows()` 方法**先保留**，Step 8 一并迁走
- com_guard 是 Windows 专用的 COM 初始化保护，迁入 plugin-api 时**必须** `#[cfg(target_os = "windows")]`
- 子步骤之间不要并行执行（避免错误传播）

#### 不能做什么
- 不要把 sdk/platform/windows/ 下任何文件搬到 plugin-api
- 不要修改 trait 签名
- 不要在迁移过程中"顺手"重构 IconCacheService 的缓存策略

---

### Step 8 — 迁移 Windows 平台实现到 platform-windows ✅

#### 前置阅读
- `src-tauri/src/sdk/platform/windows/` 14 个文件全部
- `src-tauri/src/sdk/platform/capabilities.rs` 中 `windows()` 工厂方法

#### 前置 grep 检查
```bash
grep -rn "use crate::sdk::platform::windows" src-tauri/src/
grep -rn "use crate::sdk::platform::capabilities::PlatformCapabilities::windows" src-tauri/src/
```

#### 精确指令
1. 复制 `src-tauri/src/sdk/platform/windows/*.rs` 14 个文件到 `crates/platform-windows/src/`
2. 移动 `task_template.xml` 到 `crates/platform-windows/assets/`
3. 修改文件中所有 `use crate::sdk::...` → `use zerolaunch_plugin_api::...`
4. 修改 `include_str!` 路径（如有），从相对原位置改为相对 platform-windows
5. 在 `crates/platform-windows/Cargo.toml` 加入所有 Windows 平台依赖（windows、windows-core、winreg、uiautomation、rdev、arboard、whoami、lnk、everything-rs gated 等）—— 参考 §6.3 完整模板
6. 修改 `crates/platform-windows/src/lib.rs`：
   ```rust
   #![cfg(target_os = "windows")]
   //! Windows platform implementations.

   pub mod app_enumerator;
   pub mod app_launcher;
   // ... 14 个 mod
   pub mod capabilities;

   pub use app_enumerator::WindowsAppEnumerator;
   pub use app_launcher::WindowsAppLauncher;
   // ... 全部 Windows 类型 re-export

   pub use capabilities::windows_capabilities;
   ```
7. 创建 `crates/platform-windows/src/capabilities.rs`：
   ```rust
   use zerolaunch_plugin_api::{PlatformCapability, PlatformCapabilities};

   /// 返回 Windows 平台支持的能力集合。
   /// 从原 sdk/platform/capabilities.rs::PlatformCapabilities::windows() 迁入。
   pub fn windows_capabilities() -> PlatformCapabilities {
       // 复制原 windows() 方法体
   }
   ```
8. **删除** `src-tauri/src/sdk/platform/windows/` 整个目录
9. **删除** `src-tauri/src/sdk/platform/capabilities.rs` 中的 `impl PlatformCapabilities { fn windows() }`（已在 Step 7.14 把枚举迁出，剩下的 windows() 工厂方法此时彻底删除）
10. 修改 `src-tauri/src/sdk/platform/mod.rs`：删除 `pub mod windows;`，改为 `pub use zerolaunch_platform_windows::*;`
11. **关键**：修改 `src-tauri/Cargo.toml`：加 `zerolaunch-platform-windows = { workspace = true }` 依赖

#### 改前 / 改后对比
**改前**（src-tauri/src/sdk/platform/mod.rs）：
```rust
pub mod windows;
pub mod capabilities;
```

**改后**：
```rust
pub use zerolaunch_platform_windows::*;
pub use zerolaunch_plugin_api::{PlatformCapability, PlatformCapabilities};
```

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build            # 必须通过
cargo run              # 启动 Tauri 应用
# 手测：
# 1. 全局热键唤醒搜索栏
# 2. 输入文字看到搜索结果
# 3. 选中应用按 Enter 启动
```

#### 常见陷阱
- task_template.xml 路径未更新 → 运行时找不到资源
- everything-rs 有架构限定（仅 x86_64），platform-windows Cargo.toml 必须保留 `[target.'cfg(all(target_os = "windows", target_arch = "x86_64"))'.dependencies]`
- com_guard 在 plugin-api 中 cfg windows，platform-windows 中可直接用 plugin-api 提供的版本，不要重复实现

#### 不能做什么
- 不要修改 Windows 实现的代码逻辑
- 不要改 trait 实现的方法签名
- 不要"优化" Windows API 调用

---

### Step 9 — 迁移 host_api.rs 到 plugin-api，并改造 build() ✅

#### 前置阅读
- **整文件读完** `src-tauri/src/sdk/host_api.rs` 1246 行
- §3.1 全文

#### 前置 grep 检查
```bash
grep -rn "HostApi::builder" src-tauri/src/
grep -rn "fn build_app" src-tauri/src/
grep -rn "HostApi::build" src-tauri/src/
```

#### 精确指令
1. 在 `crates/plugin-api/src/host/` 下按 §1.1 拆为 7 个文件：
   - `error.rs`：HostApiError
   - `open_target.rs`：OpenTarget
   - `cache_level.rs`：CacheLevel
   - `sdk_config.rs`：PluginSdkConfig
   - `host_api.rs`：HostApi 结构 + impl
   - `builder.rs`：HostApiBuilder + 改造后的 build()
   - `plugin_handle.rs`：PluginHandle 结构 + 全部方法
2. 在 `crates/plugin-api/src/host/mod.rs` 重新导出
3. 在 `crates/plugin-api/src/lib.rs` 加 `pub mod host; pub use host::*;`
4. **关键改造 1**：HostApiBuilder 加 `capabilities: Option<PlatformCapabilities>` 字段 + setter
5. **关键改造 2**：HostApiBuilder::build() 去 `#[cfg(target_os = "windows")]`，返回 `Result<HostApi, HostApiBuildError>`，所有 `expect("missing xxx")` 改为 `.ok_or(HostApiBuildError::MissingComponent("xxx"))?`
6. **关键改造 3**：HostApi struct 新增 `capabilities: PlatformCapabilities` 字段（从 builder 传入）
7. 新增 `HostApiBuildError` 错误类型，加入 host/error.rs 或单独 host/builder.rs
8. **关键**：修改 `src-tauri/src/sdk/host_api.rs`：
   - 删除原全部内容
   - 改为 `pub use zerolaunch_plugin_api::{HostApi, HostApiBuilder, HostApiBuildError, HostApiError, OpenTarget, CacheLevel, PluginSdkConfig, PluginHandle};`
9. 在 `crates/platform-windows/src/lib.rs` 新增 `build_windows_host_api_builder()` 函数（按 §3.1 完整模板）
10. **关键**：修改 `src-tauri/src/lib.rs::build_app()` 中构造 HostApi 的代码：
    - 改前：手动调用 `HostApi::builder(...).icon_extractor(...)...build()`，注入每个 Windows 实现
    - 改后：`build_windows_host_api_builder(icon_cache_dir).storage_service(...).app_resource(...).<5个回调>().build().expect("HostApi 构造失败")`

#### 改前 / 改后对比

**改前**（src-tauri/src/lib.rs::build_app() 片段）：
```rust
let host_api = HostApi::builder(icon_cache_dir)
    .icon_extractor(Arc::new(WindowsIconExtractor::new()))
    .shell_executor(Arc::new(WindowsShellExecutor::new()))
    // ... 14 个平台实现
    .storage_service(storage)
    .app_resource(app_resource)
    .notify_callback(notify_cb)
    .hide_window_callback(hide_cb)
    .show_window_callback(show_cb)
    .is_window_visible_callback(visible_cb)
    .set_window_position_callback(set_pos_cb)
    .build();  // panic 版本
```

**改后**：
```rust
use zerolaunch_platform_windows::build_windows_host_api_builder;

let host_api = build_windows_host_api_builder(icon_cache_dir)
    .storage_service(storage)
    .app_resource(app_resource)
    .notify_callback(notify_cb)
    .hide_window_callback(hide_cb)
    .show_window_callback(show_cb)
    .is_window_visible_callback(visible_cb)
    .set_window_position_callback(set_pos_cb)
    .build()
    .expect("HostApi 构造失败");
```

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build         # 必须通过
cargo run           # 启动 Tauri 应用
# 手测全部：
# 1. 全局热键唤醒搜索栏
# 2. 输入文字看到搜索结果（验证 IconExtractor）
# 3. Enter 启动应用（验证 ShellExecutor）
# 4. Ctrl+Enter 管理员启动（验证 elevation）
# 5. 设置面板各 tab 可正常打开（验证 StorageService、AppResourceService）
# 6. 输入 =1+2 看到计算器面板（验证 Plugin trait 工作）
```

#### 常见陷阱
- HostApiBuilder::build() 改 Result 后，宿主 lib.rs 必须 `.expect()` 或 `?`，否则编译错
- capabilities 字段忘加 setter，platform-windows::build_windows_host_api_builder 内部无法注入
- HostApi 内部使用 capabilities 的代码（如 `self.capabilities.clone()` 在 register 中）要确认能找到字段
- 子模块拆分时 HostApi struct 字段访问问题——可能需要 `pub(crate)` 字段或 builder.rs 在同 crate 内访问

#### 不能做什么
- 不要在本步骤改 PluginHandle 上任何方法的签名
- 不要"优化"HostApiBuilder 的字段顺序
- 不要把 5 个宿主级回调（notify/hide/show/visible/set_position）改名

---

### Step 10 — 迁移 Plugin trait 并改 init 签名 ✅

#### 前置阅读
- §3.3 全文
- `src-tauri/src/plugin_system/types.rs` 中 Plugin trait 定义（约 :411-429）
- `src-tauri/src/plugin/triggerable/calculator_plugin.rs:111` 周围
- `src-tauri/src/plugin_system/service.rs::init_all`

#### 前置 grep 检查
```bash
# 确认所有 Plugin trait 实现位置
grep -rn "impl Plugin for" src-tauri/src/
# 确认所有 init 调用位置
grep -rn "plugin.init(" src-tauri/src/
grep -rn ".init(&ctx" src-tauri/src/
# 确认 everything_plugin 行号
grep -n "async fn init" src-tauri/src/plugin/triggerable/everything_plugin.rs
```

#### 精确指令
1. 在 `crates/plugin-api/src/plugin/plugin_trait.rs` 新建文件，把 `src-tauri/src/plugin_system/types.rs` 中的 `Plugin` trait 完整内容**复制**过去
2. **关键改造**：修改 plugin_trait.rs 中 init 签名：
   ```rust
   // 改前
   async fn init(
       &self,
       ctx: &PluginContext,
       host_api: Arc<crate::sdk::HostApi>,
   ) -> Result<(), PluginError>;
   // 改后
   async fn init(
       &self,
       ctx: &PluginContext,
       handle: Arc<PluginHandle>,
   ) -> Result<(), PluginError>;
   ```
3. 在 `crates/plugin-api/src/plugin/mod.rs` 加 `pub mod plugin_trait; pub use plugin_trait::Plugin;`
4. **关键**：修改 `src-tauri/src/plugin_system/types.rs`：
   - 删除原 Plugin trait 定义
   - 加 `pub use zerolaunch_plugin_api::Plugin;`
5. 修改 `src-tauri/src/plugin/triggerable/calculator_plugin.rs:111` 周围 init 实现：
   ```rust
   // 改前
   async fn init(
       &self,
       _ctx: &PluginContext,
       _host_api: Arc<HostApi>,
   ) -> Result<(), PluginError> {
       Ok(())
   }
   // 改后
   async fn init(
       &self,
       _ctx: &PluginContext,
       _handle: Arc<PluginHandle>,
   ) -> Result<(), PluginError> {
       Ok(())
   }
   ```
6. 同样修改 `src-tauri/src/plugin/triggerable/everything_plugin.rs`（行号以前置 grep 为准）
7. **关键**：修改 `src-tauri/src/plugin_system/service.rs::init_all`：
   ```rust
   // 改前（伪代码）
   for plugin in plugins {
       plugin.init(&ctx, host_api.clone()).await?;
   }
   // 改后
   for plugin in plugins {
       let plugin_id = plugin.metadata().id.clone();
       let handle = host_api.register(&plugin_id, PluginSdkConfig::default());
       plugin.init(&ctx, handle).await?;
   }
   ```
8. 更新 `src-tauri/src/plugin/readme.md:251` 文档示例（参数类型与名字）

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build         # 必须通过
cargo run           # 启动 Tauri 应用
# 手测：
# 1. 启动正常无 panic
# 2. 输入 =1+2 看到计算器面板（calculator_plugin.init 走通）
# 3. 输入 ` 触发 everything 搜索（everything_plugin.init 走通）
```

#### 常见陷阱
- 漏改 service.rs 的 init_all → calculator/everything 收到错误的 handle 类型 → 运行时无法工作
- service.rs 中 host_api.register 返回 Arc<PluginHandle>，注意正确传给 init
- 如果 PluginRegistry::register(plugin) 的实现位置触发 register，要避免 register 被调用两次
- everything_plugin.rs 可能存在多个 init 函数（构造函数、Plugin::init），只改 Plugin trait 实现的那个

#### 不能做什么
- 不要修改 calculator_plugin / everything_plugin 的其他方法（query / execute_action / metadata）
- 不要改 service.rs 的其他方法
- 不要改 PluginRegistry 的接口

---

### Step 11 — 添加 mock feature ✅

#### 前置阅读
- §3.4 全文
- 浏览 plugin-api 中所有 17 个平台 trait 的方法签名（确定 Stub 默认行为的实现策略）

#### 精确指令
1. 在 `crates/plugin-api/Cargo.toml` 加：
   ```toml
   [dependencies]
   mockall = { workspace = true, optional = true }

   [features]
   mock = ["dep:mockall"]
   ```
2. 在 `crates/plugin-api/src/` 下创建 `mock/` 目录
3. 创建 `mock/mod.rs`：
   ```rust
   #![cfg(feature = "mock")]

   pub mod icon;
   pub mod shell;
   pub mod window;
   pub mod path;
   pub mod app;
   pub mod autostart;
   pub mod hotkey;
   pub mod installation_monitor;
   pub mod focus_monitor;
   pub mod parameter;
   pub mod timer;
   pub mod storage;

   pub use icon::*;
   pub use shell::*;
   // ... 全部 re-export
   ```
4. 在每个 mock/<area>.rs 中实现对应的 Stub<TraitName>（默认返回 Ok / Default::default()），参考 §3.4 的 ShellExecutor 范例
5. 在 mock/mod.rs 加 `pub fn mock_plugin_handle()` 函数（按 §3.4 完整模板）
6. 修改 `crates/plugin-api/src/lib.rs`：
   ```rust
   #[cfg(feature = "mock")]
   pub mod mock;
   ```
7. 创建 `crates/plugin-api/tests/mock_handle.rs`：
   ```rust
   #![cfg(feature = "mock")]
   use std::sync::Arc;
   use zerolaunch_plugin_api::mock::mock_plugin_handle;
   use zerolaunch_plugin_api::*;

   #[tokio::test]
   async fn mock_plugin_handle_constructible() {
       let handle = mock_plugin_handle();
       // 调用每个公开方法验证不 panic
       let _ = handle.shell_open(OpenTarget::File("test".into())).await;
       let _ = handle.enumerate_apps().await;
       // ... 覆盖所有公开方法
   }
   ```

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build                                              # 默认不开 mock，必须通过
cargo build --features mock -p zerolaunch-plugin-api    # 开 mock 必须通过
cargo test --features mock -p zerolaunch-plugin-api     # mock 测试通过
```

#### 常见陷阱
- mock_plugin_handle() 内部 builder 必须填齐所有字段，少一个 build() 就返回 MissingComponent 错误
- AppResourceService::new() 是否需要参数？参考实际签名调整
- TokioTimerManager 是 plugin-api 中的纯 tokio 实现，可直接用作 mock 的 timer

#### 不能做什么
- 不要让 mock 模块影响 default feature 的 API surface（用 `#[cfg(feature = "mock")]` 严格守住）
- 不要在 mock 实现中调用 Windows API
- 不要让 mock 模块依赖 platform-windows

---

### Step 12 — 收尾清理 ✅

#### 前置 grep 检查
```bash
# 确认已无对 sdk/、core/types/、plugin_system/types 的依赖
grep -rn "use crate::sdk::" src-tauri/src/  # 应该都是 re-export
grep -rn "use crate::core::types::" src-tauri/src/  # 应该都是 re-export
grep -rn "pub mod" src-tauri/src/sdk/mod.rs  # 应该都是 use 替代
grep -rn "pub mod" src-tauri/src/core/types/mod.rs  # 应该都是 use 替代
```

#### 精确指令
1. 全文替换（用 Read + Edit 工具，**不要用** sed）：
   - `crate::sdk::` → `zerolaunch_plugin_api::`（部分 Windows 类型可能需 `zerolaunch_platform_windows::`）
   - `crate::core::types::` → `zerolaunch_plugin_api::`
   - `crate::plugin_system::types::` → `zerolaunch_plugin_api::`
2. **删除**整个 `src-tauri/src/sdk/` 目录
3. **删除**整个 `src-tauri/src/core/types/` 目录
4. **删除** `src-tauri/src/plugin_system/types.rs`、`src-tauri/src/plugin_system/cached_candidate.rs`
5. 修改 `src-tauri/src/plugin_system/mod.rs`：删除已删除模块的声明
6. 修改 `src-tauri/src/core/mod.rs`：删除 `pub mod types;`
7. 移除 src-tauri/Cargo.toml 中已不直接使用的依赖：
   - windows、windows-core、winreg、uiautomation、rdev、arboard、whoami、lnk、scraper、backtrace、widestring（这些被 platform-windows 持有）
   - 谨慎评估：fontdb、image、resvg、usvg、tiny-skia、reqwest_dav 等是否仍在 src-tauri/src/ 中**直接** import，若否则移除
8. 把 src-tauri/Cargo.toml 中共用依赖改用 `.workspace = true`

#### 验证命令
```bash
cd D:\code\ZeroLaunch-rs
cargo build --release --workspace      # 必须通过
cargo test --workspace                 # 必须全部通过
cargo clippy --workspace --all-targets # 不增加新 warning
cargo run                              # 启动 Tauri 应用
# 完整手测清单（**全部必须通过**）：
# 1. 全局热键唤醒搜索栏
# 2. 双击 Ctrl 切换窗口可见性
# 3. 输入文字看到搜索结果（含图标）
# 4. Enter 启动选中应用
# 5. Ctrl+Enter 管理员启动
# 6. Shift+Enter 窗口激活
# 7. 输入 =1+2 看到计算器面板
# 8. 计算器复制结果按钮工作
# 9. 输入 ` 触发 Everything 搜索
# 10. 设置面板各 tab 打开 / 修改 / 保存
# 11. 主题切换工作
# 12. 自启动开关工作
# 13. 退出后再启动配置不丢失
```

#### 常见陷阱
- 全文替换时漏掉某个 use 语句 → 编译错误（按错误信息修复）
- 误删某个 src-tauri/src/sdk/<file>.rs 中实际被使用的代码（不是 re-export 桥）→ 运行时崩溃。**Step 12 之前**确认每个文件都只剩 re-export
- src-tauri/Cargo.toml 共用依赖改 workspace 时格式错误 → 编译失败
- 移除依赖时误删仍在使用的（如 backtrace 可能被 src-tauri/src/lib.rs 中的 panic hook 使用）

#### 不能做什么
- 不要"顺手"重构主程序代码
- 不要改 plugin/、plugin_system/、commands/、state/ 的业务逻辑
- 不要修改 lib.rs 中除 HostApi 构造外的任何代码

---

### Step 13 — 文档更新 ✅

#### 精确指令
1. 更新 `D:\code\ZeroLaunch-rs\.claude\rules\sdk.md`：
   - "模块组织"小节加注：plugin-api / platform-windows 的位置
   - 加新章节"crate 边界规范"：sdk 类型在 plugin-api、Windows 实现在 platform-windows、宿主调度在 src-tauri
2. 更新 `D:\code\ZeroLaunch-rs\.claude\rules\directory-map.md`：
   - 顶部加 workspace 结构图
   - 三层架构图改为 crate 维度（plugin-api / platform-windows / src-tauri）
3. 更新 `D:\code\ZeroLaunch-rs\.claude\rules\plugin-system.md`：
   - "Plugin Trait Init"小节修改：明确 init 签名为 `Arc<PluginHandle>`、PluginHandle 用法
4. 更新 `D:\code\ZeroLaunch-rs\CLAUDE.md`：
   - "架构"小节反映 workspace
   - "关键文件"表更新路径（部分文件已迁到 crates/）
5. 创建 `D:\code\ZeroLaunch-rs\crates\plugin-api\README.md`：
   - 第三方插件作者快速上手指南
   - 参考 §5 的 EchoPlugin 完整示例
   - 说明 mock feature 用法
6. 更新 `D:\code\ZeroLaunch-rs\docs\design\plugin-sdk.md`：
   - 文档顶部加注：SDK 已独立为 zerolaunch-plugin-api crate
   - 引用路径从 `src-tauri/src/sdk/` 改为 `crates/plugin-api/src/`

#### 验证命令
```bash
# 仅文档更新，无需 cargo build
# 但可读性检查：
cargo build --workspace  # 应仍然通过
```

#### 不能做什么
- 不要改 docs/design/ 下其他设计文档的核心思想（除非与本次 SDK 拆分直接相关）

---

## 5. 第三方插件最终样貌

里程碑 1 完成后，第三方插件作者的工作流如下。

### 5.1 Cargo.toml
```toml
[package]
name = "my-zerolaunch-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
zerolaunch-plugin-api = { path = "../ZeroLaunch-rs/crates/plugin-api" }  # 或 git/version
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
zerolaunch-plugin-api = { path = "../ZeroLaunch-rs/crates/plugin-api", features = ["mock"] }
tokio = { version = "1", features = ["macros", "rt"] }
```

### 5.2 src/lib.rs 完整骨架
```rust
use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_api::{
    Configurable, ComponentType, ConfigError,
    Plugin, PluginContext, PluginError, PluginMetadata, PluginHandle,
    Query, QueryResponse, ListItem, IconRequest,
};

pub struct EchoPlugin { metadata: PluginMetadata }

impl EchoPlugin {
    pub fn new() -> Self {
        Self { metadata: PluginMetadata {
            id: "echo".into(), name: "Echo".into(), version: "0.1.0".into(),
            description: "回显输入".into(), author: "me".into(),
            trigger_keywords: vec!["echo".into()],
            supported_os: vec!["windows".into()], priority: 50,
        }}
    }
}

impl Configurable for EchoPlugin {
    fn component_id(&self) -> &str { "echo" }
    fn component_name(&self) -> &str { "Echo" }
    fn component_type(&self) -> ComponentType { ComponentType::Plugin }
}

#[async_trait]
impl Plugin for EchoPlugin {
    fn metadata(&self) -> &PluginMetadata { &self.metadata }

    async fn init(&self, _ctx: &PluginContext, _handle: Arc<PluginHandle>)
        -> Result<(), PluginError> { Ok(()) }

    async fn query(&self, _ctx: &PluginContext, query: &Query)
        -> Result<QueryResponse, PluginError>
    {
        Ok(QueryResponse::List { results: vec![ListItem {
            id: 1, title: query.search_term.clone(), subtitle: "echo".into(),
            icon: IconRequest::Path(String::new()), score: 100.0,
            actions: vec![], target_type: "Command".into(),
            user_arg_count: 0, has_system_params: false, trigger_keywords: vec![],
        }]})
    }

    async fn execute_action(&self, _ctx: &PluginContext, _action_id: &str,
        _payload: serde_json::Value) -> Result<(), PluginError> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerolaunch_plugin_api::mock::mock_plugin_handle;

    #[tokio::test]
    async fn echo_returns_input() {
        let plugin = EchoPlugin::new();
        let handle = mock_plugin_handle();
        let ctx = PluginContext::new("test-trace");

        plugin.init(&ctx, handle).await.unwrap();

        let q = Query {
            id: "q1".into(), raw_query: "echo hello".into(),
            search_term: "hello".into(),
        };
        let resp = plugin.query(&ctx, &q).await.unwrap();
        match resp {
            QueryResponse::List { results } => assert_eq!(results[0].title, "hello"),
            _ => panic!("expected List"),
        }
    }
}
```

### 5.3 集成到主程序（里程碑 1 阶段方式）
1. 在 `D:\code\ZeroLaunch-rs\src-tauri\Cargo.toml` 加 `my-zerolaunch-plugin = { path = "../my-zerolaunch-plugin" }`
2. 在 `lib.rs::init_plugin_system()` 中找到现有 `session_router.plugin_service().register(calculator_plugin)` 处，加 `session_router.plugin_service().register(Arc::new(EchoPlugin::new()))`
3. `cargo run` 启动，输入 `echo hello` 测试

运行时动态加载（无需修改主程序源码）留给里程碑 2。

---

## 6. Cargo.toml 完整模板

### 6.1 workspace 根 `D:\code\ZeroLaunch-rs\Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/plugin-api",
    "crates/platform-windows",
    "src-tauri",
]

[workspace.package]
version = "0.6.12"
edition = "2021"
authors = ["ghost-him"]
license = "MIT"
repository = "https://github.com/ghost-him/ZeroLaunch-rs"

[workspace.dependencies]
# 通用
async-trait = "0.1.89"
anyhow = "1.0.102"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2.0.18"
tokio = { version = "1.52.3", features = ["full"] }
tokio-util = "0.7.18"
tracing = "0.1.44"
parking_lot = "0.12.5"
dashmap = { version = "6.1.0", features = ["serde"] }
once_cell = "1.21.4"
chrono = "0.4.44"
url = "2.5.8"
walkdir = "2.5.0"
zip = "8.6.0"
regex = "1.12.3"
ini = "1.3.0"
fontdb = "0.23.0"
image = "0.25.10"
resvg = "0.47.0"
usvg = "0.47.0"
tiny-skia = "0.12.0"
encoding_rs = "0.8.35"
reqwest = { version = "0.13.3", features = ["json"] }
reqwest_dav = "0.3.3"
blake3 = "1.8.5"
fnv = "1.0.7"
lru = "0.18.0"
uuid = { version = "1.23.1", features = ["v4"] }
mockall = "0.13"
# Windows
windows = "0.62.2"
windows-core = "0.62.2"
winreg = "0.56.0"
widestring = "1.2.1"
uiautomation = "0.25.0"
rdev = { version = "0.5.3", features = ["unstable_grab"] }
arboard = "3.6.1"
whoami = "2.1.2"
lnk = "0.6.4"
scraper = "0.27.0"
backtrace = "0.3.76"
everything-rs = "0.1.10"
everything-sys-bindgen = "0.1.5"
# 内部
zerolaunch-plugin-api = { path = "crates/plugin-api" }
zerolaunch-platform-windows = { path = "crates/platform-windows" }

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 6.2 `crates/plugin-api/Cargo.toml`

```toml
[package]
name = "zerolaunch-plugin-api"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "ZeroLaunch plugin SDK — traits, data types, host API surface."

[dependencies]
async-trait.workspace = true
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
tokio-util.workspace = true
tracing.workspace = true
parking_lot.workspace = true
dashmap.workspace = true
blake3.workspace = true
chrono.workspace = true
fnv.workspace = true
lru.workspace = true
once_cell.workspace = true
url.workspace = true
walkdir.workspace = true
zip.workspace = true
regex.workspace = true
ini.workspace = true
fontdb.workspace = true
image.workspace = true
resvg.workspace = true
usvg.workspace = true
tiny-skia.workspace = true
encoding_rs.workspace = true
reqwest.workspace = true
reqwest_dav = { workspace = true, optional = true }
uuid.workspace = true
mockall = { workspace = true, optional = true }

[features]
default = ["webdav"]
webdav = ["dep:reqwest_dav"]
mock = ["dep:mockall"]
```

### 6.3 `crates/platform-windows/Cargo.toml`

```toml
[package]
name = "zerolaunch-platform-windows"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Windows platform implementations of zerolaunch-plugin-api traits."

[dependencies]
zerolaunch-plugin-api.workspace = true
async-trait.workspace = true
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
tracing.workspace = true
parking_lot.workspace = true
dashmap.workspace = true
once_cell.workspace = true
backtrace.workspace = true
widestring.workspace = true
scraper.workspace = true

[target.'cfg(target_os = "windows")'.dependencies]
windows = { workspace = true, features = [
    "Win32_UI_WindowsAndMessaging", "Win32_Foundation", "Win32_Graphics_Gdi",
    "Management_Deployment", "Win32_System_Threading", "Foundation",
    "Win32_Security", "Foundation_Collections", "System",
    "Win32_System_Com", "Win32_System_Com_StructuredStorage",
    "Win32_UI_Shell", "Storage_Streams", "Win32_UI_Shell_PropertiesSystem",
    "Win32_System_Variant", "Win32_System_Registry",
    "Win32_Storage_FileSystem", "Win32_System_Console",
    "Win32_System_Diagnostics", "Win32_System_Diagnostics_ToolHelp",
    "Win32_Graphics_Dwm", "Win32_UI_Controls", "Win32_Networking",
    "Win32_Networking_WinInet", "Win32_Globalization",
    "Win32_System_LibraryLoader", "Win32_System_Environment",
] }
windows-core.workspace = true
winreg.workspace = true
uiautomation.workspace = true
rdev.workspace = true
arboard.workspace = true
whoami.workspace = true
lnk.workspace = true

[target.'cfg(all(target_os = "windows", target_arch = "x86_64"))'.dependencies]
everything-rs = { workspace = true, optional = true }
everything-sys-bindgen = { workspace = true, optional = true }

[features]
default = ["everything"]
everything = ["dep:everything-rs", "dep:everything-sys-bindgen"]
```

### 6.4 `src-tauri/Cargo.toml`（变更摘要）

```toml
[package]
name = "zerolaunch-rs"
version.workspace = true
edition.workspace = true
authors.workspace = true
description = "🚀 Lightning-fast, accurate, lightweight & pure Windows application launcher!"

[lib]
name = "zerolaunch_rs_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "^2", features = [] }

[dependencies]
# ZeroLaunch crates
zerolaunch-plugin-api.workspace = true
zerolaunch-platform-windows.workspace = true

# Tauri
tauri = { version = "^2", features = ["tray-icon", "image-ico", "image-png"] }
tauri-plugin-global-shortcut = "^2.3.1"
tauri-plugin-dialog = "2"
tauri-plugin-notification = "^2"
tauri-plugin-deep-link = "2"
tauri-plugin-shell = "2"
tauri-utils = { version = "2.9.2", features = ["schema"] }

# 共用依赖（用 workspace）
serde.workspace = true
serde_json.workspace = true
async-trait.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tokio.workspace = true
parking_lot.workspace = true
dashmap.workspace = true
tracing.workspace = true

# 主程序专属
tracing-subscriber = { version = "^0.3.23", features = ["env-filter"] }
tracing-appender = "^0.2.5"
chrono.workspace = true
fuzzy-matcher = "0.3.7"
kmeans_colors = "^0.7.1"
palette = "^0.7.6"
rand = "^0.10.1"
notify = "8.2.0"
tempfile = "3.27.0"
bincode-next = "3.0.0-rc.14"
globset = "0.4.18"
time = "0.3.47"
timer = "^0.2.0"
lazy_static = "^1.5.0"
base64 = "^0.22.1"
uuid.workspace = true

# Step 12 时按需评估这些是否仍在 src-tauri 直接使用，否则移除
# fontdb / image / resvg / usvg / tiny-skia / encoding_rs / regex / reqwest

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
portable = []

[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]
tauri-plugin-single-instance = { version = "2", features = ["deep-link"] }
```

---

## 7. 关键文件位置速查

迁移过程中需重点关注的文件：

| 用途 | 路径 | 行号 / 备注 |
|------|------|-------------|
| HostApi 当前定义 | `src-tauri/src/sdk/host_api.rs` | 全文 1246 行 — Step 9 整体迁移 + build() 改造 |
| HostApi build() 平台守 | `src-tauri/src/sdk/host_api.rs` | :1168 — Step 9 去 `#[cfg]` |
| Plugin trait 定义 | `src-tauri/src/plugin_system/types.rs` | :411-429 — Step 10 迁移 + 改 init |
| 计算器插件 init | `src-tauri/src/plugin/triggerable/calculator_plugin.rs` | :111 — Step 10 改签名 |
| Configurable trait | `src-tauri/src/core/types/configurable.rs` | 全文 — Step 4 迁移 |
| sdk 总导出 | `src-tauri/src/sdk/mod.rs` | 45 行 — Step 7 改 re-export |
| Windows 实现 | `src-tauri/src/sdk/platform/windows/` | 14 个文件 — Step 8 整体迁出 |
| HostApi 构造点 | `src-tauri/src/lib.rs::build_app()` | grep 确定 — Step 9 改 build_windows_host_api_builder |
| Plugin init 唯一调用 | `src-tauri/src/plugin_system/service.rs::init_all` | 约 :39 — Step 10 改 register + handle 注入 |

---

## 8. 验证 / 回滚策略

### 8.1 每步增量验证
- 每个 Step 后 `cargo build` 通过
- Step 4 / 5 / 7 / 8 / 9 / 10 / 12 后 `cargo test --workspace` 通过
- Step 8 / 9 / 10 / 12 后 `cargo run` 启动 + §4 各 Step 中列出的手测清单全部通过

### 8.2 最终验收（Step 13 完成后）
- ✅ `cargo build --release --workspace` 通过
- ✅ `cargo test --workspace` 全部通过
- ✅ `cargo test --features mock -p zerolaunch-plugin-api` 通过
- ✅ `cargo clippy --workspace --all-targets` 无新增 warning
- ✅ Tauri 应用 release 包启动正常，全部 IPC 命令工作
- ✅ `crates/plugin-api/README.md` 中第三方插件示例（EchoPlugin）`cargo test` 通过
- ✅ 所有 `.claude/rules/*.md` 已更新

### 8.3 回滚策略
- 每个 Step 独立 commit；message 格式：`refactor(workspace): step N — <名字>`
- 任意 Step 失败时 `git reset --hard HEAD~1`（仅在执行者本地，**禁止** force push）
- `feature/workspace-split` 分支不合入 main 直到 Step 13 完成 + code review 通过

### 8.4 异常处理
- 如某 Step 验证失败但无明确原因 → **停下来报告**，记录：当前 Step、失败的命令、完整错误输出、最近 5 次 git log。**禁止**自行修补绕过。
- 如发现本计划与代码现状不符（例如某文件实际行号与文档不一致）→ **停下来报告**，由人工决定是否更新计划。

---

## 9. 关键决策回顾

| 决策 | 选择 | why |
|------|------|-----|
| crate 数量 | 3（plugin-api / platform-windows / src-tauri） | 4+ 强迫 ConfigManager 与组件注册中心也独立，触发循环依赖 |
| Configurable 归属 | plugin-api | `Plugin: Configurable`，必须同 crate；config 实现保留主程序 |
| HostApi 归属 | plugin-api（含 builder/handle） | 插件需要 PluginHandle 类型签名；HostApi 是 builder 的产物 |
| `build()` 错误处理 | panic → Result | 插件 mock 时频繁触发 builder，清晰错误更友好 |
| `windows()` 工厂归属 | platform-windows | plugin-api 不应"知道"任何平台名称 |
| init 签名 | `Arc<HostApi>` → `Arc<PluginHandle>` | rule 已要求；当前 init 实现都不使用参数，迁移代价 ~0 |
| mock 策略 | 手写 stub + 可选 mockall | mockall 与 async_trait 配合复杂；stub 90% 场景够用 |
| 内置插件归属 | 留主程序 src-tauri | 23 个内置组件已用 PluginHandle；它们是宿主"自带电池" |
| 迁移粒度 | 每能力域独立 step | 便于排错、回滚；每步都能 cargo build |
| webdav feature | plugin-api default 开启 | 保持现状不破坏宿主；未来发布 crates.io 再考虑 default 关闭 |
| BridgeError 归属 | Step 4 时 grep 决定 | 若 Configurable 不依赖 BridgeError，则留主程序 |
| 类型恒等迁移技巧 | 搬移后用 `pub use` 桥接 | 避免一次大爆炸式修改所有引用方 |

---

## 10. 里程碑 1 完成后的形态

```
zerolaunch-rs (src-tauri)
  ├── zerolaunch-plugin-api  ◄──── 第三方插件可仅依赖此 crate 完成开发与单测
  └── zerolaunch-platform-windows
        └── zerolaunch-plugin-api
```

**向第三方插件作者交付的核心承诺**：
> 只需 `cargo add zerolaunch-plugin-api`（或 path/git 依赖），写一个 `impl Plugin` 的 struct，`cargo test --features mock` 跑 query 单测，全程不需要 Tauri / Windows / 启动器源码。

**里程碑 2 才会解决**：动态加载（dlopen / wasm / IPC）、市场分发、前端插件 UI、跨平台、权限模型。本计划严格不涉及这些。

---

## 11. 执行者总结清单（开始执行前最后核对）

执行者在开始 Step 0 前**必须**确认以下条目：

- [ ] 已通读 §0 全部内容，理解通用陷阱与红线
- [ ] 已通读 §0.1 列出的 16 个文件，能回答 §0.1 末尾的 7 个问题
- [ ] 已理解 §0.2 的 5 个核心概念（特别是"类型恒等"技巧）
- [ ] 已理解 §3 的关键技术问题解法
- [ ] 已确认 cargo / git 工具可用，当前 main 分支 cargo build 通过
- [ ] 已创建 feature/workspace-split 分支
- [ ] 准备好遇到不确定时**停下来报告**，而不是自行决策

---
---
---

# 里程碑 2 指导性计划：内置插件开发体验改善

> **本节性质**：指导性方向，不是逐步操作手册。  
> **对执行者的要求**：可以在大方向内自由发挥，但需先与用户确认具体策略。  
> **里程碑 1 的关系**：里程碑 1 已完成（plugin-api crate 抽离 + mock）。本里程碑**不依赖**里程碑 1 中存疑的 P1 问题（HostApi 归属、build_windows_host_api_builder 位置），可独立推进。

---

## A. 战略背景与决策记录

### A.1 为什么不做完整的"第三方插件市场"

经过深入讨论，**第三方插件市场（动态加载 / 子进程 IPC / .zlplugin 包 / 插件商店）暂不实施**。理由：

1. **市场现实**：快捷启动器赛道，Wox + uTools 已占据"插件市场"生态位。新启动器拼"又一个插件市场"难赢
2. **维护债**：JSON-RPC 协议、子进程管理、iframe 隔离、CLI 工具、版本兼容承诺、安全模型——每项都是**长期**维护负担，不是一次性投入
3. **用户基数倒挂**：除非 ZL 先在快捷启动器赛道获得用户，否则做插件市场是给不存在的开发者搭桥
4. **API 契约不可逆**：第三方 API 一旦发布，破坏成本极高。在用户和开发者反馈不充分时定型，是错误时机

### A.2 里程碑 2 的真实目标

**改善 ZL 自己开发内置插件的体验**——这是当前最痛、ROI 最高的事。

具体痛点：
- 当前所有 23 个内置插件的注册都是 `lib.rs::init_plugin_system()` 中的硬编码（约 100 行 Arc::new + register）
- 新增一个数据源 / 执行器 / 关键词优化器，必须改 lib.rs
- 前端 panelType → Vue 组件的映射也是硬编码 import（`src-ui-new/composables/usePluginManager.ts`）
- 修改 Plugin trait 的 init / query 后，调试只能靠完整重启 ZL（cargo run）+ 手动操作复现
- 没有 Plugin Inspector：不知道某次 query 走到哪个插件、返回了什么、是否报错
- 后端日志混在一起，排查"是不是 calculator 插件出错"很麻烦

### A.3 里程碑 2 的非目标（明确边界）

**不做**：
- 任何动态加载机制（dlopen、cdylib、WASM）
- 任何 IPC 协议（JSON-RPC、WebSocket、stdio）
- 任何 .zlplugin 打包格式 / 插件市场 / 插件 CLI 工具
- 任何"对外 API 稳定承诺"——里程碑 2 的所有改动随时可变
- 任何前端 iframe 隔离 / 跨技术栈支持

**做**：
- 后端插件注册自动化（约定优于配置 / 宏自动发现）
- 前端插件注册自动化（`import.meta.glob`）
- Plugin Inspector 调试面板
- 内置插件目录约定与脚手架
- 文档：内置插件作者手册

---

## B. 必读上下文（执行者开始前的输入）

### B.1 必读文件

| 序 | 路径 | 为什么读 |
|----|------|---------|
| 1 | `D:\code\ZeroLaunch-rs\CLAUDE.md` | 项目顶层架构 |
| 2 | `D:\code\ZeroLaunch-rs\.claude\rules\general.md` | 通用纪律 |
| 3 | `D:\code\ZeroLaunch-rs\.claude\rules\plugin-system.md` | 插件系统规范 |
| 4 | `D:\code\ZeroLaunch-rs\.claude\rules\data-flow.md` | 插件生命周期与数据流 |
| 5 | `D:\code\ZeroLaunch-rs\src-tauri\src\lib.rs` 中 `init_plugin_system` 函数（约 :420-560） | 当前注册地——本里程碑核心改造对象 |
| 6 | `D:\code\ZeroLaunch-rs\src-tauri\src\plugin\` 整目录浏览 | 23 个内置插件的现状 |
| 7 | `D:\code\ZeroLaunch-rs\src-ui-new\composables\usePluginManager.ts` | 前端硬编码 import 的位置 |
| 8 | `D:\code\ZeroLaunch-rs\src-ui-new\plugins\manager.ts` | 前端 PluginManager 单例 |
| 9 | `D:\code\ZeroLaunch-rs\src-ui-new\plugins\types.ts` | FrontendPlugin 接口 4 个扩展点 |
| 10 | `D:\code\ZeroLaunch-rs\src-ui-new\plugins\built-in\calculator-panel\` 整目录 | 唯一现有前端插件样板 |
| 11 | `D:\code\ZeroLaunch-rs\crates\plugin-api\` 整 crate 浏览 | 里程碑 1 已交付的 SDK |

### B.2 关键事实（执行者必须知道）

1. **23 个内置组件分类**（在 src-tauri/src/plugin/ 下）：
   - 6 个 Executor、5 个 DataSource、8 个 KeywordOptimizer、3 个 SearchEngine、2 个 ScoreBooster、1 个 Plugin（calculator）+ 1 个 everything_plugin（如果存在）
   - 加上 4 个 core/config/components/ 中的 CoreConfigComponent
2. **前端已 70% 就绪**：`PluginManager.register()` 本身支持运行时注册，但 `loadBuiltinPlugins()` 是硬编码 import
3. **Plugin trait 实现仅 2 个**（calculator + everything），其余是其他组件 trait
4. **后端注册 = `Arc::new(SomeImpl::new()) → config_manager.register(...) → session_router.<sub>().register(...)`**，每个组件都重复这个三步
5. **前端 panelType 字符串需与后端 `QueryResponse::CustomPanel { panel_type }` 完全一致**
6. **配置系统**：每个插件都 `impl Configurable`，schema 通过 `setting_schema()` 暴露给前端动态渲染
7. **Vite HMR 已启用**：修改 Vue 组件**已**享受 HMR；修改 plugin manager 注册逻辑**不**享受（需手动刷新）
8. **Rust 不能热加载**：任何后端代码修改必然触发 ZL 整体重启（cargo run），这是物理约束，不要试图绕过

---

## C. 改造方向（4 个工作流，按优先级）

### 工作流 1：后端插件注册自动化（高 ROI）

**问题**：`lib.rs::init_plugin_system()` 中 100+ 行硬编码 Arc::new + register。新加一个 KeywordOptimizer 要改 lib.rs。

**目标**：让新插件作者**只需在 plugin/<category>/ 下加一个 .rs 文件，不需要改 lib.rs**。

**两条候选路径**（执行者需先与用户确认选哪条）：

#### 路径 1A：inventory crate（运行时收集）
- 用 `inventory` crate（或 `linkme`）的 distributed slice 机制
- 每个插件 .rs 文件末尾加一行 `inventory::submit!(MyPluginImpl::new());`
- lib.rs 中改为遍历 inventory 收集到的所有实例进行 register
- **优点**：纯库依赖，无需 build.rs；插件作者只加一行
- **缺点**：依赖 ctor 机制，部分平台/编译模式有坑（特别是测试场景）

#### 路径 1B：build.rs 代码生成
- build.rs 扫描 `plugin/` 目录，按约定（如每个文件 `pub fn register(...)` 函数）生成 `_generated_plugin_registry.rs`
- lib.rs 中 `include!("_generated_plugin_registry.rs")` + 调用生成的注册表
- **优点**：编译期可见，零运行时依赖
- **缺点**：build.rs 复杂度高；每次新增插件触发完整 rebuild

**推荐**：路径 1A（inventory），简单直接。但**先与用户确认**——如果 inventory 在 Tauri release build 中有问题，回退路径 1B。

**关键点**：
- 不论哪条路径，**插件类型分类**必须保留（DataSource / Executor / KeywordOptimizer / SearchEngine / ScoreBooster / Plugin），因为各自注册到不同的 registry
- 可以为每个类型定义独立的 inventory slot，或定义统一的 `RegisterableComponent` enum 包装
- **必须**保持现有 register 顺序对外可见（某些组件初始化有依赖）

### 工作流 2：前端插件注册自动化（最容易）

**问题**：`src-ui-new/composables/usePluginManager.ts` 硬编码 import 每个内置前端插件。

**目标**：用 `import.meta.glob` 自动扫描 `src-ui-new/plugins/built-in/*/index.ts`。

**改造**：
```typescript
// 改前
import { calculatorPanelPlugin } from '@/plugins/built-in/calculator-panel'
pluginManager.register(calculatorPanelPlugin)

// 改后
const modules = import.meta.glob<{ default: FrontendPlugin }>(
  '@/plugins/built-in/*/index.ts',
  { eager: true }
)
for (const [path, mod] of Object.entries(modules)) {
  if (mod.default) pluginManager.register(mod.default)
}
```

**Vite HMR 收益**：新加 `plugins/built-in/foo/index.ts` 立即被发现，无需修改任何注册代码。

**注意**：
- 需要约定每个内置插件**必须** export `default: FrontendPlugin`
- 需要约定目录结构：`built-in/<plugin-id>/index.ts` + `built-in/<plugin-id>/<Component>.vue`
- 加载顺序在 glob 模式下不确定，所以需要 `FrontendPlugin` 接口加 `priority: number` 字段（manager.ts 中已部分支持）
- 错误处理：单个插件 register 失败不应阻塞其他插件加载

### 工作流 3：Plugin Inspector（调试面板）

**问题**：当前调试插件全靠 `tracing` 日志，没有可视化的"插件运行时状态"。

**目标**：在 ZL 设置面板增加"开发者 / Plugin Inspector"标签页，展示：

1. **插件清单**：所有已加载的 Plugin / DataSource / Executor / SearchEngine / KeywordOptimizer / ScoreBooster，按类别分组
2. **每个插件的状态**：
   - 元数据（id、name、version、trigger_keywords）
   - Configurable schema 与当前 settings 值
   - 启用状态（enabled / disabled）
3. **最近 N 次 query 记录**（仅 Plugin 类型）：
   - 时间戳、trace_id、原始 raw_query
   - 命中的插件 id、QueryResponse 类型、耗时
   - 错误（如有）
4. **最近 N 次 execute_action 记录**：
   - action_id、payload、结果（Ok / Err）
5. **手动触发面板**：输入一段 raw_query，点击"模拟 query"，看到完整 trace

**实现思路**：
- 在 `plugin_system/` 下新增 `inspector.rs`，维护一个 ring buffer（最近 100 次记录）
- `SessionRouter::route_query` / `route_confirm` 入口埋点，记录到 ring buffer
- 新增 IPC 命令 `inspector_get_state` / `inspector_simulate_query`
- 前端新增 `views/PluginInspector.vue`
- **仅在 dev build 启用**（用 cargo feature `inspector` 守住，避免发版时拖慢）

**优先级**：中等。可与工作流 1/2 并行做。

### 工作流 4：内置插件目录约定 + 脚手架（最低优先级）

**问题**：新增内置插件没有标准模板，新人/未来的自己得参考 calculator 一路抄。

**目标**：
1. 写一份 `docs/dev/built-in-plugin-guide.md`：内置插件作者手册
   - 5 类组件的接口对比与选择决策
   - 完整示例：从 0 写一个新 KeywordOptimizer
   - 配置 schema 写法、前端面板写法（如需要）
   - 注册机制（依赖工作流 1 完成）
2. 在 plugin/ 下提供模板目录 `_template/`：
   - `_template/data_source.rs.tmpl`
   - `_template/executor.rs.tmpl`
   - `_template/keyword_optimizer.rs.tmpl`
   - 等等

**优先级**：低，但能持续受益。文档质量高于代码生成器。

---

## D. 推荐的执行顺序

执行者**先**读 §A、§B，**与用户确认**以下问题，**再**动手：

### D.1 必须先确认的决策

1. **工作流 1 选哪条路径**：inventory crate（路径 1A） vs build.rs 代码生成（路径 1B）？默认推荐 1A
2. **是否同时做 4 个工作流**：建议分 PR 推进，工作流 2 最简单可先做（仅前端，1-2 小时）
3. **Plugin Inspector 的范围**：是否只做"清单 + 最近 query 列表"作为 MVP？还是包含"手动 simulate query"？
4. **配置 / 文档变更**：本里程碑结束后是否更新 `.claude/rules/plugin-system.md`？是否更新 CLAUDE.md？

### D.2 建议的合入顺序

```
PR1：工作流 2（前端 import.meta.glob）
  - 体量小、风险低、立即受益
  - 验证：删一个内置插件的硬编码 import，确认仍能加载

PR2：工作流 1（后端注册自动化）
  - 先与用户确认路径 1A vs 1B
  - 一次只迁移一个组件类别（如先做 KeywordOptimizer 共 8 个）
  - 验证：每个内置插件功能保持不变

PR3：工作流 3（Plugin Inspector）
  - 仅 dev build 启用
  - 可分成两个子 PR：先做后端 ring buffer，再做前端面板

PR4：工作流 4（脚手架与文档）
  - 在 PR1-3 完成后做，因为模板需要反映新的注册方式
```

---

## E. 执行红线（与里程碑 1 一致）

- ❌ 禁止修改任何插件的业务逻辑（计算器、搜索算法、配置加载等）
- ❌ 禁止修改 plugin-api crate 的对外 API（trait 签名、struct 字段名）
- ❌ 禁止"顺手"重构无关代码
- ❌ 禁止引入对动态加载 / IPC / 子进程的支持（属于将来可能的里程碑 3）
- ❌ 禁止给 Plugin / DataSource / 等 trait 加新方法（如必要，先与用户讨论）
- ❌ 禁止让自动注册机制在 release build 中改变行为（dev/release 行为必须一致，仅 Inspector 例外）
- ✅ 允许：在 plugin/ 下新增辅助文件（如 `mod.rs` 中加 inventory 注册宏）
- ✅ 允许：修改 lib.rs::init_plugin_system，删除硬编码注册
- ✅ 允许：新增 cargo feature（如 `inspector`）

---

## F. 验收标准

**全部完成后**：
1. 在 plugin/ 下新加一个 KeywordOptimizer（如 `pinyin_first_letter.rs`），不需要改 lib.rs，重启 ZL 后该 KeywordOptimizer 自动注册
2. 在 plugins/built-in/ 下新加一个前端插件目录，不需要改 usePluginManager.ts，刷新页面后自动加载
3. 设置面板有 "Plugin Inspector" 标签页，显示所有已注册插件，至少能看到 calculator
4. `docs/dev/built-in-plugin-guide.md` 存在且至少包含 1 个完整示例
5. `cargo build --release --workspace` 通过
6. `cargo test --workspace` 全部通过
7. Tauri 应用启动正常，所有 23 个内置插件功能保持不变（手测：搜索、计算器、Ctrl+Enter 管理员启动等）

---

## G. 不在本里程碑范围（明确推迟）

以下事项是**将来可能的里程碑 3+** 范围。如果执行者发现自己在做这些，**立即停下**与用户讨论：

- 任何动态库加载（.dll / .so / .dylib）
- 任何 WASM 沙箱
- 任何子进程 + IPC 协议
- 任何 .zlplugin 打包格式
- 任何插件市场 / 商店
- 任何 CLI 工具（zl-cli、zlpkg 等）
- 任何 iframe 前端隔离
- 任何对"第三方 API 稳定"的承诺
- 任何 manifest.json 之外的元数据格式（PluginMetadata 已够用）
- 任何插件权限模型（permissions、capabilities 申请）
- 任何 macOS / Linux 平台支持

---

## H. 战略上的进一步建议

### H.1 何时重启第三方插件计划

里程碑 3（第三方插件支持）应当在以下条件**全部**满足时再启动：

1. ZL 在快捷启动器赛道有**真实用户基数**（不是开发者社群，是日常使用者）
2. 用户社区出现**真实的"我想给 ZL 加个 X 功能"请求**，且这些请求覆盖范围超出您的开发能力
3. 内置插件的开发已**驾轻就熟**，证明里程碑 2 的设计经过实战检验
4. 您愿意承担**至少 6 个月**的协议设计 + 接 issue 反馈的持续投入

如果上述条件不满足，**不要**做。维护一个无人使用的插件市场比没有插件市场更糟。

### H.2 当前最值得投入的精力（按 ROI 排序）

1. **里程碑 2 的工作流 2**（前端自动注册）— 1-2 小时，立即受益
2. **里程碑 2 的工作流 1**（后端自动注册）— 1-2 天，长期受益
3. **产品本身的核心体验**（搜索质量、UI 流畅度、稳定性）— **优先于一切插件相关工作**
4. **里程碑 2 的工作流 3**（Plugin Inspector）— 中等投入，长期收益
5. **里程碑 2 的工作流 4**（脚手架与文档）— 低投入，长期收益

**绝对不做**：里程碑 3 的任何技术预研。等到 H.1 的条件满足再说。


✅ 全部确认后从 Step 0 开始顺序执行。每完成一个 Step，更新本计划文档为"该 Step 已完成 ✅"作为标记。
