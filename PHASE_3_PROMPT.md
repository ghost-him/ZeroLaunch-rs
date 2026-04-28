# Phase 3 前端重构提示词

## 当前进度

Phase 1（基础设施）和 Phase 2（搜索界面核心）已完成。`src-ui-new/` 下的代码已通过 `vue-tsc --noEmit` + `vite build` + `cargo check`。

### 已完成
- bridge 层：contract.ts / commands.ts / events.ts（前后端通信契约）
- Pinia stores：search-store / config-store / theme-store
- Composables：useSearch / useKeyboard / useSettings / useWindowResize
- 搜索界面组件：SearchBar / ResultList / ResultItem / ResultActions
- 布局组件：WindowFrame / Footer / ContextMenu
- 面板：PluginPanelHost（桩）/ EmptyState
- 设置组件：SettingsSidebar / DynamicForm / DynamicFormField / ConfigActionButton / PluginSettingsHost（桩）
- 独立设置窗口：SettingsApp.vue + settings-main.ts + setting_window.html
- 窗口尺寸自适应：useWindowResize（空闲态收缩，搜索态展开）

### 待完成（Phase 3：设置界面完善）
- DynamicFormField Array 复杂类型：Object item / Table / MasterDetail → 当前回退为 JSON 编辑 + console.warn
- PluginSettingsHost 集成 PluginManager（需 Phase 4 前置）
- 前端插件系统（Phase 4）：plugins/types.ts / manager.ts / loader.ts / plugin-store.ts

### 待完成（Phase 5：打磨）
- 国际化 i18n
- 右键菜单 ContextMenu 集成
- 浅色/深色主题完善
- 虚拟滚动优化
- 错误处理 & 边界情况（目前 commands.ts 没有统一的错误处理包装层）

## 需要注意的点

1. **不要修改 Rust `commands/` 目录下的代码** — 这些是旧系统代码，已经在新系统中通过 `bridge.rs` 和 `config_file.rs` 重新实现。但现在 commands/ 下可能仍有引用，删除前需确认无依赖。

2. **`window_position.rs` 和 `window_effect.rs`** — 前者只剩位置逻辑（size 已被前端替代），后者是旧系统窗口效果，尚未重构。这两个文件最终都会被删除，但不要在 Phase 3 就删，除非相关功能已完全在新架构中落地。

3. **设置窗口是独立 Tauri 窗口** — Rust 后端在 `lib.rs::init_setting_window()` 中创建（label=`setting_window`），前端只负责 show/hide。不要在 `useSettings.ts` 中重新创建窗口。

4. **ConfigActionButton 的 props 签名** — 需要 `configAction`（匹配 ConfigActionDef.action）和 `fieldKey`（提取返回结果中对应字段的值）。两个职责不同，不要混淆。

5. **DynamicFormField 的 Array 类型** — 当前只实现了 Primitive + Tags/Default。遇到 Object item 或 Table/MasterDetail hint 时回退为 JSON 编辑 + console.warn。实现完整 Array 类型时需支持嵌套表单。

6. **前后端类型对齐** — Rust `#[serde(rename_all = "camelCase")]`，TypeScript 侧使用 camelCase。修改类型时两端都要同步。`NEW_FRONTEND_REQUIREMENTS.md` 的 §3.2 和 §12 有详细说明。

7. **代码风格** — 参考已有代码的模式：Vue 3 `<script setup>` + TypeScript + Naive UI。组件用 props/emits 通信。复杂状态抽到 composables。

8. **`defaultEnabled` 字段** — Rust `ComponentInfo` 和 TS `ComponentInfo` 都已添加。但 Rust `Configurable` trait 的 `default_enabled()` 方法返回 `true` 是默认实现，确保新增组件不会遗漏。

## 下一步建议

1. 先对当前代码做一次 review（特别是 DynamicFormField 和 useWindowResize），确认没有遗漏
2. Start with the simplest remaining Phase 3 tasks (e.g., error handling wrapper in commands.ts)
3. 如果决定进入 Phase 4（前端插件系统），需要先设计 PluginManager 的 API 再写代码
