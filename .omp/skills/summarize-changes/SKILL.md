---
name: summarize-changes
description: 总结当前代码更改，生成结构化的 commit message 或变更摘要
---

# summarize-changes — 代码更改总结

分析当前 Git 工作区的变更（diff），理解修改了什么、为什么修改，生成简洁的 commit message。

## 触发方式

```
/summarize-changes [--staged]
```

- 不带参数：从 `git diff`（工作区 + 暂存区所有未提交变更）获取内容。
- `--staged`：只总结暂存区变更，从 `git diff --cached` 获取内容。

## 执行流程

0. **安全检查**（仅默认模式，`--staged` 模式跳过）：
   - 运行 `.omp/skills/summarize-changes/check-changes.sh` 获取工作区全貌：
     - 退出码 0（无未暂存/未跟踪文件）：展示输出后直接进入步骤 1。
     - 退出码 1（存在未暂存或未跟踪文件）：展示输出，通过 `ask` 询问是否继续/切换 `--staged`/中止。

1. **收集变更信息**：运行 `git diff HEAD --stat` 和 `git diff HEAD`（或 `--cached` 对应版本）获取变更内容。

2. **容量检测**：stat 中变更文件 > 15 个时，禁止逐一文件详解，改为按**变更目的**分组概括。

3. **单文件上下文限制**：单个文件变更块上下文 > 200 行或变更行数 > 500 行时，禁止 Read 完整文件，仅基于 diff 片段分析。

4. **理解改动**：
   - 阅读 diff，理解改了什么、为什么改。**不需逐一罗列每处修改**，同类变更合并为一条概括描述。
   - **输出篇幅与变更规模成正比**：小改动（≤5 文件、≤100 行 diff net）的 body 控制在 5–10 行内；中型 ≤15 行。body 不设逐行 70 字符限制，精简优先。
   - 关注**问题根因**和**高阶解决方案**，而非逐行翻译 diff。

5. **分析根因**（仅修复类变更）：从 diff 反推发生了什么错误。

6. **生成 commit message**：

   ```
   <type>(<scope>): <一句用户视角的话，描述提交后的最终效果>

   **🤔 背景与动机 (Why)**
   2–4 个要点，描述问题或痛点。

   **✨ 解决方案与影响 (What & Impact)**
   2–4 个要点，描述高阶解决方案和核心影响。
   ```

   - **header ≤72 字符**（conventional commit 标准），且不含标点结尾。
   - **body 每行 ≤100 字符**（commitlint `body-max-line-length`）。
   - 使用中文 body，**不含**双引号 `"`。
   - **每节要点 ≤4 个**，用抽象概括代替逐项枚举（不列函数名、文件数、测试数）。
   - **禁止在 body 中嵌入超过 50 字符的代码/路径/符号引用**。必须用自然语言描述行为，而非复现代码符号：
     - ❌ `在 validate_settings() 之前调用 component.apply_settings(component.get_default_settings())`
     - ✅ `在注册时预置 schema 默认值，使校验前已持有符合约束的初始状态`
     - ❌ `src-tauri/src/core/config/manager.rs:54-64`
     - ✅ 只描述「在哪层做了什么」即可，不列具体行号
   - **Scope 规则**：取变更文件最多的顶级目录为 scope；均匀分布在 3+ 互不相关目录则省略 scope；禁止组合型 scope。

7. **输出结果**：展示给用户，不执行 `git commit`。

## 输出示例

```
fix(core): 在注册时预置 schema 默认值避免零值与约束冲突

**🤔 背景与动机 (Why)**
- 组件注册先于持久化配置加载，校验时读取的是 struct 零值而非 schema 默认值。
- min_items(1) 等约束下的空数组等零值导致「当前配置值无效」错误。

**✨ 解决方案与影响 (What & Impact)**
- 注册流程中先应用 schema 默认值作为初始状态，再执行校验。
- 持久化配置随后加载覆盖，不影响用户已保存的值。
```

## 注意事项

- 专注**总结变更**，不继续扩展新改动。
- **同类变更合并描述**，不逐行罗列 diff 细节。
- 涉及依赖版本变更时在 body 里注明原因。
