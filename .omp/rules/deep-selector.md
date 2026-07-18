---
description: :deep() 选择器使用规范 — 允许覆盖 UI 库样式以适配布局，但避免穿透业务组件
condition: ":deep("
scope: "tool:read(*.vue), tool:edit(*.vue), tool:write(*.vue)"
---

# :deep() 选择器使用规范

## 使用原则

`:deep()` 是 Vue `<style scoped>` 中穿透作用域边界的机制。**允许有限使用**，但需遵循以下原则：

### ✅ 允许使用

1. **覆盖 UI 库组件内部样式**（Naive UI 等）以适配 flex 布局或调整视觉表现：
   - `:deep(.n-tabs)` — 将 n-tabs 从 block 改为 flex 使其可伸缩
   - `:deep(.n-tabs-pane-wrapper)` — 让 tab 内容区域填充空间
   - `:deep(.n-modal-body)` — 调整对话框内部间距

2. **覆盖第三方插件宿主样式**（iframe PostMessage 桥）

### ❌ 禁止使用

1. **穿透到业务组件内部** — 业务组件的样式封装应当通过 props/slots 控制，不应用 `:deep()` 破坏封装
2. **作为全局样式替代品** — 如果需要在多个组件间共享样式，应在 `styles/` 中定义全局 CSS 变量

## 风险说明

- **UI 库升级兼容性**：`:deep()` 依赖的类名（如 `.n-tabs-pane-wrapper`）是 UI 库内部实现细节，版本升级可能变更。升级 naive-ui 后应检查所有 `:deep(.n-*)` 选择器是否仍生效
- **CSS 优先级冲突**：UI 库的 CSS 可能使用 `!important` 或高特异性选择器，`:deep()` 可能不足以覆盖。用复合类选择器（如 `.my-wrapper.n-tabs`）提高优先级
- **性能影响**：过度使用 `:deep()` 增加样式计算成本，应限制在布局适配所需的最小范围

## 推荐模式

```vue
<style scoped>
/* 正确：用复合类选择器提高优先级 */
.my-tabs-wrapper.n-tabs {
  display: flex;
  flex-direction: column;
}

/* 正确：仅穿透必要层级 */
.my-tabs-wrapper :deep(.n-tabs-pane-wrapper) {
  flex: 1;
  overflow: hidden;
}

/* 错误：不必要的穿透 */
.my-component :deep(.child-component .inner-element) { }
</style>
```
