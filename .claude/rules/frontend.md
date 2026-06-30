---
paths:
  - "src-ui-new/**"
---

# 前端规范

## 技术栈

- Vue 3 + Composition API (`<script setup lang="ts">`)
- Naive UI 组件库
- Pinia 状态管理
- TypeScript 严格模式
- Vite 构建
- vue-i18n 国际化

## 核心约束

### 组件编写

- **必须** 使用 `<script setup lang="ts">`。**禁止** 使用 Options API
- **必须** 使用 `defineProps<T>()` 和 `defineEmits<T>()` 的泛型形式
- 所有组件样式 **必须** 使用 `<style scoped>`
- 颜色、字号、间距 **必须** 使用 CSS 变量（如 `var(--text-primary)`）
- **必须** 通过 `bridge/commands.ts` 封装所有 Tauri API 调用
- 所有业务逻辑 **必须** 通过 IPC 委托后端（见 [general.md](general.md) 的前后端职责边界）

### 响应式状态

- 对象/数组更新 **必须** 使用展开运算符创建新引用：`state.value = { ...state.value, [key]: val }`（直接 mutate 如 `state.value.key = val` 不会触发 Vue 的响应式追踪）
- Store 暴露的方法 **必须** 封装状态更新逻辑

### Store 模式

- 使用 `defineStore('id', () => { ... })` setup 语法。**禁止** 使用 Options Store
- 每个 store 管理 **一个** 关注点（search、config、theme、plugin）
- Store 在 `main.ts` / `settings-main.ts` 中 mount 前初始化。**禁止** 在组件内首次创建 store

### 多窗口架构

- 搜索窗口入口：`main.ts` → `App.vue` → `SearchView.vue`
- 设置窗口入口：`settings-main.ts` → `SettingsApp.vue` → `SettingsView.vue`
- 两窗口共享 `bridge/`、`stores/`、`composables/`、`components/`
- 跨窗口状态同步通过 Tauri 事件（`config-changed`、`system-theme-changed`）
- **禁止** 假设两窗口共享内存状态。每窗口有独立的 Pinia 实例

### CSS 变量契约

- 所有外观配置（颜色、尺寸、字体）由后端 `appearance` 组件管理
- 后端设置变更 → 前端 `applyAppearanceSettings()` 更新 CSS 变量 → 组件自动响应
- `styles/variables.css` 定义所有 CSS 变量的 **静态默认值**。`styles/transitions.css` 定义全局过渡动画
- **必须** 通过 `setProperty('--var', val)` 操作 CSS 变量
- 暗色模式通过 `html.dark` class 切换。**禁止** 使用 `@media (prefers-color-scheme)`

### Schema 驱动的设置 UI

- 后端定义 `SettingDefinition`（通过 SchemaBuilder）→ 前端通用渲染
- **禁止** 为特定组件创建专用 Vue 设置页面（除非该组件有 `DetailPreviewPanel` 类扩展需求）
- `DynamicFormField.vue` 是唯一的字段渲染分发器。新增字段类型 → 在此添加分支
- 新增 `SettingType` 变体 → 同步更新 `bridge/contract.ts` + `utils/schemaTypes.ts`

### 键盘快捷键

- 搜索窗口的键盘处理集中在 `composables/useKeyboardRouter.ts`，具体处理器在 `composables/keyboard/` 子目录（`searchHandler.ts`、`inlineParamHandler.ts`、`inlinePluginHandler.ts`、`fullPagePluginHandler.ts`、`paramPanelHandler.ts`）
- **禁止** 在子组件中添加全局 `keydown` 监听器。由 composable 统一管理
- 插件面板激活时（immersive mode），搜索快捷键 **必须** 被抑制

### 类型安全

- 所有 IPC 类型在 `bridge/contract.ts` 中定义，与 Rust 字段级 `#[serde(rename = "camelCaseKey")]` 保持同步
- Schema 类型守卫集中在 `utils/schemaTypes.ts`。**禁止** 在组件中内联类型判断
- **禁止** 使用 `any` 类型。使用 `unknown` + 类型守卫

### 国际化

- 所有用户可见文本 **必须** 使用 `t('key')` 或 `$t('key')`。语言文件放 `i18n/locales/`，支持 `zh-Hans` 和 `en`

### Pre-mount 初始化

- 外观主题 **必须** 在 app.mount() 之前加载并应用（防止白屏闪烁）
- 顺序：createPinia → createApp → useThemeStore().loadFromBackend() → app.mount()
