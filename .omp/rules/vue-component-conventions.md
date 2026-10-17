---
description: Vue 组件编写规范 — script setup + Composition API，defineProps/defineEmits 泛型，style scoped，CSS 变量
condition: "<script setup|defineProps|defineEmits|<style"
scope: "tool:edit(*.vue), tool:write(*.vue)"
---

# Vue 组件编写规范

- **必须** 使用 `<script setup lang="ts">`。**禁止** 使用 Options API
- **必须** 使用 `defineProps<T>()` 和 `defineEmits<T>()` 的泛型形式
- 所有组件样式 **必须** 使用 `<style scoped>`
- 颜色 **必须** 使用 CSS 变量（如 `var(--text-primary)`；主色背景上的文字用 `var(--text-on-accent)`）。**禁止** 硬编码颜色（变量集见 `css-variable-contract` 规则）
- 字号 **应** 使用 `var(--font-size-*)`（`sm`/`md`/`base`/`lg`/`xl`）——这些变量可被用户的外观配置覆盖，硬编码 px 会忽略用户的字号设置。仓库现存历史硬编码字号（标题类居多，见 `SettingsView.vue`、`PluginMarket.vue`、`ListDetailPanel.vue`），新增或修改的样式 **必须** 优先复用已有 token；确无匹配 token 的标题类取值允许硬编码，但不得扩散（新取值应同时补 token）
- **必须** 通过 `bridge/commands.ts` 封装所有 Tauri API 调用
