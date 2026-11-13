---
description: embedding 输入模板契约 — 模板仅存于 model_profiles.rs 档案库，task_type 必填，{0}/{name:default} 占位符，无档案模型裸文本直传
condition: "model_profiles|task_templates|SemanticTask|template_for|profile_for|compose_embedding_texts|EmbeddingModelProfile|task_type|EmbeddingTemplateArgs"
scope: "tool:read(crates/zerolaunch-rs/src/core/model/model_profiles.rs), tool:edit(crates/zerolaunch-rs/src/core/model/model_profiles.rs), tool:write(crates/zerolaunch-rs/src/core/model/model_profiles.rs), tool:read(crates/zerolaunch-rs/src/core/model/compose.rs), tool:edit(crates/zerolaunch-rs/src/core/model/compose.rs), tool:write(crates/zerolaunch-rs/src/core/model/compose.rs), tool:read(crates/plugin-api/src/services/model/types.rs), tool:edit(crates/plugin-api/src/services/model/types.rs), tool:write(crates/plugin-api/src/services/model/types.rs), tool:read(crates/zerolaunch-rs/src/builtin_plugin/score_booster/**), tool:edit(crates/zerolaunch-rs/src/builtin_plugin/score_booster/**), tool:write(crates/zerolaunch-rs/src/builtin_plugin/score_booster/**), tool:read(crates/zerolaunch-rs/src/builtin_plugin/triggerable/translator/**), tool:edit(crates/zerolaunch-rs/src/builtin_plugin/triggerable/translator/**), tool:write(crates/zerolaunch-rs/src/builtin_plugin/triggerable/translator/**)"
---

# Embedding 输入模板契约

语义任务 → 输入文本的模板映射由**模型档案库**（`core/model/model_profiles.rs`）唯一持有。`task_templates` 是 `(SemanticTask, 模板字符串)` 静态表。

## 模板格式

- 模板占位符用命名槽：`{0}`（input 文本）、`{name:default}`（命名参数，如 `{title:none}` 取 title 参数、缺省用 `none`）
- 渲染在 `compose.rs` 的 `compose_embedding_texts()` 完成
- **无档案模型**（`profile_for()` 返回 None）：所有任务一律裸文本直传，不加任何前缀/模板

## 约束

- **禁止** 在消费方（ai-search 等插件、translator）内联拼接任务前缀/模板字符串——必须走 `template_for(task, model_id)`/`compose_embedding_texts`
- 新增内置模型档案 → 在 `model_profiles.rs` 的 `PROFILES` 表添加 `EmbeddingModelProfile { id_prefix, task_templates }` 条目；修改模板只改档案库，不动消费方
- `SemanticTask` 枚举（`plugin-api/src/services/model/types.rs`）新增变体时：检查 `model_profiles.rs` 每个档案的 `task_templates` 是否需补对应模板——缺失变体在 `template_for` 返回 None，按裸文本直传兜底，不会报错但语义分可能异常
- 消费方请求必须带 `task_type`（`EmbeddingTemplateArgs` 命名语义字段），**禁止** 自行拼装最终输入文本后当裸文本传——那会绕过档案库模板
