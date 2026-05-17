---
paths:
  - "src-tauri/src/api/**"
  - "zl-cli/src/**"
---

# HTTP API 模块规范

## 架构原则

- `ReadApi` trait 是只读查询的 **单一契约**。**必须** 在 `api/read_api.rs` 中定义，所有传输层实现此 trait。
- 当前传输层仅有 HTTP/axum，未来可扩展 gRPC、WebSocket 等。所有新传输层 **必须** 实现 `ReadApi` trait。
- 路由处理器依赖 `Extension<Arc<dyn ReadApi>>`。**禁止** 在路由处理器中直接使用 `ServiceLocator` 或 `tauri::State`。
- **禁止** 在 `api/` 中直接 import 或使用 `tauri::State`。所有运行时服务通过 `ServiceLocator::get_state()` 获取。
- `api/` 模块可引用 `plugin_system/`、`core/`、`sdk/`。**禁止** 反向依赖（`core/` 引用 `api/`）。

## ReadApi Trait

- trait 方法仅定义 **只读查询**。**禁止** 添加写入、执行、修改状态的方法。
- `ReadApiImpl` 是零大小结构体。**禁止** 在 `ReadApiImpl` 中存储任何状态。
- 每个方法内部使用 `ServiceLocator::get_state()` 访问 AppState，然后通过 SessionRouter / ConfigManager 获取数据。
- 内部类型（`SearchCandidate`、`QueryResponse`、`ComponentInfo` 等）**必须** 通过 `api/types.rs` 的 DTO 转换为对外类型。
- RwLock 守卫 **必须** 在 `.await` 前释放（遵循 `.claude/rules/general.md` 的 RwLock 守卫生命周期规范）。

## 端点约束

- **只读**：API **仅** 暴露 `GET` 端点。**禁止** 添加 `POST`、`PUT`、`DELETE`、`PATCH` 端点。
- **回环绑定**：服务器 **必须** 仅绑定 `127.0.0.1`。**禁止** 绑定 `0.0.0.0` 或其他网络接口。
- **无认证**：不实施认证机制 — 绑定 127.0.0.1 足以保证进程内安全。**禁止** 引入 token、API key 等认证方案。
- **无 CORS**：回环绑定不需要 CORS。**禁止** 引入 `tower-http::cors` 或任何 CORS 中间件。

## DTO 约定

- `api/types.rs` 中的所有响应类型 **必须** 标注 `#[derive(Debug, Clone, Serialize, Deserialize)]` 和 `#[serde(rename_all = "camelCase")]`。
- 列表响应 **必须** 使用 `totalCount` 字段表示总数，数组字段名使用复数形式（`results`、`candidates`、`components`、`plugins`）。
- 内部类型到 DTO 的转换 **必须** 通过 `impl From<InternalType> for ApiType` 实现。
- 新增端点时：先在 `api/types.rs` 定义 DTO → 在 `ReadApi` trait 添加方法 → 在 `ReadApiImpl` 实现 → 在 `routes.rs` 添加处理器 → 在 `server.rs` 注册路由。

## 错误处理

- `ApiError` 是 HTTP API 的 **唯一** 错误类型。所有处理器 **必须** 返回 `Result<Json<T>, ApiError>`。
- `ApiError` **必须** 实现 `IntoResponse`，自动映射：
  - `not_found()` → HTTP 404
  - `invalid_query()` → HTTP 400
  - `internal()` → HTTP 500
- 错误响应 JSON 格式：`{ "error": "描述", "status": 404, "componentId": "可选" }`。
- **禁止** 从路由处理器中 panic。所有错误 **必须** 通过 `ApiError` 传播。

## CLI 工具 (`zl`)

- 二进制入口 `src/bin/zl.rs`，通过 `cargo build --bin zl` 独立编译。
- CLI 通过 HTTP 与主进程通信。**禁止** 在 CLI 中 import `zerolaunch_rs_lib` 或任何内部 crate 类型。
- 端口发现 **必须** 支持跨平台：
  - Windows：`%APPDATA%\ZeroLaunch-rs\.zl-port`
  - macOS：`~/Library/Application Support/ZeroLaunch-rs/.zl-port`
  - Linux：`$XDG_DATA_HOME/ZeroLaunch-rs/.zl-port` 回退 `~/.local/share/ZeroLaunch-rs/.zl-port`
- 端口发现优先级：`--port` 标志 > `.zl-port` 文件 > 默认值 45678。
- `--json` 标志 **必须** 输出合法 JSON。非 `--json` 模式输出人类可读文本。
- 新增子命令时：先在 `Commands` 枚举中添加变体，再实现对应的 `cmd_*` 函数。

## 跨平台

- 服务器端写端口文件 **必须** 通过 SDK 的 `path_resolver.resolve_path(KnownPath::AppDataDir)` 获取 app data 目录。
- CLI 端读端口文件 **必须** 使用 `#[cfg(target_os = ...)]` 条件编译适配各平台。
- 新增平台相关逻辑时：**禁止** 硬编码 Windows 路径（如 `C:\Users\xxx`）。使用标准路径宏或 `dirs` crate。

## 生命周期

- HTTP 服务器 **必须** 在 `init_app_state` 末尾（Phase 3 Plugin 初始化之后）启动。
- HTTP 服务器 **必须** 在 `ExitRequested` 处理器中通过 `oneshot::Sender` 优雅关闭。
- 关闭信号 sender **必须** 通过 `AppState::set_http_server_shutdown()` 存储，通过 `take_http_server_shutdown()` 取出。
- **禁止** 在路由处理器内部访问 `AppState::http_server_shutdown`。

## 禁止事项

- **禁止** 在 HTTP API 中添加写入端点（POST/PUT/DELETE/PATCH）。
- **禁止** 绑定到 `0.0.0.0` 或非 loopback 地址。
- **禁止** 引入 `tower-http::cors` 或任何认证中间件。
- **禁止** 在 `api/` 中直接使用 `tauri::State` 或 `taiur` 特定类型。
- **禁止** 在 CLI 中 import 内部 lib 类型。
- **禁止** 硬编码平台路径（如 `%APPDATA%`）；使用 `#[cfg]` 分支或 PathResolver。
- **禁止** 在路由处理器中持有 RwLock 守卫跨越 `.await` 点。
