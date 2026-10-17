---
description: host↔plugin 载荷前向兼容契约 — 改 plugin-api 载荷字段/枚举变体/默认能力时必须遵守，避免旧版消费方整表解析失败
condition:
  - 'serde\(other\)'
  - 'capabilities'
  - 'pub enum'
scope: "tool:edit(crates/plugin-api/src/services/**), tool:write(crates/plugin-api/src/services/**), tool:edit(src-tauri/src/core/model/**), tool:write(src-tauri/src/core/model/**), tool:edit(src-tauri/src/builtin_plugin/config/**), tool:write(src-tauri/src/builtin_plugin/config/**)"
---

# host ↔ plugin 载荷前向兼容契约

`crates/plugin-api` 的类型是**已发布 SDK**（第三方插件按版本依赖），同一份 JSON 载荷由宿主与插件双方的
`serde` 读写。给这些类型"顺手加一个变体"或"体贴地放进默认值"，就会让旧插件在 `host/model.list` 这类
RPC 上**整表反序列化失败**——历史上 `ChatCapability::Tools` 正是这样切断 ai-search 的语义增强。

## 强制规则

1. **新增字段**：必须 `#[serde(default)]`（或 `Option`），缺失时回落到安全默认值；**禁止** `deny_unknown_fields`。
2. **"集合/提示型"枚举**（能力清单、相似度等"声明自己支持什么"的值）：必须带 `#[serde(other)]` 兜底变体
   （如 `Unknown` + `#[serde(rename = "unknown")]`），旧消费方遇到未知取值按"未声明该能力"降级而非整表失败。
   注意兜底只对**用该版本编译的消费方**生效，所以**在加新变体之前**就要补上：当前 `ChatCapability` /
   `EmbeddingCapability` 已具备，新增同类枚举照此办理。
3. **禁止把新变体写进默认值**：新增能力不得出现在默认配置里。检查三处——`default_*` 函数、
   schema `.default(...)`、provider 的 `unwrap_or` 回落分支。默认即声明会让"能力闸门"（provider 按能力校验请求）
   彻底失效，并立刻打断旧消费方；新能力**必须**由用户在模型条目里显式勾选。
4. **分派关键枚举变体**（决定行为的，如 `ModelKind::Chat/Embedding`、`ModelChatRole`）不能靠兜底降级：
   这类新变体属于载荷级 breaking，必须走 `PROTOCOL_VERSION` major（`crates/plugin-protocol/src/lib.rs` 的
   `protocol_version_compatible`）并 tick SDK 版本，再走 plugin-template → 下游同步。
5. **回归测试**：改上述类型时，在 `crates/plugin-api/src/services/model/types.rs` 的 `tests` 中补一条
   "未知取值仍能解析"的断言（照 `unknown_capabilities_are_tolerated` 的形状写）。

## 正反例

- ❌ 给 `ChatCapability` 加 `Tools`，同时把 `default_chat_capabilities()` 改成 `[Reasoning, Tools]`：
  能力闸门失效 + 旧插件 `host/model.list` 整表失败。
- ✅ 只加变体 + 保留 `Unknown` 兜底，`tools` 由用户在模型条目里勾选。
