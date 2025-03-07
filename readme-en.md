

<div align="center">
<!--
    <p align="center">
         <img src="./Web/src/assets/logo.png" height="128" alt="ZeroLaunch-logo"/> 
    </p>
-->
    <h1>🚀 ZeroLaunch-rs 🚀</h1>
</div>

<div align="center"><h3>✨ Lightning-fast precision, lightweight and pure Windows application launcher! ✨</h3></div>

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

## 📕 One-Sentence Introduction

ZeroLaunch is an application launcher meticulously crafted for the Windows platform, dedicated to delivering an ultra-efficient and rapid search experience, allowing you to instantly find and launch desired applications.

> This project was developed for personal needs and will be continuously maintained and optimized to ensure long-term stable operation and functional completeness.

## 🖥️ Interface Preview

[![Main Interface Preview](asset/主界面.png)](asset/picture.md)

*Click image to view full feature screenshot collection*

**Customizable background image**

## ✨ Core Features

### 🔒 Privacy First
Fully offline operation, no internet connection required. Your data remains strictly on your device. We adhere to a zero-data-collection principle, ensuring all processing is localized and your information stays secure.

### ⚡ Intelligent Search
Utilizes quadruple matching technology (full name/fuzzy/pinyin/initial letters), supports mixed Chinese-English queries. Combined with real-time dynamic sorting algorithms and multi-threaded processing, it delivers millisecond-level responses even on low-end hardware.

### 🌐 Lightweight & Focused
Specializes in application search functionality—streamlined yet powerful, providing precise and rapid results.

## 🔬 Features

### Primary Features

* **Application Search**: Quickly locate and launch traditional applications and UWP apps with seamless accessibility.
* **Application Wake**: Intelligently identifies and brings existing application windows to the foreground for effortless task switching.
* **Customizable UI**: Supports custom background images, option colors, search font colors/sizes, display font colors/sizes, and candidate item count adjustments.

---
### Secondary Features

* **Search Algorithm Tuning**: Fine-tune search algorithms to meet personalized needs.
* **Custom Program Management**: Add blocklists to exclude specific programs and manually register applications in custom installation paths.
* **File Search**: Add frequently accessed files for quick retrieval.
* **Web Search**: Create custom web search shortcuts for commonly used websites.
* **Command Shortcuts**: Define custom commands for rapid execution.
* **Config Sync**: Store configuration files in cloud-synced directories for seamless settings synchronization.

## 🚀 Quick Start

### Hotkey Cheatsheet

| Function                | Hotkey           |
|-------------------------|------------------|
| Toggle Search Bar       | `Alt + Space`    |
| Navigate Items          | `↑/↓` or `Ctrl+k/j` |
| Launch Selected App     | `Enter`          |
| Admin Launch            | `Ctrl + Enter`   |
| Clear Search            | `Esc`            |
| Hide Interface          | Click Outside    |
| Focus Existing Window   | `Shift + Enter` |

### Feature Implementation Guide

For detailed instructions on program/file/command additions and search algorithm tuning, see: [User Guide](doc/Feature_Implementation_Guide_en.md)

## 🚩 Downloads

* Gitee: [release](https://gitee.com/ghost-him/ZeroLaunch-rs/releases)
* Github: [release](https://github.com/ghost-him/ZeroLaunch-rs/releases)
* Gitcode: [release](https://gitcode.com/ghost-him/ZeroLaunch-rs/releases)

## 🛠️ Developer Guide

### Requirements

* Rust v1.82.0
* Node.js v22.11.0
* Bun v1.2.3

### Build Steps

```bash
# Clone repo
git clone https://github.com/ghost-him/ZeroLaunch-rs.git

# Install dependencies
bun install

# Dev Mode
bun run tauri dev

# Production Build
bun run tauri build
```

Output Path: `./src-tauri/target/release/`

## 📦 Data Structure

```
%APPDATA%\ZeroLaunch-rs\
├── logs/                               # Runtime logs
└── ZeroLaunch_local_config.json        # Remote config file location (default)
```

## 📌 Known Limitations

### Short Query Search

⚠️ Results may lack precision when input length < 3 characters

## 🤝 Acknowledgments

Built upon these outstanding open-source projects:

* [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - Core Chinese-to-Pinyin dictionary
* [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP app indexing solution
* [bootstrap](https://icons.bootcss.com/) - Partial application icons
* [icon-icons](https://icon-icons.com/zh/) - Partial application icons

## 🎯 Roadmap

### Short-Term Goals

* Implement regex-based keyword/path filtering
* Dark theme support
* Fix residual UWP app indexing issues
* Error handling optimizations

### Long-Term Vision

> To be addressed after completing above goals

* Linux support (Wayland first)

## ❤️ Support the Developer

If this tool helps you, give the project a **star**! A single **star** makes the developer's day brighter!

**This content was translated by DeepSeek-R1.**