---
description: Serde 序列化规范 — 必须用字段级 #[serde(rename)] 显式标注每个字段和 variant，禁止用 rename_all
condition: "#\\[serde\\(rename|rename_all|derive\\(.*Serialize"
scope: "tool:edit(*.rs), tool:write(*.rs)"
interruptMode: tool-only
---

# Serde 序列化规范

IPC 类型 JSON 键名统一使用 **camelCase**，Rust 与 TypeScript 两侧 **必须** 一致。

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

## 前后端同步

- 前端类型在 `bridge/contract.ts` 中 **必须** 与 Rust 类型 `commands/bridge.rs` 和 `commands/config_file.rs` 保持同步
- 新增/重命名 Rust IPC 类型字段时：**同一 commit** 中更新 `bridge/contract.ts`
