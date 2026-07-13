---
description: IPC 命令参数约定 — 0-2个简单参数用扁平参数，3个及以上用单个反序列化结构体
condition: "#\\[tauri::command\\]"
scope: "tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**)"
interruptMode: tool-only
---

# IPC 命令参数约定

- 0～2 个简单参数的命令：使用扁平参数
  - **正确**：`fn bridge_query(raw_query: String)`
- 3 个及以上参数的命令：使用单个反序列化结构体
  - **正确**：`fn bridge_confirm(payload: ConfirmPayload)`
- 所有结构体参数 **必须** `#[derive(Deserialize)]`
