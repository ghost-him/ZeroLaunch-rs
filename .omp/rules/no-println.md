---
description: 禁止使用 println!/eprintln! 宏 — 必须用 tracing crate 进行日志记录
condition: "println!|eprintln!"
scope: "tool:edit(src-tauri/**), tool:write(src-tauri/**), tool:edit(crates/**), tool:write(crates/**), tool:edit(zerolaunch-cli/**), tool:write(zerolaunch-cli/**)"
---

你在 Rust 代码中使用了 `println!()` 或 `eprintln!()` 宏。本项目必须使用 `tracing` crate 进行日志记录，禁止使用标准库的打印宏。

改用 `tracing` 宏：
- `println!("xxx")` → `info!("xxx")`（启动阶段关键步骤）或 `debug!("xxx")`（运行时频繁调用）
- `eprintln!("xxx")` → `warn!("xxx")`（可恢复错误）或 `error!("xxx")`（不可恢复错误）

**豁免场景**（stdout/stderr 是产品接口而非日志通道，不适用本规则）：
- `zerolaunch-cli`：`stdout` 输出查询结果（如 `zl query --json` 的 JSON）、`stderr` 输出错误提示——这是 CLI 的功能输出；内部诊断才用 tracing
- `src-tauri/src/logging.rs`：tracing 初始化之前的兜底 `eprintln!`（tracing 未就绪时的唯一错误通道）
- `xtask/`：构建编排 CLI（`xtask/src/main.rs`），`println!` 是面向用户的功能输出（构建目标列表、目录切换提示等）——本规则 scope 不含 `xtask/`，这些调用不是违规

注意：NEVER 在日志中输出用户敏感信息（密码、完整路径中的用户名等）。
