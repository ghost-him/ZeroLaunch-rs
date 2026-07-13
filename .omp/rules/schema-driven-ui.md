---
description: Schema 驱动设置 UI — 后端 SettingDefinition 定义 → 前端通用渲染，禁止为特定组件创建专用设置页
condition: "SettingDefinition|SchemaBuilder|DynamicFormField|SettingType|ArrayUiHint"
scope: "tool:edit(*.vue), tool:edit(*.ts), tool:write(*.vue), tool:write(*.ts)"
interruptMode: tool-only
---

# Schema 驱动的设置 UI

- 后端定义 `SettingDefinition`（通过 SchemaBuilder）→ 前端通用渲染
- **禁止** 为特定组件创建专用 Vue 设置页面（除非该组件有 `DetailPreviewPanel` 类扩展需求）
- `DynamicFormField.vue` 是唯一的字段渲染分发器。新增字段类型 → 在此添加分支
- 新增 `SettingType` 变体 → 同步更新 `bridge/contract.ts` + `utils/schemaTypes.ts`
