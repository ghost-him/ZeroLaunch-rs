---
name: bump-version
description: ZeroLaunch-rs 完整发版流程：主程序与 crates 版本号 bump、just style、commit、注释型 tag、双远端推送、crates.io 发布（只发有实改的 crate）与 Release workflow 触发
argument-hint: "<版本号> [发布重点] — 如 v1.5.5；发布重点为可选多行文本，写入 tag 注释并影响 AI 更新摘要"
---

# bump-version — 版本号变更与发版

变更工作区 Cargo 版本与 crates 版本号，格式化代码，提交、打 tag、推送远端，并发布本次有实质变更的 crates。

## 触发方式

```
/bump-version v1.0.1
/bump-version v1.6.0 重点：AI 搜索语义排序重写；插件市场安装卡顿修复
```

## 执行流程

1. **前置检查 (Pre-flight Check)**：
   - 检查当前 Git 工作区是否干净（无未提交的修改）。如果不干净，提示用户先 commit 或 stash 现有修改，并退出该流程。
   - 解析并验证版本号格式（必须是有效的 semver，如 `X.Y.Z`，自动去除可能存在的 `v` 前缀）。
   - 确认本地与远端都还没有该 tag：`git tag -l v{version}`；远端用第 7 步的 HTTPS 方式查询。
   - 参数中版本号之外的内容即「发布重点」（可选，可多行/多行分隔），原样写入 tag 注释。
   - 版本号未明确给出时：按本仓既有实践取**下一个 patch**（`v1.5.2`、`v1.5.3`、`v1.5.4` 都含 `feat` 提交但仍走 patch）；minor/major 只在用户明确要求时使用。

2. **更新主程序版本号（1 处）**：
   - **工作区根 `Cargo.toml`**：更新 `[workspace.package] version = "X.Y.Z"`（`app` 与 `crates/zerolaunch-rs` 均继承，无需改子 crate）。
   - 安装包版本由 `xtask` 从 `cargo metadata` 读取，不维护第二份版本号。

3. **更新 crates 版本号（只升本次真有内容变更的）**：
   - 判定唯一证据是**与 crates.io 已发布 tarball 逐字节对比**，不要凭 git log 猜：

     ```bash
     W=./.zlpub-tmp; rm -rf "$W"; mkdir -p "$W"
     for pair in "zerolaunch-plugin-api:crates/plugin-api:0.2.5" \
                 "zerolaunch-plugin-protocol:crates/plugin-protocol:0.2.3" \
                 "zerolaunch-plugin-host:crates/plugin-host:0.2.3" \
                 "zerolaunch-plugin-sdk-rust:crates/plugin-sdk-rust:0.2.3" \
                 "zerolaunch-platform-windows:crates/platform-windows:0.2.4"; do
       n=${pair%%:*}; rest=${pair#*:}; d=${rest%%:*}; v=${rest##*:}
       curl -sSL -A zl-check -o "$W/$n.crate" "https://static.crates.io/crates/$n/$n-$v.crate"
       tar -xzf "$W/$n.crate" -C "$W"
       echo "=== $n $v ==="
       diff -r -q --exclude=target --exclude=.cargo_vcs_info.json \
         --exclude=Cargo.lock --exclude=Cargo.toml.orig --exclude=Cargo.toml \
         "$W/$n-$v" "$d" || true
     done
     rm -rf "$W"
     ```

     版本号取「本地 Cargo.toml 与 crates.io 当前值」——两者相等说明上次已发、本次需要重发。
   - 忽略项只有 `Cargo.toml`、`Cargo.toml.orig`、`Cargo.lock`、`.cargo_vcs_info.json`；其余差异都是真实漂移。
   - **无内容差异 → 不升号、不重发**；根 `Cargo.toml` 里 `[workspace.dependencies]` 指向它的 `version` 保持旧值（caret 下限不能超过已发布内容）。
   - **每个 crate 只升自己的下一个 patch**：禁止为了统一而跳号，也不强求整批同号（0.x 下各 crate 版本不同是正常的）。
   - 改了 crate 版本后，同步根 `Cargo.toml` 的 `zerolaunch-* = { path = ..., version = ... }` pin。
   - 2026-11 `v1.5.5` 实例：`plugin-api 0.2.5 → 0.2.6`（`AppInfo` 新增 `search_keywords`）、`platform-windows 0.2.4 → 0.2.5`（包名英文别名）；`protocol` / `host` / `sdk-rust` 逐字节一致，不动。

4. **同步 Lockfile 与格式化**：
   - 运行 `cargo check --workspace`，确保 `Cargo.lock` 得到更新且全工作区可编译。
   - 运行 `just style`（clippy --fix + `cargo fmt --all` + `cargo package` 预检，含 crates.io 索引刷新）。
   - 核对 `Cargo.lock` 中每个 `zerolaunch*` 条目都是新号。

5. **提交前安全校验**：
   - 运行 `git diff --stat` 展示即将提交的文件变更列表。
   - **必须** 通过 `ask` 工具询问用户确认，例如：
     > 以下文件将被提交（含 just style 格式化后的改动）：
     > （展示 git diff --stat 输出）
     > 是否确认提交？
   - 如果用户拒绝，**中断流程**并提示用户可手动检查或恢复。
   - 如果用户确认，继续下一步。

6. **Git 提交与 Tag**：
   - `git add -A`
   - `git commit -m "chore: 变更版本号为 v{version}"`
   - 创建**注释型** tag（轻量 tag 没有注释，AI 摘要拿不到发布重点）：
     - 无发布重点：`git tag -a v{version} -m "release v{version}"`
     - 有发布重点：`git tag -a v{version} -m "release v{version}

{发布重点原样多行文本}"`
   - tag 注释由 `.github/workflows/release.yml` 的 `Generate AI summary` 读取，作为**最高优先级**决定 AI 摘要写哪些内容：建议写 1-3 条用户可感知的改动（如“重点：…”“次重点：…”），不要把内部重构写进去。
   - tag 已推送后要改注释须 `git tag -f -a` + 强推，会改写远端 tag；仅未推送时安全，推送了就留到下个版本。

7. **推送到两个远端（GitHub + Gitee）**：
   - `origin` 配置了两条 push URL（GitHub + Gitee），且 SSH 到 GitHub 可能被劫持（连接落到 `198.18.0.77` 被关闭）。此时**不要**用 `git push origin`，改用显式 URL 分别推送；GitHub 走 HTTPS + `gh` 凭据：
     ```bash
     GIT_TERMINAL_PROMPT=0 git -c credential.helper= \
       -c credential.helper='!gh auth git-credential' \
       push https://github.com/ghost-him/ZeroLaunch-rs.git main v{version}

     git push git@gitee.com:ghost-him/ZeroLaunch-rs.git main v{version}
     ```
   - 若 GitHub SSH 正常，也可 `git push --follow-tags`；出现 partial push（一端成功一端失败）时**只补推失败的那个远端**，禁止 `git pull --rebase`。
   - 推送后核对两端：`git ls-remote <url> refs/heads/main refs/tags/v{version}`。
   - tag 推送即触发 Release workflow，不需要单独手动触发。

8. **发布 crates.io（只发有变更的，顺序有硬约束）**：
   - 顺序：`plugin-api` →（等索引刷新）`plugin-protocol` / `platform-windows` →（等索引刷新）`plugin-host` / `plugin-sdk-rust`。
   - 工作区必须干净；正式版本**不要**加 `--allow-dirty`。一律用 `-p`，不要在工作区根裸发。
   - `cargo publish -p <crate>`；若报 `no matching package named ... found`，属依赖尚未上架，等 1–5 分钟重试。
   - 发布后逐个核验：
     ```bash
     curl -s -A zl-check "https://crates.io/api/v1/crates/<name>" \
       | grep -o '"max_stable_version":"[^"]*"' | head -1
     ```

9. **主程序 Release**：
   - tag 推送后 `.github/workflows/release.yml`（Release Build）自动开始；是否等待它跑完按用户意愿，不强制。
   - 查看状态：`gh run list --limit 3`；发布产物：`gh release view v{version}`。

10. **收尾核验清单**：
   - 本地与两个远端的 `main`、`v{version}` 指向同一个提交；
   - `Cargo.lock` 里所有 `zerolaunch*` 版本与实际发布一致；
   - crates.io 的 `max_stable_version` 等于新号；
   - `git status` 干净（临时对比目录已删除）；
   - 下游插件仓库 pin 的是 `"0.2"` 之类 caret 范围时无需改 pin；要用新 SDK 执行 `cargo update -p zerolaunch-plugin-api`。

## 错误处理

- 如果未提供参数，提示用法。
- 如果版本号格式不合规，中止并提示。
- 如果 `just style` 或任意前置步骤失败，**中断**并保留现场，提示用户错误信息。
- 如果 crates 发布会话中途失败：已成功发布的 crate 不回滚（crates.io 不可覆盖），从失败的 crate 继续，必要时升 patch 重试。
- 如果 GitHub 推送失败：先判断是 SSH 劫持还是认证问题；前者换 HTTPS，后者检查 `gh auth status`（需要 `repo` 与 `workflow` scope）。
