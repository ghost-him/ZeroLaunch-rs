---
description: 禁止手写 SVG 图标 — 必须使用 lucide-vue-next 提供的组件
condition: "(<svg|h\\(\\s*['\"]svg)"
scope: "tool:edit(**/*.vue), tool:write(**/*.vue)"
---

# 图标库使用规范

禁止在 `.vue` 文件中手写 `<svg>` 标签（硬编码 SVG path）。所有图标 **必须** 使用 `lucide-vue-next` 提供的 Vue 组件。

## 适用范围

| 内容 | 必须用 lucide-vue-next | 允许手写 SVG |
|------|------------------------|-------------|
| 交互图标（齿轮、搜索、折叠箭头等） | ✓ | — |
| 状态图标（信息、警告、空状态等） | ✓ | — |
| 装饰性图标（列表项前缀、分类图标等） | ✓ | — |
| `IconDisplay.vue` 中的后备图标 | ✓（已更换为 `<Upload />`） | — |
| 完全由后端动态提供的图标（如插件图标 URL） | — | ✓（走 `<img>` 或 background-image） |
| 纯 CSS 绘制的装饰形状（圆点、线、箭头等非图标内容） | — | ✓ |

## 判断标准

**以下情况算违规：**

```vue
<!-- ❌ 禁止：手写 SVG path -->
<n-icon :size="16">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M12 2L2 7l10 5 10-5-10-5z" />
  </svg>
</n-icon>

<!-- ❌ 禁止：通过 Vue h() 函数手写 SVG -->
h('svg', …)
```

**以下情况正确：**

```vue
<!-- ✓ 正确：从 lucide-vue-next 导入组件 -->
<script setup lang="ts">
import { Settings } from 'lucide-vue-next'
</script>

<template>
  <n-icon :size="16">
    <Settings />
  </n-icon>
</template>

<!-- ✓ 正确：动态图标（由后端提供 URL） -->
<IconDisplay :src="dynamicIconUrl" />
```

## 使用指南

### 安装

```bash
bun add lucide-vue-next
```

### 导入

```typescript
import { Settings, Search, Puzzle, Palette, Info, Box, Bug, File, Layers, Upload, DollarSign, ChevronDown, ChevronUp } from 'lucide-vue-next'
```

### 配合 NIcon

所有 lucide-vue-next 组件可直接嵌入 Naive UI 的 `<n-icon>` 插槽：

```vue
<n-icon :size="16" color="var(--text-secondary)">
  <Search />
</n-icon>
```

### 图标查找

- Lucide 官网：https://lucide.dev/icons/
- 项目已使用的图标可在 `sidebarIcons.ts` 的 `iconMap` 中查阅
