---
description: 禁止创建 .bak/.copy/.old/.backup 等备份文件 — Git 历史是唯一备份
condition: ".*"
scope: "tool:write(*.bak), tool:write(*.copy), tool:write(*.old), tool:write(*.backup), tool:write(*_backup*), tool:write(*copy*)"
repeatMode: after-gap
---

你正在创建一个文件名暗示为备份或副本的文件（如 `.bak`、`.copy`、`.old`、`_backup`、含 `copy` 的文件名）。

Git 历史是唯一备份。NEVER 创建这类文件。直接修改原文件，需要恢复时用 `git checkout` 或 `git stash`。

同时禁止：
- `lib copy.rs`、`old_search.rs` 等文件名暗示副本的文件
- `// temp`、`// TODO: remove` 标记存活超过一次会话的代码
