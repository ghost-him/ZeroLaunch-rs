---
description: GPUI 设置页按组件实现，schema 只保留数据契约
condition: "SettingDefinition|SchemaBuilder|SchemaKind|WidgetHint|FieldUiMetadata|ArrayUiKind"
scope: "tool:edit(crates/plugin-api/src/config/**), tool:write(crates/plugin-api/src/config/**), tool:edit(crates/zerolaunch-rs/src/builtin_plugin/config/**), tool:write(crates/zerolaunch-rs/src/builtin_plugin/config/**), tool:edit(app/src/settings/**), tool:write(app/src/settings/**)"
---

# 设置 schema 的职责

- `SettingDefinition` / `SchemaKind` / `WidgetHint` 是数据契约：默认值、校验、持久化、AI 说明书检索与第三方插件 schema 共用。
- GPUI 内置设置页按组件实现视图，直接读写强类型设置；不要为通用 schema 再造第二套渲染协议。
- 第三方插件没有 UI 脚本时，使用 text / number / toggle / select / path 的极简兜底表单。
- 新增 `SchemaKind` 变体时，同时更新 GPUI 字段渲染与第三方插件兜底表单。
- 设置页的数据来源始终是 `ConfigManager`；视图不得缓存第二份配置真相。
