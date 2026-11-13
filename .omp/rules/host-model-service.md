---
description: 宿主统一模型服务层 — ModelManager 聚合 ModelProvider，经 ModelService trait 暴露，translate/语义搜索消费 host/model.* RPC
condition: "ModelManager|ModelProvider|ModelService|model\\.list|model\\.chat|model\\.embedding|model\\.similarity|MODEL_OLLAMA_CONFIG_ID|MODEL_OPENAI_CONFIG_ID|ModelError"
scope: "tool:read(crates/zerolaunch-rs/src/core/model/**), tool:edit(crates/zerolaunch-rs/src/core/model/**), tool:write(crates/zerolaunch-rs/src/core/model/**), tool:read(crates/zerolaunch-rs/src/plugin_framework/host_handler.rs), tool:edit(crates/zerolaunch-rs/src/plugin_framework/host_handler.rs), tool:write(crates/zerolaunch-rs/src/plugin_framework/host_handler.rs), tool:read(crates/plugin-api/src/services/model/**), tool:edit(crates/plugin-api/src/services/model/**), tool:write(crates/plugin-api/src/services/model/**), tool:read(crates/zerolaunch-rs/src/builtin_plugin/triggerable/translator/**), tool:edit(crates/zerolaunch-rs/src/builtin_plugin/triggerable/translator/**), tool:write(crates/zerolaunch-rs/src/builtin_plugin/triggerable/translator/**)"
---

# 宿主统一模型服务层

宿主（crates/zerolaunch-rs）对 AI 模型的访问统一收敛到 `crates/zerolaunch-rs/src/core/model/` 模型服务层，**不分散在消费插件中**。

## 结构

- `ModelManager`（`core/model/mod.rs`）聚合所有 `Arc<dyn ModelProvider>`，本身 `impl ModelService`。按 `model_id` 的 `{provider}/` 前缀路由到对应 provider
- 模型配置组件 id：`MODEL_OLLAMA_CONFIG_ID = "model-ollama-config"`、`MODEL_OPENAI_CONFIG_ID = "model-openai-config"`（`core/model/settings.rs` 常量）
- provider 实现位于 `core/model/`：`ollama_provider.rs`、`openai_compatible_provider.rs`、`embedding_cache.rs`（缓存）、`model_profiles.rs`（embedding 模板档案）、`compose.rs`（模板渲染）
- trait 定义在 `crates/plugin-api/src/services/model/`：`ModelService`（消费侧：list_models/chat/stream_chat/embedding/similarity）、`ModelProvider`（实现侧，额外含 `provider_id`/`cache_namespace`/`is_available`）

## 约束

- **禁止** 在 `builtin_plugin/` 的消费插件中直接实现模型接入/请求逻辑——统一经 `PluginHandle` 的 model 能力或宿主内部 `ModelManager` 调用
- 新增模型 provider 类型 → 注册进 `ModelManager::register_builtin_providers` + `handle_config_event`（`MODEL_*_CONFIG_ID` 配置变更时重建对应 provider）
- 配置组件 `apply_settings` 解析失败必须拒绝保存（如 `model_ollama_config.rs`），防止静默清空模型配置
- provider 的 `cache_namespace()` 用 `provider:base_url`，**禁止** 把全量 models JSON 掺入命名空间——否则无关配置改动会清空整个 embedding 缓存

## 对外 RPC

- 第三方插件经 `host/model.list`、`host/model.chat`、`host/model.embedding`、`host/model.similarity`（`plugin-protocol/src/methods.rs`）访问宿主模型能力
- `host_handler.rs` 分发这些 RPC 到 `ModelManager`，`ModelError` → JSON-RPC 错误码映射在 `model_error_to_rpc`
