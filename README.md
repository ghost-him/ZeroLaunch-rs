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


[![Gitee star](https://gitee.com/ghost-him/ZeroLaunch-rs/badge/star.svg?theme=dark)](https://gitee.com/ghost-him/ZeroLaunch-rs/stargazers)
[![Gitee fork](https://gitee.com/ghost-him/ZeroLaunch-rs/badge/fork.svg?theme=dark)](https://gitee.com/ghost-him/ZeroLaunch-rs/members)
[![GitHub stars](https://img.shields.io/github/stars/ghost-him/ZeroLaunch-rs.svg?style=social)](https://github.com/ghost-him/ZeroLaunch-rs/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/ghost-him/ZeroLaunch-rs.svg?style=social)](https://github.com/ghost-him/ZeroLaunch-rs/network/members)
[![GitCode stars](https://gitcode.com/ghost-him/ZeroLaunch-rs/star/badge.svg)](https://gitcode.com/ghost-him/ZeroLaunch-rs/stargazers)

</div>

<div align="center">

![Platform](https://img.shields.io/badge/Platform-Windows_11-0078d7?logo=windows11&logoColor=white)
[![GPLv3 License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
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
采用四重匹配技术（全称/模糊/拼音/首字母），支持中英文混合查询，配合实时动态排序算法和多线程并发处理，即使是低配电脑也能带来毫秒级的响应。

### 🌐 轻巧纯粹
专注于应用程序搜索功能，简洁而不简单，为您提供精准、快速的结果。

## 🔬 软件功能

### 主要功能

* **应用程序搜索**：快速检索并启动传统应用程序及UWP应用，提供流畅的程序访问体验。
* **应用程序唤醒**：智能识别并将已打开的窗口置前，实现便捷的多任务切换。
* **自定义外观界面**：高度自定义化，支持自定义背景图片，选项颜色，搜索字体颜色与大小，显示字体颜色与大小，显示候选个数等多项内容。

---
### 次要功能

* 自定义搜索算法：支持对搜索算法做微调，从而满足个性化设置。
* 自定义程序添加：支持添加屏蔽字来避免某些程序的加载，支持添加自定义安装路径的程序。
* 自定义文件搜索：支持自定义添加文件搜索，满足少数常用文件的搜索功能。
* 自定义网页搜索：支持自定义添加网页搜索，满足少数常用网页的搜索功能。
* 自定义命令搜索：支持自定义添加命令，满足少数常用命令的搜索功能。
* 自定义配置文件的保存路径：支持自定义本地存储与 WebDAV 实现网络存储。

## 🚀 快速入门

### 快捷键速查

| 功能                | 快捷键           |
|---------------------|------------------|
| 呼出搜索栏          | `Alt + Space`    |
| 上下选择项目        | `↑/↓` 或 `Ctrl+k/j` |
| 启动选中程序        | `Enter`          |
| 管理员权限启动（仅限普通应用）      | `Ctrl + Enter`   |
| 清空搜索框          | `Esc`            |
| 隐藏搜索界面        | 点击外部区域      |
| 打开已打开的窗口     | `Shift + Enter` |

### 常见功能的实现

程序添加，文件添加，命令添加，搜索算法微调等功能的实现以及**常见的问题**的解决办法详见以下文档：[使用指南](doc/Feature_Implementation_Guide_cn.md)

## 🚩 程序下载

* Gitee: [release](https://gitee.com/ghost-him/ZeroLaunch-rs/releases)
* Github: [release](https://github.com/ghost-him/ZeroLaunch-rs/releases)
* Gitcode: [release](https://gitcode.com/ghost-him/ZeroLaunch-rs/releases)

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
└── ZeroLaunch_local_config.json        # 远程配置文件的存放地址，默认为此文件夹
```

## 📌 已知限制

### 短词搜索

⚠️ 输入长度 < 3 字符时，搜索结果可能不够精确

## 🤝 开源致谢

本项目基于以下优秀开源项目构建：

* [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - 中文转拼音核心词典
* [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP应用索引方案
* [bootstrap](https://icons.bootcss.com/) - 提供了部分的程序图标
* [icon-icons](https://icon-icons.com/zh/) - 提供了部分的程序图标

## 🎯 todo

### 软件目标

* 使用正则表达式来做关键字屏蔽与路径屏蔽
* 暗色主题
* 错误处理优化
* 使用object_store库支持S3协议的存储服务
* 支持毛玻璃特效（目前tauri有bug，需要等它修复）

### 长期目标

> 当以上目标都完成时才开始实现以下功能

* 支持linux系统（wayland优先）

## ❤️ 支持作者

如果这个程序对你有帮助，就给作者点一个 **star** 吧，一个 **star** 就能让作者开心一整天！