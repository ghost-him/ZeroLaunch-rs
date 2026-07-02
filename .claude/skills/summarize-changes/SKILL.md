---
name: summarize-changes
description: 总结当前代码更改的范围、动机和内容，生成结构化的 commit message 或变更摘要
---

# summarize-changes — 代码更改总结

分析当前 Git 工作区的变更（diff），理解修改了什么、为什么修改，生成规范的 commit message 或变更摘要。

## 触发方式

```
/summarize-changes [--staged]
```

- 不带参数：自动从 `git diff`（工作区 + 暂存区所有未提交的变更）获取变更内容。
- `--staged`：只总结暂存区（staged）中的变更，从 `git diff --cached` 获取内容。

## 执行流程

0. **安全检查**（仅默认模式，`--staged` 模式跳过此步骤）：
   - 运行 `.claude/skills/summarize-changes/check-changes.sh` 获取工作区全貌：
     - 该脚本一次性输出暂存区、未暂存、未跟踪三类文件的状态报告，并以退出码表示结论：
       - **退出码 0**（无未暂存/未跟踪文件）：将脚本输出展示给用户后直接进入步骤 1，无需询问。
       - **退出码 1**（存在未暂存或未跟踪文件）：将脚本输出展示给用户，然后通过 `AskUserQuestion` 工具询问：
         > 检测到工作区存在未暂存或未跟踪的文件。当前默认模式将生成【所有本地变更】（含已暂存 + 未暂存）的摘要。是否继续？
         - 选项 1：”继续，生成所有本地变更摘要” — 保持默认行为，汇总全部未提交变更。
         - 选项 2：”切换为 --staged 模式” — 仅汇总已暂存的变更，忽略未暂存文件。
         - 选项 3：”中止” — 取消本次操作。
         - 若用户选择选项 2，后续步骤按 `--staged` 模式执行。

1. **收集变更信息**：
   - 根据用户参数确定变更范围：
     - 不带参数：总结所有未提交的本地变更。必须运行 `git diff HEAD --stat` 和 `git diff HEAD` 获取内容。
     - `--staged`：只总结暂存区中的变更。必须运行 `git diff --cached --stat` 和 `git diff --cached` 获取内容。

2. **容量检测（前置步骤）**：
   - 根据用户参数运行对应的 stat 命令查看变更文件数：
     - 不带参数：运行 `git diff HEAD --stat`
     - 带 `--staged`：运行 `git diff --cached --stat`
   - 若输出中的变更文件 **> 15 个**，禁止逐一文件详解，改为按**变更目的**分组（如“依赖升级”、“日志格式调整”、“错误处理增强”），每组概括影响的核心文件数量即可，不展开具体代码行。

3. **单文件上下文限制（前置过滤）**：
   - 在阅读具体 diff 之前，先扫描 `git diff` 输出的变更行数（+++ 和 --- 的总行数）。
   - 若某个文件的变更块（hunk）上下文超过 200 行，或预估 Token 过大：
     - **禁止**使用 `Read` 工具读取该文件的完整内容。
     - **强制**仅基于 `git diff` 中展示的变更片段（变更块及其周围 3-5 行上下文）进行分析。
     - 目的：防止单个超大类或重构文件撑爆上下文窗口。

4. **理解改动**：
   - 逐一阅读每个变更文件的 diff，理解：
     - 改了什么文件、什么函数/模块。
     - 改动的具体内容（新增、删除、修改）。
     - 改动的目的（修复 bug、重构、新增功能、升级适配等）。
   - 如果 diff 涉及不熟悉的代码区域，快速读取附近文件的上线文确认。
   - 注意：diff 中的行号是行号。`Read` 工具的输出格式为 `<line-number>\t<line-content>`，两者视觉上易于混淆，务必注意区分。
   - 不可以执行写操作的命令（如 `git add`、`git commit` 等），只可执行读操作的命令用于分析和总结。

5. **分析根因（如果是修复类变更）**：
   - 如果是修复类变更（修复 panic/错误/异常），从 diff 反推发生了什么错误，确认修复是否完整。

6. **生成 commit message**：
   - 格式遵循 Conventional Commits 2.1.0 规范：

     ```
     <type>(<scope>): <精简描述>

     <详细说明（可选，必要时带上背景/动机/影响）>
     ```

   - **type**: fix / feat / chore / refactor / docs / style / perf / test / ci / build / revert
   - **scope**: 模块名或目录名，如 plugin-system / cli-server / config / sdk
   - 第一行不超过 72 字符
   - 使用中文 body 说明背景和动机
   - **Scope 推导规则（硬性约束）**：
     1. 取变更文件数量最多的顶级目录作为 Scope（如 `src/tauri/` 变更最多，取 `tauri`）。
     2. 若变更文件均匀分布在 3 个及以上互不相关的目录，**省略 Scope**（即 `fix: xxx`）。
     3. 禁止生成组合型 Scope（如 `core/ui` 或 `core+ui`）。

7. **输出结果**：
   - 展示最终的 commit message 给用户。
   - 不执行 `git commit`，只输出内容。

## 输出示例

```
fix(cli-server): 将 axum 路由参数从 `:param` 语法迁移到 `{param}` 语法

项目依赖的 axum 已升级到 0.8（从 0.7 起），路径参数语法从
`:param` 变更为 `{param}`。旧语法导致 CLI HTTP 服务器启动时 panic。

变更范围：
- src-tauri/src/cli_server/server.rs

受影响的路由（共 11 处）：
- `/v1/config/{id}/` 相关 7 个路由（schema、settings、reset、enabled、actions）
- `/v1/plugins/{id}/` 相关 4 个路由（reload、uninstall、manifest、logs）
```

## 注意事项

- 专注**总结变更**，不继续扩展新改动。
- diff 里的文件内容不要直接复制到 commit message 中，而是概括性质和范围。
- 如果变更涉及外部依赖版本变更（如 crate 升级），在 commit body 里注明原因。
- 对于大型变更，按文件或模块分组描述。
