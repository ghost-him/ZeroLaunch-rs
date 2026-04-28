# ZeroLaunch-rs 新前端需求文档

> **文档状态**: Code Review 后修订版 (v2)  
> **项目名称**: ZeroLaunch-rs  
> **技术栈**: Tauri 2.x (Rust 后端) + Vue 3 (TypeScript 新前端) + Naive UI  
> **目标平台**: Windows (x86_64 / aarch64)  
> **修订日期**: 2026-04-27

---

## 0. 关于旧前端

**`src-ui/` 目录下的旧前端将被完全删除。** 旧前端基于旧版后端架构（`commands/` 模块中的每个功能对应一个 Tauri Command），与新系统的三层架构（SDK → Core → PluginSystem）不兼容。

旧前端仅作为功能参考和 API 调用模式参考保留在新前端开发期间，**不应被修改或增量重构**。

### 新旧对比

| 维度     | 旧前端                             | 新前端                           |
| -------- | ---------------------------------- | -------------------------------- |
| 通信方式 | 每个功能写一个命令（50+ Commands） | 通过通用桥接层（~14 个命令）     |
| UI 框架  | Element Plus                       | Naive UI                         |
| 状态管理 | Pinia + 分散式状态                 | Pinia + 集中式状态 + Schema 驱动 |
| 插件化   | 不支持                             | 支持前端插件注册机制（含第三方） |
| 后端架构 | 单体命令层                         | PluginSystem + ConfigManager     |
| 配置管理 | 手动映射配置项                     | Schema 驱动动态表单 + 配置动作   |

---

## 1. UI 框架选型分析

### 推荐结论：**Naive UI**

理由：

1. **零 CSS 引入** — Tauri 窗口本身就是无头窗口，不需要全量 UI 框架 CSS，Naive UI 的按需 CSS 注入更契合
2. **TypeScript 原生** — 新前端全面使用 TypeScript，Naive UI 的 TS 类型推导更友好
3. **更好的主题自定义** — 通过 `NConfigProvider` 统一控制主题变量，适合桌面应用的多主题（浅色/深色）切换
4. **更小的 Bundle** — Tauri 应用是桌面应用，减少前端资源体积有利于启动速度
5. **没有历史包袱** — 这是一次破坏性重构，无需兼容旧代码，选择最新的方案即可
6. **模态/弹窗更轻量** — 启动器场景不需要 Element Plus 那样的企业级表格/表单复杂度

---

## 1.5 窗口架构设计

### 双独立窗口模式

本应用采用**两个独立窗口**的设计：

| 窗口     | 尺寸             | 用途                                | 入口文件          |
| -------- | ---------------- | ----------------------------------- | ----------------- |
| 搜索窗口 | 紧凑（~600x400） | 主搜索界面，支持三种页面形态        | `App.vue`         |
| 设置窗口 | 较大（~800x600） | 配置管理界面，Schema 驱动的动态表单 | `SettingsApp.vue` |

### 窗口打开机制

设置窗口通过 Tauri 的 `WebviewWindow` API 打开：

```typescript
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

async function openSettingsWindow(): Promise<void> {
  const existing = await WebviewWindow.getByLabel('settings')
  if (existing) {
    existing.setFocus()
    return
  }

  new WebviewWindow('settings', {
    url: '/settings.html',
    title: '设置',
    width: 800,
    height: 600,
    resizable: true,
    center: true,
  })
}
```

### 窗口间状态同步

两个窗口通过后端事件系统同步状态：

```
设置窗口修改配置 → 后端发布 config-changed 事件
                                    ↓
搜索窗口监听事件 → 判断是否需要 refresh_candidates
```

---

## 2. 前端架构概览

### 2.1 三层前端架构

```
┌─────────────────────────────────────────────────────────────┐
│                     UI 组件层                                │
│  (components/, views/)                                      │
│  - 搜索栏 / 结果列表 / 插件面板 / 设置面板                  │
├─────────────────────────────────────────────────────────────┤
│                     Plugin 层（前端插件系统）                 │
│  (plugins/)                                                  │
│  - 注册自定义面板渲染组件                                    │
│  - 注册结果项渲染组件                                        │
│  - 注册结果项额外动作                                        │
│  - 注册自定义设置面板                                        │
├─────────────────────────────────────────────────────────────┤
│                     桥接层 (Bridge)                          │
│  (bridge/)                                                   │
│  - invoke 后端 Tauri Commands                               │
│  - 事件监听 (backend → frontend push)                       │
│  - 共享类型定义                                              │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 目录结构

```
src-ui-new/
├── App.vue                          # 根组件（视图路由 + 主题注入）
├── main.ts                          # 入口
├── styles/                          # 全局样式
│   ├── variables.css                # CSS 变量
│   ├── themes.css                   # 主题定义
│   └── transitions.css              # 过渡动画
│
├── bridge/                          # 前后端桥接层
│   ├── index.ts                     # 统一导出
│   ├── commands.ts                  # Tauri invoke 封装（所有命令）
│   ├── events.ts                    # Tauri 事件监听/取消
│   └── contract.ts                  # 前后端共享类型定义（单一真相源）
│
├── composables/                     # 可复用逻辑（Composition API）
│   ├── useSearch.ts                 # 搜索逻辑
│   ├── useTheme.ts                  # 主题切换
│   ├── useKeyboard.ts              # 窗口内键盘事件
│   ├── usePluginManager.ts         # 前端插件管理器
│   └── useSettings.ts              # 设置读写
│
├── components/                      # 通用 UI 组件
│   ├── search/                      # 搜索相关
│   │   ├── SearchBar.vue            # 搜索栏（含输入框）
│   │   └── SearchIcon.vue           # 图标渲染
│   ├── results/                     # 结果相关
│   │   ├── ResultList.vue           # 结果列表容器（虚拟滚动）
│   │   ├── ResultItem.vue           # 单条结果（默认渲染）
│   │   └── ResultActions.vue        # 动作按钮组
│   ├── panel/                       # 面板相关
│   │   ├── PluginPanelHost.vue      # 插件面板宿主（动态组件）
│   │   └── EmptyState.vue           # 空状态提示
│   ├── settings/                    # 设置通用组件
│   │   ├── DynamicForm.vue          # Schema 驱动动态表单
│   │   ├── DynamicFormField.vue     # 单个字段渲染器
│   │   ├── SettingsSidebar.vue      # 设置侧边栏
│   │   ├── PluginSettingsHost.vue   # 插件自定义设置宿主
│   │   └── ConfigActionButton.vue   # 配置动作按钮
│   ├── layout/                      # 布局组件
│   │   ├── WindowFrame.vue          # 窗口框架
│   │   ├── Footer.vue               # 底栏（状态信息 + 设置入口）
│   │   └── ContextMenu.vue          # 右键菜单
│   └── common/                      # 通用组件
│       ├── IconDisplay.vue          # 图标显示
│       └── LoadingIndicator.vue     # 加载指示器
│
├── views/                           # 页面
│   ├── SearchView.vue               # 主搜索视图（默认页面）
│   └── SettingsView.vue             # 设置主视图
│
├── plugins/                         # 前端插件系统
│   ├── manager.ts                   # PluginManager 核心
│   ├── types.ts                     # 插件类型定义
│   ├── registry.ts                  # 插件注册表
│   ├── loader.ts                    # 第三方插件加载器
│   └── built-in/                    # 内置插件实现
│       ├── calculator-panel/        # 计算器面板渲染
│       └── web-search/              # 网页搜索增强
│
├── stores/                          # 状态管理（Pinia）
│   ├── search-store.ts              # 搜索 & 会话状态
│   ├── config-store.ts              # 配置组件 & Schema 缓存
│   ├── theme-store.ts               # 主题状态
│   └── plugin-store.ts              # 前端插件运行时状态
│
├── i18n/                            # 国际化
│   ├── index.ts
│   └── locales/
│       ├── en.json
│       └── zh-Hans.json
│
└── utils/                           # 工具函数
    ├── debounce.ts
    └── format.ts
```

---

## 3. 前后端通信契约（Bridge API）

> **设计原则**: 本节定义的是前后端之间的**接口契约**，同时约束后端 Rust 侧 `#[tauri::command]` 的签名和前端 TypeScript 侧的调用方式。当前 `commands/` 目录下的代码为早期原型，不作为最终参考。

### 3.1 核心设计决策

| 决策                      | 说明                                                                                 |
| ------------------------- | ------------------------------------------------------------------------------------ |
| **JSON 结构体传参**       | 超过 2 个参数的命令使用一个反序列化结构体，避免 Tauri command 扁平参数难以扩展的问题 |
| **Tagged Union 区分模式** | `QueryResponse` 采用 `mode` 字符串 + 条件字段区分三种页面形态                        |
| **后端生成 trace_id**     | 前端不需要关心追踪 ID，后端自行生成和管理                                            |
| **事件推送单向**          | 后端通过 Tauri Event 推送状态变更，前端只订阅不轮询                                  |

### 3.2 共享类型定义（`bridge/contract.ts`）

```typescript
// ============================================================
// 搜索 & 会话相关类型
// ============================================================

/**
 * 后端查询返回值 — 最核心的类型，决定前端渲染哪种视图模式
 * 使用 Discriminated Union 模式，确保类型安全
 */
type BridgeQueryResponse =
  | { mode: 'search'; results: ListItem[] }
  | { mode: 'empty' }
  | {
      mode: 'plugin_panel'
      panelType: string
      panelData: unknown
      panelActions: ResultAction[]
    }
  | {
      mode: 'plugin_immersive'
      panelType: string
      panelData: unknown
      panelActions: ResultAction[]
    }

/**
 * 模式说明:
 * - 'search'          - 形态1: 搜索栏 + 结果列表
 * - 'empty'           - 搜索栏 + 空列表/提示
 * - 'plugin_panel'    - 形态2: 搜索栏 + 插件自定义面板
 * - 'plugin_immersive' - 形态3: 完全自定义面板（搜索栏隐藏）
 */

/** 搜索结果项 */
interface ListItem {
  id: number
  title: string
  subtitle: string
  icon: string
  score: number
  actions: ResultAction[]
}

/** 单个执行动作 */
interface ResultAction {
  id: string
  label: string
  icon: string
  is_default: boolean
  shortcut_key: string
}

/** 确认执行负载（前端 → 后端） */
interface ConfirmPayload {
  candidate_id: number
  action_id: string
  query_text: string
  user_args?: string[]
}

// ============================================================
// 会话状态机
// ============================================================

/**
 * 会话状态 - 使用 Discriminated Union 模式
 * 每种状态携带该状态所需的数据，确保类型安全
 */
type SessionState =
  | { type: 'idle' }
  | { type: 'searching'; query: string }
  | { type: 'results'; query: string; results: ListItem[]; selectedIndex: number }
  | { type: 'noResults'; query: string }
  | { type: 'pluginPanel'; query: string; panelType: string; panelData: unknown; panelActions: ResultAction[]; keepSearchBar: boolean }

/**
 * 状态转换规则：
 * - idle → searching: 用户开始输入
 * - searching → results: 查询返回结果
 * - searching → noResults: 查询无结果
 * - searching → pluginPanel: 查询触发插件
 * - results → idle: 用户清空输入
 * - noResults → idle: 用户清空输入
 * - pluginPanel → idle: 用户退出插件
 * - 任意状态 → idle: 按 ESC 或窗口隐藏
 */


// ============================================================
// 配置相关类型
// ============================================================

/** 组件概览信息 */
interface ComponentInfo {
  component_id: string
  component_name: string
  component_type: ComponentType
  enabled: boolean
  default_enabled: boolean
}

/** 组件类型枚举 */
type ComponentType =
  | 'DataSource'
  | 'KeywordOptimizer'
  | 'SearchEngine'
  | 'ScoreBooster'
  | 'ActionExecutor'
  | 'Plugin'
  | 'Core'

/** 组件配置 Schema（含显示名） */
interface ComponentSchema {
  component_id: string
  component_name: string
  component_type: ComponentType
  settings: SettingDefinition[]
}

/** 配置项声明式定义 */
interface SettingDefinition {
  field: FieldDefinition
  group?: string
  order: number
  /** 关联的配置动作标识符，非空时前端在配置项旁渲染操作按钮 */
  config_action?: string
}

/** 字段定义（SettingDefinition 和 ArrayItem::Object 共用） */
interface FieldDefinition {
  key: string
  label: string
  description: string
  setting_type: SettingType
  default_value: any
  visible: boolean
  editable: boolean
}

/** 配置值控件类型 */
type SettingType =
  | 'Text'
  | { Number: { min?: number; max?: number; step?: number } }
  | 'Boolean'
  | { Select: { options: string[] } }
  | { Path: { mode: 'File' | 'Directory' } }
  | 'Color'
  | 'Json'
  | { Array: { item: ArrayItem; min_items?: number; max_items?: number; ui_hint: ArrayUiHint } }

/** 数组元素类型 */
type ArrayItem =
  | { Primitive: PrimitiveType }
  | { Object: FieldDefinition[] }

type PrimitiveType =
  | 'Text'
  | { Number: { min?: number; max?: number; step?: number } }
  | 'Boolean'
  | { Select: { options: string[] } }
  | { Path: { mode: 'File' | 'Directory' } }
  | 'Color'

type ArrayUiHint = 'Default' | 'Table' | 'MasterDetail' | 'Tags'

/** 配置动作定义 */
interface ConfigActionDef {
  action: string
  label: string
  description: string
}

// ============================================================
// 错误处理类型
// ============================================================

/** 错误代码枚举 */
type ErrorCode =
  | 'INVALID_QUERY'
  | 'COMPONENT_NOT_FOUND'
  | 'VALIDATION_FAILED'
  | 'ACTION_FAILED'
  | 'PLUGIN_ERROR'
  | 'CONFIG_ERROR'
  | 'NETWORK_ERROR'
  | 'INTERNAL_ERROR'

/** 统一错误类型 */
interface BridgeError {
  code: ErrorCode
  message: string
  details?: Record<string, unknown>
  componentId?: string
}

/** 统一返回类型 */
type BridgeResult<T> =
  | { success: true; data: T }
  | { success: false; error: BridgeError }
```

### 3.3 错误处理规范

#### 前端错误处理策略

```typescript
// bridge/commands.ts
import { useNotification } from 'naive-ui'

const notification = useNotification()

async function invokeCommand<T>(
  cmd: string, 
  args?: Record<string, unknown>
): Promise<BridgeResult<T>> {
  try {
    const result = await invoke<BridgeResult<T>>(cmd, args)
    
    if (!result.success) {
      handleError(result.error)
    }
    
    return result
  } catch (e) {
    const error: BridgeError = {
      code: 'INTERNAL_ERROR',
      message: String(e),
    }
    handleError(error)
    return { success: false, error }
  }
}

function handleError(error: BridgeError): void {
  notification.error({
    title: getErrorTitle(error.code),
    content: error.message,
    duration: 5000,
  })
  
  // 记录到控制台
  console.error(`[${error.code}] ${error.message}`, error.details)
}

function getErrorTitle(code: ErrorCode): string {
  const titles: Record<ErrorCode, string> = {
    INVALID_QUERY: '查询无效',
    COMPONENT_NOT_FOUND: '组件未找到',
    VALIDATION_FAILED: '验证失败',
    ACTION_FAILED: '操作失败',
    PLUGIN_ERROR: '插件错误',
    CONFIG_ERROR: '配置错误',
    NETWORK_ERROR: '网络错误',
    INTERNAL_ERROR: '内部错误',
  }
  return titles[code]
}
```

#### 后端 Rust 错误处理

```rust
// 所有 Tauri Command 返回 Result<T, BridgeError>
#[tauri::command]
async fn bridge_query(raw_query: String) -> Result<BridgeResult<BridgeQueryResponse>, BridgeError> {
    // ...
}

impl From<ConfigError> for BridgeError {
    fn from(e: ConfigError) -> Self {
        BridgeError {
            code: ErrorCode::ConfigError,
            message: e.to_string(),
            component_id: None,
            details: None,
        }
    }
}
```

### 3.4 Tauri Commands 清单

> **命名规范**: 搜索/会话类命令统一 `bridge_` 前缀；配置类命令统一 `config_` 前缀。

#### 搜索 & 会话命令

| Command                       | 参数                      | 返回                  | 说明                                               |
| ----------------------------- | ------------------------- | --------------------- | -------------------------------------------------- |
| `bridge_query`                | `raw_query: string`       | `BridgeQueryResponse` | 查询入口。输入变化时调用，后端路由到搜索管道或插件 |
| `bridge_confirm`              | `payload: ConfirmPayload` | `void`                | 执行动作。用户选择候选项并触发动作时调用           |
| `bridge_wake`                 | —                         | `void`                | 搜索栏唤醒。捕获系统参数快照（剪贴板、窗口句柄等） |
| `bridge_reset`                | —                         | `void`                | 重置会话。窗口隐藏/关闭时调用                      |
| `bridge_get_session_mode`     | —                         | `SessionMode`         | 获取当前会话模式                                   |
| `bridge_refresh_candidates`   | —                         | `number`              | 强制刷新候选项缓存，返回缓存数量                   |
| `bridge_get_candidates_count` | —                         | `number`              | 获取当前缓存的候选项数量                           |

#### 配置管理命令

| Command                     | 参数                                    | 返回                | 说明                                   |
| --------------------------- | --------------------------------------- | ------------------- | -------------------------------------- |
| `config_get_all_components` | —                                       | `ComponentInfo[]`   | 获取所有可配置组件概览                 |
| `config_get_schema`         | `component_id: string`                  | `ComponentSchema`   | 获取组件配置 Schema（含显示名）        |
| `config_get_settings`       | `component_id: string`                  | `Value`             | 获取组件当前配置值                     |
| `config_apply_settings`     | `component_id: string, settings: Value` | `void`              | 应用配置（验证→应用→回调→事件→持久化） |
| `config_reset_settings`     | `component_id: string`                  | `void`              | 重置配置为默认值                       |
| `config_set_enabled`        | `component_id: string, enabled: bool`   | `void`              | 启用/禁用组件                          |
| `config_get_actions`        | `component_id: string`                  | `ConfigActionDef[]` | 获取组件支持的配置动作列表             |
| `config_execute_action`     | `component_id: string, action: string`  | `Value`             | 执行配置动作，返回结果 JSON            |

#### 后端 Rust 侧命令签名参考

```rust
// 搜索 & 会话 — 使用结构体反序列化保持清晰
#[tauri::command]
async fn bridge_query(raw_query: String) -> Result<BridgeQueryResponse, String>;

#[tauri::command]
async fn bridge_confirm(payload: ConfirmPayload) -> Result<(), String>;

#[tauri::command]
async fn bridge_wake() -> Result<(), String>;

#[tauri::command]
fn bridge_reset();

#[tauri::command]
fn bridge_get_session_mode() -> String;

#[tauri::command]
fn bridge_refresh_candidates() -> usize;

#[tauri::command]
fn bridge_get_candidates_count() -> usize;

// 配置管理
#[tauri::command]
fn config_get_all_components() -> Vec<ComponentInfo>;

#[tauri::command]
fn config_get_schema(component_id: String) -> Result<ComponentSchema, String>;

#[tauri::command]
fn config_get_settings(component_id: String) -> Result<serde_json::Value, String>;

#[tauri::command]
fn config_apply_settings(component_id: String, settings: serde_json::Value) -> Result<(), String>;

#[tauri::command]
fn config_reset_settings(component_id: String) -> Result<(), String>;

#[tauri::command]
fn config_set_enabled(component_id: String, enabled: bool) -> Result<(), String>;

#[tauri::command]
fn config_get_actions(component_id: String) -> Vec<ConfigActionDef>;

#[tauri::command]
fn config_execute_action(component_id: String, action: String) -> Result<serde_json::Value, String>;
```

### 3.4 后端事件推送（Backend → Frontend Push）

前端在 `bridge/events.ts` 中统一管理事件的订阅和取消。

| 事件名               | 载荷                                                         | 触发时机      | 前端响应                                                                                  |
| -------------------- | ------------------------------------------------------------ | ------------- | ----------------------------------------------------------------------------------------- |
| `config-changed`     | `{ component_id: string, component_type: string }`           | 配置变更后    | 若设置页打开且对应组件可见，刷新 Schema 与当前值；搜索页检查是否需要 `refresh_candidates` |
| `config-error`       | `{ component_id: string, error: string }`                    | 配置应用失败  | Naive UI `useNotification` 弹出错误提示                                                   |
| `installation-event` | `{ event_type: 'install' \| 'uninstall', app_name: string }` | 程序安装/卸载 | `useNotification` 通知 + 静默调用 `bridge_refresh_candidates`                             |

```typescript
// bridge/events.ts 设计
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export function onConfigChanged(callback: (payload: { component_id: string; component_type: string }) => void): Promise<UnlistenFn>
export function onConfigError(callback: (payload: { component_id: string; error: string }) => void): Promise<UnlistenFn>
export function onInstallationEvent(callback: (payload: { event_type: 'install' | 'uninstall'; app_name: string }) => void): Promise<UnlistenFn>
```

---

## 4. 页面形态（窗口内容映射）

搜索窗口在不同会话模式下呈现不同的内容，这是新前端最核心的状态机。

### 4.1 形态定义

```
会话状态          query 是否为空     前端渲染内容               用户输入行为
═══════════════════════════════════════════════════════════════════════════════

SessionMode::None
  └→ 空闲态       query === ''       仅搜索框                  输入文字 → bridge_query
                                     无结果列表，无底栏
                                     (窗口最小紧凑形态)

SessionMode::Search
  ├→ 有结果态     query !== ''       搜索框 + ResultList        上下键选择，Enter 确认
                  mode='search'     + Footer                  继续输入 / Tab 切换动作
  │
  └→ 无结果态     query !== ''       搜索框 + "无结果"提示     继续输入修正关键词
                  mode='empty'      + Footer                  Backspace 清空 → 退回空闲态

SessionMode::Plugin
  ├→ 面板搜索态   任意              搜索框 + PluginPanel       可在搜索框继续输入，
                  mode='plugin      (keep_search_bar=true)    ESC 退回搜索模式
                  _panel'                                    清空搜索框 → 退回空闲态

  └→ 沉浸面板态   任意              纯 PluginPanel             键盘事件全部交给面板，
                  mode='plugin      (搜索框隐藏)              面板提供退出机制
                  _immersive'
```

**关键规则**：
- 空闲态（`query === ''`）是最简形态，只渲染搜索框。窗口高度收缩到仅搜索框的高度。
- 底栏（Footer）只在有搜索框且 query 非空时显示。
- 结果列表只在有搜索框、query 非空、且 `sessionMode === 'search'` 时显示。

### 4.2 `bridge_query` 返回 → `SearchView.vue` 渲染决策树

```typescript
// composables/useSearch.ts 中的核心逻辑（伪代码）

/** 前端自己判定空闲态，不发起后端查询 */
const isIdle = computed(() => store.query === '')

async function doQuery(raw: string) {
  store.query = raw

  // 空闲态不查询
  if (raw === '') {
    store.results = []
    store.sessionMode = 'none'
    store.panelType = null
    return
  }

  const resp = await bridgeQuery(raw)

  switch (resp.mode) {
    case 'search':
      store.results = resp.results
      store.sessionMode = 'search'
      store.selectedIndex = 0
      break

    case 'empty':
      store.results = []
      store.sessionMode = 'search'
      break

    case 'plugin_panel':
      store.sessionMode = 'plugin'
      store.panelType = resp.panelType
      store.panelData = resp.panelData
      store.panelActions = resp.panelActions ?? []
      store.keepSearchBar = true
      break

    case 'plugin_immersive':
      store.sessionMode = 'plugin'
      store.panelType = resp.panelType
      store.panelData = resp.panelData
      store.panelActions = resp.panelActions ?? []
      store.keepSearchBar = false
      break
  }
}
```

### 4.3 `SearchView.vue` 模板结构

```vue
<template>
  <div class="search-view">
    <!-- 搜索框：只在非沉浸态显示 -->
    <SearchBar v-if="searchStore.keepSearchBar" />

    <!-- 结果列表：只在搜索模式 && 非空闲态显示 -->
    <ResultList v-if="searchStore.sessionMode === 'search' && searchStore.query !== ''" />

    <!-- 空闲态提示：搜索模式 && query 为空 && 之前没进入过搜索 -->
    <EmptyState v-else-if="searchStore.query === '' && searchStore.sessionMode !== 'plugin'" />

    <!-- 插件面板：非空闲插件态 -->
    <PluginPanelHost v-else-if="searchStore.sessionMode === 'plugin' && searchStore.panelType" />

    <!-- 底栏：只在非空闲 && 非沉浸态显示 -->
    <Footer v-if="searchStore.query !== '' && searchStore.keepSearchBar" />
  </div>
</template>
```

### 4.4 `PluginPanelHost.vue` 设计

该组件是插件自定义面板的宿主。它通过 `panel_type` 查找 `PluginManager` 中注册的面板渲染器，动态挂载。

```vue
<template>
  <div class="plugin-panel-host">
    <component
      :is="panelComponent"
      v-if="panelComponent"
      v-bind="panelProps"
    />
    <div v-else class="fallback-panel">
      <!-- 没有找到对应渲染器时的降级显示 -->
      <pre>{{ JSON.stringify(pluginStore.panelData, null, 2) }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { usePluginStore } from '@/stores/plugin-store'
import { useSearchStore } from '@/stores/search-store'

const pluginStore = usePluginStore()
const searchStore = useSearchStore()

const panelComponent = computed(() =>
  pluginStore.getPanelComponent(searchStore.panelType)
)

const panelProps = computed(() => ({
  data: searchStore.panelData,
  actions: searchStore.panelActions,
}))
</script>
```

---

## 5. 设置页面设计

### 5.1 整体布局

```
┌──────────────────────────────────────────────────────────┐
│  设置                                          [关闭]    │
│                                                          │
│  ┌─────────────┬────────────────────────────────────┐    │
│  │             │                                    │    │
│  │  常规       │  [组件列表]                        │    │
│  │  搜索管道   │  ┌──────────────────────────────┐  │    │
│  │  插件       │  │ 存储配置                    │  │    │
│  │  外观       │  │ 快捷键                      │  │    │
│  │  关于       │  │ 安装监控                    │  │    │
│  │             │  │ ...                          │  │    │
│  │             │  └──────────────────────────────┘  │    │
│  │             │  → 点击任一组件 → 展开配置表单      │    │
│  │             │                                    │    │
│  └─────────────┴────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

### 5.2 侧边栏分组逻辑

侧边栏条目由 `ComponentType` 按以下规则映射：

| 侧边栏条目   | 包含的 ComponentType                                                               | 图标      | 说明                                  |
| ------------ | ---------------------------------------------------------------------------------- | --------- | ------------------------------------- |
| **常规**     | `Core`                                                                             | settings  | 核心系统组件（热键、存储、自启动等）  |
| **搜索管道** | `DataSource`, `KeywordOptimizer`, `SearchEngine`, `ScoreBooster`, `ActionExecutor` | search    | 按 trait 分标签页                     |
| **插件**     | `Plugin`                                                                           | extension | 独立功能插件（计算器、Everything 等） |
| **外观**     | —（纯前端配置）                                                                    | palette   | 主题、字体等                          |
| **关于**     | —（纯前端信息）                                                                    | info      | 版本、许可                            |

**实现方式**: `getAllComponents()` → 按 `component_type` 分组 → 生成侧边栏条目。

```typescript
// composables/useSettings.ts
function buildSidebarItems(components: ComponentInfo[]): SidebarItem[] {
  const core = components.filter(c => c.component_type === 'Core')
  const pipeline = components.filter(c =>
    ['DataSource', 'KeywordOptimizer', 'SearchEngine', 'ScoreBooster', 'ActionExecutor'].includes(c.component_type)
  )
  const plugins = components.filter(c => c.component_type === 'Plugin')

  return [
    { key: 'core', label: '常规', icon: 'settings', type: 'list', items: core },
    { key: 'pipeline', label: '搜索管道', icon: 'search', type: 'tabs', items: pipeline },
    { key: 'plugins', label: '插件', icon: 'extension', type: 'list', items: plugins },
    { key: 'appearance', label: '外观', icon: 'palette', type: 'static' },
    { key: 'about', label: '关于', icon: 'info', type: 'static' },
  ]
}
```

### 5.3 "搜索管道" 标签页布局

```
搜索管道
┌──────────────────────────────────────────────────────────┐
│  [数据源] [关键词优化器] [搜索引擎] [分数提升器] [动作执行器]    │  ← NTabs
│                                                          │
│  ┌───────────────────────────────────────────────────┐   │
│  │ 程序数据源                  [启用/禁用]            │   │
│  │ AppSource - 索引 UWP 和沙盒应用                   │   │
│  │                                                    │   │
│  │ 网页数据源                  [启用/禁用]            │   │
│  │ UrlSource - 索引网页快捷方式                       │   │
│  │                                                    │   │
│  │ 书签数据源                  [启用/禁用]            │   │
│  │ BookmarkSource - 索引浏览器书签                    │   │
│  └───────────────────────────────────────────────────┘   │
│                                                          │
│  → 点击任一数据源 → 展开 DynamicForm 配置项              │
└──────────────────────────────────────────────────────────┘
```

### 5.4 组件项点击后的交互流

```
组件列表项
  ├── 折叠状态: 组件名称 + 描述 + 启用开关
  └── 展开状态:
      ├── DynamicForm (基于 ComponentSchema.settings 渲染)
      ├── ConfigActionButton (基于 SettingDefinition.config_action 渲染)
      └── 操作栏: [应用] [重置为默认]
```

### 5.5 配置动作按钮交互流

以书签数据源的"自动检测浏览器"为例：

```
1. DynamicFormField 检查 SettingDefinition.config_action
   → 非空 → 渲染 ConfigActionButton 组件

2. ConfigActionButton 挂载时:
   → 调用 config_get_actions(component_id)
   → 查找匹配 action === config_action 的 ConfigActionDef
   → 显示按钮: label（如 "自动检测"）

3. 用户点击按钮:
   → 按钮进入 loading 状态
   → 调用 config_execute_action(component_id, action)
   → 后端执行 Configurable::execute_config_action()
   → 返回 JSON 结果

4. 前端根据返回的 JSON 更新表单字段:
   → 字段匹配: JSON key → FieldDefinition.key
   → 更新 DynamicForm 对应字段的值
   → 提示用户"已检测到 N 个浏览器"
```

### 5.5.1 配置动作 UI 布局规范

**布局位置**：配置动作按钮放置在配置项输入框的右侧，使用次要按钮样式。

```
┌─────────────────────────────────────────────────────────────┐
│ 浏览器路径                                                  │
│ ┌─────────────────────────────────────────┐ ┌────────────┐ │
│ │ C:\Program Files\Chrome\chrome.exe      │ │  自动检测  │ │
│ └─────────────────────────────────────────┘ └────────────┘ │
│ 选择用于打开 URL 的浏览器                                   │
└─────────────────────────────────────────────────────────────┘
```

**多配置动作场景**：如果一个配置项有多个关联动作，使用下拉按钮：

```
┌─────────────────────────────────────────────────────────────┐
│ 搜索路径                                                    │
│ ┌─────────────────────────────────────────┐ ┌────────────▼┐ │
│ │ C:\Users\...\Documents                  │ │  添加路径   │ │
│ └─────────────────────────────────────────┘ └────────────┘ │
│                                              ├────────────┤ │
│                                              │ 添加默认   │ │
│                                              │ 清除全部   │ │
│                                              └────────────┘ │
│ 搜索文件的目录列表                                          │
└─────────────────────────────────────────────────────────────┘
```

**组件实现**：

```vue
<template>
  <div class="config-action-wrapper">
    <n-input v-model:value="localValue" :disabled="!field.editable" />
    <n-button
      v-if="actionDef"
      :loading="loading"
      :disabled="!field.editable"
      @click="executeAction"
    >
      {{ actionDef.label }}
    </n-button>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useConfigStore } from '@/stores/config-store'
import type { FieldDefinition, ConfigActionDef } from '@/bridge/contract'

const props = defineProps<{
  componentId: string
  field: FieldDefinition
  modelValue: any
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: any): void
}>()

const configStore = useConfigStore()
const loading = ref(false)
const actionDef = ref<ConfigActionDef | null>(null)

onMounted(async () => {
  const actions = await configStore.getActions(props.componentId)
  actionDef.value = actions.find(a => a.action === props.field.config_action) ?? null
})

async function executeAction() {
  if (!actionDef.value) return
  
  loading.value = true
  try {
    const result = await configStore.executeAction(
      props.componentId,
      actionDef.value.action
    )
    
    // 如果返回结果包含与当前字段 key 匹配的值，更新表单
    if (result && typeof result === 'object') {
      const fieldValue = result[props.field.key]
      if (fieldValue !== undefined) {
        emit('update:modelValue', fieldValue)
      }
    }
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.config-action-wrapper {
  display: flex;
  gap: 8px;
  align-items: center;
}

.config-action-wrapper .n-input {
  flex: 1;
}
</style>
```

### 5.5.2 DynamicFormField 实现状态

| SettingType                  | 状态             | 说明                                     |
| ---------------------------- | ---------------- | ---------------------------------------- |
| `Text`                       | 已实现           |                                          |
| `{ Number: ... }`            | 已实现           |                                          |
| `Boolean`                    | 已实现           |                                          |
| `{ Select: ... }`            | 已实现           |                                          |
| `{ Path: { mode: 'File' } }` | 已实现           | 接入 `@tauri-apps/plugin-dialog`         |
| `{ Path: { mode: 'Directory' } }` | 已实现      | 接入 `@tauri-apps/plugin-dialog`         |
| `Color`                      | 已实现           |                                          |
| `Json`                       | 已实现           |                                          |
| `{ Array: { item: Primitive, uiHint: 'Tags' } }` | 已实现 | `n-dynamic-tags` 组件                  |
| `{ Array: { item: Primitive, uiHint: 'Default' } }` | 已实现 | 简单列表 + 添加/删除按钮          |
| `{ Array: { item: Primitive, uiHint: 'Table' } }` | **TODO** | 暂回退为 JSON 编辑，控制台输出警告 |
| `{ Array: { item: Primitive, uiHint: 'MasterDetail' } }` | **TODO** | 暂回退为 JSON 编辑，控制台输出警告 |
| `{ Array: { item: Object, ... } }` | **TODO**       | 暂回退为 JSON 编辑，控制台输出警告 |

### 5.6 配置提交流程

```
用户点击 [应用]
  → DynamicForm 收集所有字段值（已根据 Schema 校验类型）
  → config_apply_settings(component_id, settings)
    → 后端 validate → apply → on_settings_changed → 事件广播 → 持久化
  → 前端收到成功 → useMessage.success("配置已保存")

用户点击 [重置为默认]
  → useDialog.warning 确认
  → config_reset_settings(component_id)
  → 重新加载 config_get_settings 刷新表单
```

---

## 6. 前端插件系统设计

### 6.1 前端插件与后端组件的映射关系

**核心原则**：前端插件只与后端的 `Plugin` 类型组件一一对应，管道类组件无前端插件。

| 后端组件类型        | 是否有前端插件 | 说明                                     |
| ------------------- | -------------- | ---------------------------------------- |
| `Plugin` (独立功能) | ✅ 一一对应     | 如计算器插件：后端处理逻辑，前端渲染面板 |
| `DataSource`        | ❌              | 无前端插件，但有 Schema 驱动的设置面板   |
| `KeywordOptimizer`  | ❌              | 无前端插件，但有 Schema 驱动的设置面板   |
| `SearchEngine`      | ❌              | 无前端插件，但有 Schema 驱动的设置面板   |
| `ScoreBooster`      | ❌              | 无前端插件，但有 Schema 驱动的设置面板   |
| `ActionExecutor`    | ❌              | 无前端插件，但有 Schema 驱动的设置面板   |
| `Core`              | ❌              | 无前端插件，但有 Schema 驱动的设置面板   |

**映射机制**：
- 后端 `Plugin` 通过 `plugin_id` 标识
- 前端插件通过 `pluginId` 与后端 `Plugin` 关联
- 后端 `CustomPanel.panel_type` 与前端 `PanelProvider.matchType` 匹配

### 6.2 设计哲学

| 后端职责              | 前端职责                  |
| --------------------- | ------------------------- |
| 处理查询逻辑          | 渲染查询结果              |
| 管理插件注册/生命周期 | 管理 UI 组件注册/生命周期 |
| 返回结构化数据        | 将数据渲染为 DOM          |
| 处理配置读写          | 渲染配置界面              |

**前后端插件通过 `panel_type`（字符串）耦合**：后端 `CustomPanel.panel_type` = 前端 `PanelProvider.matchType`。

### 6.3 前端插件类型定义（`plugins/types.ts`）

```typescript
import type { Component } from 'vue'
import type { ListItem, ResultAction } from '@/bridge/contract'

/** 前端插件完整契约 */
interface FrontendPlugin {
  /** 元数据 */
  id: string
  name: string
  version: string
  description: string

  /** 生命周期 */
  onInit?: () => Promise<void>
  onDestroy?: () => Promise<void>

  /** 面板渲染：匹配后端 CustomPanel.panel_type */
  panelProvider?: PanelProvider

  /** 结果项渲染：自定义特定结果的显示方式 */
  resultItemProvider?: ResultItemProvider

  /** 结果项额外动作：为特定结果类型注入额外操作按钮 */
  actionInjector?: ActionInjector

  /** 自定义设置面板：覆盖 DynamicForm，提供完全自定义的设置 UI */
  settingsProvider?: SettingsProvider
}

/** 面板渲染提供者 — 用于形态2/形态3 */
interface PanelProvider {
  /** 匹配后端 CustomPanel.panel_type */
  matchType: string
  /** Vue 组件 (props: { data: any, actions: ResultAction[] }) */
  component: Component
}

/** 结果项渲染提供者 — 替换默认 ResultItem.vue */
interface ResultItemProvider {
  /** 匹配的目标类型 (与后端 SearchCandidate.target.target_type() 对应) */
  matchTypes: string[]
  /** 自定义渲染组件 (props: { item: ListItem, selected: boolean, index: number }) */
  component: Component
  /** 优先级，数字越小越优先（内置默认渲染器优先级 = 100） */
  priority: number
}

/** 动作注入器 — 为特定结果类型动态添加操作按钮 */
interface ActionInjector {
  /** 匹配的目标类型 */
  matchTypes: string[]
  /** 返回需要注入的额外动作 */
  getActions: (item: ListItem) => ResultAction[]
  /** 优先级 */
  priority: number
}

/** 自定义设置面板提供者 — 覆盖 DynamicForm */
interface SettingsProvider {
  /** 匹配后端 Configurable::component_id() */
  matchComponentId: string
  /** 自定义设置 Vue 组件 (props: { currentSettings: any, onSave: (s: any) => void }) */
  component: Component
}
```

### 6.3 PluginManager（`plugins/manager.ts`）

```typescript
import type { FrontendPlugin, PanelProvider, ResultItemProvider, ActionInjector, SettingsProvider } from './types'
import type { Component } from 'vue'

class PluginManager {
  private plugins: Map<string, FrontendPlugin> = new Map()
  private panelProviders: Map<string, PanelProvider> = new Map()
  private resultItemProviders: ResultItemProvider[] = []
  private actionInjectors: ActionInjector[] = []
  private settingsProviders: Map<string, SettingsProvider> = new Map()

  /** 注册一个前端插件 */
  async register(plugin: FrontendPlugin): Promise<void> {
    this.plugins.set(plugin.id, plugin)

    if (plugin.panelProvider) {
      this.panelProviders.set(plugin.panelProvider.matchType, plugin.panelProvider)
    }
    if (plugin.resultItemProvider) {
      this.resultItemProviders.push(plugin.resultItemProvider)
      this.resultItemProviders.sort((a, b) => a.priority - b.priority)
    }
    if (plugin.actionInjector) {
      this.actionInjectors.push(plugin.actionInjector)
      this.actionInjectors.sort((a, b) => a.priority - b.priority)
    }
    if (plugin.settingsProvider) {
      this.settingsProviders.set(plugin.settingsProvider.matchComponentId, plugin.settingsProvider)
    }

    await plugin.onInit?.()
  }

  /** 注销插件 */
  async unregister(pluginId: string): Promise<void> {
    const plugin = this.plugins.get(pluginId)
    if (!plugin) return

    await plugin.onDestroy?.()
    this.plugins.delete(pluginId)

    if (plugin.panelProvider) {
      this.panelProviders.delete(plugin.panelProvider.matchType)
    }
    // ... 清理其他索引
  }

  /** 按 panel_type 查找面板组件 */
  getPanelComponent(panelType: string): Component | null {
    return this.panelProviders.get(panelType)?.component ?? null
  }

  /** 按结果类型查找自定义渲染组件（链式匹配，返回第一个命中） */
  getResultItemComponent(targetType: string): Component | null {
    for (const provider of this.resultItemProviders) {
      if (provider.matchTypes.includes(targetType)) {
        return provider.component
      }
    }
    return null
  }

  /** 为给定结果项收集额外动作 */
  getExtraActions(item: ListItem, targetType: string): ResultAction[] {
    const actions: ResultAction[] = []
    for (const injector of this.actionInjectors) {
      if (injector.matchTypes.includes(targetType)) {
        actions.push(...injector.getActions(item))
      }
    }
    return actions
  }

  /** 按 component_id 查找自定义设置组件 */
  getSettingsComponent(componentId: string): Component | null {
    return this.settingsProviders.get(componentId)?.component ?? null
  }
}

/** 全局单例 */
export const pluginManager = new PluginManager()
```

### 6.4 第三方插件支持（`plugins/loader.ts`）

#### 插件格式

第三方插件是一个标准 ES Module，默认导出一个 `FrontendPlugin` 对象：

```typescript
// my-calculator-plugin.js
export default {
  id: 'my-calculator',
  name: '高级计算器',
  version: '1.0.0',
  description: '支持科学计算的计算器面板',
  panelProvider: {
    matchType: 'calculator',
    component: () => import('./CalculatorPanel.vue'),
  },
}
```

#### 加载机制

```typescript
// plugins/loader.ts
import { pluginManager } from './manager'
import type { FrontendPlugin } from './types'
import { readDir, readTextFile } from '@tauri-apps/plugin-fs'

interface ThirdPartyPluginManifest {
  name: string
  version: string
  entry: string   // 相对于插件目录的入口文件路径
}

/** 从插件目录加载第三方插件 */
export async function loadThirdPartyPlugins(pluginsDir: string): Promise<void> {
  const entries = await readDir(pluginsDir)

  for (const entry of entries) {
    if (!entry.isDirectory) continue

    const manifestPath = `${pluginsDir}/${entry.name}/plugin.json`
    const manifestContent = await readTextFile(manifestPath)
    const manifest: ThirdPartyPluginManifest = JSON.parse(manifestContent)

    const entryPath = `${pluginsDir}/${entry.name}/${manifest.entry}`
    // 动态导入插件模块
    const module = await import(/* @vite-ignore */ entryPath)
    const plugin: FrontendPlugin = module.default

    await pluginManager.register(plugin)
    console.log(`[PluginLoader] 加载第三方插件: ${plugin.id}`)
  }
}
```

#### 插件目录结构

```
plugins/                          # 用户可配置的第三方插件目录
├── my-calculator/
│   ├── plugin.json               # { "name": "...", "version": "1.0", "entry": "index.js" }
│   ├── index.js
│   └── CalculatorPanel.vue
└── my-notes/
    ├── plugin.json
    └── index.js
```

### 6.5 内置插件 vs 第三方插件

| 维度     | 内置插件                              | 第三方插件                           |
| -------- | ------------------------------------- | ------------------------------------ |
| 位置     | `src-ui-new/plugins/built-in/`        | 用户配置的插件目录                   |
| 加载时机 | 应用启动时 `pluginManager.register()` | 启动后 `loadThirdPartyPlugins(path)` |
| 打包     | 编译进 bundle                         | 运行时动态加载                       |
| 安全审查 | 代码审查                              | 用户自行承担风险                     |
| 更新     | 随应用更新                            | 独立更新                             |

---

## 7. 状态管理（Pinia Stores）

### 7.1 SearchStore（`stores/search-store.ts`）

```typescript
interface SearchStore {
  // === 查询状态 ===
  query: string
  results: ListItem[]
  selectedIndex: number
  isSearching: boolean

  // === 会话状态 ===
  sessionMode: SessionMode        // 'none' | 'search' | 'plugin'
  cachedCount: number

  // === 派生状态 ===
  /** 空闲态：用户未输入任何文字，只显示搜索框 */
  isIdle: boolean                  // query === ''

  // === 插件面板状态（仅在 sessionMode='plugin' 时有意义） ===
  panelType: string | null
  panelData: unknown
  panelActions: ResultAction[]
  keepSearchBar: boolean           // true=形态2, false=形态3

  // === 动作 ===
  doQuery(raw: string): Promise<void>
  doConfirm(index: number, actionId?: string): Promise<void>
  doWake(): Promise<void>
  doReset(): void
  selectNext(): void
  selectPrev(): void
  refreshCandidates(): Promise<number>
}
```

**注意**：`doWake()` 和 `doReset()` 都会将 `query` 置空 → 触发空闲态，窗口缩回只有搜索框的最小形态。
**注意**：空闲态下 `bridge_query` 不会被调用，前端自行判定 `query === ''` 即为空闲态。

### 7.2 ConfigStore（`stores/config-store.ts`）

```typescript
interface ConfigStore {
  // === 数据 ===
  /** 所有组件信息，按 component_id 索引 */
  components: Record<string, ComponentInfo>
  /** 组件 Schema 缓存，按 component_id 索引 */
  schemas: Record<string, ComponentSchema>
  /** 组件当前配置值缓存，按 component_id 索引 */
  settings: Record<string, any>

  // === 加载状态 ===
  isLoading: boolean
  loadError: string | null

  // === 动作 ===
  /** 加载所有组件概览 */
  loadAllComponents(): Promise<void>
  /** 获取指定组件的 Schema（优先缓存） */
  getSchema(componentId: string): Promise<ComponentSchema>
  /** 获取指定组件的当前配置值 */
  getSettings(componentId: string): Promise<any>
  /** 应用配置 */
  applySettings(componentId: string, settings: any): Promise<void>
  /** 重置为默认 */
  resetSettings(componentId: string): Promise<void>
  /** 启用/禁用 */
  setEnabled(componentId: string, enabled: boolean): Promise<void>
  /** 获取配置动作 */
  getActions(componentId: string): Promise<ConfigActionDef[]>
  /** 执行配置动作 */
  executeAction(componentId: string, action: string): Promise<any>
}
```

### 7.3 ThemeStore（`stores/theme-store.ts`）

```typescript
import { darkTheme, type GlobalTheme } from 'naive-ui'

interface ThemeStore {
  isDark: boolean
  naiveTheme: GlobalTheme | null     // darkTheme 或 null（null = 浅色）

  toggleTheme(): void
  setTheme(isDark: boolean): void
  /** 跟随系统主题（通过 system-theme-changed 事件同步） */
  setFollowSystem(follow: boolean): void
}
```

### 7.4 PluginStore（`stores/plugin-store.ts`）

```typescript
interface PluginStore {
  loadedPlugins: FrontendPlugin[]

  getPanelComponent(panelType: string): Component | null
  getResultItemComponent(targetType: string): Component | null
  getExtraActions(item: ListItem, targetType: string): ResultAction[]
  getSettingsComponent(componentId: string): Component | null
}
```

---

## 8. 键盘事件与快捷键

### 8.1 职责边界

| 类型                                                     | 负责方                   | 实现                                  |
| -------------------------------------------------------- | ------------------------ | ------------------------------------- |
| **全局快捷键**（Alt+Space 唤醒、双击 Ctrl 等）           | 后端 SDK `HotkeyManager` | Rust 平台实现，触发 Tauri window 操作 |
| **窗口内键盘事件**（上下键选择、Enter 确认、Esc 退出等） | 前端 `useKeyboard.ts`    | Vue 组合式函数，监听 `keydown` 事件   |

### 8.2 窗口内键盘映射（`composables/useKeyboard.ts`）

```typescript
// 键盘事件处理，根据当前 sessionMode 分派

const KEYMAP = {
  // === 搜索模式 ===
  ArrowDown:  () => searchStore.selectNext(),
  ArrowUp:    () => searchStore.selectPrev(),
  Enter:      () => searchStore.doConfirm(searchStore.selectedIndex),
  Escape:     () => searchStore.query ? searchStore.query = '' : hideWindow(),
  Backspace:  () => { if (searchStore.query === '') doWake() },
  Tab:        () => cycleActions(),
  Home:       () => searchStore.selectedIndex = 0,
  End:        () => searchStore.selectedIndex = searchStore.results.length - 1,

  // Ctrl+数字: 快速触发对应索引的动作（1-based）
  ...Object.fromEntries(
    Array.from({ length: 9 }, (_, i) => [
      `Digit${i + 1}`,
      () => searchStore.doConfirm(i)  // Ctrl+1 → index 0
    ])
  ),

  // === 插件面板模式 ===
  // 如果 keepSearchBar=true, 搜索栏仍接受输入
  // 如果 keepSearchBar=false, 所有按键交给 PlugPanelHost 处理
}
```

### 8.3 键盘事件只在窗口可见时处理

```typescript
// useKeyboard.ts
import { onMounted, onUnmounted } from 'vue'

export function useKeyboard() {
  function handleKeydown(e: KeyboardEvent) {
    if (searchStore.sessionMode === 'plugin' && !searchStore.keepSearchBar) {
      return  // 形态3：键盘事件全交给面板
    }
    const handler = getHandler(e)
    handler?.(e)
  }

  onMounted(() => window.addEventListener('keydown', handleKeydown))
  onUnmounted(() => window.removeEventListener('keydown', handleKeydown))
}
```

---

## 9. Naive UI 主题集成

### 9.1 搜索窗口根组件（App.vue）

```vue
<template>
  <n-config-provider :theme="themeStore.naiveTheme" :locale="zhCN" :date-locale="dateZhCN">
    <n-notification-provider>
      <n-message-provider>
        <n-dialog-provider>
          <SearchView />
        </n-dialog-provider>
      </n-message-provider>
    </n-notification-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { zhCN, dateZhCN } from 'naive-ui'
import { useThemeStore } from './stores/theme-store'
import SearchView from './views/SearchView.vue'

const themeStore = useThemeStore()
</script>
```

### 9.2 设置窗口根组件（SettingsApp.vue）

```vue
<template>
  <n-config-provider :theme="themeStore.naiveTheme" :locale="zhCN" :date-locale="dateZhCN">
    <n-notification-provider>
      <n-message-provider>
        <n-dialog-provider>
          <SettingsView />
        </n-dialog-provider>
      </n-message-provider>
    </n-notification-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { zhCN, dateZhCN } from 'naive-ui'
import { useThemeStore } from './stores/theme-store'
import SettingsView from './views/SettingsView.vue'

const themeStore = useThemeStore()
</script>
```

### 9.3 入口脚本

```typescript
// main.ts - 搜索窗口入口
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './styles/index.css'

const app = createApp(App)
app.use(createPinia())
app.mount('#app')

// settings-main.ts - 设置窗口入口
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import SettingsApp from './SettingsApp.vue'
import './styles/index.css'

const app = createApp(SettingsApp)
app.use(createPinia())
app.mount('#app')
```

### 9.4 打开设置窗口的方式

```typescript
// composables/useSettings.ts
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

export function useSettings() {
  async function openSettings(): Promise<void> {
    const existing = await WebviewWindow.getByLabel('settings')
    if (existing) {
      await existing.setFocus()
      return
    }

    new WebviewWindow('settings', {
      url: '/settings.html',
      title: '设置',
      width: 800,
      height: 600,
      resizable: true,
      center: true,
    })
  }

  return { openSettings }
}
```

---

## 10. 后端侧需要的配合变更

> 以下变更需要在 Rust 后端侧完成，确保与前端桥梁层设计对齐。

### 10.1 `QueryResponse::CustomPanel` 增加 `keep_search_bar`

```rust
// src-tauri/src/plugin_system/types.rs

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QueryResponse {
    List { results: Vec<ListItem> },
    CustomPanel {
        panel_type: String,
        data: serde_json::Value,
        actions: Vec<ResultAction>,
        keep_search_bar: bool,     // ← 新增
    },
    Empty,
}
```

### 10.2 `bridge_query` 透传所有 `QueryResponse` 数据

```rust
// 当前代码在 commands/bridge.rs 中，CustomPanel 分支只返回 mode: "plugin"
// 需要改为完整透传：
match response {
    QueryResponse::List { results } => { /* 保持不变 */ },
    QueryResponse::CustomPanel { panel_type, data, actions, keep_search_bar } => {
        Ok(BridgeQueryResponse {
            mode: if keep_search_bar { "plugin_panel" } else { "plugin_immersive" }.to_string(),
            results: vec![],
            panel_type: Some(panel_type),
            panel_data: Some(data),
            panel_actions: Some(actions.into_iter().map(Into::into).collect()),
        })
    },
    QueryResponse::Empty => {
        Ok(BridgeQueryResponse {
            mode: "empty".to_string(),
            results: vec![],
        })
    },
}
```

**注意**：`BridgeQueryResponse` 结构体也需要添加 `#[serde(rename_all = "camelCase")]` 注解，确保序列化后的 JSON 字段名为 camelCase。

### 10.3 `ComponentSchema` 增加 `component_name`

```rust
// src-tauri/src/core/config/models.rs

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentSchema {
    pub component_id: String,
    pub component_name: String,    // ← 新增，从 Configurable::component_name() 获取
    pub component_type: ComponentType,
    pub settings: Vec<SettingDefinition>,
}
```

### 10.4 `bridge_confirm` 使用结构体反序列化

```rust
#[derive(Deserialize)]
pub struct ConfirmPayload {
    pub candidate_id: u64,
    pub action_id: String,
    pub query_text: String,
    pub user_args: Option<Vec<String>>,
}

#[tauri::command]
pub async fn bridge_confirm(
    state: tauri::State<'_, Arc<AppState>>,
    payload: ConfirmPayload,
) -> Result<(), String> {
    // ...
}
```

### 10.5 Tauri 命令重命名对齐

| 旧命令名                   | 新命令名                    |
| -------------------------- | --------------------------- |
| `get_all_components`       | `config_get_all_components` |
| `get_component_schema`     | `config_get_schema`         |
| `get_component_settings`   | `config_get_settings`       |
| `apply_component_settings` | `config_apply_settings`     |
| `reset_component_settings` | `config_reset_settings`     |
| `set_component_enabled`    | `config_set_enabled`        |
| `get_config_actions`       | `config_get_actions`        |
| `execute_config_action`    | `config_execute_action`     |

---

## 11. 开发阶段规划

```
阶段 1: 基础设施搭建
├── 创建 src-ui-new 目录
├── 配置 Vite + Vue 3 + TypeScript + Naive UI + Pinia
├── 实现 bridge/contract.ts（共享类型定义）
├── 实现 bridge/commands.ts（Tauri invoke 封装）
├── 实现 bridge/events.ts（事件监听）
├── 实现后端侧类型/命令变更（§10）
└── 验证前后端通信

阶段 2: 搜索界面核心
├── stores/search-store.ts + stores/theme-store.ts
├── composables/useSearch.ts + useKeyboard.ts
├── SearchBar 组件
├── ResultList + ResultItem 组件（默认渲染）
├── PluginPanelHost 组件（动态面板宿主）
├── Footer 组件
├── SearchView 视图（三种形态条件渲染）
└── 搜索 → 选择 → 确认 → 执行 完整闭环

阶段 3: 设置界面
├── stores/config-store.ts
├── composables/useSettings.ts
├── SettingsSidebar 组件（按 ComponentType 动态分组）
├── DynamicForm + DynamicFormField 组件（Schema 驱动）
├── ConfigActionButton 组件（配置动作）
├── PluginSettingsHost 组件（自定义设置面板）
├── SettingsView 视图
└── 配置读写完整闭环

阶段 4: 前端插件系统
├── plugins/types.ts + plugins/manager.ts
├── plugins/loader.ts（第三方插件加载）
├── plugins/registry.ts（插件注册表）
├── 内置插件实现（calculator-panel, web-search）
├── stores/plugin-store.ts
└── 端到端插件工作流验证

阶段 5: 完善 & 打磨
├── 浅色/深色主题切换
├── 国际化 (i18n)
├── 右键菜单 (ContextMenu)
├── 事件驱动的 UI 更新（配置变更通知等）
├── 错误处理 & 边界情况
└── 性能优化（虚拟滚动、防抖）
```

---

## 12. 前后端类型同步机制

### 12.1 问题背景

前后端共享类型定义（如 `BridgeQueryResponse`、`ListItem`、`ComponentInfo` 等）需要保持一致。手动维护 `bridge/contract.ts` 容易出错，可能导致运行时错误。

### 12.2 解决方案：使用 ts-rs 自动生成

**ts-rs** 是一个 Rust crate，可以在编译时从 Rust 类型自动生成 TypeScript 类型定义。

#### 后端配置

```toml
# Cargo.toml
[dependencies]
ts-rs = "8.0"
```

#### Rust 类型定义示例

```rust
use ts_rs::TS;
use serde::Serialize;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src-ui-new/bridge/generated.ts")]
pub struct ListItem {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub score: f64,
    pub actions: Vec<ResultAction>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src-ui-new/bridge/generated.ts")]
pub enum BridgeQueryResponse {
    Search { results: Vec<ListItem> },
    Empty,
    PluginPanel {
        panel_type: String,
        panel_data: serde_json::Value,
        panel_actions: Vec<ResultAction>,
    },
    PluginImmersive {
        panel_type: String,
        panel_data: serde_json::Value,
        panel_actions: Vec<ResultAction>,
    },
}
```

**注意**：`#[serde(rename_all = "camelCase")]` 确保 Rust 的 snake_case 字段序列化为 camelCase，ts-rs 也会生成对应的 camelCase TypeScript 类型。

#### 生成的 TypeScript 类型

```typescript
// src-ui-new/bridge/generated.ts (自动生成，请勿手动修改)

export interface ListItem {
  id: number;
  title: string;
  subtitle: string;
  icon: string;
  score: number;
  actions: ResultAction[];
}

export type BridgeQueryResponse =
  | { Search: { results: ListItem[] } }
  | "Empty"
  | { PluginPanel: { panelType: string; panelData: unknown; panelActions: ResultAction[] } }
  | { PluginImmersive: { panelType: string; panelData: unknown; panelActions: ResultAction[] } };
```

### 12.3 前端使用方式

```typescript
// bridge/contract.ts
// 从生成的文件导入类型，作为单一真相源
export type { 
  ListItem, 
  BridgeQueryResponse, 
  ComponentInfo,
  ComponentSchema,
  // ...
} from './generated'

// 补充前端特有的类型定义
export interface FrontendPlugin {
  // ...
}
```

### 12.4 CI 检查

在 CI 中添加类型一致性检查：

```yaml
# .github/workflows/type-check.yml
- name: Generate TypeScript types
  run: cargo build --features ts-rs

- name: Check for changes
  run: |
    if git diff --exit-code src-ui-new/bridge/generated.ts; then
      echo "Type definitions are up to date"
    else
      echo "Type definitions are out of date. Please run 'cargo build' to regenerate."
      exit 1
    fi
```

### 12.5 类型同步流程

```
1. 后端修改 Rust 类型定义
   ↓
2. cargo build 触发 ts-rs 生成 TypeScript 类型
   ↓
3. 生成的类型写入 src-ui-new/bridge/generated.ts
   ↓
4. 前端从 generated.ts 导入类型
   ↓
5. CI 检查确保类型同步
```
