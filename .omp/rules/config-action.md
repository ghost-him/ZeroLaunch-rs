---
description: ConfigAction 规范 — 配置保存前的副作用测试（如 WebDAV 连通性），纯查询/计算操作，field-level 可含有限副作用
condition: "execute_config_action|config_actions|ConfigActionDef|ConfigAction"
scope: "tool:edit(*.rs), tool:write(*.rs)"
---

# ConfigAction 规范

## 用途：保存前测试

如果副作用必须决定配置能否保存（如 WebDAV 连通性测试）：**必须** 使用 `ConfigAction`。通过 `config_actions() → Vec<ConfigActionDef>` 声明，在 `async fn execute_config_action(&self, action: &str, params: &serde_json::Value) -> Result<serde_json::Value, String>` 中实现。

前端 **必须** 将 `config_execute_action` 作为单独的用户触发操作调用，与保存流程解耦。

## field-level ConfigAction 例外

`FieldDefinition.config_action` 标记的字段级动作 **允许** 有限副作用，但受以下约束：

1. **必须** 在 `FieldDefinition` 的文档注释中说明副作用（如"立即写入缓存，不持久化字段值"）
2. **必须** 是异常安全的：失败时不得留下不一致状态，`execute_config_action` 返回 `Err` 后前端可重试
3. **禁止** 修改组件内部配置状态（`self.settings`）
4. **禁止** 调用 ConfigManager、SessionRouter 等框架层方法
5. **允许** 的副作用类型：
   - 写入插件层缓存（如图标缓存）
   - 文件系统写入（不超过 100ms）
   - 轻量级 HTTP 请求（不超过 500ms timeout）
6. 副作用动作的返回值 **必须** 包含 `"success": true/false` 字段
## 参数传递

- `async fn execute_config_action(&self, action: &str, params: &serde_json::Value) -> Result<serde_json::Value, String>` 签名支持参数
- 无参数的动作（如 `detect_browsers`）：前端不传 params，后端收到 `Value::Null`
- 有参数的动作（如 `read_bookmarks`）：前端传 `{ paramKey: value }`，后端从 params 中提取
- **禁止** 在顶层 `ConfigAction`（来自 `config_actions()`）的 `execute_config_action` 中修改组件内部状态。field-level ConfigAction 例外见上
- 动作返回值格式：
  - 填充某个字段 → 返回 `{ "fieldKey": [...] }` 或 `{ "fieldKey": value }`
  - 返回预览数据 → 返回数组 `[{ ... }, { ... }]`
  - **禁止** 返回需要前端特殊解析的非 JSON 数据
