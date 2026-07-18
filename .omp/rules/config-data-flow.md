---
description: 配置数据流分层规范 — 运行时类型、存储类型、转换函数的三层职责分离
condition: "get_settings"
scope: "tool:read(src-tauri/src/builtin_plugin/config/**), tool:edit(src-tauri/src/builtin_plugin/config/**), tool:write(src-tauri/src/builtin_plugin/config/**), tool:read(src-tauri/src/core/bias_rule.rs), tool:edit(src-tauri/src/core/bias_rule.rs), tool:write(src-tauri/src/core/bias_rule.rs), tool:read(src-tauri/src/bootstrap.rs), tool:edit(src-tauri/src/bootstrap.rs), tool:write(src-tauri/src/bootstrap.rs), tool:read(src-tauri/src/plugin_framework/session_router.rs), tool:edit(src-tauri/src/plugin_framework/session_router.rs), tool:write(src-tauri/src/plugin_framework/session_router.rs), tool:read(src-tauri/src/plugin_framework/candidate_pipeline.rs), tool:edit(src-tauri/src/plugin_framework/candidate_pipeline.rs), tool:write(src-tauri/src/plugin_framework/candidate_pipeline.rs)"
---

# 配置数据流分层规范
| 层 | 位置 | 包含 | 禁止 |
|---|---|---|---|
| **运行时类型** | `core/` 或 `crates/plugin-api/` | 消费方需要的纯数据 struct，`#[derive(Debug, Clone)]` | `#[derive(Serialize, Deserialize)]`、持久化逻辑 |
| **存储类型**（Settings） | `builtin_plugin/config/<component>.rs` | serde struct，`#[derive(Serialize, Deserialize)]`，每字段 `#[serde(rename, default)]` | 引用 `ConfigManager`、包含业务方法 |
| **转换函数** | `builtin_plugin/config/<component>.rs` | `pub(crate) fn settings_to_xxx(settings: &Settings) -> RuntimeType` | 依赖 `ConfigManager` |

## 依赖方向

- 运行时类型（core/）→ 无业务依赖，被所有层引用
- 存储类型 + 转换函数（builtin_plugin/）→ 可引用 `core/` 中的运行时类型
- 消费方（bootstrap.rs / plugin_framework/）→ 可引用 builtin_plugin/ 中的 Settings 类型和转换函数

**禁止**：
- 存储类型/转换函数所在模块 import `ConfigManager`
- 运行时类型做 serde 反序列化（那属于转换函数的职责）
- 消费方直接 `serde_json::from_value::<RuntimeType>` 跳过 Settings 类型

## 消费方标准写法

```rust
// bias 示例
let rules: Vec<BiasRule> = config_manager
    .get_settings("bias-config")
    .and_then(|v| serde_json::from_value::<BiasSettings>(v).ok())
    .map(|s| bias_settings_to_rules(&s))
    .unwrap_or_default();

// hotkey 示例
let hotkey_config = config_manager
    .get_settings("hotkey-config")
    .and_then(|v| serde_json::from_value::<HotkeySettings>(v).ok())
    .map(|s| settings_to_hotkey_config(&s));
```

## 三种生命周期角色

| 组件 | 运行时类型位置 | 存储类型位置 | 转换函数位置 |
|---|---|---|---|
| 快捷键（hotkey） | `plugin-api/services/hotkey/types.rs` (HotkeyConfig) | `hotkey_config.rs` (HotkeySettings) | `hotkey_config.rs` |
| 固定偏移（bias） | `core/bias_rule.rs` (BiasRule) | `bias_config.rs` (BiasSettings) | `bias_config.rs` |

新增类似组件时**必须**按此表创建对应的三件套。
