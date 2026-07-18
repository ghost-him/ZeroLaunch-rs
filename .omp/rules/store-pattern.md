---
description: Pinia Store 模式 — setup 语法 defineStore，每个 store 管理一个关注点，在 mount 前初始化
condition: "defineStore"
scope: "tool:edit(*.vue), tool:edit(*.ts), tool:write(*.vue), tool:write(*.ts)"
---

# Store 模式

- 使用 `defineStore('id', () => { ... })` setup 语法。**禁止** 使用 Options Store
- 每个 store 管理 **一个** 关注点（search、config、theme、plugin）
- Store 在 `main.ts` / `settings-main.ts` 中 mount 前初始化。**禁止** 在组件内首次创建 store
