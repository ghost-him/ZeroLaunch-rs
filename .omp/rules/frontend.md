---
description: 前端总览：技术栈、目录结构、多窗口架构
condition: ".*"
scope: "tool:read(src-ui/**), tool:edit(src-ui/**), tool:write(src-ui/**)"
interruptMode: never
---

# 前端总览

## 技术栈

- Vue 3 + Composition API (`<script setup lang="ts">`)
- Naive UI 组件库
- Pinia 状态管理
- TypeScript 严格模式
- Vite 构建
- vue-i18n 国际化

## 目录结构

| 目录 | 职责 | 放置规则 |
|------|------|---------|
| `bridge/` | IPC 契约层 | 类型定义 + 命令封装 + 事件监听 |
| `stores/` | Pinia 状态管理 | 每个关注点一个 store |
| `composables/` | 可复用逻辑 Hook | 有副作用的逻辑封装 |
| `router/` | Vue Router 配置 | 路由定义 |
| `views/` | 页面级组件 | 每个窗口入口对应一个 View |
| `components/` | UI 组件 | 按功能域子目录组织 |
| `components/settings/fields/` | 设置字段渲染器 | 每种 SettingType 一个组件 |
| `components/settings/fields/array/` | 数组 UI 策略 | 每种 ArrayUiHint 一个组件 |
| `plugins/built-in/` | 内置前端插件 | `import.meta.glob` 自动发现，目录约定 `built-in/<id>/index.ts` |
| `plugins/third-party-host/` | 第三方插件宿主 | iframe 宿主 + PostMessage 通信桥 |
| `plugins/built-in/_template/` | 前端插件模板 | 参考实现，不被 glob 扫描 |
| `utils/` | 纯工具函数 | 无副作用的工具 |
| `styles/` | 全局样式 | CSS 变量定义（variables.css + transitions.css） |
| `i18n/` | 国际化 | 语言文件 |

## 多窗口架构

- 搜索窗口入口：`main.ts` → `App.vue` → `SearchView.vue`
- 设置窗口入口：`settings-main.ts` → `SettingsApp.vue` → `SettingsView.vue`
- 两窗口共享 `bridge/`、`stores/`、`composables/`、`components/`
- 跨窗口状态同步通过 Tauri 事件（`config-changed`、`system-theme-changed`）
- **禁止** 假设两窗口共享内存状态。每窗口有独立的 Pinia 实例
