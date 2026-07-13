---
description: IPC 命令总览：命名与文件放置、前缀对照表
condition: ".*"
scope: "tool:read(src-tauri/src/commands/**), tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**), tool:read(src-ui/bridge/**), tool:edit(src-ui/bridge/**), tool:write(src-ui/bridge/**), tool:read(src-tauri/src/cli_server/**), tool:edit(src-tauri/src/cli_server/**), tool:write(src-tauri/src/cli_server/**)"
interruptMode: never
---

# Tauri Command 总览

## 命名与文件放置

- 所有命令 **必须** 在 `lib.rs` 中通过 `generate_handler![]` 注册
- 前缀与文件对应关系（具体命令以各文件中 `#[tauri::command]` 标注为准）：

| 前缀 | 文件 | 域 |
|------|------|-----|
| `bridge_` | `commands/bridge.rs` | 搜索/会话 |
| `config_` | `commands/config_file.rs` | 配置管理 |
| `resource_` | `commands/resource.rs` | 资源管理 |
| `plugin_` | `commands/plugin.rs` | 插件管理 |
| `inspector_` | `commands/inspector.rs` | 插件检查器 |
| `cli_` | `commands/cli.rs` | CLI HTTP 服务器 |
| `debug_` | `commands/debug.rs` | 调试工具 |
- 引入新前缀时 **必须** 同步更新此表
