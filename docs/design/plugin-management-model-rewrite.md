# 插件管理模型重写目标

> 状态：目标文档（重写蓝图）。当前实现（2026-08-08 冲突预检下沉版）功能正确但复杂度失控，本文档记录"同等功能与职责、更简单实现"的重写目标与验收标准。重写落地前，当前实现按现状维护（含已修复项）。

---

## 1. 背景与动机

2026-08-08 代码审查（`code-review-2026-08-08-component-id-collision-precheck`）确认：当前插件管理模型存在**两套并行的注册流程**与**状态重放式崩溃恢复**，复杂度已以 bug 形式显性化：

| # | 问题 | 根因 |
|---|---|---|
| 1 | 崩溃重启自碰撞（阻塞级回归）：任何已注册第三方插件崩溃后重启被永久放弃 | `restart_loop` 复制了 `load()` 的 spawn/discover/预检/登记流程，预检时机与 `load()` 语义错位 |
| 2 | reload 时序竞态（F2，偶发 reload 失败） | 冲突判定数据源放在 CM（异步事件驱动），被迫注入同步查询闭包，引入跨任务时序依赖 |
| 3 | 放弃清理覆盖 2/4 路径（spawn/discover 失败臂残留 CM/SR 僵尸 + 孤儿进程） | "等重启结果再决定是否清理"的设计需要覆盖所有失败路径的纪律，纪律未守住 |

复杂度来源（结构性，非单点缺陷）：

1. **两套并行的注册流程**：`PluginHostManager::load()` 与 `restart_loop` 各写一遍 spawn → discover → build_components → 冲突预检 → 登记 → 回调。restart_loop 不是复用 load，而是复制。
2. **状态重放式崩溃恢复**：崩溃后不清理、等重启结果再决定 → 需要旧快照（`adapters_cache`）+ 成功/放弃双回调（`RestartCallback`/`RestartAbandonedCallback`）+ 全失败路径清理纪律。
3. **冲突判定数据源在 CM**：CM 注册经 `PluginRuntimeEvent` 广播异步更新，预检却同步直查 → 跨 crate 注入查询闭包（`component_id_checker` 三件套 + bootstrap 装配 + `load()` 签名 +1 参数）→ 时序敏感 + 生命周期耦合。
4. **状态四处分摊**：CM registry / `hm.plugins` / `adapters_cache` / `third_party_infos` 维护同一件事的四个视图。

---

## 2. 重写目标（一句话）

**崩溃恢复 = 解注册 + 复用加载；冲突判定数据源属于加载者自己（插件生命周期域），而非被通知者（CM）。**

功能与职责不变，删除的是：一个回调类型（`RestartAbandonedCallback`）、一个注入闭包（`component_id_checker` 三件套 + bootstrap 装配）、一份快照缓存（`adapters_cache`）、restart_loop 中复制的注册流程——以及它们带来的三个 bug。

---

## 3. 功能保持不变清单（重写不得破坏）

| 功能 | 现状契约 |
|---|---|
| 第三方插件加载 | manifest 校验 → spawn 子进程 → 握手 → get_components → 注册进 CM/SR；`PluginRuntimeEvent` 事件驱动，PM→CM 事件解耦不破 |
| 组件 id 冲突拒绝 | 整包拒绝（任一组件撞 id 则整插件不注册），避免半提交；冲突给出确定性错误 |
| 崩溃自动重启 | watchdog 检测退出（`auto_restart=true`）→ 自动重拉，上限 `max_restart`，计数跨重启持久于 `PluginRestartContext` |
| 重启放弃清理 | 放弃后 CM/SR 无残留（无僵尸组件占 id 空间） |
| 卸载/重载 | `uninstall`/`reload` 语义与错误码不变 |
| 与内置组件冲突检测 | 预检必须覆盖内置组件 id（内置组件启动期注册完毕，id 集合启动后稳定） |
| 并发兜底 | 两个插件并发加载同 id 时，最终拒绝权在 CM（整包预检 + `register()` 查重） |

---

## 4. 目标架构（两个核心简化动作）

### 动作 A：崩溃即解注册，不等待重启结果

崩溃通知到达时**第一步**就用旧注册包解注册 CM/SR（并 `host_api.unregister`），之后重启流程就是一次干净的重新加载：

```
崩溃 → watchdog → restart_loop
  → 1. 立即解注册旧组件（CM/SR/host_api）——等价于现在的 on_restart_abandoned 提前到崩溃处理起点
  → 2. 计数 + max_restart 检查（超限 → 结束，无任何残留需要清理）
  → 3. 未超限 → spawn → discover → 冲突预检 → 登记 → on_restart（只发 PluginLoaded 新组件）
```

连带删除：

- `adapters_cache`（旧快照的唯一用途是"重启后再决定是否解注册"，不再需要）
- `RestartAbandonedCallback` 类型、`make_restart_abandoned_callback`、`PluginRestartContext.on_restart_abandoned`、`load()`/`reload()` 的对应参数
- 双回调不对称问题（`RestartCallback` async / `RestartAbandonedCallback` sync）整体消失
- 放弃清理覆盖纪律：spawn/discover 失败、max_restart 超限时组件早已解注册，**没有残留可漏**（原问题 3 结构性消失）
- 崩溃重启自碰撞（原问题 1）：预检时自身 id 已释放，结构性消失
- `host_api.unregister` 并入崩溃解注册路径

**语义变化（2026-08-08 已确认接受）**：崩溃→重启成功窗口内插件组件短暂从 CM/SR 消失（触发词临时失效）。插件进程已死，组件本就是僵尸，此窗口等价于"先清后挂"——这正是设想的形态，动作 A 成立。

### 动作 B：冲突判定数据源下沉到插件生命周期域

预检数据源从"CM registry 经注入闭包查询"改为"插件加载者自己已知的信息"：

- 已加载第三方插件：`hm.plugins` 中 `PluginRegistration.components` 的 id 并集（host 自有数据，控制流内同步更新，无异步时序）
- 内置组件：启动时注入一次只读快照 `Set<String>`（`PluginHostManager::set_builtin_component_ids`），内置组件启动期注册完毕、此后稳定

收益：

- `component_id_checker` 字段/setter/getter、bootstrap 装配闭包、`load()` 签名参数——删除，跨 crate 同步查询回调模式消失
- 无跨任务时序依赖——reload 时序竞态（原问题 2）结构上不存在
- **自碰撞在结构上不可能**：自己的组件在预检时尚未登记进 `hm.plugins`，无需任何"排除自身"逻辑
- 预检谓词语义从"是否被占用"变为"是否被**其他**插件或内置组件占用"，语义自明

### 保留的防线（降为两层）

1. **host 侧预检**（动作 B 的数据源）：提前拒绝 + 关进程 + 确定性错误——主防线
2. **CM 整包预检 + `register()` 查重**：并发竞态窗口（两个插件同时加载同 id）的最终防线——兜底，名实相符

事件解耦（PM→CM 仅经 `PluginRuntimeEvent` 广播）、`Configurable` 生命周期、SR 路由更新链路均不变。

---

## 5. 约束与前提（必须文档化）

- **内置组件 id 集合启动后稳定**：注入快照的前提。若未来支持运行时热装/卸载内置组件，此前提失效，需回到查询式数据源（届时再评估）。
- **崩溃窗口语义（已拍板，2026-08-08）**：动作 A 的"组件在崩溃→重启成功期间短暂消失"已确认接受，快照重放方案（现状）不再考虑。
- **并发同 id 竞态窗口仍在**：两第三方插件并发加载同 id 时双方 host 预检均可能通过，由 CM 兜底拒绝。重写不恶化此现状，也不试图闭合（预检与 CM 注册之间的窗口由兜底覆盖）。

---

## 6. 验收标准（重写完成后逐项核对）

1. 第三方插件崩溃 → 自动重启成功，触发词/数据源恢复（`max_restart` 内）
2. 崩溃后重启被拒（他插件占用/超限）→ CM/SR 无该插件残留，`host_api` 无死句柄，UI 状态与实际一致
3. 与内置组件撞 id 的插件加载被确定性拒绝，给出可区分的错误（前端可程序化识别冲突场景）
4. `reload` 连续执行 N 次不失败（无时序竞态）
5. 卸载/重载/安装错误码与语义与现状一致
6. 以下符号在代码中消失（或等价删除）：`RestartAbandonedCallback`、`component_id_checker`、`adapters_cache`、`on_restart_abandoned`、`make_restart_abandoned_callback`
7. `restart_loop` 不再包含 spawn/discover/预检/登记流程（只保留计数、上限、触发与解注册）
8. 两个确定性脚本（`check-deps-direction.sh`、`check-type-scope.sh`）与 `cargo clippy`（无 `await_holding_lock`）通过

---

## 7. 非目标（本次重写不做）

- `PluginRuntimeEvent` broadcast → mpsc 改造（容量 256，Lagged 丢事件是独立隐患，正交）
- `third_party_infos` 与 `hm.plugins` 信息模型合并（UI 状态同步是独立课题）
- 插件 UI/前端部分（Vue 组件加载、键盘、面板）——本文档只覆盖后端生命周期管理模型

---

## 8. 关联文档

- `third-party-plugin-architecture.md` — 第三方插件整体架构设想（本文档是其生命周期管理部分的演进目标）
- `../dev/` 内置插件指南与 `.omp/rules/third-party-plugin.md` — 重写落地后需同步更新（自动重启契约、冲突预检描述、`zerolaunch-component-id-collision-terminal-state` 技能）
- 审查报告 `.omp/skills/code-review/reports/code-review-2026-08-08-component-id-collision-precheck.md` — 本目标的动机与问题证据
