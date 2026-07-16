# Project Review Checklist

> 本文件是所有审查 agent 的共享前置阅读。架构相关锚点的详细定义见 `references/architecture-principles.md`。

## 架构锚点（详见 architecture-principles.md）

以下 6 条原则由 Agent 4a（P1/P2/P3 结构）与 4b（P4/P5/P6 行为）主审，其他 agent 发现架构问题也应引用：

1. **P1 职责驱动的代码放置** — 代码该放哪个模块，由它承担的职责决定（见放置约定表）
2. **P2 类型职责边界 = 使用范围** — struct 定义位置编码职责，职责决定使用范围；IPC DTO 不可泄漏到内部模块
3. **P3 编译期层级与依赖方向** — 高层可用低层，低层不可用高层（workspace 层 + src-tauri 内部模块层，见层级表）
4. **P4 运行时职责域解耦** — ConfigManager / SessionRouter / PluginManager 三大职责域运行时禁止直接方法调用，必须走事件/管道/回调
5. **P5 通信方式随关系而定** — 跨层=直接调用；同级不同职责域=事件/管道；跨进程=JSON-RPC；前后端=IPC
6. **P6 接口复用优先于新增** — 新功能优先由既有抽象组合完成，不平行造轮子

## 其他核心锚点

7. **前端是薄展示层**：业务逻辑、文件/进程/平台操作必须留在后端 IPC 之后
8. **IPC 类型契约必须双端同步**：Rust / TypeScript 字段名使用明确的 serde rename
9. **commands/ 是命令入口**，不是业务逻辑容器（薄代理）
10. **同步锁守卫不得跨 await**：`parking_lot`/`std::sync`/`DashMap` 等同步锁守卫 MUST 在 `.await` 前释放（守卫 `!Send` 或阻塞式，跨 await 会让 future `!Send` / 阻塞同锁任务）；`tokio::sync::*` 异步锁豁免。优先以 `cargo clippy::await_holding_lock` 确定性结论为准。详见 `.omp/RULES.md`
11. **代码必须与 .omp/rules/ 规定一致**；不一致时区分 A 类（代码违反规则，需改代码）和 B 类（规则已过时，需更新规则）
