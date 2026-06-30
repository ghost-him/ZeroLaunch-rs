---
name: bump-version
description: 变更 ZeroLaunch-rs 的版本号（含 workspace 根、src-tauri/Cargo.toml 及 tauri.conf.json等），运行 just style，自动 commit 并打 tag
argument-hint: "<版本号> — 如 v1.0.1 或 1.0.1（v 前缀可选）"
---

# bump-version — 版本号变更

变更工作区、Tauri 配置的版本号，自动格式化代码，提交并创建 Git Tag。

## 触发方式

```
/bump-version v1.0.1
```

## 执行流程

1. **前置检查 (Pre-flight Check)**：
   - 检查当前 Git 工作区是否干净（无未提交的修改）。如果不干净，提示用户先 commit 或 stash 现有修改，并退出该流程。
   - 解析并验证版本号格式（必须是有效的 semver，如 `X.Y.Z`，自动去除可能存在的 `v` 前缀）。

2. **更新版本号**：
   - **工作区根 `Cargo.toml`**：更新 `[workspace.package] version = "X.Y.Z"`（所有 `version.workspace = true` 的 crate 自动继承，无需逐一修改）
   - **`src-tauri/Cargo.toml`**：
     - 先检查是否为 `version.workspace = true`。
     - 如果**不是**（当前为独立版本），更新其 `version = "X.Y.Z"`。
     - 如果**是**（未来改为继承后），则跳过此项（根 Cargo.toml 已覆盖）。
   - **`src-tauri/tauri.conf.json`**：更新根级 `"version"` 为 `"X.Y.Z"`（字段在根层，如 `"version": "1.0.0"`）。
   - **`src-tauri/tauri.conf.portable.json`**：同上，更新根级 `"version"` 为 `"X.Y.Z"`。
   - **`package.json`**（根目录）：更新 `"version"` 为 `"X.Y.Z"`。

3. **同步 Lockfile 与格式化**：
   - 运行 `cargo check`（或更新 lockfile 的命令），确保 `Cargo.lock` 得到更新。
   - 运行 `just style` 进行代码格式化和 clippy 修复。

4. **提交前安全校验**：
   - 运行 `git diff --stat` 展示即将提交的文件变更列表。
   - **必须** 通过 `AskUserQuestion` 工具询问用户确认，例如：
     > 以下文件将被提交（含 just style 格式化后的改动）：
     > （展示 git diff --stat 输出）
     > 是否确认提交？
   - 如果用户拒绝，**中断流程**并提示用户可手动检查或恢复。
   - 如果用户确认，继续下一步。

5. **Git 提交与 Tag**：
   - `git add -A`
   - `git commit -m "chore: 变更版本号为 v{version}"`
   - `git tag -a v{version} -m "release v{version}"`
   - **提示用户**：已完成本地提交和 Tag 创建，请确认无误后手动执行 `git push --follow-tags`。

## 错误处理

- 如果未提供参数，提示用法。
- 如果版本号格式不合规，中止并提示。
- 如果 `just style` 或任意前置步骤失败，**中断**并保留现场，提示用户错误信息。