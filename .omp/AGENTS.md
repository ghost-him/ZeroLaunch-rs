# AGENTS.md — AI 快速导航

ZeroLaunch-rs 是 Windows 快捷启动器：`app/` 是 GPUI 宿主，`crates/zerolaunch-rs` 是不依赖 UI 框架的后端库。工作区没有 Tauri、Vue、Vite 或 Bun 构建链。

## 架构（Cargo Workspace）

```
app/                      → zerolaunch-gpui: GPUI 搜索窗、设置窗、托盘、深链接、插件 UI 脚本宿主
crates/plugin-api/        → zerolaunch-plugin-api: traits, 数据类型, HostApi/PluginHandle
crates/plugin-protocol/   → zerolaunch-plugin-protocol: JSON-RPC 协议定义, manifest schema
crates/plugin-host/       → zerolaunch-plugin-host: 子进程管理, RemotePluginAdapter, 双向 RPC
crates/plugin-sdk-rust/   → zerolaunch-plugin-sdk-rust: 第三方 Rust 插件 SDK
crates/platform-windows/  → zerolaunch-platform-windows: Windows 平台实现 + 工厂函数
crates/zerolaunch-rs/     → zerolaunch-rs: 内核、内置插件、配置、插件框架、CLI HTTP、托盘/窗口端口
zerolaunch-cli/           → zerolaunch-cli: 独立 CLI 工具 (zl query / zl plugins)
xtask/                    → 构建编排（cargo + Inno Setup + 便携 zip + wiki 快照）
```

**依赖方向**: `plugin-api ← plugin-protocol ← plugin-host ← zerolaunch-rs`、`plugin-api ← platform-windows ← zerolaunch-rs`、`plugin-api ← plugin-sdk-rust`、`app → {gpui-kit, gpui-shell, zerolaunch-rs, platform-windows, plugin-api}` — 禁止反向依赖。

## 关键文件

| 类别 | 路径 |
|---|---|
| GPUI 入口 | `app/src/main.rs` |
| 搜索窗 | `app/src/search/` |
| 设置窗 | `app/src/settings/` |
| 插件 UI 脚本宿主 | `app/src/shell.rs`（gpui-shell + `zerolaunch` HostModule） |
| GPUI Host 端口 | `app/src/host.rs` |
| 系统托盘 | `app/src/tray.rs`（独立消息循环 + tray-icon/muda） |
| 深链接 | `app/src/deeplink.rs`（注册 `zerolaunch-rs://` 并写入等待表） |
| 内核装配 | `crates/zerolaunch-rs/src/kernel.rs` |
| 进程级 Tokio runtime | `crates/zerolaunch-rs/src/runtime.rs`（CLI 与 GPUI 共用） |
| AppState | `crates/zerolaunch-rs/src/state/app_state.rs` |
| ConfigManager | `crates/zerolaunch-rs/src/core/config/manager.rs` |
| AI 助手 | `crates/zerolaunch-rs/src/core/agent/` |
| 内置插件 | `crates/zerolaunch-rs/src/builtin_plugin/` |
| SessionDispatcher | `crates/zerolaunch-rs/src/plugin_framework/session_dispatcher.rs` |
| 插件生命周期 | `crates/zerolaunch-rs/src/plugin_framework/plugin_manager.rs` |
| 插件市场 | `crates/zerolaunch-rs/src/plugin_market/mod.rs` |
| CLI HTTP 服务器 | `crates/zerolaunch-rs/src/cli_server/server.rs` |
| HostApi + Builder | `crates/zerolaunch-rs/src/sdk.rs` |
| 内置组件注册表 | `crates/zerolaunch-rs/src/plugin_framework/builtin_registry.rs` |
| Plugin Inspector | `crates/zerolaunch-rs/src/plugin_framework/inspector.rs` |
| 界面语言包 | `crates/zerolaunch-rs/locales/` |
| 图标资源 | `app/icons/` |
| 构建工具 | `xtask/src/main.rs`、`installer/zerolaunch.iss` |

## 后端模块（`crates/zerolaunch-rs/src/`）

```
kernel.rs           → 内核装配（HostApi/ConfigManager/PluginManager、后台任务）
runtime.rs          → 进程级 Tokio runtime
sdk.rs              → 平台抽象层 re-export 桥
core/               → ConfigManager, Configurable trait, AI 助手 agent, Host 端口
builtin_plugin/     → 内置插件实现
plugin_framework/   → SessionDispatcher, Registry, PluginManager, 插件 UI 投影
plugin_market/      → 插件市场（GitHub topic 仓库发现 / release 解析 / 下载）
cli_server/         → 本地 HTTP API 服务器（axum）
state/              → AppState
updater/            → 版本更新检查
window/             → 窗口位置工具函数（走 Host 端口）
utils/              → 通用工具
logging.rs          → 日志初始化
```

`app/` 直接调用后端库；没有 IPC 命令层。窗口、输入、渲染与用户交互归 `app/`，配置、插件、会话、持久化与平台调度归后端库。

## 目录结构与文件放置规范

```
ZeroLaunch-rs/
├── app/                                ← GPUI 宿主
│   ├── src/main.rs                     ← 应用启动、窗口、内核接线
│   ├── src/search/                     ← 搜索窗
│   ├── src/settings/                   ← 设置、插件管理、市场、关于、检查器
│   ├── src/shell.rs                    ← gpui-shell 脚本宿主
│   └── icons/                          ← 运行时图标资源
├── crates/
│   ├── plugin-api/                     ← traits、数据类型、HostApi/PluginHandle
│   ├── plugin-protocol/                ← JSON-RPC、manifest schema、codec
│   ├── plugin-host/                    ← 子进程与双向 RPC
│   ├── plugin-sdk-rust/                ← Rust 插件 SDK
│   ├── platform-windows/               ← Windows 平台实现
│   └── zerolaunch-rs/                  ← 后端库 + locales/ + assets/wiki/
├── zerolaunch-cli/                     ← 独立 CLI
├── installer/zerolaunch.iss            ← Inno Setup 安装包
└── xtask/                              ← 构建编排
```

### 新文件放置决策树

```
新增一个功能 →
├─ 需要平台 API？
│  └─ 是 → crates/plugin-api/src/services/ 定义 trait → platform-windows 实现 → HostApi/PluginHandle 暴露
├─ 是系统级配置（非插件）？
│  └─ 是 → crates/zerolaunch-rs/src/builtin_plugin/config/
├─ 是新的数据源/执行器/搜索引擎？
│  └─ 是 → crates/zerolaunch-rs/src/builtin_plugin/ 对应子目录
├─ 是窗口、面板、设置页或输入交互？
│  └─ 是 → app/src/ 对应视图
├─ 是插件 UI 脚本能力？
│  └─ 是 → app/src/shell.rs + HostModule
├─ 是构建/打包/发布逻辑？
│  └─ 是 → xtask/ 或 installer/
└─ 其他
   └─ 放 crates/zerolaunch-rs/src/utils/
```

### 内置插件与插件框架

- 每个内置插件实现 `Configurable`（配置）+ 对应领域 trait（`DataSource`、`ActionExecutor` 等），**必须**通过 `PluginHandle` 访问平台能力。
- 新增插件在对应目录添加 `.rs` + `inventory::submit!` 即自动注册，无需修改 `lib.rs`。
- `SessionDispatcher` 是运行时中枢；`plugin_manager.rs` 是第三方插件生命周期唯一入口。
- 禁止在 `plugin_framework/` 定义配置 schema 或持久化逻辑（属于 `core/`）。
- 新增界面文案写 `crates/zerolaunch-rs/locales/*.json`；插件文案由插件包提供。

## 编码设计规范

- **优雅与精简优先**：优先扩展现有抽象；在引入新模块、trait 或兼容层之前，先确认现有架构确实无法容纳需求。
- **不要过度设计**：小功能用最小实现；只有现有架构确实无法容纳需求时才扩展。
- **禁止为历史遗留加兼容层**：不为旧契约做 shim、别名、过渡字段；契约不匹配时直接修正。
- **注释只解释当前代码的作用**：描述行为、参数、返回、流程，不记录历史变更或备选方案。
- **单一权威**：每个事实只存在一次；派生值由消费方计算，不复制第二份。

## 构建与验证

```bash
# 开发运行
cargo run -p zerolaunch-gpui

# 全 workspace 检查
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

# 安装包 / 便携包（需要 Inno Setup 6 的 iscc）
cd xtask
cargo run --bin xtask build-installer --arch x64
cargo run --bin xtask build-portable --arch x64
```

构建前如提示缺少 wiki 快照，先运行 `cargo run --bin xtask sync-wiki`。便携包通过 `portable` feature 把数据目录解析到 exe 所在目录。

## 从哪里开始

- **AI 约束规则** → `.omp/RULES.md` + `.omp/rules/`（TTSR 条件规则）
- **第三方插件开发** → `crates/plugin-api/README.md`
- **构建与发布** → `xtask/README.md`、`installer/zerolaunch.iss`
- **可复用操作技能** → `.omp/skills/`

## CodeGraph

仓库根存在 `.codegraph/` 时，定位或理解代码优先用 CodeGraph：MCP `codegraph_explore`，或 shell `codegraph explore "<symbols or question>"` / `codegraph node <symbol-or-file>`。不要先用 grep/read 重建它已索引的信息。
