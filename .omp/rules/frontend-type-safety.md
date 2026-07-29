---
description: 前端类型安全 — IPC 类型在 contract.ts 定义，类型守卫集中在 schemaTypes.ts
condition: "contract\\.ts|schemaTypes|isMyType"
scope: "tool:edit(*.ts), tool:edit(*.vue), tool:write(*.ts), tool:write(*.vue)"
---

# 前端类型安全

- 所有 IPC 类型在 `bridge/contract.ts` 中定义，与 Rust 字段级 `#[serde(rename = "camelCaseKey")]` 保持同步
- Schema 类型守卫集中在 `utils/schemaTypes.ts`。**禁止** 在组件中内联类型判断

