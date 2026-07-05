# 🛠️ 开发者指南

感谢您对 ZeroLaunch-rs 的贡献！本文档将引导您如何参与项目的开发。

## 环境要求

在开始开发之前，请确保您已安装以下工具：

- **Rust**: v1.90.0+ (推荐安装最新稳定版)
- **Bun**: v1.2.22+
- **Just**: (可选) 用于快速执行开发指令
- **CodeGraph**: (推荐) 代码知识图谱索引，用于 Claude Code 上下文增强

---

## AI 辅助开发（推荐）

本项目已针对 **OMP (Oh My Pi)** 建立了一套完整的 **Harness Engineering** 体系，强烈建议使用 OMP 作为日常开发的 AI 辅助工具。

### Harness 基础设施

| 设施 | 路径 | 用途 |
| ---- | ---- | ---- |
| 工程纪律（粘性规则） | `.omp/RULES.md` | 始终生效的硬性工程纪律，永不滚出上下文 |
| 条件规则 | `.omp/rules/` (7 个域规则文件) | TTSR 条件触发，按文件路径加载架构、编码规范、数据流等上下文 |
| 技能 | `.omp/skills/` (4 个技能) | 自动化常见开发任务（详见下方） |
| CodeGraph | `.codegraph/` (代码知识图谱索引) | 毫秒级符号级代码检索，支撑 OMP 精准定位与理解代码 |
| 项目导航 | `.omp/AGENTS.md` | 项目入口文档，提供架构速览与关键文件清单 |
| 权限配置 | `.omp/config.yml` | 预配置常用命令权限，减少交互式授权弹窗 |

#### CodeGraph 安装与初始化

[CodeGraph](https://github.com/colbymchenry/codegraph) 是本项目 Harness 体系的核心组件，它将代码库预计算为 SQLite 知识图谱，提供毫秒级符号级检索能力。OMP 通过 CodeGraph MCP Server 可以在单次调用中获取任意符号的完整源码、调用链和被依赖关系，大幅减少文件读取与搜索的 Token 消耗。

**安装 CodeGraph：**

```bash
bun install -g @colbymchenry/codegraph 
```

**在本项目中初始化：**

```bash
cd ZeroLaunch-rs
codegraph init
```

初始化完成后会在项目根目录生成 `.codegraph/` 目录。CodeGraph 会自动监听文件变更并增量更新索引，无需手动维护。

> **提示**：确保 `.codegraph/` 已添加到 `.gitignore`，该目录不应提交到版本控制。

#### 工程纪律（粘性规则）

`.omp/RULES.md` — 始终生效的粘性工程纪律：async 契约、RwLock 守卫、死代码、前后端边界、用户交互、日志规范等。始终加载，不随文件路径触发。详见 [`.omp/RULES.md`](.omp/RULES.md)。

#### 条件规则文件 (.omp/rules/) 清单

| 规则文件 | 覆盖域 |
| -------- | ------ |
| `rules/plugin-system.md` | 插件系统架构：SessionRouter、Pipeline、Registry、PluginManager、事件通道 |
| `rules/sdk.md` | SDK 层规范：PluginHandle、HostApi、trait 定义与实现分离 |
| `rules/commands.md` | IPC 命令规范：薄代理模式、命令前缀与文件对应关系 |
| `rules/config.md` | 配置系统规范：ConfigManager、Configurable trait、SchemaBuilder |
| `rules/frontend.md` | 前端架构规范：组件目录、状态管理、IPC 契约层 |
| `rules/third-party-plugin.md` | 第三方插件开发与加载规范 |
| `rules/data-flow.md` | 数据流规范：前端 ↔ IPC ↔ 后端 ↔ 插件 的数据流向 |

### 支持技能（Skills）

在 OMP 中通过 `/` 前缀调用，以下是日常开发中会使用到的 skill：

#### 1. `/summarize-changes` — 代码更改总结

分析当前 Git 工作区的变更，生成符合 [Conventional Commits](https://www.conventionalcommits.org/) 规范的 commit message。

```
/summarize-changes           # 总结所有未提交的本地变更
/summarize-changes --staged  # 仅总结暂存区 (staged) 变更
```

技能会自动：
- 分析 diff 变更的文件、函数、模块
- 反推修复类变更的根因
- 按变更文件数量自动推导 scope
- 输出结构化的 commit message（含中文 body 说明）

> **注意**：此技能仅生成 commit message，不会执行 `git commit`。

#### 2. `/code-review` — 代码审查

利用多 agent 并行审查机制，对您的代码变更进行深度的多维度审查。它会从逻辑正确性、回归风险、架构边界耦合、以及项目规则一致性等多个方面分析代码变更。

以下是支持审核的范围，个人更推荐使用自然语言来描述审查的范围：

```
/code-review                       # 审查当前工作区所有未提交的变更（默认）
/code-review --staged              # 仅审查暂存区变更
/code-review --range <ref>..<ref>  # 审查指定 git range 的变更
/code-review --count <N>           # 审查最近 N 次 commit 的变更
/code-review --branch              # 审查当前分支下的全部变更
```

技能会自动：
- 对变更文件按模块分组，并行分发给多个 reviewer agent
- 每个 reviewer 独立审查，覆盖逻辑正确性、回归风险、架构边界、规则一致性
- 对审查发现进行对抗性验证，过滤误报
- 汇报分级结果，标注严重性与影响范围

> **注意**：审查结果是辅助工具，请根据实际情况**选择性接受建议**，**避免过度优化**。代码审查可能会指出一些潜在的改进点，但并非必须全部修复——请结合代码的上下文和实际需求做出判断。

> **💡 最佳实践**：建议在独立分支上完成代码变更，然后使用 `/code-review 审核本分支下的所有更改` 对整个分支的全部更改进行一站式审查。相比逐一审查工作区未提交的零散变更，分支级别的审查能提供更完整的上下文，也方便在审查后统一迭代修复。

---

## 构建步骤

### 克隆仓库

```bash
git clone https://github.com/ghost-him/ZeroLaunch-rs.git
cd ZeroLaunch-rs
```

### 安装依赖

```bash
bun install
# 用于简化常用指令
cargo install just
```

### 开发模式

启动开发服务器进行实时开发和调试：

```bash
bun run tauri dev
```

### 生产构建

使用 `xtask` 自动化构建工具，详见 [xtask/README.md](xtask/README.md)：

```bash
cd xtask
cargo run --bin xtask build-installer --arch x64   # 构建 x64 安装包
cargo run --bin xtask build-all                     # 构建所有版本
cargo run --bin xtask clean                         # 清理构建产物
```

## 数据目录结构

了解程序的数据存储结构有助于调试和开发。

### 数据根目录

程序分为**安装包版本**与**便携版**两个版本，数据根目录不同：

- **安装包版本**：`$HOME\.ZeroLaunch-rs\`
- **便携版**：软件 `exe` 所在目录

### 目录结构

```
.ZeroLaunch-rs/                           # 数据根目录（安装包版本）/ exe 所在目录（便携版）
├── logs/                                 # 运行日志
├── icons/                                # 程序图标缓存
└── config/                               # 配置文件目录
    └── zerolaunch_config.json            # 主配置数据库
```

---

## 贡献流程

### 推荐工作流（OMP）

使用 OMP 时，推荐的开发流程为：

```
1. 创建功能分支     → 基于 main 创建独立分支（如 feature/my-change）
       ↓
2. 编写/修改代码
       ↓
3. just style          → 自动格式化 Rust 代码 + Clippy 修复
       ↓
4. /code-review        → 多 agent 并行审查代码变更（建议选择性接受，避免过度优化）
       ↓
5. /summarize-changes  → 生成规范的 commit message
       ↓
6. git commit          → 使用步骤 5 生成的信息提交
       ↓
7. git push            → 推送到远程分支
       ↓
8. 创建 Pull Request   → 在 GitHub 上提 PR
```

### 传统工作流

1. Fork 本仓库并克隆到本地
2. 基于 `main` 分支创建您的功能分支 (`git checkout -b feature/AmazingFeature`)
3. 编写/修改代码
4. 运行 `just style` 格式化代码（等效于 `cargo fmt --all` + `cargo clippy --workspace --fix --allow-dirty --allow-staged`）
5. 运行 `cargo check` 确保编译通过
6. 运行 `/code-review` skill 审查代码变更（使用方式看上文的 skill 介绍）
7. 提交您的更改，推荐使用 `/summarize-changes` skill 来一键生成 commit 信息
8. 推送到您的 Fork 分支 (`git push origin feature/AmazingFeature`)
9. 在 GitHub 上创建 Pull Request

### 代码风格

请确保您的代码遵循项目现有的代码风格：

- **Rust 代码**：使用 `just style` 完成代码的格式化（等效于 `cargo fmt --all` + `cargo clippy --workspace --fix --allow-dirty --allow-staged`）
- **TypeScript/Vue 代码**：遵循现有的代码风格惯例

### 提交前检查清单

提交 PR 前，请确保：

- [ ] 代码能够成功编译（运行 `cargo check`）
- [ ] 代码已格式化（运行 `just style`）
- [ ] 所有现有功能仍然正常工作
- [ ] 新功能包含适当的测试（不强制）

### Commit Message 规范

本项目遵循 [Conventional Commits 2.1.0](https://www.conventionalcommits.org/) 规范：

```
<type>(<scope>): <精简描述>

<详细说明（可选）>
```

常用 type：

| type | 用途 |
| ---- | ---- |
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `refactor` | 重构（不改变功能） |
| `chore` | 杂项（依赖更新、构建脚本等） |
| `docs` | 文档变更 |
| `style` | 代码格式调整 |
| `perf` | 性能优化 |
| `test` | 测试相关 |

建议使用 `/summarize-changes` 自动生成规范的 commit message。

### 问题报告

如果您发现了 bug 或有改进建议，请在 GitHub Issues 中报告。提交 Issue 时，请尽量提供：

- 问题的详细描述（如果有图片，可将图片附带上）
- 复现步骤
- 系统环境信息（Windows 版本、Rust 版本等）
- 相关的日志输出（可在 `C:\Users\[username]\.ZeroLaunch-rs\logs\` 目录下找到，或直接在设置页面中导出）

---

## 许可证

本项目采用 GPLv3 许可证。参与贡献即表示您同意将您的贡献代码置于相同的许可证下。

## 第三方依赖

本项目使用了以下优秀的开源库和资源：

- [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - 中文转拼音核心词典
- [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP 应用索引方案
- [bootstrap](https://icons.bootcss.com/) - 程序图标
- [icon-icons](https://icon-icons.com/zh/) - 程序图标
- [Follower-v2.0](https://github.com/MrBeanCpp/Follower-v2.0) - 全屏检测方案

## 资源与联系方式

- **GitHub 仓库**: [ghost-him/ZeroLaunch-rs](https://github.com/ghost-him/ZeroLaunch-rs)
- **项目官网**: [zerolaunch.ghost-him.com](https://zerolaunch.ghost-him.com)
- **反馈问题**: [GitHub Issues](https://github.com/ghost-him/ZeroLaunch-rs/issues)

感谢您的每一份贡献！🙏
