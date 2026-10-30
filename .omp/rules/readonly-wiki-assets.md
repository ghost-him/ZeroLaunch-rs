---
description: src-tauri/assets/wiki 是 GitHub wiki 的自动拉取镜像 — 禁止在此目录编辑或新建页面，手册正文改 wiki 检出目录
condition: ".*"
scope: "tool:edit(src-tauri/assets/wiki/**), tool:write(src-tauri/assets/wiki/**)"
---

你正在修改 `src-tauri/assets/wiki/` 下的文件。

**立即停止**：该目录是从 GitHub wiki 自动拉取的镜像（构建期嵌入为「使用手册」）。在这里改的内容无法同步回 wiki，下次拉取会被覆盖，改动会静默丢失。

正确做法：

- 用户手册正文改 `ZeroLaunch-rs.wiki/`（与主仓并列的 wiki 检出目录），按技能 `zerolaunch-wiki-authoring` 的写作规范编辑，并在该目录单独提交 / 推送。
- 代码改动需要同步手册时，只改 wiki 检出目录，**不要**再在 `src-tauri/assets/wiki/` 里重复修改一次。
- 本机没有 wiki 检出目录时，明确告知用户需要先 `git clone <仓库>.wiki`，不要退回修改镜像目录来「凑合」。
