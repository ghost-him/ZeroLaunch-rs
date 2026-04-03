# AGENTS.md - AI 协作指南

本文档用于指导 AI 助手快速理解 ZeroLaunch-rs 项目，并提供协作时的注意事项。

---

## 一、项目概述

**ZeroLaunch-rs** 是一个 Windows 平台的快速启动器，使用 Tauri + Vue 技术栈：

| 层级 | 技术栈             | 目录         |
| ---- | ------------------ | ------------ |
| 后端 | Rust + Tauri       | `src-tauri/` |
| 前端 | Vue 3 + TypeScript | `src-ui/`    |

### 核心功能

- 程序搜索与启动（支持 UWP、普通程序、URL、书签）
- 拼音搜索与模糊匹配
- 语义搜索（AI 向量相似度）
- Everything 文件搜索集成
- 计算器插件
- 多存储后端（本地、WebDAV、OneDrive）

---

## 二、项目结构速览

```
src-tauri/src/
├── commands/           # Tauri IPC 命令层（API 入口）
├── core/               # 核心功能
│   ├── ai/             # AI 模型加载与推理
│   └── storage/        # 存储后端实现
├── modules/            # 业务模块（旧架构，逐步迁移）
│   ├── program_manager/    # 程序管理核心
│   ├── config/             # 配置管理
│   ├── icon_manager/       # 图标管理
│   └── ...
├── plugin/             # 插件实现（新架构）
│   ├── data_source/        # 数据源实现
│   ├── keyword_optimizer/  # 关键字优化器
│   ├── launcher/           # 启动器实现
│   ├── score_booster/      # 分数提升器
│   ├── search_engine/      # 搜索引擎实现
│   └── triggerable/        # 触发型插件
├── plugin_system/      # 插件系统核心（trait 定义、注册中心、分发器）
├── state/              # 应用状态
└── utils/              # 工具函数
```

---

## 三、插件系统架构

### 3.1 Trait 继承关系

```
                    ┌───────────────────┐
                    │   Configurable    │  ← 基础配置能力（所有组件都必须实现）
                    └─────────┬─────────┘
                              │
        ┌─────────┬───────────┼───────────┬─────────────┬─────────────┐
        ▼         ▼           ▼           ▼             ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐ ┌─────────────┐ ┌───────────┐ ┌─────────┐
│DataSource │ │KeywordOpt │ │SearchEng  │ │ScoreBooster │ │  Launcher  │ │ Plugin  │
└───────────┘ └───────────┘ └───────────┘ └─────────────┘ └───────────┘ └─────────┘
```

### 3.2 各 Trait 职责

| Trait              | 职责             | 核心方法                                                 |
| ------------------ | ---------------- | -------------------------------------------------------- |
| `Configurable`     | 配置管理基础     | `setting_schema()`, `get_settings()`, `apply_settings()` |
| `DataSource`       | 提供搜索候选项   | `fetch_candidates()`                                     |
| `KeywordOptimizer` | 扩展/优化关键字  | `optimize()`, `uses_context()`, `get_priority()`         |
| `SearchEngine`     | 计算匹配分数     | `calculate_scores()`                                     |
| `ScoreBooster`     | 个性化排序优化   | `record()`, `boost()`                                    |
| `Launcher`         | 执行启动操作     | `supported_method()`, `launch()`                         |
| `Plugin`           | 完整独立功能单元 | `query()`, `execute_action()`                            |

### 3.3 关键文件

| 文件                                                                     | 内容            |
| ------------------------------------------------------------------------ | --------------- |
| [plugin_system/types.rs](src-tauri/src/plugin_system/types.rs)           | 所有 Trait 定义 |
| [plugin_system/registry.rs](src-tauri/src/plugin_system/registry.rs)     | 插件注册中心    |
| [plugin_system/dispatcher.rs](src-tauri/src/plugin_system/dispatcher.rs) | 查询分发器      |
| [plugin/readme.md](src-tauri/src/plugin/readme.md)                       | 插件目录说明    |

---

## 四、协作注意事项

### 4.1 工作流程

> **重要**：请遵循以下流程，除非用户明确要求直接执行。

1. **先设计，后实现**
   - 收到任务后，先分析现有代码
   - 输出设计方案/修改计划，等待用户确认
   - 用户确认后，再进行代码更改

2. **分步骤执行**
   - 复杂任务拆分为多个小步骤
   - 每完成一步，简要说明，再继续下一步

3. **代码审查范围**
   - 只检查用户明确要求检查的内容
   - 不要主动扩展检查范围

4. **任务完成后总结**
   - 在所有的任务完成以后，需要做一个总结性的内容，输出当前任务的目标是什么，完成了什么
   - 为用户输出 commit 信息，信息的枚式如下所示，但是不可以代替用户完成 commit

### 4.2 代码风格

1. **默认添加简单的注释**
   - 除非用户明确要求，否则不要在函数内部添加代码注释
   - 默认在每个函数的开头添加该函数的作用，参数值与返回值的含义
   - 保持代码简洁

2. **遵循现有模式**
   - 新增组件时，参考同类组件的实现方式
   - 保持命名、结构、trait 实现的一致性

3. **配置读取注意**
   - 读取 JSON 数值配置时，使用 `as_f64()` 而非 `as_i64()`
   - 原因：前端可能传入 `30.0` 形式，`as_i64()` 会静默返回 `None`

### 4.3 语言要求

- 使用中文与用户交流
- 代码中的用户可见文本使用中文（如配置项 label）
- commit 信息使用中文

### 4.4 Commit 信息格式

```
<type>: <subject>

<body>
```

type 类型：
- `feat`: 新功能
- `fix`: 修复 bug
- `refactor`: 重构
- `docs`: 文档更新
- `style`: 代码格式调整
- `test`: 测试相关
- `chore`: 构建/工具相关

示例：
```
refactor: 将 convert_search_keywords 中的基本函数封装为可插拔的 KeywordOptimizer 插件

- 新增 VersionNumberRemover: 移除程序名中的版本号
- 新增 SpaceNormalizer: 规范化空格
- 新增 LowerCaseConverter: 小写转换
```