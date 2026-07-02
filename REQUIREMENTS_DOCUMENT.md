# ZeroLaunch-rs 软件需求文档

> **文档版本**: 1.0  
> **项目名称**: ZeroLaunch-rs  
> **技术栈**: Tauri 2.x (Rust 后端) + Vue 3 (TypeScript 前端) + Element Plus UI  
> **目标平台**: Windows (x86_64 / aarch64)

---

## 目录

1. [1. 系统概述](#1-系统概述)
2. [2. 启动与初始化流程](#2-启动与初始化流程)
3. [3. 主搜索窗口（前端界面）](#3-主搜索窗口前端界面)
4. [4. 系统托盘](#4-系统托盘)
5. [5. 欢迎页面](#5-欢迎页面)
6. [6. 设置窗口](#6-设置窗口)
7. [7. 程序搜索与管理模块（后端核心）](#7-程序搜索与管理模块后端核心)
8. [8. 快捷键管理系统](#8-快捷键管理系统)
9. [9. 配置管理系统](#9-配置管理系统)
10. [10. 云存储/远程配置同步](#10-云存储远程配置同步)
11. [11. Everything 集成搜索](#11-everything-集成搜索)
12. [12. 参数解析器](#12-参数解析器)
13. [13. 刷新调度器](#13-刷新调度器)
14. [14. 图标管理器](#14-图标管理器)
15. [15. 书签加载器](#15-书签加载器)
16. [16. 浏览器书签检测](#16-浏览器书签检测)
17. [17. AI 语义搜索（实验性功能）](#17-ai-语义搜索实验性功能)
18. [18. 版本检查器](#18-版本检查器)
19. [19. 调试与日志系统](#19-调试与日志系统)
20. [20. 国际化（i18n）](#20-国际化i18n)
21. [21. 窗口特效与外观](#21-窗口特效与外观)
22. [22. 内置命令系统](#22-内置命令系统)
23. [23. 应用退出与清理流程](#23-应用退出与清理流程)
24. [附录A: 全部 Tauri Command 列表](#附录a-全部-tauri-command-列表)

---

## 1. 系统概述

ZeroLaunch-rs 是一款 Windows 平台的应用快速启动器，支持模糊搜索、拼音匹配、语义搜索等多种搜索算法，通过全局快捷键唤醒搜索窗口，实现快速启动应用程序、打开文件、访问网址、执行自定义命令等功能。

### 1.1 核心能力

| 能力            | 描述                                              |
| --------------- | ------------------------------------------------- |
| 应用搜索与启动  | 扫描指定目录，索引可执行程序，支持多种搜索算法    |
| 全局快捷键唤醒  | 通过 Alt+Space（可自定义）全局快捷键唤醒搜索窗口  |
| Everything 集成 | 集成 Everything SDK 实现全盘文件搜索（仅 x86_64） |
| 自定义 Web 搜索 | 通过关键字触发自定义网址搜索                      |
| 自定义命令      | 执行自定义命令行指令                              |
| 内置命令        | 设置、刷新数据库、游戏模式切换等系统操作          |
| 书签搜索        | 索引浏览器书签文件                                |
| 语义搜索        | 基于嵌入模型的 AI 语义搜索（实验性）              |
| 参数模板        | 支持带参数占位符的启动模板                        |
| 远程配置同步    | 支持 WebDAV / 本地路径 / OneDrive 配置同步        |
| 多语言          | 支持简体中文、繁体中文、英语                      |

---

## 2. 启动与初始化流程

### 2.1 启动阶段划分

系统启动按照以下阶段有序进行：

**阶段 1: 基础资源初始化**
- 初始化日志系统（`init_logging`）
- 记录应用启动信息（`log_application_start`）
- 初始化 Windows COM 库
- 注册 Tauri 插件（shell、single_instance、deep_link、dialog、notification）
- 注册图标路径

**阶段 2: 核心状态初始化**
- 创建 `AppState` 全局状态实例
- 初始化 `StorageManager`（存储管理器）
- 初始化 `RuntimeConfig`（运行时配置）
- 初始化 `ProgramManager`（程序管理器）
- 初始化 `IconManager`（图标管理器）
- 初始化 `BookmarkLoader`（书签加载器）
- 初始化 `RefreshScheduler`（刷新调度器）
- 初始化 `EverythingManager`（Everything 管理器，仅 x86_64）
- 加载远程配置数据
- 初始化语义后端（如启用）

**阶段 3: UI 组件初始化**
- 创建并配置主搜索窗口（`main` 窗口）
- 创建设置窗口（`setting_window`）
- 初始化系统托盘图标和菜单

**阶段 4: 交互服务初始化**
- 启动快捷键管理器（`ShortcutManager`），注册全局快捷键

**阶段 5: 配置应用和外部服务**
- 应用运行时配置到各个子系统
- 注册深度链接（Deep Link）
- 显示桌面通知"应用已启动"

### 2.2 单实例保护

通过 `tauri-plugin-single-instance` 插件实现，当检测到第二个实例启动时，向前端发送通知提示"已经运行了一个实例"。

### 2.3 深度链接

注册 URL 协议处理（`zerolaunch-rs://`），用于接收外部传入的 URL 参数，通过 `AsyncWaitingHashMap` 存储待处理的深度链接请求。

### 2.4 首次启动检测

- 读取本地配置文件失败（文件不存在）时判断为首次启动
- 首次启动时：显示欢迎窗口，创建默认本地配置
- 欢迎页面版本更新时（`WELCOME_PAGE_VERSION` 变更），也会再次显示欢迎窗口

---

## 3. 主搜索窗口（前端界面）

### 3.1 窗口概览

- 窗口标签: `main`
- 默认大小: 根据屏幕 DPI 自适应
- 窗口特性: 透明背景、无标题栏、可拖动（根据配置）、圆角效果
- 前端路由路径: `/`

### 3.2 窗口布局结构

```
┌─────────────────────────────────────┐
│  SearchBar (搜索栏)                  │  ← 固定高度（可配置）
├─────────────────────────────────────┤
│  ParameterPanel (参数面板, 条件显示) │  ← 仅在参数输入模式下显示
├─────────────────────────────────────┤
│  ResultList / EverythingPanel       │  ← 搜索结果区域（根据模式切换）
│  ┌───────────────────────────────┐  │
│  │ [图标] 程序名               │  │
│  │ [图标] 程序名               │  │
│  │ ...                          │  │
│  └───────────────────────────────┘  │
├─────────────────────────────────────┤
│  Footer (底栏, 可隐藏)              │  ← 固定高度（可配置）
└─────────────────────────────────────┘
```

### 3.3 SearchBar（搜索栏）

**功能描述**:
- 提供文本输入框，支持键盘输入
- 支持自定义占位符文本（`search_bar_placeholder`）
- 支持自定义字体、字号、字体颜色、占位符颜色
- 支持搜索栏动画效果（输入时光标和文本过渡动画）
- 点击输入框以外区域（输入框容器但非 input 元素）可拖动窗口
- 右键搜索栏弹出上下文菜单

**上下文菜单项**:
1. 打开设置界面
2. 刷新数据库

**交互行为**:
- 输入文本后，实时触发 `handle_search_text` 命令向后端发送搜索请求
- 支持连续输入的防抖处理（输入状态管理）

### 3.4 AnimatedInput（动画输入组件）

**功能描述**:
- 自定义动画输入组件，配合搜索栏使用
- 自定义光标渲染，支持闪烁动画和位置过渡动画
- 支持动态动画模式（dynamic 属性控制）

### 3.5 ResultList（结果列表）

**功能描述**:
- 显示搜索结果列表
- 每一行包含: 图标 + 程序名称（+ 显示启动命令路径，可选配置）
- 支持高亮匹配字符（使用 `<mark>` 标签）
- 支持选中项高亮
- 支持滚动模式（当结果数量超过 `scroll_threshold` 配置值时启用）
- 支持鼠标点击选中、Ctrl+点击、右键上下文菜单

**右键上下文菜单项**:
1. 打开文件位置
2. 以管理员身份运行
3. 屏蔽该结果

**数据来源**:
- 正常模式: `handle_search_text` 返回的搜索结果 `Array<[number, string, string]>`
- Alt 按住模式: `command_get_latest_launch_program` 返回的最近启动程序列表
- 图标: 通过 `load_program_icon` 异步加载，支持批量预加载

### 3.6 ParameterPanel（参数面板）

**功能描述**:
- 当启动的程序需要参数时显示
- 分步引导用户输入参数（支持位置参数 `{}`）
- 显示当前进度（第N个/总共M个参数）
- 提供参数预览功能，实时展示填充后的完整命令行
- 支持确认（下一步/启动）和取消操作

**交互流程**:
1. 用户选择某个程序启动，后端检测到该程序需要参数
2. 前端展示参数面板，引导用户输入第一个参数
3. 用户输入并按确定，进入下一个参数输入
4. 所有参数收集完成后，点击"启动"执行最终命令
5. 用户可以随时取消

### 3.7 EverythingPanel（Everything 集成面板）

**功能描述**:
- 当用户切换到 Everything 搜索模式时显示
- 显示文件搜索结果（文件名 + 目录路径）
- 支持文件图标加载
- 显示搜索状态和消息提示
- 仅 x86_64 架构支持

**交互行为**:
- 搜索文本变化时自动触发 `handle_everything_search`
- 点击结果执行 `launch_everything_item`
- 支持键盘导航（上下方向键选择）

### 3.8 SubMenu（右键上下文菜单）

**功能描述**:
- 通用的弹出菜单组件，支持自定义菜单项
- 支持键盘导航（上下选择、Enter 确认）
- 自动调整位置确保菜单在窗口内显示
- 支持暗色/亮色主题

### 3.9 Footer（底栏）

**功能描述**:
- 左侧显示状态文本（默认显示 app_config.tips）
- 右侧显示搜索状态信息
- 支持通过配置隐藏（`footer_height` 为 0 时隐藏）
- 支持拖拽窗口（如果启用）

**右侧状态信息**:
- 当前搜索算法名称（标准/LaunchyQT/语义等）
- "最近打开"（Alt 键按下时）
- "正在刷新数据集..."
- "当前正在后台加载程序图标..."
- "行内参数模式"
- 语义搜索回退提示

### 3.10 输入上下文与快捷键处理

输入上下文（`InputContext`）枚举:
- `MainSearch`: 主搜索模式（默认）
- `Everything`: Everything 文件搜索模式
- `ParameterInput`: 参数输入模式

系统根据当前输入上下文分发键盘事件到对应的快捷键处理器:
1. **MainSearchShortcutHandler**: 主搜索页面快捷键处理
2. **EverythingShortcutHandler**: Everything 搜索快捷键处理
3. **ParameterInputShortcutHandler**: 参数输入模式快捷键处理

### 3.11 主搜索窗口快捷键一览

| 快捷键                             | 功能                                       |
| ---------------------------------- | ------------------------------------------ |
| `Alt+Space` (可自定义)             | 唤醒/隐藏主搜索窗口                        |
| `Alt`                              | 临时切换到"最近使用"排序                   |
| `Enter`                            | 启动选中项                                 |
| `Ctrl+Enter`                       | 以管理员身份启动选中项                     |
| `Shift+Enter`                      | 置顶当前已打开的窗口                       |
| `Space` (当 `space_is_enter` 启用) | 同 Enter                                   |
| `Esc`                              | 清空输入（有内容时）/ 隐藏窗口（空输入时） |
| `↑/↓` 或 `Ctrl+J/K`                | 上/下移动选择                              |
| `→` 或 `Ctrl+L`                    | 显示右键菜单                               |
| `←` 或 `Ctrl+H`                    | 关闭右键菜单                               |
| `Ctrl+E`                           | 切换到 Everything 搜索模式                 |
| `Ctrl+U` (Everything 模式)         | 启用路径匹配模式                           |
| 双击 Ctrl                          | 唤醒/隐藏主搜索窗口（可选配置）            |

### 3.12 窗口行为

- **唤醒窗口**: 触发全局快捷键后，窗口在鼠标所在显示器或当前活动窗口所在显示器显示
- **窗口定位**: 支持配置垂直位置比例（0=顶部，1=底部）、跟随鼠标位置
- **窗口隐藏**: Esc 键、失去焦点、手动调用 `hide_window`
- **窗口大小**: 支持配置宽度、搜索栏高度、结果项高度、底栏高度
- **主题**: 支持跟随系统/浅色/深色模式

---

## 4. 系统托盘

### 4.1 托盘图标

- 应用启动后立即显示在系统托盘中
- 图标根据主题模式（`tray_theme_mode`）切换浅色/深色版本
- 鼠标悬停显示 tooltip: `ZeroLaunch-rs v{version}`

### 4.2 托盘菜单

| 菜单项                    | 功能说明                            |
| ------------------------- | ----------------------------------- |
| 打开设置界面              | 显示主设置窗口                      |
| ---                       | 分隔线                              |
| 刷新数据库                | 手动触发程序数据库刷新              |
| 重新注册快捷键            | 重新注册所有全局快捷键              |
| 游戏模式 (禁用全局快捷键) | 切换复选框状态，启用/禁用全局快捷键 |
| ---                       | 分隔线                              |
| 退出程序                  | 执行退出清理并关闭应用              |

### 4.3 游戏模式

- 勾选"游戏模式"后，所有全局快捷键被注销
- 取消勾选后，重新注册所有全局快捷键
- 游戏模式状态下，双击 Ctrl 和全局快捷键均不响应
- 状态通过 `AppState.game_mode` 维护，并同步到托盘菜单的复选框状态
- 尝试重新注册快捷键时若处于游戏模式，会提示用户先关闭游戏模式

---

## 5. 欢迎页面

### 5.1 触发条件

- 首次启动应用
- 欢迎页面版本号发生变化

### 5.2 页面内容

- 标题/副标题: "欢迎使用 ZeroLaunch" / "一个可以容忍错别字的 Windows 应用启动器"
- 快速上手四步骤（Steps 组件）:
  1. 按下 Alt + Space 随时唤醒搜索框
  2. 输入应用名、拼音或缩写，高效匹配
  3. 使用方向键选择，按下 Enter 即可启动
  4. 在设置中打造你的专属启动器
- 快捷键速查表:
  - Alt + Space: 唤醒/隐藏主窗口
  - Enter: 启动选中项
  - Alt: 临时按最近使用时间排序
  - Ctrl + Enter: 以管理员权限启动
  - Esc: 清空输入或隐藏窗口
  - Shift + Enter: 置顶当前已打开的窗口
  - ↑/↓: 上/下移动选择
  - Ctrl + J/K: 上/下移动选择 (Vim模式)
- 底部按钮: "访问官网"、"打开设置"

---

## 6. 设置窗口

### 6.1 窗口概览

- 窗口标签: `setting_window`
- 前端路由基路径: `/setting_window`
- 布局: 左侧导航菜单 + 右侧内容区域
- 底部始终显示"保存配置文件"按钮

### 6.2 设置导航结构

```
设置
├── 常规设置 (General)
├── 外观设置
│   ├── 搜索栏与结果栏设置 (SearchStyle)
│   ├── 背景图片设置 (Background)
│   └── 程序窗口设置 (Window)
├── 程序搜索
│   ├── 设置搜索路径 (Paths)
│   ├── 设置屏蔽路径 (Blocklist)
│   ├── 设置固定偏移量 (Keywords)
│   ├── 设置别名 (Aliases)
│   └── 额外设置 (Advanced)
├── 图标管理 (Icons)
├── 其他搜索
│   ├── 网址 (WebSearch)
│   ├── 命令 (CustomCommands)
│   ├── 内置命令 (BuiltinCommands)
│   ├── Everything 设置 (Everything)
│   └── 浏览器书签 (Bookmarks)
├── 远程管理 (ConfigPath)
├── 快捷键 (Shortcuts)
├── 关于 (About)
└── 调试模式 (Debug, 仅在 is_debug_mode 开启时显示)
```

### 6.3 设置页面详述

#### 6.3.1 常规设置 (General)

**配置项**:
- 搜索栏占位符文本
- 底部提示文本
- 搜索结果数量
- 语言选择
- 日志级别（debug/info/warn/error）
- 开机自启动
- 静默启动
- 启用拖动窗口
- 窗口跟随鼠标
- Esc 键优先隐藏窗口
- 唤醒失败时启动新实例
- 在全屏时唤醒窗口
- 空格键等于 Enter 键
- 滚动模式阈值
- 显示启动命令

#### 6.3.2 外观设置 - 搜索栏与结果栏 (SearchStyle)

**颜色配置**:
- 浅色模式和深色模式分别配置
- 选中项高亮颜色
- 字体颜色
- 搜索栏字体颜色
- 搜索栏背景颜色
- 搜索栏占位符字体颜色
- 底栏字体颜色
- 整体背景颜色

**字体配置**:
- 搜索栏字体族 + 字体大小
- 结果栏字体族 + 字体大小
- 底栏字体族 + 字体大小
- 搜索栏动画开关
- 显示启动命令开关

#### 6.3.3 外观设置 - 背景图片 (Background)

**功能**:
- 选择背景图片（文件选择对话框）
- 删除背景图片
- 计算图片主题色
- 背景图片大小（cover/contain/auto）
- 背景图片位置（center/top/bottom/left/right 等）
- 背景图片重复（no-repeat/repeat/repeat-x/repeat-y）
- 背景图片透明度
- 毛玻璃效果（None/Acrylic/Mica/Tabbed）

#### 6.3.4 外观设置 - 窗口 (Window)

**配置项**:
- 窗口垂直方向偏移比例因子
- 搜索栏高度（px）
- 结果栏单项高度（px）
- 底栏高度（px）
- 程序宽度（px）
- 使用 Windows 系统圆角
- 窗口圆角大小

#### 6.3.5 程序搜索 - 设置搜索路径 (Paths)

**功能**:
- 管理扫描目录列表（增删改）
- 每个目录配置：目标路径、搜索深度、匹配类型（正则/通配符）、文件扩展名列表、排除关键词列表、符号链接模式、最大符号链接深度
- 支持拖放文件与文件夹
- 支持常见扩展名快捷添加
- 显示总记录数

#### 6.3.6 程序搜索 - 设置屏蔽路径 (Blocklist)

**功能**:
- 管理屏蔽路径列表
- 支持添加文件夹或文件
- 可通过浏览按钮选择

#### 6.3.7 程序搜索 - 设置固定偏移量 (Keywords)

**功能**:
- 为目标关键字设置固定偏移量（权重偏移）
- 包含备注字段

#### 6.3.8 程序搜索 - 设置别名 (Aliases)

**功能**:
- 为程序设置别名，增加搜索匹配路径

#### 6.3.9 程序搜索 - 额外设置 (Advanced)

**配置项**:
- 搜索算法选择（标准 / Skim / LaunchyQT / 语义）
- 扫描 UWP 应用开关
- 启用 LRU 搜索缓存 + 缓存容量设置
- 排序算法参数:
  - 历史总分权重系数
  - 近期习惯权重系数
  - 短期热度权重系数
  - 查询亲和度权重系数
  - 查询亲和时间衰减常数
  - 查询亲和冷却时间
  - 短期热度衰减常数
  - 总开关

#### 6.3.10 图标管理 (Icons)

**功能**:
- 启用联网加载网页图标
- 启用图标缓存
- 打开图标缓存文件夹
- 程序图标列表（支持搜索、分页）
- 自定义更改程序图标
- 数据列: 图标、程序名称、路径、操作

#### 6.3.11 其他搜索 - 网址 (WebSearch)

**功能**:
- 管理自定义 Web 搜索引擎
- 每个条目: 关键字 + 目标网址 URL
- 示例: 关键字 "bd" + URL "https://www.baidu.com/s?wd={}" → 搜索 "bd 内容" 将打开百度搜索

#### 6.3.12 其他搜索 - 命令 (CustomCommands)

**功能**:
- 管理自定义命令
- 每个条目: 关键字 + 命令内容

#### 6.3.13 其他搜索 - 内置命令 (BuiltinCommands)

**功能**:
- 管理内置命令的启用/禁用状态
- 管理内置命令的搜索关键词
- 支持恢复默认、全部启用、全部禁用

**内置命令列表**:
| 命令                  | 功能           | 默认关键词       |
| --------------------- | -------------- | ---------------- |
| OpenSettings          | 打开设置       | settings, 设置   |
| RefreshDatabase       | 刷新数据库     | refresh, 刷新    |
| RetryRegisterShortcut | 重新注册快捷键 | reshortcut, 注册 |
| ToggleGameMode        | 切换游戏模式   | gamemode, 游戏   |
| ExitProgram           | 退出程序       | exit, 退出       |

#### 6.3.14 其他搜索 - Everything 设置 (Everything)

**配置项**:
- 排序阈值（字符数达到该值后启用排序）
- 排序方式（多种排序维度，如名称、路径、大小、日期等）
- 结果数量限制
- Everything 快捷键配置（启用路径匹配的快捷键）

#### 6.3.15 其他搜索 - 浏览器书签 (Bookmarks)

**功能**:
- 自动检测已安装的浏览器
- 管理书签来源列表（增删改）
- 每个来源: 名称、书签文件路径、启用开关
- 书签列表编辑: 排除某书签、自定义搜索关键字
- 提示: 修改需保存配置文件后生效

#### 6.3.16 远程管理 (ConfigPath)

**功能**:
- 配置存储目的地（Local / WebDAV / OneDrive）
- Local 配置: 目标目录路径
- WebDAV 配置: 主机 URL、账号、密码、目标目录
- 保存到本地的更新频率
- 测试连通性
- 显示当前远程配置目录路径

#### 6.3.17 快捷键 (Shortcuts)

**功能**:
- 自定义配置各快捷键的按键组合
- 使用 ShortcutInput 组件进行交互式快捷键设置

**可配置的快捷键**:
| 用途              | 默认值      |
| ----------------- | ----------- |
| 打开搜索栏        | Alt + Space |
| 切换到 Everything | Ctrl + E    |
| 上移              | Ctrl + J    |
| 下移              | Ctrl + K    |
| 左移（关闭菜单）  | Ctrl + H    |
| 右移（打开菜单）  | Ctrl + L    |
| 双击 Ctrl 开关    | 关闭        |

#### 6.3.18 关于 (About)

**内容**:
- 应用名称、版本号
- 官网链接
- 技术栈说明

#### 6.3.19 调试模式 (Debug)

> 仅在 `is_debug_mode` 为 true 时显示

**功能**:
- 性能测试:
  - 测试搜索算法耗时
  - 测试索引文件耗时
- 搜索关键字生成（输入程序路径，生成多种格式的关键字）
- 程序搜索测试（输入关键词，显示搜索匹配结果详情）

### 6.4 设置保存机制

- 前端使用 Pinia 状态管理库管理配置状态
- 配置修改后存储在 `dirtyConfig` 中，直到用户点击"保存配置文件"
- 保存时调用 `command_save_remote_config` 将配置持久化
- 保存后触发 `emit_update_setting_window_config` 事件刷新设置页
- 部分页面（远程管理、快捷键、关于、调试）切换前要求先保存

---

## 7. 程序搜索与管理模块（后端核心）

### 7.1 架构

```
ProgramManager
├── ProgramLoader（程序加载器）
│   ├── 扫描目录
│   ├── 加载 UWP 程序
│   ├── 索引网页
│   ├── 加载自定义命令
│   ├── 索引浏览器书签
│   └── 生成嵌入向量（语义搜索）
├── ProgramLauncher（程序启动器）
│   ├── 启动路径程序
│   ├── 启动 UWP 程序
│   ├── 打开文件
│   ├── 打开 URL
│   ├── 执行命令
│   └── 内置命令
├── SearchEngine（搜索引擎）
│   └── TraditionalSearchEngine（传统搜索）
├── SearchModel（多种匹配算法）
│   ├── StandardSearchModel（标准匹配）
│   ├── SkimSearchModel（Skim 模糊匹配）
│   └── LaunchySearchModel（LaunchyQT 匹配）
├── ProgramRanker（程序排序器）
│   ├── 历史分值
│   ├── 近期习惯分值
│   ├── 短期热度分值
│   ├── 查询亲和度分值
│   └── 综合加权排序
├── WindowActivator（窗口唤醒器）
├── IconManager（图标管理器）
└── ParameterResolver（参数解析器）
```

### 7.2 程序加载（ProgramLoader）

**扫描路径配置**:
- 支持配置多个扫描目录
- 每个目录可配置：最大深度、文件模式（正则/通配符）、排除关键词、符号链接模式
- 扫描 UWP 应用（可选）
- 索引网页（可选）
- 加载自定义命令

**程序类型** (`LaunchMethodKind`):
- `Path`: 可执行文件路径
- `PackageFamilyName`: UWP 程序包名
- `File`: 文件（使用系统默认程序打开）
- `Url`: 网址
- `Command`: 命令行命令
- `BuiltinCommand`: 内置命令

**程序唯一标识**:
- 每个程序分配一个 `program_guid`（u64 类型）
- 使用 `program_locater`（DashMap）建立 GUID 到注册表下标的快速查找

### 7.3 搜索算法

**三种搜索模式**:

1. **标准搜索算法（Standard）**: 作者设计的综合匹配算法，高准确率高容错
2. **Skim 匹配算法（Skim）**: 主流模糊匹配算法
3. **LaunchyQT 算法**: 对 LaunchyQT 的移植实现

**搜索流程**:
1. 用户输入文本
2. 文本预处理（小写化、去多余空格）
3. 所有程序并行计算匹配分数
4. 加入程序的静态偏移量（`stable_bias`）
5. 通过 ProgramRanker 进行排序加权
6. 返回排序后的结果列表

### 7.4 程序排序器（ProgramRanker）

**排序维度**:
- **历史总分**: 基于程序累计启动次数
- **近期习惯**: 基于最近7天内启动频率
- **短期热度**: 基于最近启动时间（时间越短分数越高）
- **查询亲和度**: 基于特定搜索词与程序的关联度（带时间衰减和冷却机制）

**每个维度都有独立的可配置权重系数**。

### 7.5 LRU 搜索缓存

- 可配置启用/禁用
- 可配置缓存容量
- 缓存搜索结果，减少重复搜索耗时

### 7.6 搜索接口（`handle_search_text`）

```rust
#[tauri::command]
pub async fn handle_search_text(search_text: String) -> Vec<(u64, String, String)>
```

返回 `(program_guid, display_name, launch_path)` 三元组列表。

---

## 8. 快捷键管理系统

### 8.1 架构

`ShortcutManager` 分为两层:
- **Tauri 层**: 使用 `tauri-plugin-global-shortcut` 注册系统级全局快捷键
- **应用层**: 使用 `rdev` 库监听双击 Ctrl 事件（独立线程）

### 8.2 全局快捷键

通过 Tauri 的 `GlobalShortcut` API 注册，支持的按键组合包括:
- 字母键 (A-Z)
- 数字键 (0-9)
- 特殊键 (Space、Tab、CapsLock)
- 修饰键 (Ctrl、Alt、Shift、Meta)

**默认注册的快捷键**:
- `Alt + Space`: 唤醒/隐藏主搜索窗口
- `Ctrl + E`: 切换到 Everything 搜索模式

### 8.3 双击 Ctrl 检测

- 使用 `rdev` 库在独立线程中监听全局按键
- 400ms 窗口内检测两次 Ctrl 按下触发
- 可通过配置启用/禁用
- 游戏模式下不响应

### 8.4 快捷键注册/注销

- 支持动态注册所有快捷键
- 支持动态注销所有快捷键（游戏模式切换时）
- 注册失败时弹出通知提示

### 8.5 快捷键配置

- 支持在设置页面自定义各快捷键
- 配置保存后，通过 `update_shortcut_manager` 刷新快捷键管理器

---

## 9. 配置管理系统

### 9.1 配置层次

```
RuntimeConfig
├── AppConfig（应用配置）
├── UiConfig（界面配置）
│   ├── LightModeColors（浅色主题色）
│   └── DarkModeColors（深色主题色）
├── ShortcutConfig（快捷键配置）
├── ProgramManagerConfig（程序管理器配置）
│   ├── RankerConfig（排序器配置）
│   └── LoaderConfig（加载器配置）
├── WindowState（窗口状态）
├── IconManagerConfig（图标管理器配置）
├── EverythingConfig（Everything 配置）
├── RefreshSchedulerConfig（刷新调度器配置）
└── BookmarkLoaderConfig（书签加载器配置）
```

### 9.2 配置持久化

- **本地配置** (`LocalConfig`): 存储存储目的地、WebDAV 认证信息等，保存在 `%APPDATA%/ZeroLaunch-rs/local_config.json`
- **远程配置** (`RemoteConfig`): 所有应用运行配置，存储在配置目的地（Local/WebDAV/OneDrive），文件名为 `zerolaunch.json`
- 通过 `PartialRuntimeConfig` 实现配置的部分更新（增量更新）

### 9.3 配置保存流程

1. 前端通过 `command_save_remote_config` 发送增量配置
2. 后端更新 `RuntimeConfig`
3. 根据需要更新托盘图标主题
4. 将配置序列化并上传到存储后端
5. 触发各子系统的配置更新回调

---

## 10. 云存储/远程配置同步

### 10.1 存储后端

三种存储目的地:
1. **Local**: 本地文件系统存储
2. **WebDAV**: 通过 WebDAV 协议同步到远程服务器
3. **OneDrive**: OneDrive 云存储（当前已注释掉，功能停用）

### 10.2 StorageManager 架构

```
StorageManager
├── StorageClient trait
│   ├── LocalStorage（本地实现）
│   ├── WebDAVStorage（WebDAV 实现）
│   └── OneDriveStorage（已注释）
├── 上传/下载文件（支持字符串和字节）
├── 缓存机制（按更新次数延迟上传）
└── 连通性测试
```

### 10.3 缓存上传策略

- 支持配置 `save_to_local_per_update`，控制每几次修改后执行一次上传
- 为 0 时直接上传
- 大于 0 时缓存修改，达到次数后再上传
- `upload_all_file_force` 强制上传所有缓存文件

### 10.4 存储的文件

- `zerolaunch.json`: 远程配置
- `background.png`: 背景图片

---

## 11. Everything 集成搜索

### 11.1 支持平台

- 仅 x86_64 架构支持
- 需要 `Everything64.dll`

### 11.2 功能

- 通过 Everything SDK 实现全盘文件搜索
- 支持文件图标加载
- 支持文件路径匹配模式
- 支持多种排序方式（名称、路径、大小、日期等）
- 支持结果数量限制
- 支持排序阈值（输入字符数达到后自动排序）

### 11.3 交互

- 通过 `Ctrl+E` 快捷键从主搜索切换到 Everything 搜索模式
- 切换后搜索框输入直接触发 Everything 搜索
- 搜索结果包含文件名和完整路径
- 点击结果通过系统默认程序打开文件

### 11.4 aarch64 架构

- aarch64 架构下，Everything 功能不可用
- 前端显示"当前架构不支持 Everything 搜索"提示

---

## 12. 参数解析器

### 12.1 功能

解析启动模板中的参数占位符，支持三种参数类型:

| 参数类型 | 语法          | 说明                     |
| -------- | ------------- | ------------------------ |
| 位置参数 | `{}`          | 用户提供的自定义参数     |
| 剪贴板   | `{clip}`      | 唤醒前的剪贴板内容       |
| 窗口句柄 | `{hwnd}`      | 当前活动窗口句柄         |
| 选中文本 | `{selection}` | 唤醒前活动窗口的选中文本 |

### 12.2 行内参数模式（Inline Parameter Mode）

- 支持在搜索输入框中直接输入多个参数（用空格分隔）
- 可以通过特定语法解析参数并直接启动，无需弹出参数面板
- 提供参数数量和校验检查

### 12.3 参数面板模式

- 分步引导输入模式
- 每步收集一个参数
- 实时预览最终命令
- 最终一步变为"启动"按钮

---

## 13. 刷新调度器

### 13.1 刷新触发源

| 触发源   | 说明                           |
| -------- | ------------------------------ |
| 定时刷新 | 按配置的时间间隔自动刷新       |
| 安装监控 | 监控开始菜单等目录的变化       |
| 手动触发 | 用户通过托盘菜单或内置命令触发 |

### 13.2 配置项

- `auto_refresh_interval_mins`: 自动刷新间隔（分钟），0 表示禁用定时刷新
- `enable_installation_monitor`: 启用安装监控
- `monitor_debounce_secs`: 监控事件的防抖时间（秒）

### 13.3 安装监控器

- 监控 `%APPDATA%\Microsoft\Windows\Start Menu` 等目录变化
- 检测到变化后，经过防抖时间再触发刷新
- 通过条件变量机制与调度线程通信

### 13.4 刷新回调

- 刷新回调由 `ProgramManager` 注册
- 刷新时重新加载和索引所有程序
- 触发 `update_search_bar_window` 事件通知前端更新

---

## 14. 图标管理器

### 14.1 功能

- 管理程序图标的提取、缓存和加载
- 支持本地图标缓存（文件系统）
- 支持联网加载网页图标
- 支持 Everything 文件图标提取
- 支持自定义替换程序图标

### 14.2 图标缓存

- 图标以 PNG 格式缓存在本地 `icon_cache` 目录中
- 缓存键为程序的 `icon_request_json`
- 可配置启用/禁用图标缓存
- 支持清空缓存

### 14.3 图标加载

- 图标通过 `load_program_icon` 命令异步加载
- 前端使用 Blob URL 渲染图标
- 支持批量预加载（前 100 个普通图标和所有 URL 图标并发加载）

---

## 15. 书签加载器

### 15.1 功能

- 加载和索引浏览器书签文件
- 将书签转换为可搜索的 URL 程序项
- 支持多个书签源
- 支持排除特定书签
- 支持自定义书签搜索关键字

### 15.2 配置

- `sources`: 书签来源列表（名称、文件路径、启用状态）
- `overrides`: 书签覆盖规则（排除、自定义标题）

---

## 16. 浏览器书签检测

### 16.1 功能

- 自动检测已安装的浏览器
- 读取浏览器书签文件内容
- 获取书签来源列表和覆盖配置

### 16.2 支持的浏览器

通过 `detect_installed_browsers` 命令自动检测系统已安装的浏览器及其书签文件路径。

---

## 18. 版本检查器

### 18.1 功能

- 通过 GitHub Releases API 检查最新版本
- 返回最新版本的标签名

### 18.2 接口

```rust
#[tauri::command]
pub async fn command_get_latest_release_version() -> String
```

---

## 19. 调试与日志系统

### 19.1 日志系统

- 使用 `tracing` crate 实现结构化日志
- 日志级别: debug / info / warn / error（可配置）
- 支持日志文件输出和滚动
- 支持日志导出为 ZIP 压缩包

### 19.2 调试功能

- 性能测试: 搜索算法耗时、索引耗时
- 搜索关键字生成
- 程序搜索测试（显示详细匹配结果）
- 仅在 `is_debug_mode` 开启时可用

---

## 20. 国际化（i18n）

### 20.1 支持语言

- 简体中文 (`zh-Hans`)
- 繁体中文 (`zh-Hant`)
- 英语 (`en`)

### 20.2 实现

- 前端: Vue I18n（`vue-i18n`）
- 后端: 自定义 `Translator` 类
- 语言包存储在 `src-ui/i18n/locales/` 目录
- 语言选择存储在 `AppConfig.language` 中

### 20.3 后端翻译

- 托盘菜单文本
- 通知消息
- 系统级文本

---

## 21. 窗口特效与外观

### 21.1 毛玻璃效果

| 效果类型 | 说明                      |
| -------- | ------------------------- |
| None     | 无效果（纯色背景）        |
| Acrylic  | 亚克力效果（Win10 1803+） |
| Mica     | 云母效果（Win11 22h2+）   |
| Tabbed   | 标签式效果（Win11 22h2+） |

- 仅在启用"使用 Windows 系统圆角"时生效
- 未启用系统圆角时，由前端 CSS 模拟毛玻璃效果

### 21.2 窗口圆角

- 支持使用 Windows 系统原生圆角（`DWMWCP_ROUND`）
- 支持自定义 CSS 圆角大小
- 仅 Windows 11 22h2 及以上版本支持系统圆角
- 可通过 `use_windows_sys_control_radius` 配置切换

### 21.3 背景图片

- 支持自定义背景图片
- 可配置大小、位置、重复方式、透明度
- 图片存储在远程配置目录中（文件名为 `background.png`）

---

## 22. 内置命令系统

### 22.1 内置命令类型

| 命令                    | 功能           |
| ----------------------- | -------------- |
| `OpenSettings`          | 打开设置窗口   |
| `RefreshDatabase`       | 刷新程序数据库 |
| `RetryRegisterShortcut` | 重新注册快捷键 |
| `ToggleGameMode`        | 切换游戏模式   |
| `ExitProgram`           | 退出程序       |

### 22.2 行为

- 内置命令作为特殊的 Program 注册到 ProgramManager
- 可配置启用/禁用
- 可自定义搜索关键词
- 执行时不执行窗口隐藏等常规启动流程

---

## 23. 应用退出与清理流程

### 23.1 触发条件

- 用户从托盘菜单选择"退出程序"
- 系统发送退出请求事件

### 23.2 清理流程

1. 阻止默认退出（`api.prevent_exit()`）
2. 保存当前配置到文件（`save_config_to_file`）
3. 强制上传所有缓存文件到远程存储
4. 记录应用关闭日志
5. 调用 `app_handle.exit(0)` 退出

### 23.3 线程模型

- 使用 `AtomicBool`（`IS_EXITING`）防止重复退出
- 清理在异步任务中执行，不会阻塞主线程

---

## 附录A: 全部 Tauri Command 列表

| Command                                    | 功能                     | 所属文件             |
| ------------------------------------------ | ------------------------ | -------------------- |
| `command_save_remote_config`               | 保存远程配置             | config_file.rs       |
| `command_load_remote_config`               | 加载远程配置             | program_service.rs   |
| `command_load_local_config`                | 加载本地配置             | config_file.rs       |
| `command_save_local_config`                | 保存本地配置             | config_file.rs       |
| `command_check_validation`                 | 测试存储连通性           | config_file.rs       |
| `load_program_icon`                        | 加载程序图标             | program_service.rs   |
| `get_program_count`                        | 获取程序总数             | program_service.rs   |
| `command_get_program_url_status`           | 获取程序 URL 状态        | program_service.rs   |
| `launch_program`                           | 启动程序                 | program_service.rs   |
| `get_program_info`                         | 获取程序信息             | program_service.rs   |
| `refresh_program`                          | 刷新程序数据库           | program_service.rs   |
| `handle_search_text`                       | 搜索程序                 | program_service.rs   |
| `handle_everything_search`                 | Everything 搜索          | program_service.rs   |
| `launch_everything_item`                   | 启动 Everything 搜索结果 | program_service.rs   |
| `everything_enable_path_match`             | 启用 Everything 路径匹配 | program_service.rs   |
| `get_launch_template_info`                 | 获取启动模板信息         | program_service.rs   |
| `update_search_bar_window`                 | 更新搜索窗口配置         | ui_command.rs        |
| `get_background_picture`                   | 获取背景图片             | ui_command.rs        |
| `get_remote_config_dir`                    | 获取远程配置目录         | ui_command.rs        |
| `select_background_picture`                | 选择/上传背景图片        | ui_command.rs        |
| `hide_window`                              | 隐藏主窗口               | ui_command.rs        |
| `show_setting_window`                      | 显示设置窗口             | ui_command.rs        |
| `show_welcome_window`                      | 显示欢迎窗口             | ui_command.rs        |
| `get_dominant_color`                       | 计算图片主题色           | ui_command.rs        |
| `get_everything_icon`                      | 获取 Everything 文件图标 | ui_command.rs        |
| `command_is_system_dark_mode`              | 检查系统深色模式         | ui_command.rs        |
| `command_get_latest_release_version`       | 获取最新版本号           | utils.rs             |
| `command_get_default_remote_data_dir_path` | 获取默认远程数据目录     | utils.rs             |
| `command_get_system_fonts`                 | 获取系统字体列表         | utils.rs             |
| `command_read_file`                        | 读取文件内容             | utils.rs             |
| `command_export_logs`                      | 导出日志为 ZIP           | utils.rs             |
| `command_search_programs_lightweight`      | 轻量级程序搜索           | utils.rs             |
| `command_update_program_icon`              | 更新程序图标             | utils.rs             |
| `command_add_forbidden_path`               | 添加屏蔽路径             | utils.rs             |
| `command_get_program_path`                 | 获取程序路径             | utils.rs             |
| `command_get_arch`                         | 获取系统架构             | utils.rs             |
| `command_download_model`                   | 下载 AI 模型             | utils.rs             |
| `open_target_folder`                       | 打开程序所在文件夹       | program_service.rs   |
| `command_unregister_all_shortcut`          | 注销所有快捷键           | shortcut.rs          |
| `command_register_all_shortcut`            | 注册所有快捷键           | shortcut.rs          |
| `test_search_algorithm`                    | 测试搜索算法             | debug.rs             |
| `test_search_algorithm_time`               | 测试搜索算法耗时         | debug.rs             |
| `test_index_app_time`                      | 测试索引耗时             | debug.rs             |
| `get_search_keys`                          | 获取搜索关键字           | debug.rs             |
| `command_get_search_status_tip`            | 获取搜索状态提示         | program_service.rs   |
| `command_get_latest_launch_program`        | 获取最近启动程序         | program_service.rs   |
| `command_open_icon_cache_dir`              | 打开图标缓存目录         | ui_command.rs        |
| `command_open_models_dir`                  | 打开模型目录             | program_service.rs   |
| `detect_installed_browsers`                | 检测已安装浏览器         | browser_bookmarks.rs |
| `read_browser_bookmarks`                   | 读取浏览器书签           | browser_bookmarks.rs |
| `get_bookmark_sources`                     | 获取书签来源             | browser_bookmarks.rs |
| `update_bookmark_sources`                  | 更新书签来源             | browser_bookmarks.rs |
| `get_bookmark_overrides`                   | 获取书签覆盖配置         | browser_bookmarks.rs |
| `update_bookmark_overrides`                | 更新书签覆盖配置         | browser_bookmarks.rs |
| `command_get_path_info`                    | 获取路径信息             | utils.rs             |
