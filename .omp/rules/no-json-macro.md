---
name: no-json-macro
description: 禁止在内部模块用 json! 手写 JSON；内部数据必须用 serde 结构体，只有对外接口可传输动态 JSON
condition: 'json!\s*\('
scope: "tool:edit(src-tauri/**), tool:write(src-tauri/**), tool:edit(crates/**), tool:write(crates/**), tool:edit(zerolaunch-cli/**), tool:write(zerolaunch-cli/**)"
---

# 内部数据必须用结构体序列化，禁止手写 JSON

## 约束

- Rust 内部模块构造、传递、序列化数据时，**必须** 使用 `#[derive(Serialize, Deserialize)]` 的结构体 / 枚举，字段显式 `#[serde(rename = "...")]`（见 `serde-rename` 规则）。
- **禁止** 用 `json!({...})` 在内部拼数据；也禁止让 `serde_json::Value` 作为内部 DTO 字段或函数参数穿过业务层。
- 内部产物（诊断包、缓存文件、快照、导出格式）同样适用：先定义类型，再 `to_vec` / `to_vec_pretty`。
- 原因：结构体让编译器校验字段名与类型；`json!` 的键名只是字符串，改名、漏字段、类型漂移编译器都不会报错，失败推迟到运行期。

## 允许使用 `json!` / `Value` 的边界

只有「对外接口」可以在形状确实动态时直接处理 JSON，且必须限制在边界函数内：

1. IPC 命令中形状由动态 schema 决定的既有例外：`config_get_settings`、`config_execute_action`、`inspector_get_state`。
2. 插件 JSON-RPC 协议载荷（`plugin-protocol` / `plugin-host` 的 `params` / `result`），能力由第三方插件决定。
3. 第三方 HTTP API 的请求 / 响应（如插件市场的 GitHub API）——解析后必须立刻转成强类型再进入内部。
4. 测试中的期望值 / 夹具构造。

边界规则：

- `json!` 只能出现在上述边界函数的函数体内，**不得** 把 `Value` 继续传给内部模块。
- 即使是边界，能定类型就定类型：实体字段必须用现有类型序列化；`json!` 只允许做单键外层包装（如 `{ "models": entries }`）。
- 已有权威类型或投影类型时，直接复用它，不要用 `json!` 重拼同类形状（见 `projection-single-source`）。

## 示例

```rust
// ❌ 内部手写 JSON：字段名是字符串，编译期无法校验
fn build_diagnostics(components: &[ComponentInfo]) -> Vec<u8> {
    let payload = json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "components": components.iter().map(|c| json!({ "id": c.id })).collect::<Vec<_>>(),
    });
    serde_json::to_vec(&payload).unwrap()
}
```

```rust
// ✅ 结构体 + serde：字段、类型、序列化键名都由编译器守
#[derive(Serialize)]
struct DiagnosticsSnapshot {
    /// 应用版本。
    #[serde(rename = "appVersion")]
    app_version: String,
    /// 组件清单，复用现有权威投影。
    #[serde(rename = "components")]
    components: Vec<ComponentInfo>,
}

fn build_diagnostics(components: Vec<ComponentInfo>) -> Result<Vec<u8>, BridgeError> {
    let snapshot = DiagnosticsSnapshot {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        components,
    };
    to_vec_pretty(&snapshot).map_err(|e| BridgeError::internal(e.to_string()))
}
```

## 自查

- 这个 `json!` 是在对外边界函数里吗？不是 → 改成结构体。
- 结构体的字段都有 `#[serde(rename)]` 与注释吗？（`serde-rename`、`struct-doc-comment`）
- 是否已有可复用的类型？有 → 直接用，不要再造投影或手拼键名。
