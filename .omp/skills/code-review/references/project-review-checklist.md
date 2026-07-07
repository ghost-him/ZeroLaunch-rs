# Project Review Checklist

## 核心审查锚点

1. **前端是薄展示层**：业务逻辑、文件/进程/平台操作必须留在后端 IPC 之后
2. **IPC 类型契约必须双端同步**：Rust / TypeScript 字段名使用明确的 serde rename
3. **commands/ 是命令入口**，不是业务逻辑容器
4. **插件系统优先沿用既有抽象**：PluginHandle、ExecutorRegistry、CandidatePipeline、SearchPipeline、Configurable 生命周期
5. **PluginManager 与配置/路由系统通过事件解耦**（PluginRuntimeEvent → ConfigManager → ConfigEvent → SessionRouter）
6. **同步锁守卫不得跨 await**：`parking_lot`/`std::sync`/`DashMap` 等同步锁守卫 MUST 在 `.await` 前释放（守卫 `!Send` 或阻塞式，跨 await 会让 future `!Send` / 阻塞同锁任务）；`tokio::sync::*` 异步锁豁免（守卫 `Send` 且 `read()`/`write()` 本身可 `.await`，设计上允许跨 await）。详见 `.omp/RULES.md`
7. **workspace 依赖方向不可反转**
8. **代码必须与 .omp/rules/ 规定一致**；不一致时区分 A 类（代码违反规则）和 B 类（规则需更新）

## 本次审查的特别考量

- 本次变更不涉及任何应用代码，只需要验证：
  - 配置文件格式正确（YAML、JSON）
  - 目录结构完整，无遗漏文件
  - 内容迁移完整（旧 `.claude/` 中的关键内容是否都出现在 `.omp/` 中）
  - 权限白名单是否意外缩减
  - 跨引用链接（如 `CONTRIBUTING.md` 中的路径引用、规则文件间的交叉引用）是否正确更新
