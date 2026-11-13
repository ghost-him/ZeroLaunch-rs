---
name: no-skip-serializing-if
description: 禁止 #[serde(skip_serializing_if)] — 跨进程传输数据必须完整，空值序列化为 null/空数组而非跳过字段
condition:
  - 'skip_serializing_if'
scope: "tool:edit(crates/zerolaunch-rs/src/builtin_plugin/**), tool:write(crates/zerolaunch-rs/src/builtin_plugin/**), tool:edit(crates/zerolaunch-rs/src/core/model/**), tool:write(crates/zerolaunch-rs/src/core/model/**), tool:edit(crates/zerolaunch-rs/src/plugin_framework/**), tool:write(crates/zerolaunch-rs/src/plugin_framework/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**), tool:edit(crates/plugin-protocol/src/**), tool:write(crates/plugin-protocol/src/**), tool:edit(crates/zerolaunch-rs/src/plugin_market/**), tool:write(crates/zerolaunch-rs/src/plugin_market/**)"
---

# 禁止 skip_serializing_if（跨进程数据完整传输）

后端向第三方插件或 CLI 传输数据时 **必须** 完整传递：禁止使用
`#[serde(skip_serializing_if = "...")]` 跳过空/默认字段。

## 规则

- **禁止** 编写 `#[serde(skip_serializing_if = "...")]`，包括 `Option::is_none`、
  `Vec::is_empty`、`is_false` 等任何谓词
- 空值/默认值照常序列化，协议消费方保持完整形状：
  - `Option<T>` → 序列化为 `null`
  - `Vec<T>` → 序列化为 `[]`
  - `bool`/数值默认值 → 照常输出
- `#[serde(default)]` **必须保留**（反序列化方向保护，见 `serde-defaults` 规则）——
  它只影响反序列化，不参与序列化跳过

**正确**：

```rust
#[derive(Serialize)]
#[serde(tag = "kind")]
pub enum SessionStateEvent {
    Plugin {
        #[serde(rename = "interaction")]
        interaction: PanelInteraction,
        #[serde(rename = "panelContent")]
        panel_content: Option<PluginPanelContent>, // 序列化为 null，不跳过
        #[serde(rename = "inputMatch")]
        input_match: Option<InputMatch>,           // 序列化为 null，不跳过
    },
}
```

**错误**：

```rust
#[serde(rename = "panelContent", skip_serializing_if = "Option::is_none")]
panel_content: Option<PluginPanelContent>,
```

## 原因

- 跨进程协议是「完整形状」契约：插件与 CLI 消费方按固定结构反序列化，
  跳过字段制造 `undefined`/缺省分支，消费方需到处判空且无法区分「字段缺失」与「字段为空」
- 空数组/`null` 的体积开销可忽略，完整性收益远大于省下的几个字节
