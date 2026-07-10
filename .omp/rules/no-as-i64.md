---
description: 禁止用 as_i64() 从 serde_json::Value 读取数值 — 遇到浮点数静默返回 0
condition: "\\.as_i64\\("
scope: "tool:edit(*.rs), tool:write(*.rs)"
---

你正准备用 `as_i64()` 从 `serde_json::Value` 读取数值。前端可能对整数字段发送浮点数，`as_i64()` 遇到浮点数会静默返回 0，导致配置值丢失。

改用 `as_f64()` 再转型：

    value.as_f64().map(|v| v as i32).unwrap_or(default)

禁止：`value.as_i64().unwrap_or(default)`
