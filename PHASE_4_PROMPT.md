# Phase 5 前端打磨与完善提示词

## 当前进度

Phase 1（基础设施）、Phase 2（搜索界面核心）、Phase 3（设置界面完善）、Phase 4（前端插件系统）已完成。

### Phase 1-4 全部成果

**Phase 1 — 基础设施：**
- bridge 层：contract.ts / commands.ts / events.ts（前后端通信契约）
- Pinia stores：search-store / config-store / theme-store / plugin-store
- Composables：useSearch / useKeyboard / useSettings / useWindowResize / usePluginManager

**Phase 2 — 搜索界面核心：**
- 搜索界面组件：SearchBar / ResultList / ResultItem / ResultActions
- 布局组件：WindowFrame / Footer / ContextMenu
- 面板：PluginPanelHost / EmptyState
- 独立设置窗口：SettingsApp.vue + settings-main.ts + setting_window.html
- 窗口尺寸自适应：useWindowResize（空闲态收缩，搜索态展开）

**Phase 3 — 设置界面完善 + Phase 2 缺口修复：**
- DynamicFormField Array 类型完整实现（Primitive/Object × Tags/Default/Table/MasterDetail）
- SettingsSidebar 静态条目（外观 / 关于）
- ConfigActionButton + toast 反馈
- Bridge events 接入（config-changed / installation-event）
- Theme 持久化（localStorage + prefers-color-scheme）
- Escape 键隐藏窗口
- 后端修正（删除 QueryResponse::WebView、SearchPipeline engine 非 Option 化）

**Phase 4 — 前端插件系统 + 端到端 CalculatorPlugin：**
- 后端 CalculatorPlugin 完整实现（触发词 `=`、递归下降表达式求值器、CustomPanel 响应 + copy_result 动作）
- `ListItem` 新增 `targetType` 字段（后端 types.rs + bridge.rs，前端 contract.ts 同步）
- 前端插件基础设施：
  - `plugins/types.ts` — FrontendPlugin + 四种 Provider 接口（PanelProvider / ResultItemProvider / ActionInjector / SettingsProvider）
  - `plugins/manager.ts` — PluginManager 全局单例（register / unregister / 四个索引查找 + 生命周期钩子）
  - `plugins/built-in/calculator-panel/` — 内置计算器面板（index.ts + CalculatorPanel.vue）
  - `stores/plugin-store.ts` — Pinia store 包装 PluginManager
  - `composables/usePluginManager.ts` — loadBuiltinPlugins() 幂等加载
- 组件集成：
  - `PluginPanelHost.vue` — 通过 PluginManager.getPanelComponent(panelType) 动态渲染
  - `PluginSettingsHost.vue` — 通过 PluginManager.getSettingsComponent(componentId) 查找自定义设置组件
  - `ResultItem.vue` — 支持自定义渲染器 + 动作注入合并
- `search-store.ts` doConfirm 新增 plugin 模式分支 + try-catch 错误处理
- SessionRouter 新增 `plugin_service()` 公开访问器，lib.rs 注册 CalculatorPlugin
- 配置：tsconfig + vite 新增 `@/` 路径别名
- 验收：`vue-tsc --noEmit` + `cargo check` 均零错误通过

### 已完成（Phase 5：打磨与完善）

- **国际化 i18n** — vue-i18n 框架 + 中英文 locale 文件，Naive UI locale 同步
- **右键菜单 ContextMenu 集成** — ResultItem 右键弹出动作菜单
- **错误处理完善** — 后端 BridgeError 类型（15 个命令全量改造）+ 前端统一错误通知
- **搜索管道动态重建** — SearchEngine/ScoreBooster 配置变更时自动重建 SearchPipeline
- **外观配置后端化** — 新建 AppearanceConfigComponent（theme + language），theme/language 统一由 ConfigManager 管理（不再使用 localStorage）
- **SessionMode 优化** — Plugin(Option<String>) → Plugin(String)，dispatcher 返回 plugin_id

### 待完成（后续阶段）

- **EverythingPlugin 后端实现** — 属于程序功能补全，非前端重构范围，后续独立完成
- **第三方插件加载器** — plugins/loader.ts（plugin.json manifest + 动态 import）
- **内置 web-search 插件** — WebSearchPanel 渲染组件
- **ResultItemProvider 实际用例** — 为特定 targetType 注册自定义结果渲染器
- **ActionInjector 实际用例** — 为特定 targetType 注入额外动作按钮
- **SettingsProvider 实际用例** — 为特定 Plugin 组件提供自定义设置面板
- **虚拟滚动优化** — 当前 top_k=10，前端最多接收 10 条结果，无实际收益，暂不实施
- **useKeyboard 增强** — 插件模式下的键盘事件处理

---

## 架构参考

### 三层架构

```
┌──────────────────────────────────┐
│  Plugin / PluginSystem           │  ← 业务层：插件实现与编排
├──────────────────────────────────┤
│  Core (core/)                    │  ← ConfigManager, Configurable trait
├──────────────────────────────────┤
│  SDK (sdk/)                      │  ← HostApi, platform abstractions
└──────────────────────────────────┘
```

**依赖规则**：`sdk/ → core/ → plugin/` — 禁止反向依赖。

### 前后端插件映射

| 后端组件类型 | 是否有前端插件 | 说明 |
|------------|-------------|------|
| `Plugin` (独立功能) | 一一对应 | 如 CalculatorPlugin ↔ CalculatorPanel（Phase 4 已打通） |
| `DataSource` / `KeywordOptimizer` / `SearchEngine` / `ScoreBooster` / `ActionExecutor` / `Core` | 无前端插件 | 有 Schema 驱动的设置面板 |

**耦合方式**：后端 `CustomPanel.panel_type`（字符串） = 前端 `PanelProvider.matchType`（字符串）。

### 关键文件位置（Phase 4 后）

| 类别 | 路径 |
|------|------|
| 后端 bridge 命令 | `src-tauri/src/commands/bridge.rs`（7 个命令） |
| 后端 config 命令 | `src-tauri/src/commands/config_file.rs`（8 个命令） |
| 后端类型定义 | `src-tauri/src/plugin_system/types.rs` |
| 后端 SessionRouter | `src-tauri/src/plugin_system/session_router.rs` |
| 后端 ConfigManager | `src-tauri/src/core/config/manager.rs` |
| 后端 CalculatorPlugin | `src-tauri/src/plugin/triggerable/calculator_plugin.rs` |
| 前端类型契约 | `src-ui-new/bridge/contract.ts` |
| 前端命令封装 | `src-ui-new/bridge/commands.ts` |
| 前端事件监听 | `src-ui-new/bridge/events.ts` |
| 前端插件类型 | `src-ui-new/plugins/types.ts` |
| 前端插件管理器 | `src-ui-new/plugins/manager.ts` |
| 前端插件 Store | `src-ui-new/stores/plugin-store.ts` |
| 前端搜索 store | `src-ui-new/stores/search-store.ts` |
| 前端配置 store | `src-ui-new/stores/config-store.ts` |

---

## 需要注意的点

1. **PluginManager 已就绪** — Phase 4 建立了完整的插件注册/查找机制。Phase 5 新增插件只需：
   - 创建 Vue 组件
   - 编写 `index.ts` 导出 `FrontendPlugin` 对象
   - 在 `usePluginManager.ts` 中注册即可

2. **前端插件只与后端 Plugin 类型组件对应** — 管道组件（DataSource / KeywordOptimizer / SearchEngine / ScoreBooster / ActionExecutor）和 Core 组件没有前端插件，它们的设置界面由 DynamicForm + Schema 驱动。

3. **DynamicFormField 递归渲染机制** — Object 数组的子字段通过 `<DynamicFormField>` 递归渲染自身。传入的 `definition` 由 `FieldDefinition` 包装为 `{ field: fd, order: 0 }`。`FieldDefinition` 统一从 `bridge/contract.ts` 导入，组件内不要重复定义。

4. **bridge events 已接入** — `onConfigChanged` / `onInstallationEvent` 已在 SearchView 和 SettingsView 中监听。新增功能时注意不要重复注册或遗漏取消。

5. **前后端类型对齐** — Rust `#[serde(rename_all = "camelCase")]`，TypeScript 侧使用 camelCase。修改类型时两端都要同步。`ListItem` 已包含 `targetType` 字段，供 ResultItemProvider / ActionInjector 匹配。

6. **Path 别名已配置** — `@/` 映射到 `src-ui-new/`，tsconfig.json + vite.config.ts 均已配置。新文件使用 `@/` 前缀导入。

7. **代码风格** — Vue 3 `<script setup>` + TypeScript + Naive UI。组件用 props/emits 通信。复杂状态抽到 composables。`noUnusedLocals: true` + `noUnusedParameters: true`，未使用的 `v-for` 变量前缀 `_`。

8. **Theme/Language 已由 ConfigManager 统一管理** — 新建 `AppearanceConfigComponent`（Core 组件），theme（system/light/dark）和 language（zh-Hans/en）均由后端持久化。前端 theme-store 通过 `config_get_settings("appearance")` 加载，`config_apply_settings` 保存。

9. **ResultActions 交互模型** — `search-store` 有 `selectedActionIndex` 状态。Tab 键循环选中 action；Ctrl+1-9 触发第 N 个 action；Enter 触发当前选中的 action（或默认 action）。`ResultActions` 组件通过 `@execute(actionIndex)` emit 支持鼠标点击触发。

10. **设置窗口跨窗口同步** — SettingsView `onMounted` 注册 `onConfigChanged` 监听，当其他窗口修改当前选中组件的配置时自动重载。DynamicForm 通过 `watch(currentSettings)` 同步外部更新到 `localValues`。

11. **虚拟 candidate_id 约定** — 插件模式下 `doConfirm` 使用 `candidateId: 0`（虚拟值），后端按 `plugin_id` 路由而不依赖 `candidate_id`。此约定已在 `search-store.ts` 和 `CalculatorPanel.vue` 中注释说明。

12. **doConfirm 错误处理** — Phase 4 Code Review 后添加了 try-catch，执行失败时不清理输入状态，允许用户重试。

---

## Phase 5 设计要点

Phase 5 的核心目标是**打磨与完善**，将项目从"功能可用"提升到"产品可用"。

### 5.1 必须完成的模块

#### 国际化 i18n（`src-ui-new/i18n/`）

```
src-ui-new/i18n/
├── index.ts           # i18n 初始化 + vue-i18n 配置
└── locales/
    ├── zh-Hans.json   # 简体中文
    └── en.json        # 英文
```

- 使用 `vue-i18n` 库
- 覆盖搜索界面静态文本（placeholder、无结果、就绪等）
- 覆盖设置界面静态文本（常规、搜索管道、插件、外观、关于等）
- Theme store 持久化语言选择

#### 右键菜单集成（`ContextMenu`）

- `ContextMenu` 组件已存在但未使用
- 在 `ResultItem` 上右键时弹出 ContextMenu
- 菜单项应包含：执行默认动作、打开文件位置（Path 类型）、以管理员运行（Path/App 类型）、复制路径（Path 类型）

#### 虚拟滚动（`ResultList`）

- 当结果数超过 ~50 项时启用虚拟滚动
- 使用 `@tanstack/vue-virtual` 或 Naive UI 内置方案
- 保持现有键盘导航（上下键选择、Enter 确认）

#### 统一错误处理

- `commands.ts` 添加统一错误包装层（目前错误被静默吞掉）
- 使用 Naive UI `useNotification` 或 `useMessage` 显示全局错误
- 后端关键路径添加日志

### 5.2 可选但推荐的模块

#### EverythingPlugin 后端实现

- 文件位置：`src-tauri/src/plugin/triggerable/everything_plugin.rs`（当前为空）
- 触发词：`>` 或 `/`
- 通过 Everything SDK 搜索文件系统
- 返回搜索模式的 `QueryResponse::List`（而非 CustomPanel）

#### 搜索管道动态重建

- `session_router.rs` 中有 TODO 标记
- SearchEngine / ScoreBooster 配置变更时重建 `SearchPipeline`
- 不关闭已有插件即动态替换引擎

#### 第三方插件加载器（`plugins/loader.ts`）

```
src-ui-new/plugins/
└── loader.ts   # 第三方插件加载
```

- 读取指定目录下的 `plugin.json` manifest
- 动态 `import()` 插件模块
- 调用 `pluginManager.register()` 注册

#### 更多内置插件或 Provider 用例

- `web-search` 面板：后端 WebSearchPlugin 返回 CustomPanel，前端渲染 WebSearchPanel
- 为 `Url` targetType 注册 `ResultItemProvider`，在结果项中显示域名图标
- 为 `Path` targetType 注册 `ActionInjector`，注入「以管理员运行」动作

### 5.3 与 Phase 4 的衔接

- PluginManager 已经就绪，新增插件只需注册即可
- `ListItem.targetType` 已可用，ResultItemProvider / ActionInjector 可直接匹配
- Bridge events 通路已确保插件配置变更实时反映
- Theme 持久化机制可为 i18n 提供一致的语言选择持久化

---

## 验收标准

1. `vue-tsc --noEmit` 零错误
2. `cargo check` 零错误
3. i18n 框架正常工作，至少支持中英文切换
4. ContextMenu 在结果项右键时正确弹出，菜单项功能正常
5. 大结果集（>100 项）时虚拟滚动启用，键盘导航不受影响
6. 前端错误通过统一通知组件展示，不再静默吞掉
7. `bridge/events.ts` 的监听器在组件卸载时正确取消
