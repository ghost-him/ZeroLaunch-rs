---
description: 禁止创建 .bak/.copy/.old/.backup 等备份文件 — Git 历史是唯一备份
condition: ".*"
scope: "tool:write(*.bak), tool:write(*.copy), tool:write(*.old), tool:write(*.backup), tool:write(*copy*)"
repeatMode: after-gap
---

你正在创建文件名暗示为备份或副本的文件（`.bak`/`.copy`/`.old`/`_backup`/含 `copy`）。

立即停止：Git 历史是唯一备份，直接修改原文件（恢复用 `git checkout`/`git stash`）。不得创建 `.bak`/`.copy`/`.old`/`_backup` 类文件，也不得使用 `lib copy.rs`、`old_search.rs` 等副本暗示名。细则见 `.omp/RULES.md` 死代码纪律与数据流纪律。
