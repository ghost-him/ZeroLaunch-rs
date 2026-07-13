---
name: code-review
description: 面向 ZeroLaunch-rs 的项目专属代码审查技能。对当前工作区、staged 变更、当前分支全量变更、指定 git range 或最近 N 次 commit 的聚合变更进行多 agent 并行审查，重点验证逻辑正确性、确认是否引入新回归、检查架构边界耦合、以及验证与 .omp/rules/ 规则的一致性。
argument-hint: "[范围: working-tree | --staged | branch | <git range> | 最近 N 次 commit]"
---

## 用途

对 ZeroLaunch-rs 的代码变更执行**项目专属**代码审查。默认只读，不直接修改代码。5 个并行子 agent 分别覆盖：

1. **变更建模** — 建立风险地图，为后续 agent 提供聚焦点
2. **逻辑正确性** — 控制流、状态流、数据契约、异步时序、错误路径
3. **新回归** — 严格只计"变更前没有、变更后出现"的问题
4. **架构边界** — 越层、反向依赖、抽象绕过、职责漂移
5. **规则一致性** — 与 `.omp/rules/` 的规定是否一致（不一致可能是代码违反规则，也可能是规则已过时需更新）

## 触发方式

```text
/code-review                                    # 默认: 审查工作区变更 (git diff HEAD)
/code-review --staged                           # 仅审查暂存区
/code-review 审查本分支中的所有更改               # 审查当前分支相对默认分支的全量变更
/code-review <git range>                        # 审查指定范围，如 main..HEAD、HEAD~3..HEAD
/code-review 对过去5次commit做审核               # 最近 N 次 commit 聚合审查
/code-review 审核最近3个 commit                  # 同上
```

**范围解析优先级**：明确 git range > "最近 N 次 commit" 描述 > "本分支" 描述 > 默认 working-tree。

当 `N > 1` 或范围为多 commit 时，进入**跨 commit 审核模式**：先聚合审查，再对大 commit 单独下钻。

## 执行流程

### 第一阶段：确定范围并收集上下文（脚本驱动）

1. 根据用户参数运行上下文收集脚本：

```bash
# mode 取值: working-tree | staged | branch | range | commits
bash .omp/skills/code-review/scripts/collect-context.sh <mode> [range_or_n]
```

脚本一次性输出：变更统计、按子系统分组的文件清单、子系统交叉数、需加载的规则文件、依赖方向检查结果（确定性）、构建/lint 建议、**IPC 命令三方一致性检查（确定性）**、大 commit 分类表。

2. 若脚本建议执行构建检查（任意 `.rs` 变更），执行 `cd src-tauri && cargo check`；并优先运行 `cargo clippy`——其中 `clippy::await_holding_lock` 可**确定性**检出「RwLock/Mutex 守卫跨 `.await`」这一核心规则违规，胜过肉眼看 diff。
3. 若变更涉及前端或 IPC 契约，必要时执行 `bun run build`。
4. 命令不存在或成本过高时，回退到静态分析并在结论中说明。

### 第二阶段：并行发现（5 个子 agent）

同时启动 5 个只读审查 agent。各 agent 的提示词模板见 `references/agent-prompts.md`。

所有 agent 共享前置约束：

- **只读**，不修改文件
- 先读 `references/project-review-checklist.md`
- 阅读第一阶段脚本输出的上下文报告
- 按变更路径加载 `.omp/rules/` 中最相关的规则文件
- 若存在 `.codegraph/`，优先使用 CodeGraph

| Agent          | 职责                         | 严重程度上限     |
| -------------- | ---------------------------- | ---------------- |
| 1 — 变更建模   | 建立风险地图                 | 不评级（信息性） |
| 2 — 逻辑正确性 | 控制流/状态流/契约/异步/错误 | 阻塞             |
| 3 — 新回归     | 仅计本次引入的回归           | 阻塞             |
| 4 — 架构边界   | 越层/反向依赖/抽象绕过       | 阻塞             |
| 5 — 规则一致性 | 与 `.omp/rules/` 的一致性    | 阻塞（A 类违规） |

Agent 5 的核心区分：不一致发现分为 **A 类**（代码违反规则，需改代码）和 **B 类**（规则已过时，需更新规则）。A 类问题使用阻塞级提示。

### 第三阶段：大 commit 下钻

仅在多 commit 范围审查时执行。第二阶段的 `collect-context.sh` 已通过 `classify-commits.sh` 输出大 commit 分类表。

大 commit 判定标准（满足任一）：

- 变更文件数 ≥ 8
- 插入 + 删除总行数 ≥ 300
- 跨越 ≥ 2 个核心子系统

对每个大 commit 启动独立审查 agent（提示词模板见 `references/agent-prompts.md` 末尾）。只有在聚合审查完成后才决定是否下钻，不机械逐个重审。

### 第四阶段：主 Agent 汇总

阅读 5 份聚合审查报告（及大 commit 报告如有），执行以下步骤：

#### 4a. 冲突检测与复核

**若不同子 agent 对同一代码位置的结论相互冲突**（例如 Agent 2 认为某处有逻辑错误，Agent 4 认为该设计合理），主 agent **必须**：

1. 自行阅读相关代码与 diff，独立确认实际情况
2. 在汇总报告中单独列出"冲突复核"章节，说明：
   - 冲突来源（哪两个 agent、对哪个位置、各自结论是什么）
   - 复核过程（主 agent 读了哪些代码、依据什么得出结论）
   - 最终判定（采纳哪方结论，或两方都不完全正确）

不跳过这一步，不简单"少数服从多数"。

#### 4b. 结构化审查结论

输出以下结构：

1. **总体审查结论**：`通过` / `建议修改后合并` / `阻塞合并`
2. **聚合视角结论**：所有变更合起来的净影响
3. **阻塞问题**：真正会导致错误、回归或架构破坏的问题
4. **高优先级问题**：应在合并前处理
5. **已确认回归**：单独成节
6. **架构耦合问题**：单独成节，指出应回归到哪条既有约束
7. **规则不一致问题**：按 A 类（代码违反规则）和 B 类（规则需更新）分别列出
8. **冲突复核**（如有）：4a 步骤的结果
9. **大 commit 下钻结论**（如有）：按 commit 列出
10. **疑点但证据不足**：说明为什么不能下结论
11. **既有问题**：标明"不是本次引入"
12. **建议手动验证清单**：3-8 条最值得回归测试的行为路径

#### 4c. 输出风格

- 先给结论，再给证据
- 建设性、可执行；优先说明"为什么是问题"和"最小修复方向"
- 不把代码风格细节包装成架构问题
- 没有证据就放进"疑点但证据不足"，不用"可能有问题"替代
- 对"回归"一词保持严格：**在正式输出前，必须在思考中证明是本次引入**

#### 4d. 结果持久化到文件

结构化审查结论生成后，**必须**将完整审查报告写入文件：

- **目录**: `.omp/skills/code-review/reports/`（若不存在，使用 `mkdir -p` 创建）
- **文件名格式**: `code-review-YYYY-MM-DD-简短摘要.md`
  - `YYYY-MM-DD` 为审查执行当天的日期
  - `简短摘要` 描述审查范围，如 `working-tree`、`main-to-HEAD`、`staged`、`last-3-commits`、`review-plugin-system` 等（使用英文 kebab-case）
- **文件内容**: 包含完整的结构化审查结论（4b 的所有章节），从"总体审查结论"到"建议手动验证清单"，Agent 1-5 的报告摘要可精简纳入而不丢失关键信息。文件开头加一行元数据：`<!-- 审查日期: YYYY-MM-DD | 范围: <范围> | 结论: <通过/建议修改/阻塞> -->`
- **写入方式**: 使用 `Write` 工具写入。若同日期同范围的文件已存在，则追加或覆盖均可（新文件头部注明"覆盖前次报告"）
- **时机**: 在向用户输出审查结论的同时或之后立即执行，确保结果不丢失

## 判定准则

最终判断以以下项目约束为审查锚点（详见 `references/project-review-checklist.md`）：

- 前端是薄展示层；业务逻辑、文件/进程/平台操作必须留在后端 IPC 之后
- IPC 类型契约必须 Rust / TypeScript 双端同步，字段名使用明确的 serde rename
- `commands/` 是命令入口，不是业务逻辑容器
- 插件系统优先沿用既有抽象：`PluginHandle`、`ExecutorRegistry`、`CandidatePipeline`、`SearchPipeline`、`Configurable` 生命周期
- `PluginManager` 与配置/路由系统通过事件解耦，不重新拉回直接依赖
- 同步锁守卫不得跨 `await`（`parking_lot`/`std::sync`/`DashMap` 等；`tokio::sync` 异步锁豁免）
- workspace 依赖方向不可反转
- 代码必须与 `.omp/rules/` 中的规定一致；不一致时区分"代码违反规则"与"规则已过时"

## 脚本清单

| 脚本                              | 用途                                                                                                                                          |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `scripts/lib.sh`                  | 共享库：子系统分类、核心子系统判定、路径→规则文件映射（`collect-context.sh` 与 `classify-commits.sh` 共同 source，避免分类逻辑漂移）          |
| `scripts/collect-context.sh`      | 收集审查上下文（diff stat、子系统分类、规则映射、依赖检查、IPC 命令检查、大 commit 分类）                                                     |
| `scripts/classify-commits.sh`     | 对多 commit 范围中的每个 commit 做大 commit 分类                                                                                              |
| `scripts/check-deps-direction.sh` | 检查 workspace 依赖方向是否合规（确定性）                                                                                                     |
| `scripts/check-ipc-commands.sh`   | 确定性交叉校验 IPC 命令：`#[tauri::command]` 定义 ↔ `generate_handler!` 注册 ↔ 前端 `invoke` 调用，检出未注册/未定义/前端调用不存在命令等漂移 |

脚本输出进入上下文，脚本代码本身不消耗上下文 token。能用脚本确定性判断的检查项一律用脚本，不交给 LLM 推断，当前覆盖：

- 依赖方向合规性（`check-deps-direction.sh`）
- 文件分类 / 规则映射 / 核心子系统交叉 / 大 commit 判定（`lib.sh` + `collect-context.sh` + `classify-commits.sh`）
- IPC 命令定义/注册/前端调用三方一致性（`check-ipc-commands.sh`）
- RwLock/Mutex 守卫跨 await（交由 `cargo clippy` 的 `await_holding_lock` 而非 LLM）

> 注：当前分支 `refactor/plugin-system` 已将 `src-tauri/src/plugin_system/` 重命名为 `plugin_framework/`、`plugin/` 重命名为 `builtin_plugin/`、`sdk/`→`sdk.rs`。`lib.sh` 同时保留新旧目录名映射，因此在历史 commit（含旧名）与当前分支上都能正确分类。若 `.omp/rules/` 与 `AGENTS.md` 仍使用旧目录名，属于文档滞后，正是 Agent 5 应作为 **B 类（规则需更新）** 报告的对象。

## 注意事项

你只可以一步一步的按照该步骤处理。不可以做其他更多的事。不要在没有明确指令的情况下修改代码。

由于脚本执行或 cargo check 的时间会比较长，所以你必须将超时时间设置为至少 5 分钟（推荐 10 分钟），以防止运行超时。
