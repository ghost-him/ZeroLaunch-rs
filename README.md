<div align="center">
<!--
    <p align="center">
         <img src="./Web/src/assets/logo.png" height="128" alt="ZeroLaunch-logo"/> 
    </p>
-->
    <h1>🚀 ZeroLaunch-rs 🚀</h1>
</div>

<div align="center"><h3>✨ 极速精准、轻量纯粹的 Windows 应用程序启动器！✨</h3></div>

<div align="center">

![Platform](https://img.shields.io/badge/Platform-Windows_11-0078d7?logo=windows11&logoColor=white)
[![GPLv3 License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Gitee star](https://gitee.com/ghost-him/ZeroLaunch-rs/badge/star.svg?theme=dark)](https://gitee.com/ghost-him/ZeroLaunch-rs/stargazers)
[![GitHub stars](https://img.shields.io/github/stars/ghost-him/ZeroLaunch-rs.svg?style=social)](https://github.com/ghost-him/ZeroLaunch-rs/stargazers)
[![GitCode stars](https://gitcode.com/ghost-him/ZeroLaunch-rs/star/badge.svg)](https://gitcode.com/ghost-him/ZeroLaunch-rs/stargazers)

</div>

<div align="center">

[简体中文](README.md) | [繁體中文](readme-cn2.md) | [English](readme-en.md)

</div>


<div align="center">
    <a href="https://gitee.com/ghost-him/ZeroLaunch-rs" target="_blank">Gitee</a> •
    <a href="https://github.com/ghost-him/ZeroLaunch-rs" target="_blank">GitHub</a> •
    <a href="https://gitcode.com/ghost-him/ZeroLaunch-rs" target="_blank">GitCode</a>
</div>

## 📕 一句话介绍

ZeroLaunch 是一款专为 Windows 平台精心打造的应用程序启动器，致力于提供极致高效、快捷的搜索体验，让您瞬间找到并启动所需应用。

> 该项目因个人需要而开发，因此该项目将持续维护与优化，确保其长期稳定运行与功能完善。

## 🖥️ 软件界面

[![主界面预览](asset/主界面.png)](asset/picture.md)

*点击图片查看完整功能截图集*

**背景图片可自定义**

## ✨ 核心特性

### 🔒 隐私至上
完全离线运行，无需网络连接，您的数据始终保留在设备中。我们坚持零数据采集原则，严格遵循本地化处理，确保您的信息安全。

### ⚡ 智能搜索
采用三重匹配技术（全称/模糊/拼音），支持中英文混合查询，配合实时动态排序算法和多线程并发处理，带来流畅高效的搜索体验。

### 🌐 轻巧纯粹
专注于应用程序搜索功能，简洁而不简单，为您提供精准、快速的结果。

## 🚩 程序下载

* Gitee: [release](https://gitee.com/ghost-him/ZeroLaunch-rs/releases)
* Github: [release](https://github.com/ghost-him/ZeroLaunch-rs/releases)
* Gitcode: [release](https://gitcode.com/ghost-him/ZeroLaunch-rs/releases)

## 🚀 快速入门

### 快捷键速查

| 功能                | 快捷键           |
|---------------------|------------------|
| 呼出搜索栏          | `Alt + Space`    |
| 上下选择项目        | `↑/↓` 或 `Ctrl+k/j` |
| 启动选中程序        | `Enter`          |
| 管理员权限启动      | `Ctrl + Enter`   |
| 清空搜索框          | `Esc`            |
| 隐藏搜索界面        | 点击外部区域      |

### 三步配置同步

1. **选择同步目录**
   进入设置 → 其他设置 → 选择目标路径（推荐使用网盘同步目录）

2. **自动同步配置**

```plaintext
    [同步目录]
        ├── ZeroLaunch_remote_config.json      # 程序配置
        └── background.jpg   # 背景图片
```

3. **多设备共享**

在其他设备安装后指向同一目录即可同步所有设置

## ⚙️ 高级配置

### 路径管理策略

搜索路径示例：

```plaintext
C:\Program Files\ (深度5层)
├── App1/              ✔️ 索引
│   └── Subfolder/     ✔️ 索引
└── App2/
 └── .../
     └── Layer5/    ✔️ 索引 (第5层)
         └── Layer6 ❌ 忽略
```

#### 排除规则：

使用前缀完全匹配机制，例如排除 `C:\Temp` 将阻止所有以该路径开头的目录索引

#### 权重调优公式

程序的最终权重 = 算法匹配度 + ∑(关键词权重)

示例配置：

|关键词	|权重|	效果|
|---|---|---|
|卸载|-5000|完全排除卸载程序|
|beta|+2.5|提升测试版优先级|
|文档|-1.0|降低文档类结果排序|

## 🛠️ 开发者指南

### 环境要求

* Rust v1.82.0
* Node.js v22.11.0
* Bun v1.2.3

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/ghost-him/ZeroLaunch-rs.git

# 安装依赖
bun install

# 开发模式
bun run tauri dev

# 生产构建
bun run tauri build
```

构建产物路径：`./src-tauri/target/release/`

## 📦 数据目录结构

```
%APPDATA%\ZeroLaunch-rs\
├── logs/                               # 运行日志
└── ZeroLaunch_local_config.json        # 配置文件的存放地址
```

## 📌 已知限制

### 短词搜索

⚠️ 输入长度 < 3 字符时，搜索结果可能不够精确

## 🤝 开源致谢

本项目基于以下优秀开源项目构建：

* [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - 中文转拼音核心词典
* [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP应用索引方案
* [icon-icons](https://icon-icons.com/zh/) - 提供了该程序的图标

## 🎯 todo

### 软件目标

* 使用正则表达式来做关键字屏蔽与路径屏蔽
* 添加一键恢复默认的配置文件保存地址
* 自定义一键运行的命令（key+命令的形式，内置可能会用到的命令，默认不开启）
* 暗色主题
* 调试功能（比如，可以查看搜索算法运行的结果，临时添加搜索的条目，可以查看关键字生成算法的运行结果，性能评估）
* uwp 应用还存在部分无法索引的问题，尚未确定原因
* 错误处理优化

### 长期目标

> 当以上目标都完成时才开始实现以下功能

* 支持linux系统（wayland优先）