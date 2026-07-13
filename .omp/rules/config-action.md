---
description: ConfigAction 规范 — 配置保存前的副作用测试（如 WebDAV 连通性），纯查询/计算操作，禁止修改组件状态
condition: "execute_config_action|config_actions|ConfigActionDef|ConfigAction"
scope: "tool:edit(*.rs), tool:write(*.rs)"
interruptMode: never
---

# ConfigAction 规范

## 用途：保存前测试

如果副作用必须决定配置能否保存（如 WebDAV 连通性测试）：**必须** 使用 `ConfigAction`。通过 `config_actions() → Vec<ConfigActionDef>` 声明，在 `execute_config_action(&self, action: &str, params: &serde_json::Value) -> Result<serde_json::Value, String>` 中实现。

前端 **必须** 将 `config_execute_action` 作为单独的用户触发操作调用，与保存流程解耦。

## 参数传递

- `execute_config_action(&self, action: &str, params: &serde_json::Value) -> Result<serde_json::Value, String>` 签名支持参数
- 无参数的动作（如 `detect_browsers`）：前端不传 params，后端收到 `Value::Null`
- 有参数的动作（如 `read_bookmarks`）：前端传 `{ paramKey: value }`，后端从 params 中提取
- **禁止** 在 `execute_config_action` 中修改组件内部状态。它是 **纯查询/计算** 操作
- 动作返回值格式：
  - 填充某个字段 → 返回 `{ "fieldKey": [...] }` 或 `{ "fieldKey": value }`
  - 返回预览数据 → 返回数组 `[{ ... }, { ... }]`
  - **禁止** 返回需要前端特殊解析的非 JSON 数据
