---
description: 禁止使用 println!/eprintln! 宏 — 必须用 tracing crate 进行日志记录
condition: "println!|eprintln!"
scope: "tool:edit(*.rs), tool:write(*.rs)"
interruptMode: tool-only
---

你在 Rust 代码中使用了 `println!()` 或 `eprintln!()` 宏。本项目必须使用 `tracing` crate 进行日志记录，禁止使用标准库的打印宏。

改用 `tracing` 宏：
- `println!("xxx")` → `info!("xxx")`（启动阶段关键步骤）或 `debug!("xxx")`（运行时频繁调用）
- `eprintln!("xxx")` → `warn!("xxx")`（可恢复错误）或 `error!("xxx")`（不可恢复错误）

注意：NEVER 在日志中输出用户敏感信息（密码、完整路径中的用户名等）。
