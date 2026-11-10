---
description: 当前 Vue 设置 UI 由 schema 通用渲染；GPUI 迁移后改为按组件实现设置视图，schema 仅保留数据契约
condition: "SettingDefinition|SchemaBuilder|DynamicFormField|SchemaKind|WidgetHint|FieldUiMetadata|ArrayUiKind"
scope: "tool:edit(crates/plugin-api/src/config/**), tool:write(crates/plugin-api/src/config/**), tool:edit(src-tauri/src/builtin_plugin/config/**), tool:write(src-tauri/src/builtin_plugin/config/**), tool:edit(src-ui/**/*.vue), tool:edit(src-ui/**/*.ts), tool:write(src-ui/**/*.vue), tool:write(src-ui/**/*.ts)"
---

# Schema 驱动的设置 UI

- 后端定义 `SettingDefinition`（通过 SchemaBuilder）→ 前端通用渲染
- **禁止** 为特定组件创建专用 Vue 设置页面（除非该组件有 `DetailPreviewPanel` 类扩展需求）
- `DynamicFormField.vue` 是唯一的字段渲染分发器。新增字段类型 → 在此添加分支
- 新增 `SchemaKind` 变体（Rust 侧 `crates/plugin-api/src/config`）或 `WidgetHint`/`FieldUiMetadata` 变体 → 前端同步更新 `bridge/contract.ts` + `utils/schemaTypes.ts`；`ArrayUiKind`（前端 `schemaTypes.ts`）与 Rust `WidgetHint::Array` 系列对应

## 迁移期边界（GPUI）

- 本规则只约束当前 Vue 前端（`src-ui/`）与第三方插件配置的跨进程契约。
- GPUI 设置页按组件实现专用视图（迁移阶段 4），不再由 `SettingDefinition` 通用渲染；`SettingDefinition` / `SchemaKind` 仅保留为数据契约（默认值、校验、持久化、AI 说明书、第三方插件 schema）。
- 禁止为新的 GPUI 视图继续扩展 `WidgetHint` 的渲染语义；阶段 4 删除 `DynamicForm.vue` 后，本规则随 `src-ui/` 一并删除。
