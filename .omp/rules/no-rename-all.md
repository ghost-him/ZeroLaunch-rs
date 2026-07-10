---
description: 禁止用 serde rename_all 替代字段级 rename — 在 enum 上不重命名内部字段
condition: "rename_all\\s*="
scope: "tool:edit(*.rs), tool:write(*.rs)"
interruptMode: never
---

你用了 `#[serde(rename_all = "...")]`。本项目要求字段级 `#[serde(rename = "...")]` 显式标注。

原因：`rename_all` 在 externally tagged enum 上只重命名 variant 标签名，不会重命名 variant 内部字段名，导致前后端 JSON 字段名不一致。

正确做法：对每个字段和 variant 单独标注：

    #[derive(Serialize, Deserialize)]
    pub enum MyEnum {
        #[serde(rename = "myVariant")]
        MyVariant {
            #[serde(rename = "innerField")]
            inner_field: String,
        },
    }
