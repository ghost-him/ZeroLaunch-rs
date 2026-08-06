---
name: no-skip-serializing-if
description: 禁止 #[serde(skip_serializing_if)] — 后端向前端传输数据必须完整，空值序列化为 null/空数组而非跳过字段
condition:
  - 'skip_serializing_if'
scope: "tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**), tool:edit(src-tauri/src/builtin_plugin/**), tool:write(src-tauri/src/builtin_plugin/**), tool:edit(src-tauri/src/plugin_framework/**), tool:write(src-tauri/src/plugin_framework/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**), tool:edit(crates/plugin-protocol/src/**), tool:write(crates/plugin-protocol/src/**)"
---

# 禁止 skip_serializing_if（IPC 数据完整传输）

后端向前端传输数据时 **必须** 完整传递：禁止使用 `#[serde(skip_serializing_if = "...")]`
跳过空/默认字段。

## 规则

- **禁止** 编写 `#[serde(skip_serializing_if = "...")]`，包括 `Option::is_none`、
  `Vec::is_empty`、`is_false` 等任何谓词
- 空值/默认值照常序列化，前端契约保持完整形状：
  - `Option<T>` → 序列化为 `null`，TS 侧 `T | null`
  - `Vec<T>` → 序列化为 `[]`，TS 侧必填数组
  - `bool`/数值默认值 → 照常输出
- `#[serde(default)]` **必须保留**（反序列化方向保护，见 `serde-defaults` 规则）——
  它只影响反序列化，不参与序列化跳过

**正确**：

```rust
#[derive(Serialize, Deserialize)]
pub struct SessionStateEvent {
    #[serde(rename = "panel")]
    panel: Option<PluginPanelInfo>,   // 序列化为 null，不跳过
    #[serde(rename = "triggerKeywords", default)]
    trigger_keywords: Vec<String>,    // 序列化为 []，不跳过
}
```

**错误**：

```rust
#[serde(rename = "panel", skip_serializing_if = "Option::is_none")]
panel: Option<PluginPanelInfo>,
```

## 原因

- 跨 IPC 契约是「完整形状」契约：前端 `contract.ts` 类型与后端结构一一对应，
  跳过字段制造 `undefined` 分支，前端需到处判空且无法区分「字段缺失」与「字段为空」
- 空数组/`null` 的体积开销可忽略，完整性收益远大于省下的几个字节
