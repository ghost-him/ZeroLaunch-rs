---
description: Vue 组件编写规范 — script setup + Composition API，defineProps/defineEmits 泛型，style scoped，CSS 变量
condition: "<script setup|defineProps|defineEmits|<style"
scope: "tool:edit(*.vue), tool:write(*.vue)"
interruptMode: tool-only
---

# Vue 组件编写规范

- **必须** 使用 `<script setup lang="ts">`。**禁止** 使用 Options API
- **必须** 使用 `defineProps<T>()` 和 `defineEmits<T>()` 的泛型形式
- 所有组件样式 **必须** 使用 `<style scoped>`
- 颜色、字号 **必须** 使用 CSS 变量（如 `var(--text-primary)`、`var(--font-size-sm)`）。**禁止** 在颜色和字号上使用硬编码值。
- **必须** 通过 `bridge/commands.ts` 封装所有 Tauri API 调用
- 所有业务逻辑 **必须** 通过 IPC 委托后端（见 `.omp/RULES.md` 的前后端职责边界）
