# ZeroLaunch-rs 新前端需求文档

> **文档状态**: 初稿  
> **项目名称**: ZeroLaunch-rs  
> **技术栈**: Tauri 2.x (Rust 后端) + Vue 3 (TypeScript 新前端)  
> **目标平台**: Windows (x86_64 / aarch64)

---

## 0. 关于旧前端

**`src-ui/` 目录下的旧前端将被完全删除。** 旧前端基于旧版后端架构（`commands/` 模块中的每个功能对应一个 Tauri Command），与新系统的三层架构（SDK → Core → PluginSystem）不兼容。

旧前端仅作为功能参考和 API 调用模式参考保留在新前端开发期间，**不应被修改或增量重构**。

### 新旧对比

| 维度     | 旧前端                             | 新前端                       |
| -------- | ---------------------------------- | ---------------------------- |
| 通信方式 | 每个功能写一个命令（50+ Commands） | 通过通用桥接层（~10 个命令） |
| UI 框架  | Element Plus                       | Naive UI                     |
| 状态管理 | Pinia + 分散式状态                 | 集中式状态 + Schema 驱动     |
| 插件化   | 不支持                             | 支持前端插件注册机制         |
| 后端架构 | 单体命令层                         | PluginSystem + ConfigManager |
| 配置管理 | 手动映射配置项                     | Schema 驱动动态表单          |

---

## 1. UI 框架选型分析

### 方案对比: Element Plus vs Naive UI

| 维度            | Element Plus                             | Naive UI                           |
| --------------- | ---------------------------------------- | ---------------------------------- |
| 构建工具        | Vue 3 (Composition API)                  | Vue 3 + **纯 TypeScript**          |
| Tree Shaking    | 需手动配置                               | **开箱即用**                       |
| TypeScript 支持 | 基础类型                                 | **一等公民支持**                   |
| CSS 引入        | 需全量引入 `element-plus/dist/index.css` | **零 CSS 引入**                    |
| 主题定制        | CSS 变量覆盖 / SCSS 变量覆盖             | **`NConfigProvider` 全局主题变量** |
| Bundle Size     | 较大（即使按需引入也有额外开销）         | 更小                               |
| 组件数量        | 极多（企业级）                           | 丰富（够用）                       |
| 自定义样式      | 容易与全局样式冲突                       | 隔离性好                           |
| Vue 3 原生度    | 从 Vue 2 迁移而来                        | **原生 Vue 3 编写**                |
| 社区生态        | 更成熟，中文文档优质                     | 活跃，文档优质                     |
| 适合场景        | 后台管理系统                             | 桌面应用 / 轻量级 UI               |

### 推荐结论：**Naive UI**

理由：

1. **零 CSS 引入** — Tauri 窗口本身就是无头窗口，不需要全量 UI 框架 CSS，Naive UI 的按需 CSS 注入更契合
2. **TypeScript 原生** — 新前端全面使用 TypeScript，Naive UI 的 TS 类型推导更友好
3. **更好的主题自定义** — 通过 `NConfigProvider` 统一控制主题变量，适合桌面应用的多主题（浅色/深色）切换
4. **更小的 Bundle** — Tauri 应用是桌面应用，减少前端资源体积有利于启动速度
5. **没有历史包袱** — 这是一次破坏性重构，无需兼容旧代码，选择最新的方案即可
6. **模态/弹窗更轻量** — 启动器场景不需要 Element Plus 那样的企业级表格/表单复杂度

---

## 2. 前端架构概览

### 2.1 三层前端架构

```
┌─────────────────────────────────────────────────────────────┐
│                     UI 组件层                                │
│  (components/, views/)                                      │
│  - 搜索栏 / 结果列表 / 设置面板 / 欢迎页                    │
├─────────────────────────────────────────────────────────────┤
│                     Plugin 层（前端插件系统）                 │
│  (plugins/)                                                  │
│  - 注册自定义结果渲染组件                                    │
│  - 注册设置面板                                              │
│  - 注册动作执行器                                            │
├─────────────────────────────────────────────────────────────┤
│                     桥接层 (Bridge)                          │
│  (bridge/)                                                   │
│  - invoke 后端通用接口                                       │
│  - 事件监听 (backend → frontend)                             │
│  - 类型定义                                                  │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 目录结构

```
src-ui-new/
├── App.vue                          # 根组件
├── main.ts                          # 入口
├── styles/                          # 全局样式
│   ├── variables.css                # CSS 变量
│   ├── themes.css                   # 主题定义
│   └── transitions.css              # 过渡动画
│
├── bridge/                          # 前后端桥接层
│   ├── index.ts                     # 统一导出
│   ├── commands.ts                  # invoke 封装
│   ├── events.ts                    # Tauri 事件监听
│   ├── types.ts                     # 类型定义
│   └── useBridge.ts                 # 可组合式桥接接口
│
├── composables/                     # 可复用逻辑
│   ├── useSearch.ts                 # 搜索逻辑
│   ├── useTheme.ts                  # 主题切换
│   ├── useShortcuts.ts              # 快捷键
│   ├── usePlugins.ts                # 前端插件管理
│   └── useSettings.ts              # 设置管理
│
├── components/                      # 通用 UI 组件
│   ├── search/                      # 搜索相关
│   │   ├── SearchBar.vue            # 搜索栏
│   │   ├── SearchInput.vue          # 动画输入框
│   │   └── SearchIcon.vue           # 图标渲染
│   ├── results/                     # 结果相关
│   │   ├── ResultList.vue           # 结果列表容器
│   │   ├── ResultItem.vue           # 单条结果
│   │   └── ResultAction.vue         # 动作菜单
│   ├── settings/                    # 设置通用组件
│   │   ├── DynamicForm.vue          # Schema 驱动动态表单
│   │   ├── SettingSection.vue       # 设置分区
│   │   └── SettingTabs.vue          # 设置标签页
│   ├── layout/                      # 布局组件
│   │   ├── WindowFrame.vue          # 窗口框架
│   │   ├── Footer.vue               # 底栏
│   │   └── ContextMenu.vue          # 右键菜单
│   └── common/                      # 通用组件
│       ├── IconDisplay.vue          # 图标显示
│       └── LoadingIndicator.vue     # 加载指示器
│
├── views/                           # 页面
│   ├── search/                      # 搜索页面
│   │   └── SearchView.vue           # 主搜索视图
│   ├── settings/                    # 设置页面
│   │   ├── SettingsView.vue         # 设置主视图
│   │   ├── general/                 # 常规设置
│   │   ├── appearance/              # 外观设置
│   │   ├── plugins/                 # 插件管理（未来）
│   │   └── about/                   # 关于
│   └── WelcomeView.vue              # 欢迎页
│
├── plugins/                         # 前端插件系统
│   ├── plugin-manager.ts            # 插件管理器
│   ├── plugin-types.ts              # 插件类型定义
│   └── built-in/                    # 内置插件
│       ├── web-search/              # 网页搜索插件
│       ├── bookmark-search/         # 书签搜索插件
│       └── command-runner/          # 命令运行插件
│
├── stores/                          # 状态管理
│   ├── search-store.ts              # 搜索状态
│   ├── config-store.ts              # 配置状态
│   ├── session-store.ts             # 会话状态
│   └── plugin-store.ts              # 插件状态
│
├── i18n/                            # 国际化
│   ├── index.ts
│   └── locales/
│       ├── en.json
│       └── zh-Hans.json
│
└── utils/                           # 工具函数
    ├── color.ts
    ├── debounce.ts
    └── format.ts
```

---

## 3. 桥接层设计（前后端通信）

### 3.1 设计原则

- **极简命令数量**：后端暴露约 10 个通用命令，前端通过它们完成所有操作
- **类型安全**：前后端共享类型定义（后端 Rust struct → 前端 TypeScript interface）
- **事件驱动**：后端通过 Tauri Events 向前端推送状态变更（配置变更、安装监控等）
- **单向数据流**：前端发起请求 → 后端处理 → 返回结果/触发事件

### 3.2 Bridge API 定义

```typescript
// === 搜索相关 ===

interface SearchRequest {
  raw_query: string
}

interface QueryResponse {
  results: ListItem[]
  mode: 'search' | 'plugin'
}

interface ListItem {
  id: number
  title: string
  subtitle: string
  icon: string
  score: number
  actions: ResultAction[]
}

interface ResultAction {
  id: string
  label: string
  icon: string
  is_default: boolean
  shortcut_key: string
}

interface ConfirmPayload {
  candidate_id: number
  action_id: string
  query_text: string
  user_args?: string[]
}

// === 配置相关 ===

interface ComponentInfo {
  component_id: string
  component_name: string
  component_type: string
  enabled: boolean
}

interface ComponentSchema {
  component_id: string
  component_type: string
  settings: SettingDefinition[]
}

interface SettingDefinition {
  field: FieldDefinition
  group?: string
  order: number
}

interface FieldDefinition {
  key: string
  label: string
  description: string
  setting_type: string
  default_value: any
  visible: boolean
  editable: boolean
}

// === 会话相关 ===

type SessionMode = 'none' | 'plugin' | 'search'
```

### 3.3 后端 Commands 清单

| Command                       | 参数                           | 返回                | 说明                 |
| ----------------------------- | ------------------------------ | ------------------- | -------------------- |
| `bridge_query`                | `trace_id, raw_query`          | `QueryResponse`     | 查询入口             |
| `bridge_confirm`              | `trace_id, action_id, payload` | `void`              | 执行动作             |
| `bridge_wake`                 | -                              | `void`              | 唤醒搜索栏，捕获快照 |
| `bridge_reset`                | -                              | `void`              | 重置会话             |
| `bridge_get_session_mode`     | -                              | `SessionMode`       | 当前会话模式         |
| `bridge_refresh_candidates`   | -                              | `number`            | 刷新候选项缓存       |
| `bridge_get_candidates_count` | -                              | `number`            | 缓存候选项数量       |
| `get_all_components`          | -                              | `ComponentInfo[]`   | 所有可配置组件       |
| `get_component_schema`        | `component_id`                 | `ComponentSchema`   | 组件配置 Schema      |
| `get_component_settings`      | `component_id`                 | `Value`             | 组件当前配置         |
| `apply_component_settings`    | `component_id, settings`       | `void`              | 应用配置             |
| `reset_component_settings`    | `component_id`                 | `void`              | 重置为默认           |
| `set_component_enabled`       | `component_id, enabled`        | `void`              | 启用/禁用            |
| `execute_config_action`       | `component_id, action`         | `Value`             | 执行配置动作         |
| `get_config_actions`          | `component_id`                 | `ConfigActionDef[]` | 获取配置动作         |

### 3.4 后端事件推送

| 事件名               | 载荷                               | 触发时机       |
| -------------------- | ---------------------------------- | -------------- |
| `config-changed`     | `{ component_id, component_type }` | 配置变更时     |
| `config-error`       | `{ component_id, error }`          | 配置应用失败时 |
| `installation-event` | `{ event_type, app_name }`         | 安装/卸载事件  |

---

## 4. 前端插件化设计

### 4.1 为什么前端需要插件化

后端 PluginSystem 支持通过触发词路由到不同插件处理查询。这个能力可以扩展到前端：

- **计算器插件**：用户输入 `=1+1` → 直接在前端计算结果展示
- **Everything 插件**：用户输入 `# file` → 触发 Everything 搜索，展示文件结果
- **网页搜索插件**：用户输入 `bd ZeroLaunch` → 触发网页搜索，预览结果
- **自定义 UI 插件**：插件可以注册自己的结果渲染组件

### 4.2 前端插件类型定义

```typescript
interface FrontendPlugin {
  // 插件元数据
  id: string
  name: string
  description: string
  
  // 生命周期
  onInit?(): Promise<void>
  onDestroy?(): Promise<void>
  
  // 配置面板组件（可选）
  settingsComponent?: Component
  
  // 渲染扩展
  resultRenderers?: ResultRenderer[]
}

interface ResultRenderer {
  // 匹配哪些后端返回的结果类型
  matchResultType: string
  // 自定义渲染组件
  component: Component
  // 优先级（数字越小越优先）
  priority: number
}
```

### 4.3 插件化工作流

```
用户输入 "=1+1"
  → 后端通过触发词路由到 CalculatorPlugin
  → 后端返回 QueryResponse::CustomPanel
  → 前端检测到 panel_type = "calculator"
  → PluginManager 查找已注册的 calculator 渲染器
  → 渲染计算器自定义 UI 组件
```

### 4.4 实现考量

- 前端插件管理器（`PluginManager`）负责插件的注册、生命周期管理
- 通过后端 `ConfigManager` 管理插件的启用/禁用状态
- 插件可注册独立设置面板（通过 Schema 或自定义 Vue 组件）
- **当前阶段仅设计骨架，内置插件优先，第三方插件接口后续扩展**

---

## 5. 窗口布局设计

### 5.1 搜索窗口（主界面）

```
┌─────────────────────────────────────┐
│  SearchBar (搜索栏)                  │  ← 固定高度
├─────────────────────────────────────┤
│                                     │
│  ResultList (结果列表)               │  ← 可滚动
│  ┌───────────────────────────────┐  │
│  │ [图标]  程序名      动作按钮  │  │
│  │ [图标]  程序名      动作按钮  │  │
│  │ [图标]  程序名      动作按钮  │  │
│  │ ...                           │  │
│  └───────────────────────────────┘  │
│                                     │
│  PluginPanel (插件面板, 条件渲染)    │  ← 插件自定义 UI
│                                     │
├─────────────────────────────────────┤
│  Footer (底栏)                      │  ← 可选，固定高度
│  状态信息                     设置  │
└─────────────────────────────────────┘
```

### 5.2 设置窗口

```
┌─────────────────────────────────────────────┐
│  设置                                        │
│  ┌─────────┬─────────────────────────────┐   │
│  │ 常规    │  [动态表单]                  │   │
│  │ 外观    │                              │   │
│  │ 数据源  │  Schema 驱动渲染             │   │
│  │ 搜索    │  支持多种类型：               │   │
│  │ 快捷键  │  Text / Number / Boolean     │   │
│  │ 关于    │  Array / Path / Color        │   │
│  │ 调试    │                              │   │
│  └─────────┴─────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

---

## 6. 核心状态管理

### 6.1 Store 设计

```typescript
// 搜索状态
interface SearchStore {
  query: string                    // 当前查询文本
  results: ListItem[]              // 搜索结果
  selectedIndex: number            // 选中项索引
  isSearching: boolean             // 是否正在搜索
  sessionMode: SessionMode         // 当前模式
  cachedCount: number              // 缓存的候选项数量
}

// 配置状态
interface ConfigStore {
  components: Map<string, ComponentInfo>  // 所有组件信息
  schemas: Map<string, ComponentSchema>   // 组件 Schema 缓存
  settings: Map<string, any>              // 组件配置值缓存
  
  // 动作
  loadAllComponents(): Promise<void>
  getSchema(id: string): Promise<ComponentSchema>
  applySettings(id: string, settings: any): Promise<void>
  setEnabled(id: string, enabled: boolean): Promise<void>
}

// 主题状态
interface ThemeStore {
  isDark: boolean
  configProviderTheme: GlobalTheme  // Naive UI 主题对象
}
```

### 6.2 Naive UI 主题集成

利用 `NConfigProvider` 实现统一的主题切换，无需为每个组件手动传入颜色：

```vue
<template>
  <n-config-provider :theme="theme">
    <n-notification-provider>
      <n-message-provider>
        <router-view />
      </n-message-provider>
    </n-notification-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { darkTheme } from 'naive-ui'
import { useThemeStore } from './stores/theme-store'

const themeStore = useThemeStore()
const theme = computed(() => themeStore.isDark ? darkTheme : null)
</script>
```

---

## 7. 开发计划（建议）

```
阶段 1: 基础设施搭建
├── 创建 src-ui-new 目录
├── 配置 Vite + Vue 3 + TypeScript + Naive UI
├── 实现桥接层 (bridge/)
├── 实现 Rust 侧桥接命令
└── 验证前后端通信

阶段 2: 搜索界面核心
├── SearchBar 组件
├── ResultList + ResultItem 组件
├── IconDisplay 组件
├── 快捷键系统
└── 搜索流程闭环

阶段 3: 设置界面
├── DynamicForm（Schema 驱动动态表单）
├── SettingSection / SettingTabs
├── 数据源配置面板
├── 快捷键配置面板
└── 主题切换

阶段 4: 插件系统骨架
├── PluginManager 实现
├── PluginType 定义
├── 内置插件迁移
└── 插件配置面板

阶段 5: 细节完善
├── 国际化迁移
├── 动画/过渡效果
├── 窗口管理完善
└── 性能优化
```

---

## 8. 关键设计决策

### 8.1 使用 Naive UI 而非 Element Plus

详见第 1 节。核心要点：Naive UI 对 TypeScript 的原生支持、零 CSS 引入、更好的桌面应用主题定制。

### 8.2 通用桥接层替代大量 Commands

- 旧系统：50+ 个独立的 Tauri Command
- 新系统：~10 个通用命令，通过 SessionRouter + ConfigManager 的通用接口完成所有操作
- 好处：前端不需要知道后端的具体实现，后端修改不影响前端接口

### 8.3 Schema 驱动配置表单

- 后端 `Configurable` trait 定义 `setting_schema()`，返回结构化 `SettingDefinition`
- 前端 `DynamicForm` 组件根据 Schema 自动渲染表单
- 数据源、搜索引擎、增强器等所有组件的配置面板由同一个组件统一渲染
- 新增后端组件时前端无需修改

### 8.4 前端插件化的渐进式实现

- 阶段 1-3：不实现前端插件机制，所有组件硬编码
- 阶段 4：实现插件管理器骨架
- 未来：逐步将内置功能迁移为插件，设计第三方插件接口

---

## 9. 与旧前端的功能差异（初版不实现）

| 功能            | 旧系统 | 新系统初版                                | 优先级 |
| --------------- | ------ | ----------------------------------------- | ------ |
| Everything 集成 | ✅ 支持 | ❌ 暂不实现                                | 低     |
| AI 语义搜索     | ✅ 支持 | ❌ 暂不实现                                | 低     |
| 浏览器书签      | ✅ 支持 | 通过 `BookmarkSource` 数据源支持          | 中     |
| 参数模板        | ✅ 支持 | 通过 `ExecutionContext.user_args` 支持    | 中     |
| 内置命令        | ✅ 支持 | 通过 `BuiltinCommand` Executor 支持       | 高     |
| Web 搜索        | ✅ 支持 | 通过 `UrlSource` 数据源支持               | 高     |
| 自定义命令      | ✅ 支持 | 通过 `CommandSource` 数据源支持           | 高     |
| 远程配置同步    | ✅ 支持 | 通过 `ConfigManager.save_to_storage` 支持 | 中     |

虽然存在以上的差异，但是均可以通过运行时根据setting schema动态生成对应的设置页面