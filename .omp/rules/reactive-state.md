---
description: Vue 响应式状态更新 — 容器内对象/数组采用整体替换风格，Store 方法封装更新逻辑
condition: "\\.value\\s*="
scope: "tool:edit(*.vue), tool:edit(*.ts), tool:write(*.vue), tool:write(*.ts)"
---

# 响应式状态更新

- Store 状态统一存放在 `ref()`/`reactive()` 容器中；容器内对象更新采用整体替换风格：
  `state.value = { ...state.value, [key]: val }`（数组同理 `state.value = [...state.value, item]`）
- 说明：`ref` 容器内层对象是深层 reactive 代理，直接写 `state.value.key = val` 也能触发追踪；
  整体替换是代码库既有的不可变更新约定，保证更新点集中、diff 可读——按约定执行，不以"能否触发"为判据
- Store 暴露的方法 **必须** 封装状态更新逻辑，组件不直接改写 store 状态
