---
description: 前端类型安全 — IPC 类型按同域成组声明在 contract.ts / commands.ts，类型守卫集中在 schemaTypes.ts
condition: "bridge/contract|bridge/commands|schemaTypes|isMyType|export interface|export type"
scope: "tool:edit(*.ts), tool:edit(*.vue), tool:write(*.ts), tool:write(*.vue)"
---

# 前端类型安全

- IPC 类型声明在两处，按「同域成组」归属，**禁止**把同一域的类型拆到两个文件：
  - `bridge/contract.ts` — 被多个模块/视图复用的跨域类型（搜索会话、配置 schema、Plugin Inspector、Debug Tools、help_ask 等）
  - `bridge/commands.ts` — 与命令函数同址的命令载荷类型（`BridgeError`、`InstalledPluginInfo`、`PluginManifest`、`MarketRepo`、`MarketCardMeta`、`MarketPackagePreview`、`MarketListResult`、`CliInfo`）
  - 判据：类型若依赖 `commands.ts` 本地声明的类型（如 `MarketCardMeta` 依赖 `PluginManifest`），或仅被单一视图消费，留在 `commands.ts`——`contract.ts` 不 import `commands.ts`，搬过去会造出两文件互相 `import type`（类型循环）
- 两种归属的字段名都 **必须** 与 Rust 字段级 `#[serde(rename = "camelCaseKey")]` 逐字对齐
- Schema 类型守卫集中在 `utils/schemaTypes.ts`。**禁止** 在组件中内联类型判断
- 边界说明：禁令针对**用户数据/IPC 载荷**的类型守卫（如判断某字段是否为 Record/数组）；对 `schema.type === 'array'` 这类 **schema 结构判别**（渲染分支）不受限——那是 UI 渲染逻辑，不是数据守卫

