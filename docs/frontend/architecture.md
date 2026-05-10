# ZeroLaunch-rs 前端架构与前后端通信详解

## 目录

1. [整体架构概览](#1-整体架构概览)
2. [项目结构与关键文件](#2-项目结构与关键文件)
3. [入口点与窗口系统](#3-入口点与窗口系统)
4. [路由设计](#4-路由设计)
5. [状态管理 (Pinia Stores)](#5-状态管理-pinia-stores)
6. [组件树](#6-组件树)
7. [前端插件系统](#7-前端插件系统)
8. [前后端通信机制](#8-前后端通信机制)
9. [配置流转详解](#9-配置流转详解)
10. [搜索全链路详解](#10-搜索全链路详解)
11. [跨窗口状态同步](#11-跨窗口状态同步)
12. [构建与部署](#12-构建与部署)

---

## 1. 整体架构概览

ZeroLaunch-rs 是一个基于 **Tauri 2.x** 的 Windows 应用启动器。前端使用 **Vue 3 + TypeScript + Naive UI + Pinia**，后端使用 **Rust**，两者通过 Tauri 的 IPC 桥进行通信。

```
┌─────────────────────────────────────────────────────┐
│                    前端 (Vue 3)                       │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────┐  │
│  │   Views   │  │  Stores  │  │  Bridge Layer     │  │
│  │ (Search,  │──│ (Pinia)  │──│  commands.ts      │  │
│  │ Settings) │  │          │  │  events.ts        │  │
│  └──────────┘  └──────────┘  │  contract.ts      │  │
│                               └───────┬───────────┘  │
├───────────────────────────────────────┼──────────────┤
│              Tauri IPC Bridge         │ invoke()     │
│              (JSON/serde)             │ listen()     │
├───────────────────────────────────────┼──────────────┤
│                后端 (Rust)            │              │
│  ┌───────────────────────────────────▼────────────┐  │
│  │  commands/bridge.rs  │  commands/config_file.rs │  │
│  │  (7 个搜索/会话命令)   │  (8 个配置命令)          │  │
│  └──────────┬───────────┴──────────┬──────────────┘  │
│             │                      │                  │
│  ┌──────────▼──────────────────────▼──────────────┐  │
│  │            SessionRouter (中央路由器)            │  │
│  │  ┌──────────────┐  ┌────────────────────────┐  │  │
│  │  │ PluginService │  │  SearchPipeline        │  │  │
│  │  │ (插件触发匹配)  │  │  (搜索 + 评分 + 增强)   │  │  │
│  │  └──────────────┘  └────────────────────────┘  │  │
│  └────────────────────────────────────────────────┘  │
│                                                      │
│  ┌────────────────────────────────────────────────┐  │
│  │  ConfigManager (统一配置管理)                     │  │
│  │  注册/验证/持久化/广播                           │  │
│  └────────────────────────────────────────────────┘  │
│                                                      │
│  ┌────────────────────────────────────────────────┐  │
│  │  SDK / HostApi (平台抽象)                        │  │
│  │  图标/窗口/热键/Shell/剪贴板/应用枚举            │  │
│  └────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**核心原则**：后端负责所有业务逻辑（搜索、启动、配置），前端是"薄"展示层。前端不直接操作文件系统或启动程序，一切通过 IPC 命令委托给后端。

---

## 2. 项目结构与关键文件

前端代码全部位于 `src-ui-new/` 目录（旧的 `src-ui/` 已删除）。

```
src-ui-new/
├── main.ts                       # 主窗口入口
├── settings-main.ts              # 设置窗口入口
├── App.vue                       # 主窗口根组件
├── SettingsApp.vue               # 设置窗口根组件
│
├── bridge/                       # ★ IPC 桥接层 — 前后端通信核心
│   ├── contract.ts               # TypeScript 类型定义（与 Rust serde 对齐）
│   ├── commands.ts               # 所有 invoke() 调用 + 全局错误处理
│   ├── events.ts                 # 所有 Tauri 事件监听
│   └── index.ts                  # 统一导出
│
├── router/
│   └── index.ts                  # Vue Router（Hash 模式，2 条路由）
│
├── stores/                       # ★ Pinia 状态管理
│   ├── search-store.ts           # 搜索状态与会话管理
│   ├── config-store.ts           # 配置 CRUD 操作
│   ├── theme-store.ts            # 主题/语言管理
│   └── plugin-store.ts           # 前端插件注册表
│
├── composables/                  # Vue Composables
│   ├── useSearch.ts              # 输入处理（直接触发搜索）
│   ├── useKeyboard.ts            # 全局键盘导航
│   ├── useWindowResize.ts        # 窗口大小自适应
│   ├── useSettings.ts            # 设置窗口管理
│   └── usePluginManager.ts       # 内置前端插件加载
│
├── plugins/                      # ★ 前端插件系统
│   ├── types.ts                  # FrontendPlugin 接口
│   ├── manager.ts                # PluginManager 单例
│   └── built-in/
│       └── calculator-panel/     # 计算器面板插件
│           ├── index.ts
│           └── CalculatorPanel.vue
│
├── components/                   # Vue 组件
│   ├── common/                   # 通用组件
│   │   └── IconDisplay.vue       # 图标渲染器
│   ├── layout/                   # 布局组件
│   │   ├── WindowFrame.vue       # 无边框窗口框架（圆角）
│   │   ├── Footer.vue            # 状态栏 + 操作按钮
│   │   └── ContextMenu.vue       # 右键菜单
│   ├── search/
│   │   └── SearchBar.vue         # 搜索输入框
│   ├── results/
│   │   ├── ResultList.vue        # 结果列表
│   │   ├── ResultItem.vue        # 单条结果行（图标+标题+副标题）
│   │   └── ResultActions.vue     # 操作按钮（已迁至 Footer）
│   ├── panel/
│   │   ├── PluginPanelHost.vue   # 插件面板动态加载器
│   │   └── EmptyState.vue        # 空闲状态占位（已弃用，空闲态不再渲染）
│   └── settings/
│       ├── SettingsSidebar.vue   # 设置侧边栏
│       ├── DynamicForm.vue       # Schema 驱动的动态表单
│       ├── DynamicFormField.vue  # 单个字段渲染器
│       ├── ConfigActionButton.vue # 配置操作按钮
│       └── PluginSettingsHost.vue # 插件自定义设置加载器
│
├── views/
│   ├── SearchView.vue            # 搜索视图（组装所有搜索组件）
│   └── SettingsView.vue          # 设置视图
│
├── i18n/
│   ├── index.ts                  # Vue I18n 配置
│   └── locales/
│       ├── zh-Hans.json          # 简体中文
│       └── en.json               # 英文
│
└── styles/
    ├── variables.css             # CSS 变量（亮/暗主题）
    └── transitions.css           # 过渡动画
```

### 关键文件角色

| 文件                     | 角色                                                                              |
| ------------------------ | --------------------------------------------------------------------------------- |
| `bridge/commands.ts`     | 所有前端到后端的 IPC 调用集中地。每个函数封装一个 `invoke(cmd, args)`             |
| `bridge/contract.ts`     | 前后端契约类型。Rust 端 `#[serde(rename_all = "camelCase")]`，TS 端也用 camelCase |
| `bridge/events.ts`       | 后端推送事件的监听。`listen()` 订阅 `config-changed`、`config-error` 等事件       |
| `stores/search-store.ts` | 搜索核心状态机。管理 query → results → confirm 完整生命周期                       |
| `stores/config-store.ts` | 配置操作的 Pinia 封装。一一对应后端的 8 个 `config_*` 命令                        |

---

## 3. 入口点与窗口系统

项目有 **两个独立的 Tauri 窗口**，各自加载独立的 Vue 应用实例：

### 3.1 主窗口（搜索栏）

- **入口文件**：`index.html` → `main.ts` → `App.vue`
- **URL**：`tauri://localhost`（Tauri 自定义协议）
- **窗口属性**（在 `tauri.conf.json` 中配置）：
  - `decorations: false` — 无边框（由 `WindowFrame.vue` 自定义渲染窗口边框）
  - `transparent: true` — 透明背景
  - `alwaysOnTop: true` — 始终置顶
  - `skipTaskbar: true` — 不在任务栏显示
  - `resizable: false` — 大小由 `useWindowResize` composable 动态控制

**`main.ts` 启动流程**：

```
main.ts
  ├─ 1. 创建 Vue app (createApp)
  ├─ 2. 注册 Pinia + Router
  ├─ 3. ★ 挂载前：themeStore.loadFromBackend()
  │      └─ 调用 configGetSettings('appearance') 获取已保存的主题/语言
  │      └─ 目的：消除主题闪烁（flash of wrong theme）
  ├─ 4. app.mount('#app')
  └─ 5. 挂载后：usePluginManager().loadBuiltinPlugins()
         └─ 注册内置前端插件（如 calculator-panel）
```

### 3.2 设置窗口

- **入口文件**：`setting_window.html` → `settings-main.ts` → `SettingsApp.vue`
- **创建方式**：由 Rust 端 `init_setting_window()` 命令式创建 `WebviewWindow`
- **窗口属性**：950×500，默认隐藏，拦截 `CloseRequested` 事件改为隐藏而非关闭
- **无 Router**：设置窗口直接渲染 `<SettingsView />`，不经过 Vue Router

**`settings-main.ts` 启动流程**：

```
settings-main.ts
  ├─ 1. 创建 Vue app（无 Router）
  ├─ 2. 注册 Pinia
  ├─ 3. 挂载前：themeStore.loadFromBackend()（同上）
  └─ 4. app.mount('#app')
```

### 3.3 两个窗口的 Naive UI 提供者栈

`App.vue` 和 `SettingsApp.vue` 共享相同的 Naive UI 包装层：

```
NConfigProvider              ← 主题感知（naiveTheme）
  └─ NNotificationProvider   ← 通知容器
      └─ NMessageProvider    ← 消息提示
          └─ NDialogProvider ← 对话框容器
              └─ <router-view /> 或 <SettingsView />
```

两个窗口都监听 `config-changed` 事件（appearance 组件），以实现跨窗口主题同步。

---

## 4. 路由设计

定义在 `src-ui-new/router/index.ts`，使用 **Hash 模式**（`createWebHashHistory`）：

| 路径        | 组件               | 加载方式                   | 说明                           |
| ----------- | ------------------ | -------------------------- | ------------------------------ |
| `/`         | `SearchView.vue`   | 直接加载                   | 搜索栏主界面                   |
| `/settings` | `SettingsView.vue` | 懒加载 `() => import(...)` | 设置面板（仅主窗口路由中存在） |

实际上，设置功能主要由**独立的设置窗口**承载（`setting_window`），主窗口的 `/settings` 路由主要用于在主窗口中内嵌设置页面（按需）。

---

## 5. 状态管理 (Pinia Stores)

项目使用 4 个 Pinia Store，全部采用 Composition API 风格（`defineStore`）：

### 5.1 SearchStore（`search-store.ts`）

**核心状态机**，管理搜索栏的完整生命周期。

```typescript
// 状态
query: string                    // 当前输入
results: ListItem[]              // 搜索结果列表
selectedIndex: number            // 当前高亮项索引
selectedActionIndex: number      // 当前高亮的操作按钮索引
isSearching: boolean             // 已移除 — 查询时保留上一次结果，不显示加载状态
sessionMode: "none"|"search"|"plugin"  // 当前会话模式
cachedCount: number              // 缓存候选项数量
panelType: string | null         // 插件面板类型
panelData: unknown               // 插件面板数据
panelActions: ResultAction[]     // 插件面板操作按钮
keepSearchBar: boolean           // 是否保留搜索栏（插件沉浸模式 vs 面板模式）

// 计算属性
isIdle: boolean                  // 是否空闲（无输入、无插件）
selectedItem: ListItem | null    // 当前选中项

// 关键 Actions
doQuery(raw: string)             // ★ 搜索入口 → bridgeQuery()
doConfirm()                      // ★ 执行入口 → bridgeConfirm()
doWake()                         // 窗口显示 → bridgeWake()
doReset()                        // 窗口隐藏 → bridgeReset()
selectNext() / selectPrev()      // 键盘导航
refreshCandidates()              // 刷新缓存 → bridgeRefreshCandidates()
```

**doQuery 响应分派逻辑**：

```
bridgeQuery(rawQuery) 返回 BridgeQueryResponse
  ├─ mode = "search"       → 显示搜索结果列表，sessionMode = "search"
  ├─ mode = "empty"        → 显示"无结果"，sessionMode = "search"
  ├─ mode = "plugin_panel" → 显示插件面板 + 保留搜索栏，sessionMode = "plugin"
  └─ mode = "plugin_immersive" → 仅显示插件面板，隐藏搜索栏，sessionMode = "plugin"
```

### 5.2 ConfigStore（`config-store.ts`）

配置操作的 Pinia 封装，**一一对应后端的 8 个 `config_*` 命令**：

```typescript
// 状态
components: Record<string, ComponentInfo>  // 所有可配置组件
schemas: Record<string, ComponentSchema>   // Schema 缓存
settings: Record<string, any>              // 当前设置值缓存
isLoading: boolean
loadError: string | null

// Actions（每个对应一个后端 IPC 命令）
loadAllComponents()     → configGetAllComponents()
getSchema(id)           → configGetSchema(id)
getSettings(id)         → configGetSettings(id)
applySettings(id, val)  → configApplySettings(id, val)
resetSettings(id)       → configResetSettings(id)
setEnabled(id, bool)    → configSetEnabled(id, bool)
getActions(id)          → configGetActions(id)
executeAction(id, act)  → configExecuteAction(id, act)
```

### 5.3 ThemeStore（`theme-store.ts`）

管理暗/亮主题和语言切换：

```typescript
// 状态
themeMode: "system" | "light" | "dark"
systemIsDark: boolean           // 跟随系统时的实际值
naiveTheme: GlobalTheme | null  // Naive UI 主题对象
locale: "zh-Hans" | "en"

// 关键方法
loadFromBackend()               // ★ 挂载前调用。读 appearance 配置，消除主题闪烁
setTheme(mode)                  // 立即应用主题 + 防抖写回后端
applyRemoteSettings(settings)   // ★ 响应 config-changed 事件，跨窗口同步
```

**防闪烁机制**：在 `main.ts`/`settings-main.ts` 中，`loadFromBackend()` 在 Vue 挂载**之前**调用，确保 Naive UI 的 `NConfigProvider` 从一开始就拿到正确的主题对象。

**防死循环机制**：`applyRemoteSettings()` 仅在收到 `config-changed` 事件时调用，且**只读取不写回**，避免 A 窗口改 → B 窗口收到事件 → B 窗口写回 → A 窗口又收到事件 的死循环。

### 5.4 PluginStore（`plugin-store.ts`）

将 `PluginManager` 单例适配为响应式 Pinia store：

```typescript
registerPlugin(plugin)       // 注册前端插件
unregisterPlugin(id)         // 注销插件
getPanelComponent(type)      // 获取面板组件
getResultItemComponent(type) // 获取自定义结果项组件
getExtraActions(type)        // 获取额外操作按钮
getSettingsComponent(id)     // 获取插件设置组件
```

---

## 6. 组件树

### 6.1 主窗口组件树

```
App.vue
  NConfigProvider (主题感知)
    NNotificationProvider
      NMessageProvider
        NDialogProvider
          <router-view />
            SearchView.vue
              ├─ WindowFrame.vue               ← 无边框窗口框架（圆角）
              ├─ SearchBar.vue                 ← keepSearchBar 时显示
              ├─ ResultList.vue                ← search 模式且非空闲
              │   └─ ResultItem.vue × N        ← 每条搜索结果
              │       ├─ IconDisplay.vue       ← 图标渲染
              │       └─ ContextMenu.vue       ← 右键菜单
              ├─ PluginPanelHost.vue           ← 插件模式
              │   └─ <动态插件组件>             ← 如 CalculatorPanel.vue
              └─ Footer.vue                    ← 状态栏 + 操作按钮
```

### 6.2 设置窗口组件树

```
SettingsApp.vue
  NConfigProvider
    NNotificationProvider
      NMessageProvider
        NDialogProvider
          SettingsView.vue
            ├─ SettingsSidebar.vue        ← 可折叠分组侧边栏
            ├─ PluginSettingsHost.vue     ← 插件自定义设置 UI
            ├─ DynamicForm.vue            ← Schema 驱动的动态表单
            │   └─ DynamicFormField.vue × N
            │       └─ ConfigActionButton.vue ← 可选的操作按钮
```

---

## 7. 前端插件系统

前端有一套**独立于 Rust 插件系统**的前端插件机制，用于扩展 UI 层。

### 7.1 插件契约（`types.ts`）

```typescript
interface FrontendPlugin {
  id: string
  name: string
  version: string
  description?: string

  // 生命周期
  onInit?: () => void | Promise<void>
  onDestroy?: () => void | Promise<void>

  // 可选的能力提供者
  panelProvider?: PanelProvider       // 提供插件面板组件
  resultItemProvider?: ResultItemProvider   // 提供自定义结果项组件
  actionInjector?: ActionInjector     // 注入额外操作按钮
  settingsProvider?: SettingsProvider // 提供自定义设置组件
}
```

**四种 Provider 类型**：

| Provider             | 匹配条件                          | 作用                                     |
| -------------------- | --------------------------------- | ---------------------------------------- |
| `PanelProvider`      | 匹配后端返回的 `panelType` 字符串 | 渲染插件专属面板（如计算器）             |
| `ResultItemProvider` | 匹配 `targetType`                 | 使用自定义组件渲染特定类型的结果项       |
| `ActionInjector`     | 匹配 `targetType`                 | 为特定类型结果注入额外操作按钮           |
| `SettingsProvider`   | 匹配 `componentId`                | 替换 DynamicForm，显示插件自定义设置界面 |

### 7.2 PluginManager（`manager.ts`）

单例模式，内部使用 Map 存储各类型 Provider。ResultItemProvider 和 ActionInjector 按 **priority** 排序（数值越小优先级越高）。PluginStore 将其包装为响应式 store。

### 7.3 内置插件：CalculatorPanel

```typescript
// built-in/calculator-panel/index.ts
{
  id: 'calculator-panel',
  name: 'Calculator Panel',
  panelProvider: {
    panelType: 'calculator',        // 匹配 Rust CalculatorPlugin 返回的 panelType
    component: CalculatorPanel,     // Vue 组件
  }
}
```

当后端 CalculatorPlugin 返回 `QueryResponse::CustomPanel { panel_type: "calculator", ... }` 时，前端 `PluginPanelHost.vue` 通过 `pluginStore.getPanelComponent("calculator")` 获取 `CalculatorPanel.vue` 并动态渲染。

CalculatorPanel 显示表达式和计算结果。对于 `copy_result` 操作，它**在本地**通过 `navigator.clipboard` 复制，其他操作仍通过 `searchStore.doConfirm()` 委托后端。

---

## 8. 前后端通信机制

### 8.1 IPC 命令总览

前端通过 Tauri 的 `invoke(cmd, args)` 调用后端的 `#[tauri::command]` 函数。所有 IPC 调用集中封装在 `bridge/commands.ts` 中。

**共 15 个注册命令**（在 `lib.rs` 的 `generate_handler!` 中注册）：

```
搜索与会话（7 个）                    配置（8 个）
─────────────────────                ────────────────────
bridge_query                         config_get_all_components
bridge_confirm                       config_get_schema
bridge_wake                          config_get_settings
bridge_reset                         config_apply_settings
bridge_get_session_mode              config_reset_settings
bridge_refresh_candidates            config_set_enabled
bridge_get_candidates_count          config_get_actions
                                     config_execute_action
```

### 8.2 Bridge 层设计

前端 bridge 层按职责分离为三个文件：

#### `contract.ts` — 类型契约

前后端共享的类型定义。TS 端使用 camelCase，与 Rust 的 `#[serde(rename_all = "camelCase")]` 保持一致。

```typescript
// 核心类型
ListItem           // { id, title, subtitle, icon, score, actions, targetType }
ResultAction       // { id, label, icon, isDefault, shortcutKey }
BridgeQueryResponse // 可辨识联合：search | empty | plugin_panel | plugin_immersive
ConfirmPayload      // { candidateId, actionId, queryText, userArgs? }
ComponentInfo       // { componentId, componentName, componentType, enabled, defaultEnabled }
ComponentSchema     // 递归 schema 定义，用于 DynamicForm 渲染
```

#### `commands.ts` — IPC 调用封装

每个命令封装为一个类型安全的 async 函数，内部使用统一的 `invokeCommand<T>()` 辅助函数：

```typescript
// 内部辅助函数
async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)  // Tauri 原生 invoke
  } catch (e) {
    const error = tryExtractBridgeError(e)
    onError?.(error)  // 触发全局错误处理器
    throw error
  }
}

// 使用示例
export async function bridgeQuery(rawQuery: string): Promise<BridgeQueryResponse> {
  return invokeCommand('bridge_query', { rawQuery })
}

export async function configApplySettings(componentId: string, settings: unknown): Promise<void> {
  return invokeCommand('config_apply_settings', { componentId, settings })
}
```

**全局错误处理**：组件可通过 `registerErrorHandler()` 注册回调，错误时显示 Naive UI 通知。

#### `events.ts` — 事件监听

使用 Tauri 的 `listen()` API 订阅后端推送的事件：

```typescript
export function onConfigChanged(cb: (payload: ConfigChangedPayload) => void): Promise<UnlistenFn> {
  return listen<ConfigChangedPayload>('config-changed', (event) => cb(event.payload))
}

export function onConfigError(cb: (payload: ConfigErrorPayload) => void): Promise<UnlistenFn> {
  return listen<ConfigErrorPayload>('config-error', (event) => cb(event.payload))
}
```

**事件流向**：

```
后端 ConfigManager 应用设置
  → 验证通过
  → 广播 ConfigEvent::Changed { component_id, component_type }
  → lib.rs 中的广播接收器调用 sessionRouter.handle_config_event(...)
  → 前端 listen('config-changed') 回调触发
  → 各响应者：
      SearchView: 刷新候选项缓存
      SettingsView: 重新加载当前设置显示
      App.vue/SettingsApp.vue: 跨窗口主题同步
```

### 8.3 数据序列化

前后端通过 **JSON** 序列化通信，使用 **camelCase** 命名规范。

```
TypeScript 对象
  ↓ JSON.stringify (camelCase keys)
  ↓ Tauri invoke() IPC
  ↓ serde_json::from_str + #[serde(rename_all = "camelCase")]
Rust 结构体

Rust 结构体
  ↓ serde_json::to_value + #[serde(rename_all = "camelCase")]
  ↓ Tauri IPC 返回值
  ↓ JSON.parse (camelCase keys)
TypeScript 对象
```

---

## 9. 配置流转详解

配置系统是前后端通信最复杂的链路之一，涉及校验、持久化、事件广播和跨窗口同步。

### 9.1 完整配置变更流程

```
用户在设置窗口修改 "program-source" 的配置
  │
  ▼
[前端] SettingsView.vue
  │  用户在 DynamicForm 中修改字段值
  │  点击 "应用" 按钮
  ▼
[前端] configStore.applySettings("program-source", { ... })
  │
  ▼
[前端] bridge/commands.ts → invoke('config_apply_settings', { componentId, settings })
  │
  ═══════════ Tauri IPC ═══════════
  │
  ▼
[后端] commands/config_file.rs::config_apply_settings()
  │  从 AppState 获取 ConfigManager
  │  config_manager.apply_settings(&component_id, settings_value)
  ▼
[后端] ConfigManager::apply_settings()
  │  ├─ 1. 查找组件 (ComponentRegistry)
  │  ├─ 2. 验证设置 (Configurable::validate_settings)
  │  ├─ 3. 应用设置 (Configurable::apply_settings)
  │  ├─ 4. 持久化到磁盘 (local JSON + 可选 WebDAV 同步)
  │  └─ 5. ★ 广播 ConfigEvent::Changed
  │        通过 broadcast channel 发送
  ▼
[后端] lib.rs 中的广播接收器 (spawn 的 tokio task)
  │  收到 ConfigEvent
  │  ├─ 如果是 DataSource/KeywordOptimizer 变更
  │  │   → session_router.refresh_candidates() 刷新候选缓存
  │  ├─ 如果是 SearchEngine/ScoreBooster 变更
  │  │   → session_router.rebuild_search_pipeline() 重建搜索管道
  │  └─ 通过 Tauri 事件发射到前端
  │      app_handle.emit("config-changed", payload)
  │
  ═══════════ Tauri Event ═══════════
  │
  ▼
[前端] 多个监听者收到 "config-changed" 事件
  │
  ├─ SearchView.vue
  │   └─ 检查 componentType 是否影响候选数据
  │       ├─ DataSource → searchStore.refreshCandidates()
  │       └─ KeywordOptimizer → searchStore.refreshCandidates()
  │
  ├─ SettingsView.vue
  │   └─ 如果是当前正在编辑的组件
  │       └─ 重新加载设置显示 configStore.getSettings()
  │
  ├─ App.vue / SettingsApp.vue
  │   └─ 如果 componentId === "appearance"
  │       └─ themeStore.applyRemoteSettings() 跨窗口同步主题
  │
  └─ (结束)
```

### 9.2 DynamicForm — Schema 驱动的设置 UI

设置界面**不硬编码任何表单字段**，而是完全由后端返回的 JSON Schema 动态渲染：

```
configGetSchema("program-source") 返回:
{
  componentId: "program-source",
  fields: [
    {
      key: "paths",
      label: "扫描路径",
      type: "Array",
      arrayHint: "Tags",       // UI 渲染提示
      defaultValue: [],
      description: "要扫描的程序文件夹路径"
    },
    {
      key: "extensions",
      label: "文件扩展名",
      type: "Array",
      arrayHint: "Default",
      defaultValue: [".exe", ".lnk"]
    },
    {
      key: "recursive",
      label: "递归扫描",
      type: "Boolean",
      defaultValue: true
    }
  ]
}
```

**DynamicFormField.vue** 根据 `type` 字段选择对应的输入组件：

| type      | 组件                                        |
| --------- | ------------------------------------------- |
| `Text`    | `NInput`                                    |
| `Number`  | `NInputNumber`                              |
| `Boolean` | `NSwitch`                                   |
| `Select`  | `NSelect`                                   |
| `Path`    | `NInput` + `ConfigActionButton`（浏览按钮） |
| `Color`   | `NColorPicker`                              |
| `Json`    | `NInput` (type=textarea)                    |
| `Array`   | 根据 `arrayHint` 选择不同 UI                |

**Array 的 4 种 UI 变体**：

| arrayHint      | UI 形式                             |
| -------------- | ----------------------------------- |
| `Default`      | 每行一个输入框，可添加/删除行       |
| `Table`        | 表格形式，列由 item schema 决定     |
| `MasterDetail` | 左侧列表 + 右侧详情面板             |
| `Tags`         | Naive UI 的 `NDynamicTags` 标签输入 |

---

## 10. 搜索全链路详解

### 10.1 用户输入到结果显示

这是最核心的数据流，涉及 IPC 调用、后端查询路由、插件触发匹配、搜索管道和结果回传。

```
用户在搜索栏输入 "fire"
  │
  ▼
[前端] SearchBar.vue
  │  @update:value → useSearch.handleInput("fire")
  ▼
[前端] useSearch.ts
  │  handleInput("fire") → searchStore.doQuery("fire")
  ▼
  │  isSearching = true
  │  selectedIndex = 0
  ▼
[前端] bridgeQuery("fire")
  │  invoke('bridge_query', { rawQuery: "fire" })
  │
  ═══════════ Tauri IPC ═══════════
  │
  ▼
[后端] bridge.rs::bridge_query(state, raw_query: "fire")
  │  创建 Query { id, raw_query: "fire", search_term: "fire" }
  │  session_router.route_query(&trace_id, &query)
  ▼
[后端] SessionRouter::route_query()
  │
  ├─ 步骤 1: 尝试插件匹配
  │   plugin_service.query()
  │   → QueryDispatcher::dispatch_plugin()
  │   → PluginRegistry::parse_trigger("fire")
  │     │  按第一个空格拆分："fire" 没有空格
  │     │  检查 "fire" 是否匹配已注册的触发词
  │     │  CalculatorPlugin 触发词是 "="
  │     │  "fire" ≠ "=" → 无匹配
  │     └─ 返回 None
  │
  ├─ 步骤 2: 无插件匹配，走搜索模式
  │   session_mode = Search
  │
  ├─ 步骤 3: 搜索管道
  │   └─ SearchPipeline::search(cached_candidates, "fire")
  │       ├─ SearchEngine::calculate_scores()
  │       │   StandardSearchModel 对每个候选计算匹配分数
  │       │   (支持拼音、模糊匹配等)
  │       └─ ScoreBooster::boost()
  │           ├─ HistoryBooster: 提升历史常用项
  │           └─ QueryAffinityBooster: 提升与查询相关的项
  │       └─ 返回 ranked_results: Vec<ListItem>
  │
  └─ 步骤 4: 构建响应
      QueryResponse::List { results: ranked_results }
      ↓ 转换为 BridgeQueryResponse { mode: "search", results: [...] }
  │
  ═══════════ Tauri IPC ═══════════
  │
  ▼
[前端] searchStore 收到 BridgeQueryResponse
  │  mode = "search"
  │  results = [...] (ListItem 数组)
  │  isSearching = false
  ▼
[前端] ResultList.vue 渲染结果列表
  │  每个 ResultItem.vue 显示 icon, title, subtitle, actions
  │  useWindowResize 根据结果数量动态调整窗口大小
  ▼
用户看到 "fire" 的搜索结果
```

### 10.2 插件触发模式

当用户输入**触发词**（如计算器 `=`）时，流程不同：

```
用户输入 "= 2+2"
  │
  ▼
[后端] QueryDispatcher::dispatch_plugin()
  │  parse_trigger("= 2+2")
  │  → 拆分为 trigger="=", search_term="2+2"
  │  → "=" 匹配 CalculatorPlugin
  ▼
[后端] CalculatorPlugin::query(ctx, "2+2")
  │  计算表达式 "2+2" = 4
  │  返回 QueryResponse::CustomPanel {
  │    panel_type: "calculator",
  │    data: { expression: "2+2", result: 4 },
  │    actions: [{ id: "copy_result", label: "复制结果" }]
  │  }
  ▼
[前端] searchStore 收到 BridgeQueryResponse
  │  mode = "plugin_panel"
  │  panelType = "calculator"
  │  panelData = { expression: "2+2", result: 4 }
  ▼
[前端] PluginPanelHost.vue
  │  pluginStore.getPanelComponent("calculator")
  │  → 返回 CalculatorPanel.vue
  │  → <CalculatorPanel :data="panelData" :actions="panelActions" />
  ▼
用户看到计算器面板，显示 "2+2 = 4"
```

### 10.3 确认/执行流程

```
用户选中 "Firefox" 按 Enter
  │
  ▼
[前端] useKeyboard.ts
  │  Enter 键 → searchStore.doConfirm()
  │  构建 ConfirmPayload {
  │    candidateId: "firefox",
  │    actionId: "launch",
  │    queryText: "fire"
  │  }
  ▼
[前端] bridgeConfirm(payload)
  │  invoke('bridge_confirm', { payload })
  │
  ═══════════ Tauri IPC ═══════════
  │
  ▼
[后端] SessionRouter::route_confirm()
  │
  ├─ session_mode = Search
  │   1. 从 payload 提取 candidate_id = "firefox"
  │   2. 在 cached_candidates 中查找 SearchCandidate
  │   3. 构建 ExecutionContext {
  │        target, display_name,
  │        user_args,
  │        parameter_snapshot  ← 来自 bridge_wake 时捕获的快照
  │      }
  │   4. ExecutorRegistry::resolve(exec_ctx, "launch")
  │      ├─ 查找 (targetType="App", action_id="launch") → AppExecutor
  │      └─ AppExecutor::execute(exec_ctx, "launch") → 启动 Firefox
  │   5. 若 ActivationFailed → ExecutorRegistry::resolve_fallback() → 重试执行
  │   6. 记录选择（用于 HistoryBooster 学习）
  │
  └─ 成功 → 返回 Ok(())
  │
  ═══════════ Tauri IPC ═══════════
  │
  ▼
[前端] doConfirm 完成
  │  getCurrentWindow().hide()  // 隐藏搜索窗口
  │  doReset()                  // 重置会话状态
```

### 10.4 Wake 流程（参数快照）

搜索栏显示时，捕获系统参数快照，供后续执行时使用：

```
用户按热键显示搜索栏
  │
  ▼
[前端] searchStore.doWake()
  │  invoke('bridge_wake')
  │
  ═══════════ Tauri IPC ═══════════
  │
  ▼
[后端] SessionRouter::on_search_bar_wake()
  │  HostApi::capture_parameter_snapshot()
  │  捕获：
  │  ├─ 剪贴板文本
  │  ├─ 前台窗口句柄 (HWND)
  │  └─ 选中文本（如果当前窗口支持）
  │  存入 session_router.parameter_snapshot
  │
  └─ 快照在 bridge_confirm 时被注入 ExecutionContext
     下游 executor 可使用这些参数（如粘贴剪贴板内容）
```

---

## 11. 跨窗口状态同步

主窗口和设置窗口是**两个独立的 WebView**，各有自己的 Vue 实例和 Pinia store。它们通过 **后端事件广播** 保持同步。

### 11.1 主题同步

```
[主窗口] 用户切换主题为 "dark"
  │  themeStore.setTheme("dark")
  │  ├─ 立即应用 naiveTheme（本地即时响应）
  │  └─ 防抖写回：configApplySettings("appearance", { theme: "dark", ... })
  │
  ════════ Tauri IPC ════════
  ↓
[后端] ConfigManager 应用设置 → 广播 ConfigEvent
  ↓
  ════════ Tauri Event ════════
  ↓
[主窗口] App.vue 收到 config-changed (componentId="appearance")
  │  themeStore.applyRemoteSettings(settings)
  │  ★ 只读取不写回，避免死循环
  ↓
[设置窗口] SettingsApp.vue 也收到同一事件
  │  themeStore.applyRemoteSettings(settings)
  │  ★ 同样只读取不写回
  ↓
两个窗口的主题同步完成
```

### 11.2 配置同步

```
[设置窗口] 用户添加了一个程序扫描路径
  │  configStore.applySettings("program-source", { paths: [...] })
  ════════ Tauri IPC ════════
  ↓
[后端] ConfigManager 应用设置 → 广播 ConfigEvent
  ↓
  ════════ Tauri Event ════════
  ↓
[主窗口] SearchView.vue 收到 config-changed
  │  componentType = "DataSource"
  │  → searchStore.refreshCandidates()
  │  → 搜索结果在下一次搜索时反映新路径
```

---

## 12. 构建与部署

### 12.1 Vite 配置

`vite.config.ts` 关键设置：

```typescript
{
  plugins: [
    vue(),
    copyI18nPlugin()  // 将 i18n JSON 复制到 src-tauri/locales/ 供 Rust 使用
  ],
  resolve: {
    alias: { '@': 'src-ui-new/' }
  },
  server: {
    port: 12345,       // 固定端口（Tauri 需要）
    strictPort: true,
    hmr: { port: 1421 }
  },
  build: {
    rollupOptions: {
      input: {
        main: 'index.html',                    // 主窗口
        setting_window: 'setting_window.html'  // 设置窗口
      }
    }
  }
}
```

### 12.2 Tauri 构建集成

在 `tauri.conf.json` 中：

```json
{
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:12345",
    "beforeDevCommand": "bun run dev",
    "beforeBuildCommand": "bun run build"
  }
}
```

### 12.3 构建产物

Vite 构建后在 `dist/` 生成两个入口 HTML：
- `index.html` → 主窗口加载
- `setting_window.html` → 设置窗口加载

Tauri 的 `frontendDist` 指向 `dist/`，打包时将前端资源嵌入二进制文件。

---

## 附录：后端注册的 15 个 IPC 命令

### 搜索与会话（7 个）

| 前端函数                     | 后端命令                      | 功能                       |
| ---------------------------- | ----------------------------- | -------------------------- |
| `bridgeQuery(rawQuery)`      | `bridge_query`                | 核心搜索/查询分发          |
| `bridgeConfirm(payload)`     | `bridge_confirm`              | 执行选中操作               |
| `bridgeWake()`               | `bridge_wake`                 | 窗口显示，捕获系统参数快照 |
| `bridgeReset()`              | `bridge_reset`                | 窗口隐藏，重置会话         |
| `bridgeGetSessionMode()`     | `bridge_get_session_mode`     | 获取会话模式               |
| `bridgeRefreshCandidates()`  | `bridge_refresh_candidates`   | 强制刷新候选项缓存         |
| `bridgeGetCandidatesCount()` | `bridge_get_candidates_count` | 获取缓存候选项数量         |

### 配置（8 个）

| 前端函数                       | 后端命令                    | 功能                |
| ------------------------------ | --------------------------- | ------------------- |
| `configGetAllComponents()`     | `config_get_all_components` | 列出所有可配置组件  |
| `configGetSchema(id)`          | `config_get_schema`         | 获取组件设置 Schema |
| `configGetSettings(id)`        | `config_get_settings`       | 获取组件当前设置值  |
| `configApplySettings(id, val)` | `config_apply_settings`     | 应用新设置          |
| `configResetSettings(id)`      | `config_reset_settings`     | 重置为默认值        |
| `configSetEnabled(id, bool)`   | `config_set_enabled`        | 启用/禁用组件       |
| `configGetActions(id)`         | `config_get_actions`        | 获取可用配置操作    |
| `configExecuteAction(id, act)` | `config_execute_action`     | 执行配置操作        |

### 事件（3 个，后端 → 前端）

| 监听函数                  | Tauri 事件名         | 载荷                             |
| ------------------------- | -------------------- | -------------------------------- |
| `onConfigChanged(cb)`     | `config-changed`     | `{ componentId, componentType }` |
| `onConfigError(cb)`       | `config-error`       | `{ componentId, error }`         |
| `onInstallationEvent(cb)` | `installation-event` | `{ eventType, appName }`         |
