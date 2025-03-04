

<div align="center">
<!--
    <p align="center">
         <img src="./Web/src/assets/logo.png" height="128" alt="ZeroLaunch-logo"/> 
    </p>
-->
    <h1>🚀 ZeroLaunch-rs 🚀</h1>
</div>

<div align="center"><h3>✨ Lightning-fast, precise, and lightweight Windows application launcher! ✨</h3></div>

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

## 📕 Elevator Pitch

ZeroLaunch is a meticulously crafted application launcher for Windows, dedicated to delivering ultra-efficient search experiences that let you instantly locate and launch desired applications.

> This project was born from personal needs and will be continuously maintained and optimized to ensure long-term stability and functional excellence.

## 🖥️ Interface Preview

[![Main UI Preview](asset/主界面.png)](asset/picture-en.md)  
*Click image to view full screenshot gallery*

**Background Image Can Be Customized**

## ✨ Core Features

### 🔒 Privacy First
Fully offline operation with no network connectivity required. Your data stays strictly on-device. We adhere to a zero-data-collection policy and enforce localized processing to ensure information security.

### ⚡ Smart Search
Utilizes triple matching techniques (full name/fuzzy/pinyin), supports Chinese-English hybrid queries, enhanced by real-time dynamic sorting algorithms and multi-threaded concurrency processing for seamless efficiency.

### 🌐 Lightweight Focus
Specializes in application search functionality - streamlined yet sophisticated, delivering precise and rapid results.

## 🚩 Downloads

* Gitee: [release](https://gitee.com/ghost-him/ZeroLaunch-rs/releases)
* Github: [release](https://github.com/ghost-him/ZeroLaunch-rs/releases)
* Gitcode: [release](https://gitcode.com/ghost-him/ZeroLaunch-rs/releases)

## 🚀 Quick Start

### Hotkey Cheatsheet

| Function                | Hotkey           |
|-------------------------|------------------|
| Summon search bar        | `Alt + Space`    |
| Navigate items           | `↑/↓` or `Ctrl+k/j` |
| Launch selected          | `Enter`          |
| Admin launch             | `Ctrl + Enter`   |
| Clear search             | `Esc`            |
| Hide interface           | Click outside    |

### 3-Step Sync Setup

1. **Choose Sync Directory**  
   Settings → Other → Select target path (recommend cloud sync directories)

2. **Automatic Sync Structure**

```plaintext
    [Sync Directory]
        ├── ZeroLaunch_remote_config.json      # Configurations
        └── background.jpg   # Background image
```

3. **Multi-Device Sharing**  
   Point new installations to the same directory for instant sync

## ⚙️ Advanced Configuration

### Path Management Strategy

Search path example:

```plaintext
C:\Program Files\ (Depth 5)
├── App1/              ✔️ Indexed
│   └── Subfolder/     ✔️ Indexed
└── App2/
 └── .../
     └── Layer5/    ✔️ Indexed (5th layer)
         └── Layer6 ❌ Ignored
```

#### Exclusion Rules:

Full prefix matching. Excluding `C:\Temp` blocks all subdirectories starting with this path.

#### Weight Adjustment Formula

Final weight = Algorithm match score + ∑(Keyword weights)

Sample configuration:

|Keyword	|Weight|	Effect|
|---|---|---|
|Uninstall|-5000|Exclude uninstallers|
|beta|+2.5|Prioritize beta versions|
|Document|-1.0|Demote document-related results|

## 🛠️ Developer Guide

### Requirements

* Rust v1.82.0
* Node.js v22.11.0
* Bun v1.2.3

### Build Instructions

```bash
# Clone repo
git clone https://github.com/ghost-him/ZeroLaunch-rs.git

# Install dependencies
bun install

# Dev mode
bun run tauri dev

# Production build
bun run tauri build
```

Build output: `./src-tauri/target/release/`

## 📦 Data Directory

```
%APPDATA%\ZeroLaunch-rs\
├── logs/                               # Runtime logs
└── ZeroLaunch_local_config.json        # Configuration file
```

## 📌 Known Limitations

### Short Keyword Search

⚠️ Results may lack precision when input length < 3 characters

## 🤝 Acknowledgments

This project is built upon the following outstanding open-source projects:

* [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - Core dictionary for Chinese-to-Pinyin conversion
* [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP application indexing solution
* [icon-icons](https://icon-icons.com/zh/) - Provided the program's icons

## 🎯 Roadmap

### Immediate Goals

* Use regular expressions for keyword and path filtering
* Support custom search folder depth
* Add one-click restore to default configuration file save location
* Customizable one-click commands (key + command format, with built-in * optional commands disabled by default)
* Dark theme support
* Debugging features (e.g., view search algorithm results, temporarily add * search entries, inspect keyword generation results, performance evaluation)
* Address partial UWP application indexing failures (cause pending * investigation)
* Error handling optimization

### Long-Term Vision

> The implementation of the following features will only begin once all the above objectives have been completed.

* Linux (Wayland) support

**This content was translated by DeepSeek-R1.**