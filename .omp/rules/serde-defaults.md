---
description: Serde 反序列化默认值强制规范 — 所有 from_str/from_value 反序列化的 struct 必须标注 #[serde(default)]，防止老配置缺失字段导致整体反序列化失败
condition: "#\\[derive\\(.*Deserialize|serde_json::from_str|serde_json::from_value"
scope: "tool:edit(*.rs), tool:write(*.rs)"
interruptMode: tool-only
---

# Serde 默认值强制规范

所有被 `serde_json::from_str` 或 `serde_json::from_value` 反序列化的 struct，**必须** 在反序列化方向上进行缺失字段保护。

## 规则

- **必须** 给 struct 的每个字段标注 `#[serde(default)]` 或 `#[serde(default = "fn_name")]`
- `bool` 字段用 `#[serde(default)]`（默认 `false`）。如果业务默认值不是 `false`，用 `#[serde(default = "fn")]` 指定
- `f64` / `u32` / `i32` 等数值字段 **必须** 用 `#[serde(default = "default_xxx")]` 指定业务默认值（`#[serde(default)]` 会得到 0.0 / 0，破坏业务语义）
- `String` 字段：空字符串是合法默认值时用 `#[serde(default)]`，否则用 `#[serde(default = "default_xxx")]`
- `Vec<T>` / `HashMap<K,V>` / `Option<T>` 字段：用 `#[serde(default)]`
- `serde_json::Value` 字段：用 `#[serde(default)]`（默认 `Value::Null`）

## 原因

老用户的配置文件是持久化的。新版本新增字段时，老 JSON 中缺失该字段 → `serde_json::from_str` 直接失败 → ConfigManager 中的 `.unwrap_or_default()` 把**所有**用户设置静默重置为出厂值。`#[serde(default)]` 让反序列化对缺失字段宽容，单个字段回退到默认值而非整体失败。

## 反序列化防护

- **必须** 在 struct 定义处（反序列化点）做缺失字段保护，而非依赖调用处的 `.unwrap_or_default()`（调用处的兜底在 `from_str` 整体失败时才触发，此时 **所有** 字段都丢失了）
- 所有 `Configurable` impl **必须** 定义带 `#[derive(Serialize, Deserialize)]` 的强类型 `Settings` struct，每个字段标注 `#[serde(rename = "...", default)]`，用 `RwLock<Settings>` 存储
- `apply_settings()` 中 **必须** 使用 `serde_json::from_value::<Settings>(settings).unwrap_or_default()` 反序列化。`get_settings()` 中 **必须** 使用 `serde_json::to_value(&*self.settings.read()).unwrap_or_default()` 序列化
- 所有字段访问 **必须** 通过强类型 struct 的字段
- keyword_optimizer / score_booster / triggerable 等插件如果使用自定义 inner struct（如 `RwLock<FooInner>`），**必须** 改为 `RwLock<FooSettings>`（带 `#[serde(rename, default)]` 的 serde struct），inner struct 仅保留运行时状态（非配置字段）
