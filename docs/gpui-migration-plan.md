# ZeroLaunch-rs → GPUI Kit 迁移计划

> 状态：阶段 5 完成，阶段 6 待开始
> 基线版本：1.5.5
> 适用分支：迁移分支（`main` 保持 Tauri 可发布，见「迁移与发布策略」）

## 1. 目标与范围

把 ZeroLaunch-rs 从 **Tauri 2 + Vue 3** 重构为 **GPUI Kit 原生应用**，第三方插件 UI 改用 **gpui-shell**（QuickJS 脚本，GPUI 渲染，无 WebView）。

约束：

- 重构不是原样复刻：GPUI 能直接提供的能力，删除为此自建的框架代码。
- 不复用 Tauri 的兼容层；旧代码在对应阶段直接删除，不留 shim。
- 行为等价以旧版可观测结果为准：热键唤起、搜索、执行、配置持久化、插件加载、托盘、更新检查。

不做：

- 不保留 WebView / ESM / Shadow DOM 插件面板通道。
- 不保留 `src-ui/` 作为长期前端。
- 不为第三方插件提供 Rust 组件跨进程直调（见 4.4）。

## 2. 复用判断（已核对源码）

| 部分 | 结论 | 依据 |
|---|---|---|
| `crates/plugin-api`、`plugin-protocol`、`plugin-host`、`plugin-sdk-rust` | 基本原样保留 | 仅 `plugin-host/icon.rs` 使用 `tauri::async_runtime::spawn_blocking` |
| `builtin_plugin/`、`core/`、`cli_server/`、`plugin_market/`、`window/`、`utils/` | 原样保留 | 业务代码几乎零 `tauri` 引用 |
| `SessionDispatcher`（3713 行） | 复用约 90% | 仅 3 处 `tauri::async_runtime::spawn`，无 `AppHandle` |
| `commands/`（约 10k 行） | 大部分删除 | IPC 薄代理 + 前端 DTO 转换，GPUI 内直接调用 dispatcher |
| `platform-windows` | 重写外壳 | `hotkey.rs`、`focus_monitor.rs`、`platform_services.rs`、`icon.rs`、`path_resolver.rs` 绑定 Tauri |
| `src-ui/`（15.7k 行 Vue） | 重写后删除 | GPUI 原生渲染 |

保留边界：

- `plugin-api` 不依赖 GPUI。
- 插件后端组件继续走「子进程 + stdio JSON-RPC」，与 UI 脚本（宿主进程内 QuickJS）是两套模型。
- 统一使用一个全局 Tokio runtime 承载 axum、插件 RPC、网络与阻塞任务；GPUI 事件循环不与 tokio 线程混用。

## 3. 配置 Schema 系统的处理

宿主侧 UI 可以直接由 GPUI 取代，数据契约保留：

- **删除**：`WidgetHint` 作为前端渲染协议的角色、`DynamicForm.vue` 及其 fields 组件、`config_get_schema` IPC、前端 config store。内置组件改为各自实现 GPUI 设置视图。
- **保留**：`SettingDefinition` / `SchemaKind` 作为配置数据契约，继续负责默认值、校验、持久化、AI 说明书检索、第三方插件跨进程 schema。
- **插件设置页**：优先由插件用 gpui-shell 自绘；无 UI 的简单插件保留极简 schema 兜底表单（text / number / toggle / select / path），不实现 masterDetail、searchTable 等复杂控件。
- `Configurable` trait 不进 GPUI 类型；宿主侧用「组件 id → 视图构造器」注册表承接设置页。

## 4. 关键架构决策

1. **迁移分支**：迁移在独立分支进行，`main` 保持 Tauri 可发布；不做双宿主兼容层。阶段 1 后允许 `src-tauri` 暂时退出 workspace，阶段 7 删除。
2. **crate 布局**：新增 `app/` 作为 GPUI 二进制（窗口、视图、命令）；后端业务保留为库 crate，阶段 7 将 `src-tauri` 的非 UI 部分收敛为独立库并删除 Tauri 外壳。
3. **异步模型**：启动时创建全局 Tokio runtime；`SessionDispatcher` 的 `tauri::async_runtime::spawn` 全部替换为该 runtime；UI 更新经 Entity/通道回到 GPUI 线程。
4. **插件 UI**：gpui-shell 脚本运行在宿主进程内，默认拒绝所有能力，由宿主按插件授予；宿主通过 `HostModule` 暴露会话查询、确认执行、设置读写、i18n、资源访问。
5. **宿主组件复用**：脚本不能直接 import Rust 组件。宿主需要发布一个 shell package（JS 组件库，Git 依赖）供插件 `import`；HostModule 只传纯数据。
6. **系统能力**：gpui-kit 无内置托盘与全局热键，继续由 `platform-windows` 用 Win32 实现（`Shell_NotifyIcon`、`RegisterHotKey` + `RIDEV_INPUTSINK`）。

## 5. 阶段表

| 阶段 | 目标 | 主要交付 | 删除 / 替换 | 通过标准 |
|---|---|---|---|---|
| 0 | 技术验证 | 新建 `app/` 骨架（gpui-kit + gpui-shell）；验证透明无边框窗、置顶/跳过任务栏、失焦隐藏、中文 IME、多显示器 DPI、全局热键与双击 Ctrl、JS 面板 + HostModule | 无 | 热键唤起透明搜索窗，中文输入可用；JS 面板能渲染并回调宿主函数；启动/内存有基线 |
| 1 | 平台层去 Tauri | 重写 `platform-windows` 的热键、焦点监控、窗口控制、单实例；去掉 `AppHandle` | `tauri-plugin-global-shortcut`、Tauri 窗口事件 | 新宿主能注册/注销热键、正确失焦隐藏、单实例生效 |
| 2 | 内核接线 | GPUI 生命周期装配 `AppState`；全局 Tokio runtime；CLI 服务器、插件管理、配置管理、dispatcher 直接接线；事件改 Entity/通知 | `commands/` 的 IPC 包装、前端 DTO/事件发射层 | 无窗口即可完成查询、配置读写、插件加载；`zl query` 可用 |
| 3 | 搜索主窗口 | 输入、候选列表、图标异步回填、键盘导航、动作栏、参数面板、排除选择器、面板容器、窗口定位、游戏模式 | Vue 搜索相关组件 | 热键 → 输入 → 选择 → 执行全链路与旧版一致 |
| 4 | 设置与配置 UI | 设置壳（分类侧栏 + 搜索）、各内置组件的 GPUI 设置视图、插件管理、市场、关于、调试/检查器 | `DynamicForm.vue`、schema 驱动的前端表单、对应 IPC | 所有内置组件可配置并持久化；AI 说明书仍能检索 schema |
| 4.5 | 设置功能补齐 | schema data/effect action；系统字体列表；快捷键录制；图片预览；设置项搜索定位到字段 | 临时手动输入/无预览的兜底交互 | 依赖配置动作的设置项可在 GPUI 内完成；字段输入体验与旧版对齐 |
| 5 | 插件体系（gpui-shell） | 插件面板/设置页改为 JS 脚本；定义 HostModule；宿主刷新脚本视图；manifest `[ui]` 改造；插件设置入口与兜底表单；市场预检/缓存/覆盖确认与安装流程适配；检查器适配 | `zlplugin://` 协议、ESM/Shadow DOM 面板宿主 | 安装示例插件后，面板与设置页在 GPUI 内渲染并完成查询/执行 |
| 6 | 周边功能 | 托盘菜单、深链接、更新检查、AI 助手、i18n（复用 `locales/`）、日志导出、通知、剪贴板、About | Tauri 托盘/深链接插件 | 对照旧版功能清单逐项通过 |
| 7 | 拆除与发布 | 删除 `src-ui/`、Tauri 依赖与配置、Vite/Bun；workspace 收敛；视觉与窗口行为 parity；Inno Setup 安装包 + 便携 zip；CI、文档、wiki | 全部 Tauri 相关 | 干净构建、安装/升级/卸载、便携包、回归清单全绿 |

## 6. 各阶段执行要点

### 阶段 0 — 技术验证

- 建 `app/`：`gpui_kit::init`、`open_window`、`Root`、资源与图标。
- 逐项做真机验证并记录结果，任何一项不达标先解决再进入阶段 1：
  - 透明 + 无边框 + 不显示任务栏 + 置顶；
  - 失焦自动隐藏与再次唤起；
  - 中文 IME 输入、候选词、光标；
  - 多显示器 / 不同 DPI 下的窗口定位；
  - 全局热键和双击 Ctrl（原始输入）；
  - gpui-shell 加载一个 JS View，HostModule 双向调用。
- 记录启动时间、空闲内存、首次搜索结果延迟，作为后续对比基线。

### 阶段 1 — 平台层去 Tauri

- `hotkey.rs`：改为 Win32 注册与注销，保留双击 Ctrl 的原始输入实现。
- `focus_monitor.rs`：改为平台窗口焦点回调，不再依赖 `tauri::WindowEvent`。
- `platform_services.rs`：按端口组装平台能力，构造函数不再接收 `AppHandle`。
- `icon.rs`、`path_resolver.rs`：`tauri::async_runtime::spawn_blocking` 换成全局 Tokio。
- 单实例：命名互斥量 + 激活已有窗口。
- 文件对话框、打开路径改用 GPUI / 平台原生能力，替换 `tauri-plugin-dialog`、`tauri-plugin-shell`。

### 阶段 2 — 内核接线

- `bootstrap.rs` 拆分：可复用的装配逻辑迁入库，Tauri 生命周期代码删除。
- `AppState` 移除 `AppHandle` 字段。
- 事件：前端 `emit` 改为 Entity 通知或内部通道；仅保留必要的事件总线。
- `commands/` 按职责拆分：留在业务层的逻辑迁入对应模块，纯 IPC/DTO 代码删除。
- 验证：无 UI 的集成测试能完成一次查询与一次确认执行；CLI HTTP 端点正常。

### 阶段 3 — 搜索主窗口

- 输入框、结果列表、图标异步回填、键盘上下选择、Enter 执行、Esc 隐藏。
- 参数面板、排除选择器、第三方插件面板容器。
- 窗口定位、重置会话、失焦隐藏、游戏模式。
- 校验：与旧版同一组输入，比较候选、排序、图标、动作列表。

### 阶段 4 — 设置与配置 UI

- 设置壳：分类侧栏、页面路由、设置项搜索。
- 内置组件设置页：按组件类型实现 GPUI 视图，直接读写强类型设置。
- 配置事件与持久化沿用 `ConfigManager`；保存/重置/即时生效语义与旧版一致。
- 插件管理、插件市场、关于、调试/检查器。
- 删除 `DynamicForm.vue` 链路，确认没有代码再读 `WidgetHint`。

### 阶段 4.5 — 设置功能补齐

- schema data action：模型列表拉取、候选搜索、检测类字段的数据注入与合并。
- schema effect action：连接测试、图标覆盖应用等用户显式动作。
- 字体字段接入系统字体列表；快捷键字段接入录制与修饰键校验。
- 图片字段接入预览、清除与上传后的即时回显。
- 设置项搜索支持从字段结果直接定位到组件内字段。
- 通过标准：依赖配置动作的设置项不再需要手动拼值；字段输入体验与旧版对齐。

### 阶段 5 — 插件体系

- 插件 manifest `[ui]` 指向 gpui-shell 入口；插件包内提供 `main.js`。
- HostModule 规划（名称待定，建议 `zerolaunch`）：
  - 会话：当前候选、查询、确认执行、隐藏窗口；
  - 设置：读取/写入插件设置、触发 config action；
  - 文案：按插件 id 翻译；
  - 资源：受限的文件读取。
- 数据更新：宿主在会话变化时调用 `runtime.refresh`，脚本在 `render` 中拉取快照。
- 卸载插件时取消其任务、释放脚本 View。
- 插件管理页为每个第三方插件提供设置入口：有 `main.js` 时打开脚本设置页，无 UI 时使用 text/number/toggle/select/path 兜底表单。
- 删除 `zlplugin_protocol.rs` 与 `ThirdPartyPanelHost.vue`；插件市场下载/安装流程适配新包结构，并补磁盘缓存、安装预检与覆盖确认。
- 发布宿主 JS 组件库（shell package），供插件复用宿主已设计好的界面。

### 阶段 6 — 周边功能

- 托盘：图标、菜单项、状态回显、退出清理。
- 深链接：注册 `zerolaunch-rs://`，与单实例配合。
- 更新检查：沿用 GitHub Releases 查询逻辑，托盘与设置页展示。
- AI 助手：复用 `core/agent`，设置说明书继续由 schema 投影。
- i18n：复用 `locales/*.json`，宿主文案走 Rust i18n；插件文案走 HostModule。
- 日志导出、系统通知、剪贴板、About。

### 阶段 7 — 拆除与发布

- 删除 `src-ui/`、`index.html`、`setting_window.html`、`vite.config.ts`、`tsconfig*`、`package.json`、`bun.lock`、`node_modules/`。
- 删除 `src-tauri/tauri.conf*.json`、`capabilities/`、`build.rs` 的 `tauri-build`、Tauri 依赖与插件。
- workspace 收敛：`app/`、`crates/plugin-*`、`crates/platform-windows`、`zerolaunch-cli`、后端库 crate。
- 视觉与窗口 parity：间距、字号、圆角、图标、hover/动画、设置窗拖动位置记忆；大列表按需虚拟化。
- 打包：Inno Setup 安装包 + 便携 zip；同步更新 CI、签名、NOTICE、README、wiki。
- 全量回归：安装、升级、卸载、便携运行、热键、托盘、插件安装与卸载。

## 7. 迁移与发布策略

- 迁移分支与 `main` 分离；`main` 继续发 Tauri 版本，直到阶段 7 完成。
- 每个阶段结束时迁移分支必须 `cargo check` 通过（允许中间提交不可编译，但阶段收尾必须恢复）。
- 不引入「Tauri/GPUI 双实现」的长期抽象；过渡期代码在阶段 7 一并删除。
- 每阶段对照旧版做一次手动回归，记录差异；无法等价的行为在阶段验收时明确列出。

## 8. 风险与对策

| 风险 | 影响 | 对策 |
|---|---|---|
| GPUI 在 Windows 的透明窗口 / IME / DPI 行为不达标 | 搜索窗核心体验受损 | 阶段 0 实测；不达标先解决或调整窗口方案 |
| gpui-shell 仍为 M0 实验接口 | API 变动导致返工 | 锁定版本；宿主侧封装一层薄适配，不把 API 散落到业务代码 |
| gpui-kit 无托盘 / 全局热键 | 需要自写 Win32 | 继续由 `platform-windows` 承担，接口保持宿主无关 |
| 插件 UI 与后端组件两套模型 | 文档与开发体验分裂 | 在插件开发文档中明确：UI 用 gpui-shell，后端组件用 JSON-RPC |
| 脚本无法直接使用 Rust 组件 | 插件无法复用宿主 UI | 发布 JS 组件库 shell package；HostModule 只传数据 |
| MSI 改 Inno Setup | 发布流程重写 | 阶段 7 单独验证安装/升级/卸载 |

## 9. 阶段验收记录模板

每阶段完成后在本文件追加：

```
### 阶段 N 验收记录（YYYY-MM-DD）

- 提交范围：
- 通过项：
- 未通过 / 差异项：
- 临时绕过（必须在阶段 7 前删除）：
- 下一阶段前置条件：
```

### 阶段 1 验收记录（2026-11-10）

- 提交范围：
  - `crates/platform-windows`：`hotkey` 改为 `global-hotkey` + 专用消息线程；`focus_monitor` 改为 `SetWinEventHook` 前台事件钩子；`icon` 改 `tokio::task::spawn_blocking`；新增 `single_instance`（命名互斥量 + 隐藏窗口 + `WM_COPYDATA`）；平台工厂去掉 `AppHandle`，改收主窗口句柄。
  - `src-tauri`：`bootstrap` 传主窗口句柄；`lib.rs` 单实例改用平台实现；主窗口 `CloseRequested` 复用失焦回调；移除 `tauri-plugin-global-shortcut` / `tauri-plugin-single-instance` 依赖与 capability 权限。
- 通过项：
  - `cargo check --workspace` 零错误。
  - `cargo test -p zerolaunch-platform-windows`：13 passed。
  - 冒烟：`Ctrl+Alt+F24` 注册后合成按键触发回调；两个 Win32 窗口切换前台触发 `Lost`；双进程单实例通知写标记文件成功。
- 未通过 / 差异项：
  - 未启动完整 Tauri 应用做 UI 级回归；窗口隐藏/显示链路需阶段 2 接线后验证。
  - 焦点监控不再处理 `CloseRequested`，改由宿主显式复用失焦回调；行为等价但归属变化。
- 临时绕过：无（冒烟示例已删除）。
- 规则同步：
  - `.omp/rules/no-tokio-spawn-in-core.md`：增加 GPUI 迁移边界，新宿主使用全局 Tokio runtime。
  - `.omp/rules/schema-driven-ui.md`：限定当前 Vue 前端，GPUI 设置页改为按组件实现。
- 下一阶段前置条件：阶段 2 建立全局 Tokio runtime，并把 `bootstrap` / `AppState` 从 Tauri 生命周期中拆出。

### 阶段 2 验收记录（2026-11-10）

- 提交范围：
  - 新增 `runtime.rs`（进程级 Tokio runtime）与 `core/host.rs`（Host 端口、`NoopHost`、`UpdateMenuState`）；`state/tauri_host.rs` 把窗口、托盘、通知与事件映射到 Tauri。
  - `bootstrap.rs` 缩为 Tauri 外壳适配（安装 Host、平台端口、托盘）；新增 `kernel.rs` 承载 HostApi / ConfigManager / PluginManager / CLI / AppCommand / 后台任务装配，整个内核不再引用 Tauri 类型。
  - `AppState` 移除 `AppHandle` 与 `TrayManager` 字段，改为持有 `Arc<dyn Host>`；`plugin_manager`、`host_handler`、`updater`、`window`、`bridge`/`plugin`/`plugin_market` 命令与 `cli_server` 改经 Host 端口或全局 runtime。
  - 会话状态、配置变更、候选预热、检查器、图标回填、插件安装/卸载、系统主题全部改为 Host 语义方法；前端仅通过 `TauriHost` 的适配层消费旧事件名。
- 通过项：
  - `cargo check --workspace` 零错误。
  - `cargo test --workspace`：271 + 6 + 5 + 1 passed（含插件框架、历史持久化、Host API 暴露审计）。
  - 新增 `tests/kernel_headless.rs`：无窗口完成内核装配、计算器查询、插件动作确认，并通过 CLI `GET /v1/ping` 验证 bearer token 端点。
- 未通过 / 差异项：
  - `commands/` 仍保留 Tauri IPC 参数/返回 DTO 外壳；业务逻辑本就在 `SessionDispatcher` / `ConfigManager` / `PluginManager` 中，未为删除纯壳而破坏当前 Tauri 可编译壳。纯壳随阶段 3-4 的 GPUI 视图接线与阶段 7 的 Tauri 拆除一并删除。
  - `TauriHost` 仍持有 WebView 窗口与托盘，这是外壳适配的预期边界；GPUI 宿主实现同一 `Host` trait 即可复用内核。
  - 未启动完整 Tauri 应用做 UI 级回归；阶段 2 的通过标准是无窗口内核链路，窗口交互留给阶段 3。
- 临时绕过：无。
- 规则同步：
  - `.omp/rules/no-tokio-spawn-in-core.md`：由「统一 `tauri::async_runtime`」改为「统一 `crate::runtime` 全局入口」。
  - `.omp/rules/no-appstate-global.md`：移除「从 Tauri `AppHandle` 获取」表述，改为状态注入与 `Host` 端口。
- 下一阶段前置条件：搜索主窗口直接调用 `SessionDispatcher` / Host 端口，输入、候选、执行链路不再经过 Tauri IPC 命令。

### 阶段 3 验收记录（2026-11-10）

- 提交范围：
  - 新增 `app/` 二进制（`zerolaunch-gpui`）：GPUI 搜索窗、Win32 窗口适配、Host 实现、搜索状态机与渲染。
  - `app/src/host.rs`：`GpuiHost` 实现 `Host`，窗口显示/隐藏/置顶/跳过任务栏/位置/显示器信息走 Win32；内核事件经广播通道进入 GPUI 线程。
  - `app/src/search/`：输入框、候选列表（图标异步回填）、上下选择、Enter 执行、Esc 隐藏/回退、Tab 动作切换、Ctrl+1..9、行内参数、参数面板、排除选择器、插件面板容器。
  - `src-tauri/src/kernel.rs`：公开 `toggle_search_bar`，新增 `on_search_bar_focus_lost` 统一失焦保存位置/隐藏/会话复位。
- 通过项：
  - `cargo check --workspace` 零错误；`cargo build -p zerolaunch-gpui` 成功；`cargo test --workspace` 通过。
  - 真机：`Alt+Space` 唤起 815×79 搜索窗；输入 `= 1+1` 显示计算器面板与 `1+1=2`，Enter 执行后窗口隐藏且 `route_confirm` 成功。
  - 真机：输入 `note` 命中 10 个候选，图标、排序、选中态、动作栏（打开/唤醒窗口 Shift+Enter）正常；连续两次 Esc 隐藏窗口。
  - 内核异步调用改为投递到全局 Tokio runtime 后回传 GPUI 线程，消除 `no reactor running` panic。
- 未通过 / 差异项：
  - 设置窗口、托盘、深链路与更新 UI 未接入 GPUI，仍留阶段 4/6；`Host::show_settings_window` 当前为空实现。
  - 插件面板为宿主通用容器：calculator/translator 有轻量渲染，其他面板显示数据摘要；gpui-shell 脚本渲染留阶段 5。
  - 外观仍用 gpui-kit 默认主题，未读取 `appearance-config` 颜色/背景图；窗口拖动记忆未实现。
  - 搜索列表未虚拟化，按当前结果上限直接渲染。
- 临时绕过（必须在阶段 7 前删除）：`app/` 暂依赖 `zerolaunch-rs` 库（其中仍含 Tauri 外壳）；阶段 7 将非 UI 部分收敛为独立库。
- 规则同步：`no-tokio-spawn-in-core` 扩展到 `app/src/`，补充 GPUI 前台任务经 oneshot 投递到 `zerolaunch_rs_lib::runtime` 的边界。
- 下一阶段前置条件：设置壳与内置组件 GPUI 视图接入 `ConfigManager`，保留 schema 数据契约。

### 阶段 4 验收记录（2026-11-10）

- 提交范围：
  - 新增 `app/src/settings/`：分类侧栏、设置项搜索、schema 字段渲染、保存/丢弃/重置/即时保存、插件管理、市场、关于、检查器。
  - 新增 `app/build.rs`：把内置语言包复制到可执行文件目录，保证宿主启动即可加载文案。
  - `app/src/main.rs`、`host.rs`、`platform.rs`：设置窗口创建/隐藏/标题/物理 bounds、`Ctrl+,` 入口、固定可见起点与显示器缩放适配。
  - `src-tauri/src/core/host.rs`：`Host` 增加设置窗口隐藏钩子。
  - 三种语言包补齐设置页文案；`schema-driven-ui` 规则改为 GPUI 共用 schema 渲染器。
- 通过项：
  - `cargo check --workspace` 零错误；`cargo build -p zerolaunch-gpui` 成功；`cargo test --workspace` 通过。
  - 真机：`Ctrl+,` 从搜索窗打开设置窗，窗口完整落在屏幕内；分类、组件页签与表单字段渲染正常。
  - 真机：滚轮可滚动设置内容，滚动条由 `overflow_y_scrollbar` 提供。
  - 真机：调试页可读取检查器快照（45 个已注册组件）；设置窗启动/切换无 `RefCell already borrowed` 错误。
- 子 agent 漏改审查发现并修正：
  - 初始设置窗标题现在由 GPUI 启动时按当前语言写入，不再依赖语言变更回调。
  - 路径选择回调改为更新已有输入框，不再在渲染/选取回调中重新创建并订阅输入实体。
  - 数字校验与市场取消原因改为语言包文案，消除硬编码英文状态。
- 未通过 / 差异项：
  - Vue `DynamicForm.vue` 链路保留到阶段 7，避免当前 Tauri 外壳不可编译；GPUI 已不依赖它。
  - schema 的 data/effect action、系统字体列表、快捷键录制、图片预览未接入；字段可手动编辑，图片会先上传为资源 id。
  - 市场使用实时 GitHub 请求，未接入列表/元数据磁盘缓存与安装预检弹窗；安装/覆盖安装已接线。
  - 关于页未接入更新检查与日志导出（阶段 6）；调试页仅接入检查器，未接入搜索耗时工具。
  - 设置窗固定在屏幕左上偏移位置，未实现窗口拖动位置记忆；第三方插件设置页留阶段 5。
- 补做归属：
  - schema data/effect action、字体列表、快捷键录制、图片预览 → 阶段 4.5。
  - 插件设置入口与兜底表单、市场缓存/预检/覆盖确认 → 阶段 5。
  - 托盘、更新、日志导出、完整 i18n → 阶段 6。
  - 视觉 parity、窗口位置记忆、列表虚拟化 → 阶段 7。
- 临时绕过（必须在阶段 7 前删除）：`app/` 暂依赖 `zerolaunch-rs` 库（其中仍含 Tauri 外壳）。
- 规则同步：`schema-driven-ui` 的 GPUI 边界改为共用 schema 渲染器；`app/` 宿主说明更新。
- 下一阶段前置条件：进入阶段 4.5，先补 schema data/effect action 与字段输入体验。

### 阶段 4.5 验收记录（2026-11-10）

- 提交范围：
  - `app/src/settings/view.rs`：data/effect action 执行与结果写回、动态选项、系统字体缓存、图片预览缓存、快捷键录制、搜索表格查询/添加。
  - `app/src/settings/render.rs`：字段动作按钮、数组动作按钮位置、字体下拉、快捷键按钮、图片缩略图、搜索表格查询区。
  - `src-ui/i18n/locales/*.json`：补充 `clear`、`noOptions`、`entryExists` 文案。
- 通过项：
  - `cargo check --workspace` 零错误；`cargo clippy -p zerolaunch-gpui --all-targets` 零警告；`cargo build -p zerolaunch-gpui` 成功；`cargo test --workspace` 通过。
  - 真机：外观页字体下拉可展开系统字体列表并写回字段。
  - 真机：快捷键字段点击进入录制态，`Ctrl+Alt+K` 正确写回；保存按钮随脏状态启用。
  - 真机：书签数据源「自动检测浏览器」执行完成，结果按 `bookmarks_path` 合并，未产生重复条目。
  - 真机：设置项搜索「快捷键」「浏览器」「偏移」均能直接定位到组件字段；搜索表格显示查询输入与空态。
- 未验证 / 差异项：
  - 搜索表格当前为查询、添加、删除与卡片内字段编辑，未复刻旧版独立编辑弹窗。
  - 图片字段接入资源预览、清除与上传回显，未实现点击放大。
  - effect action 的连接测试需要真实模型服务，本次未触发网络请求。
- 子 agent 审查发现并修正：
  - 搜索表格添加条目和 data action 合并新条目后，补建新增数组项输入框，避免只读文本。
  - 图片资源加载失败后记录尝试标记，避免渲染循环重复请求。
  - effect 结果与字体“跟随系统”清除后同步已缓存的输入/下拉状态。
  - 快捷键录制在输入框聚焦、保存、丢弃、重置和页面切换时取消；补齐 Shift 符号键映射。
- 下一阶段前置条件：插件 manifest `[ui]` 与 gpui-shell 脚本宿主接入，替代当前插件面板/设置页容器。

### 阶段 5 验收记录（2026-11-10）

- 提交范围：
  - 新增 `app/src/shell.rs`：隔离 `ShellRuntime`、`zerolaunch` HostModule（会话/查询/确认动作/隐藏/设置读写/配置动作执行/翻译/受限资源读取）、脚本入口解析与路径边界校验。
  - `app/src/search/`：第三方面板加载 gpui-shell 视图，面板数据变化时 `ScriptView::refresh`，脚本动作回投 `SearchView`，卸载时释放视图。
  - `app/src/settings/`：插件管理页增加设置入口；有 `[ui].settingsEntry` 时打开脚本设置页，无 UI 时回退到组件 schema 表单；市场安装改为下载预检、确认覆盖后再安装。
  - `app/build.rs`：主线程栈提升到 16 MB，避免 QuickJS 运行时构造在 Windows 默认 1 MB 栈上溢出。
  - `crates/plugin-protocol`、`.omp` 规则与 `AGENTS.md`：`[ui]` 语义改为 gpui-shell 脚本；删除 `zlplugin://` 协议及 WebView 第三方面板宿主。
- 通过项：
  - `cargo check --workspace` 零错误；`cargo clippy -p zerolaunch-gpui --all-targets` 零警告；`cargo build -p zerolaunch-gpui` 成功；`CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace` 通过（大体积测试 PDB 会触发链接器 PDB/磁盘限制）。
  - 真机：临时示例插件（Node 后端 + gpui-shell 面板）被宿主加载，输入 `demo` 选中候选后，面板由脚本渲染并读取到 `session().pluginId`、面板 data 与查询文本。
  - 真机：面板脚本按钮可回调 HostModule 的 `hide`。
  - 真机：设置窗可打开插件管理页，第三方插件条目出现“设置”入口。
- 未验证 / 差异项：
  - 脚本设置页的导航点击未完成端到端验证；设置页脚本加载与渲染复用同一 `PluginUiHost::load_view` 路径。
  - 市场预检/覆盖确认已接线，未在真机完成一次真实 GitHub 安装。
  - 示例插件后端只实现最小 RPC，`validate_settings` 返回空数组，导致示例插件配置组件注册失败；该问题属于测试桩，不影响面板脚本验证。
  - `zlplugin://` 与 WebView 第三方面板删除后，旧 Tauri 前端的第三方插件 UI 不再可用，等待阶段 7 删除 `src-ui/`。
- 子 agent 审查发现并修正：
  - 市场包解析失败时清理暂存文件，避免坏包永久命中缓存；开始新预检前清理旧暂存包。
  - HostModule 增加 `settingsAction`，并让设置快照携带 `configActions`。
  - 市场预检失败提示改用 `previewFailed` 并保留错误原因；空 `settingsEntry` 回退到 schema 表单。
  - 从插件脚本设置页跳转到组件时释放脚本视图并清除设置桥授权。
  - 面板脚本加载失败/插件卸载文案接入三语言 i18n。
- 临时绕过（必须在阶段 7 前删除）：`app/` 暂依赖 `zerolaunch-rs` 库（其中仍含 Tauri 外壳）。
- 规则同步：
  - `third-party-plugin`：`manager.rs` 更正为 `plugin_manager.rs`，`zlplugin://` 段替换为 gpui-shell HostModule 契约。
  - `plugin-system`、`sdk-init-tkey-timing`、`deep-selector`、`AGENTS.md`：清理失效路径与 Shadow DOM/zlplugin 描述。

### 阶段 6 验收记录（2026-11-13）

- 状态：功能完成；暂停阶段 7，先评估迁移收益。
- 提交范围：
  - 新增 `app/src/tray.rs`：独立托盘线程与 tray-icon/muda 菜单，Host 端口同步可见性、语言、游戏模式与更新状态；退出前复用内核清理。
  - 新增 `app/src/deeplink.rs`：按当前用户注册 `zerolaunch-rs://`，首实例与第二实例的 URL 统一写入内核等待表。
  - 设置窗新增 AI 助手页，直接调用 `core/agent`；关于页接入更新检查与日志导出。
  - `app/Cargo.toml` 增加托盘、保存对话框、URL 与注册表依赖；第二实例收到深链接后退出。
- 通过项：
  - `cargo check -p zerolaunch-gpui` 零错误；`cargo clippy -p zerolaunch-gpui --all-targets` 零警告；`cargo build -p zerolaunch-gpui` 成功；`CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace` 通过。
  - 真机：托盘图标创建成功；深链接注册命令指向当前 exe，第二实例退出且主实例记录 URL 参数。
  - 真机：关于页显示最新版本状态；选择保存路径后导出 zip，压缩包包含 `logs/` 日志。
  - 真机：AI 助手页提问后收到模型答复。
- 未验证 / 差异项：
  - 托盘菜单项点击、双击唤出与退出清理未通过真实鼠标点击验证；事件已接入 `AppCommand` 消费者。
  - 「查看发布页」按钮与系统通知/剪贴板沿用既有 Host/PluginHandle 路径，未在本次重新验证。
- 子 agent 审查修正：
  - 深链接缺少域名时只记录 scheme，不再把带查询参数的完整 URL 写入可导出的日志。
  - 助手输入支持 Enter 发送；说明书请求补齐设置页路径与触发词，历史限制为最近 20 条。
  - 托盘图标加载失败后保留图标路径，用户重新开启托盘时按需重建。
- 结论：暂停阶段 7。先以同机基准比较 Tauri 与 GPUI 的冷启动、空闲内存、输入到首结果延迟、包体与插件 UI 能力；数据不支持再继续时保留 Tauri 主线。
