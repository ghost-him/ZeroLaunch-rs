# ZeroLaunch-rs 审查清单

本文件为 `code-review` skill 的项目上下文参考。开始审查前先读本文件，再按 `collect-context.sh` 输出的规则文件清单加载 `.claude/rules/` 中的对应规则。

## 1. 架构速览

ZeroLaunch-rs = Bun + Tauri 2.x + Vue 3 + Naive UI 的 Windows 快捷启动器，Cargo workspace 多 crate 架构。

**依赖方向（不可反转）**：
- `plugin-api ← plugin-protocol ← plugin-host ← src-tauri`
- `plugin-api ← platform-windows ← src-tauri`
- `plugin-api ← plugin-protocol ← plugin-sdk-rust`（第三方 Rust SDK 以子进程运行，合法依赖 protocol；层级与 plugin-host 同）

> **目录改名（当前分支 `refactor/plugin-system`）**：`src-tauri/src/plugin_system/` → `plugin_framework/`，`plugin/` → `builtin_plugin/`，`sdk/` → `sdk.rs`。下文表格沿用「层」的概念名；实际路径以改名后为准。若 `.claude/rules/`、`AGENTS.md` 仍用旧名，属文档滞后（Agent 5 的 B 类）。

**后端分层（src-tauri/src/）**：

| 层 | 目录 | 职责 | 可引用 | 禁止引用 |
|----|------|------|--------|----------|
| sdk | `sdk/` | re-export 桥 | — | core/、plugin/、plugin_system/ |
| core | `core/` | ConfigManager、Configurable、类型 | sdk/ | plugin/、plugin_system/ |
| plugin | `plugin/` | 内置插件实现 | sdk/、core/ | plugin_system/ |
| plugin_system | `plugin_system/` | SessionRouter、Pipeline、Registry | sdk/、core/、plugin/ | 被反向引用 |
| commands | `commands/` | IPC 薄代理 | 全部 | 包含业务逻辑 |

**前后端边界**：前端 = 数据展示 + 用户交互（薄展示层）；后端 = 数据持久化 + 逻辑控制。所有文件/进程/平台操作必须走 IPC。

## 2. 高风险审查区域

### IPC 契约同步
- Rust `#[serde(rename = "...")]` 与 TS `bridge/contract.ts` 必须一致
- enum variant、`action_id`、事件名、配置 key 改了一边必须同步另一边
- `check-ipc-commands.sh` 会**确定性**交叉校验命令名三方一致：`#[tauri::command]` 定义 ↔ `generate_handler!` 注册 ↔ 前端 `invoke` 调用；`[FAIL]` 表示前端调用了未注册命令 / 注册了无定义命令，`[WARN]` 表示定义了却未注册
- `collect-context.sh` 还会提示 commands/ 变更但 contract.ts 未变更（或反之）

### 搜索→确认主链路
`bridge_wake → SessionRouter.route_query → SearchPipeline / Plugin panel → bridge_confirm → SessionRouter.route_confirm → ExecutorRegistry.resolve → executor.execute`

典型断裂点：改了查询结果结构但确认链没同步、`candidateId`/`actionId`/`targetType` 路径断裂、fallback 条件失效。

### Configurable 生命周期
- `apply_settings()` 只做反序列化 + 写 RwLock，**不做**副作用
- 副作用放 `on_settings_changed()`
- 核心配置组件可在 `on_settings_changed()` 中 spawn async task，但**禁止**在 `apply_settings()` 中 spawn

### 异步与并发
- `RwLock` 守卫**不得**跨 `.await`（parking_lot guard 是 `!Send`）
- 正确做法：clone 数据到局部变量 → 释放守卫 → `.await`
- 检查新增竞态：事件顺序、缓存刷新与读取并发、窗口状态与命令执行并发

### 执行器路由
- `ExecutorRegistry::resolve(ctx, action_id)` 是唯一查找入口
- `resolve_fallback` 用于窗口激活失败回退
- `get_actions` 仅查询可用动作，**禁止**用于执行路由

## 3. "回归"的严格定义

回归必须满足**全部**条件：
1. 变更前该行为正常（或至少没有此问题）
2. 变更后该行为变坏
3. 能指出是哪个 diff hunk 导致的
4. 不是旧问题延续或历史遗留

**常见回归模式**：重构漏改调用方、改默认值/初始化顺序、改配置 schema 忘了同步链路、改 IPC 字段没同步 TS 契约、改 executor/action 路由没同步快捷键或 fallback、改前端状态结构但某窗口/store 仍用旧字段、改插件系统抽象但第三方/内置插件初始化路径没跟上。

## 4. "架构耦合变坏"的判定

以下情况视为架构耦合问题：
- 前端直接实现业务逻辑或平台调用
- command handler 内塞满跨模块业务逻辑
- 低层 crate 反向依赖高层 crate（`collect-context.sh` 的依赖检查会确定性检出）
- 绕过 `PluginHandle`、`ExecutorRegistry`、`CandidatePipeline`、`SearchPipeline`、`Configurable`
- 为局部问题引入新全局 trait/模块/层，但现有抽象其实能承载
- 把局部数据结构泄漏为多模块共享的公共契约
- 重新引入对 `ConfigManager`、`SessionRouter` 内部细节的直接耦合，而非经事件或稳定接口

## 5. 规则一致性审查要点

Agent 5 逐条检查变更是否与 `.claude/rules/` 一致。关键检查项：

| 规则文件 | 高频检查项 |
|----------|-----------|
| `general.md` | RwLock 守卫不跨 await、前后端职责边界、JSON 用 as_f64、文件命名、日志用 tracing、AppState 通过 Tauri state 获取 |
| `plugin-system.md` | apply_settings 不做副作用、ExecutorRegistry 唯一入口、PluginHandle 唯一平台能力通道、事件驱动解耦、inventory 注册 |
| `commands.md` | 命名前缀、serde rename 显式标注、命令注册、返回命名结构体非裸 JSON、trace_id 后端生成 |
| `frontend.md` | script setup、CSS 变量、Store setup 语法、Schema 驱动 UI、禁止 any、键盘快捷键集中管理 |
| `data-flow.md` | IPC 双端同步、action_id 路由完整、fallback 机制 |
| `directory-map.md` | 文件放置位置、依赖方向、模块职责 |

不一致分两类：
- **A 类（代码违反规则）**：规则仍合理，代码需改 → 阻塞级
- **B 类（规则已过时）**：代码的偏离是有意设计演进，规则需更新 → 建议级，给出规则更新方向

## 6. 路径到规则文件的映射

`collect-context.sh` 已自动完成此映射，以下供 agent 理解逻辑：

- `src-tauri/src/plugin/**`、`plugin_system/**` → `plugin-system.md`、`data-flow.md`、`general.md`
- `src-tauri/src/commands/**`、`src-ui/bridge/**`、`cli_server/**` → `commands.md`、`data-flow.md`、`general.md`
- `src-ui/**` → `frontend.md`、`general.md`
- `crates/plugin-*` → `sdk.md`、`directory-map.md`
- `src-tauri/src/core/**` → `config.md`、`data-flow.md`、`general.md`
- 大范围重构 → `directory-map.md`
