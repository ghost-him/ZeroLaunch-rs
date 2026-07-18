---
description: 弹性填充布局规范 — 在 flex 列容器中用 flex:1 替代 height:100% 实现自适应
condition: "flex: 1|height: calc\\(|height: 100%"
scope: "tool:read(*.vue), tool:edit(*.vue), tool:write(*.vue)"
---

# 弹性填充布局规范

## 核心模式

在 flex 列容器中，用 `flex: 1` + 内层 `overflow-y: auto` 替代 `height: 100%` + 外层 `overflow-y: auto`，实现内容区自适应填充：

```vue
<style scoped>
/* ✅ 正确：弹性填充模式 */
.outer {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden; /* 外层裁剪溢出 */
}
.scroll-content {
  flex: 1;
  min-height: 0;      /* 允许收缩到内容高度以下 */
  overflow-y: auto;    /* 内层管理滚动 */
}
.fixed-footer {
  flex-shrink: 0;      /* 底部操作栏不压缩 */
}

/* ❌ 错误：固定高度模式 */
.scroll-content {
  height: calc(100% - 100px); /* 硬编码偏移，不灵活 */
  overflow-y: auto;
}
</style>
```

## 模式对照表

| 行数 | 旧模式（避免） | 新模式（推荐） |
|------|---------------|---------------|
| 外层容器 | `height: 100%` + 无 flex | `display: flex; flex-direction: column` |
| 弹性填充区 | `height: calc(100% - Npx)` | `flex: 1; min-height: 0` |
| 溢出管理 | 外层 `overflow-y: auto` | 外层 `overflow: hidden` + 内层 `overflow-y: auto` |
| 固定头部/尾部 | 固定高度 + margin 计算 | `flex-shrink: 0` |
| 嵌套 flex | 各层独立计算高度 | 每层传播 `flex: 1` + `min-height: 0` |

## 关键规则

### 1. `min-height: 0` 链

在 flex 列中，子项的默认 `min-height: auto` 会阻止其收缩到内容高度以下。**所有**参与 flex 伸缩的中间节点**必须**设置 `min-height: 0`，否则内层 `overflow-y: auto` 不会生效。

```vue
<!-- 正确的 min-height: 0 链 -->
.outer { display: flex; flex-direction: column; }         <!-- 容器 -->
.middle { flex: 1; min-height: 0; }                        <!-- 中间节点 -->
.inner-scroll { flex: 1; overflow-y: auto; min-height: 0; } <!-- 实际滚动区 -->
```

### 2. `flex-shrink: 0` 标记固定区域

不希望随容器缩放而压缩的区域（标题、操作栏、状态提示）**必须**添加 `flex-shrink: 0`：

```vue
.header { flex-shrink: 0; }    /* 标题栏不压缩 */
.footer { flex-shrink: 0; }    /* 操作按钮区不压缩 */
```

### 3. 滚动所有权归最内层

- 外层使用 `overflow: hidden`，只负责裁剪，不管理滚动
- 最内层的实际内容区使用 `overflow-y: auto` 管理垂直滚动
- 确保只有一个可滚动元素活跃，避免嵌套滚动条

## 适用场景

- 设置面板的内容区（`DynamicForm.form-groups` 接管滚动）
- 列表-详情分栏布局（`ListDetailPanel.panel-detail` 用 flex 填充）
- 标签页内容区（`CategoryViewTabs` / `CategoryViewPipeline` 的 n-tabs 适配）
- 任何需要"内容填充剩余高度，操作栏保持在底部"的布局
