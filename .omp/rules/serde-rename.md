---
description: Serde 序列化规范 — 必须用字段级 #[serde(rename)] 显式标注每个字段和 variant，禁止用 rename_all
condition: "#\\[serde\\(rename|rename_all|pub(\\([^)]*\\))? struct|pub(\\([^)]*\\))? enum"
scope: "tool:edit(crates/zerolaunch-rs/src/plugin_framework/**), tool:write(crates/zerolaunch-rs/src/plugin_framework/**), tool:edit(crates/zerolaunch-rs/src/cli_server/**), tool:write(crates/zerolaunch-rs/src/cli_server/**), tool:edit(crates/zerolaunch-rs/src/core/config/**), tool:write(crates/zerolaunch-rs/src/core/config/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**), tool:edit(crates/plugin-protocol/src/**), tool:write(crates/plugin-protocol/src/**), tool:edit(crates/zerolaunch-rs/src/plugin_market/**), tool:write(crates/zerolaunch-rs/src/plugin_market/**)"
---

# Serde 序列化规范

跨进程协议（插件 JSON-RPC 与 CLI HTTP）的 JSON 键名统一使用 **camelCase**。

> **例外**：配置存储类型（`builtin_plugin/config/` 的 Settings struct）的键名使用 **snake_case**（见 config-naming 规则），本规则仅约束跨进程类型。

## 字段级 rename 强制

- **必须** 使用字段级 `#[serde(rename = "xxx")]` 显式标注每个字段和 variant 的 JSON 键名。即使 Rust 字段名与 JSON 键名相同，也必须显式标注以保持风格统一和可读性。
- `rename_all` 在 externally tagged enum 上只会重命名 variant 标签名，**不会** 重命名 variant 内部的字段名，导致前后端字段名不一致。

**正确**：

```rust
#[derive(Serialize, Deserialize)]
pub enum MyEnum {
    // 1. 显式标注变体标签名
    #[serde(rename = "myVariant")]
    MyVariant {
        // 2. 显式标注变体内部的每一个字段
        #[serde(rename = "innerField")]
        inner_field: String,
        #[serde(rename = "count")]
        count: u32,
    },

    // 无内部字段的变体也需标注
    #[serde(rename = "unitVariant")]
    UnitVariant,
}
```

- enum unit variant 和 variant with fields 均需显式标注：variant 标签加 `#[serde(rename = "...")]`，有内部字段的 variant 还需对每个内部字段标注。

## 序列化契约

- 第三方插件协议与 CLI HTTP 的字段名是跨进程契约；修改 Rust 字段时必须在同一 commit 同步协议类型或 CLI 消费方。
- GPUI 与后端库同进程，直接使用 Rust 类型，不生成 DTO 副本，也不存在跨语言类型同步步骤。
- 插件市场、会话投影等对插件或 CLI 暴露的类型同样按上述契约处理。
