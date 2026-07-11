---
description: 第三方插件运行时规范：子进程 JSON-RPC 架构、协议、Manifest 校验、生命周期、RemoteComponent、CLI HTTP、zlplugin:// 协议
condition: ".*"
scope: "tool:read(crates/plugin-protocol/**), tool:edit(crates/plugin-protocol/**), tool:write(crates/plugin-protocol/**), tool:read(crates/plugin-host/**), tool:edit(crates/plugin-host/**), tool:write(crates/plugin-host/**), tool:read(crates/plugin-sdk-rust/**), tool:edit(crates/plugin-sdk-rust/**), tool:write(crates/plugin-sdk-rust/**), tool:read(src-tauri/src/plugin_framework/**), tool:edit(src-tauri/src/plugin_framework/**), tool:write(src-tauri/src/plugin_framework/**), tool:read(src-tauri/src/cli_server/**), tool:edit(src-tauri/src/cli_server/**), tool:write(src-tauri/src/cli_server/**), tool:read(src-tauri/src/commands/plugin.rs), tool:edit(src-tauri/src/commands/plugin.rs), tool:write(src-tauri/src/commands/plugin.rs), tool:read(crates/plugin-api/src/plugin/**), tool:edit(crates/plugin-api/src/plugin/**), tool:write(crates/plugin-api/src/plugin/**), tool:read(crates/plugin-api/src/host/**), tool:edit(crates/plugin-api/src/host/**), tool:write(crates/plugin-api/src/host/**)"
interruptMode: never
---

# 第三方插件运行时规范

## 架构概览

第三方插件以**子进程 + stdio JSON-RPC 2.0** 方式运行。宿主通过 `plugin-host` crate 管理子进程生命周期，通过 `RemoteComponent` 将 JSON-RPC 调用适配为 Plugin/DataSource/ActionExecutor trait。

**宿主侧入口**集中在 `src-tauri/src/plugin_framework/`：
- `manager.rs` — 第三方插件生命周期管理（加载、卸载、发现、崩溃恢复），唯一入口
- `host_handler.rs` — 子进程 Host 管理（spawn、健康监控、优雅关闭）
- `plugin_installer.rs` — 插件安装/卸载逻辑（从 manager.rs 提取）
- `plugin_info.rs` — 插件信息类型
- `zlplugin_protocol.rs` — `zlplugin://` 自定义协议处理（原 `plugin_protocol_assets/` 已合并至此）

## 协议规范

- **传输帧**：LSP 风格 `Content-Length: N\r\n\r\n{json}` 帧，编码 UTF-8。编解码实现在 `crates/plugin-protocol/src/codec.rs`（由 plugin-host 和 plugin-sdk-rust 共享）
- **双工**：宿主→插件（`plugin/*` 命名空间）和 插件→宿主（`host/*` 命名空间）均可发起 RPC
- **超时**：query / execute_action 默认 30s，其他 5s
- **错误码**：遵循 JSON-RPC 2.0 标准码 + 自定义码（-32000 ~ -32003）

## Manifest 校验

加载时 **必须** 校验：
1. `plugin.id` 符合反向域名正则
2. `plugin.version` 符合 SemVer
3. `plugin.min_host_version` ≤ ZL 版本
4. `runtime.command` 文件存在
5. `components.provides` 至少 1 项且在已知集合内

## 子进程生命周期

- **spawn**：启动子进程 → `plugin/initialize` 握手 → `plugin/get_metadata` → `plugin/get_components`
- **健康监控**：watchdog task 检测退出，`auto_restart=true` 时自动重启，上限 `max_restart`
- **优雅关闭**：`plugin/shutdown` → 等 5s → SIGKILL
- **stderr 日志**：收集到 `plugin-logs/<plugin-id>.log`

## RemoteComponent

- 每个远程组件对应一个 `RemoteComponent`，将 trait 方法翻译为 JSON-RPC 调用
- `Configurable` trait 是同步的，`RemoteComponent` 用 `block_in_place` 桥接异步 RPC
- Schema / settings / actions 在加载时一次性拉取并缓存

## HostDispatch（反向调用）

- 插件通过 `host/*` RPC 调宿主 API
- 在 src-tauri 侧由 `plugin_framework/host_handler.rs` 实现 `HostCallHandler` trait，路由到 `PluginHandle` 方法
- `host/resource.*` 调用 **必须** 校验 `plugin_id` 命名空间

## CLI HTTP 服务器

- 监听 `127.0.0.1:0`（随机端口），Bearer token 认证
- Token 持久化到 `cli-token.json`
- 路由前缀 `/v1`，共享 plugin-api 数据模型
- **只读约束**：CLI HTTP API **仅** 提供只读查询端点（`zl query`）。可写端点已移除。新增端点也 **必须** 保持只读

## 自定义协议 zlplugin://

- 处理入口：`src-tauri/src/plugin_framework/zlplugin_protocol.rs`
- 仅允许 `ui/` 子路径
- **必须** canonicalize 后校验在 plugin 目录内（防路径遍历）
- MIME 推断基于文件扩展名

## 新增 crate 时的注意事项

- `plugin-protocol` **只** 依赖 plugin-api + serde/thiserror/toml/semver，**禁止** 引入 tokio
- `plugin-host` 依赖 plugin-api + plugin-protocol，**禁止** 直接依赖 src-tauri
- `plugin-sdk-rust` 依赖 plugin-api + plugin-protocol，**禁止** 依赖 plugin-host

## 第三方插件开发约束

- 插件可声明实现 Plugin / DataSource / ActionExecutor（第一版不开放 KeywordOptimizer / SearchEngine / ScoreBooster）
- Rust 插件推荐使用 `zerolaunch-plugin-sdk-rust`
- 全权限模式（无权限模型），但 resource 操作强制 plugin_id 命名空间隔离
