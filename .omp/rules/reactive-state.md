---
description: Vue 响应式状态更新 — 必须用展开运算符创建新引用，禁止直接 mutate
condition: "\\.value\\s*=|state\\.value"
scope: "tool:edit(*.vue), tool:edit(*.ts), tool:write(*.vue), tool:write(*.ts)"
---

# 响应式状态更新

- 对象/数组更新 **必须** 使用展开运算符创建新引用：`state.value = { ...state.value, [key]: val }`（直接 mutate 如 `state.value.key = val` 不会触发 Vue 的响应式追踪）
- Store 暴露的方法 **必须** 封装状态更新逻辑
