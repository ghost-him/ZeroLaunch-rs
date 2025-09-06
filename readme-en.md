![zerolaunch-rs](https://socialify.git.ci/ghost-him/zerolaunch-rs/image?custom_description=%F0%9F%9A%80%E6%9E%81%E9%80%9F%E7%B2%BE%E5%87%86%E3%80%81%E8%BD%BB%E9%87%8F%E7%BA%AF%E7%B2%B9%E7%9A%84+Windows+%E5%BA%94%E7%94%A8%E7%A8%8B%E5%BA%8F%E5%90%AF%E5%8A%A8%E5%99%A8%EF%BC%81%E6%8B%BC%E9%9F%B3%E6%A8%A1%E7%B3%8A%E5%8C%B9%E9%85%8D+%2B+%E6%80%A5%E9%80%9F%E5%93%8D%E5%BA%94%EF%BC%8C%E5%9F%BA%E4%BA%8E+Rust+%2B+Tauri+%2B+Vue.js+%E6%9E%84%E5%BB%BA%EF%BC%81&description=1&font=Bitter&forks=1&issues=1&language=1&logo=https%3A%2F%2Fgithub.com%2Fghost-him%2FZeroLaunch-rs%2Fblob%2Fmain%2Fsrc-tauri%2Ficons%2FSquare310x310Logo.png%3Fraw%3Dtrue&name=1&owner=1&pattern=Floating+Cogs&pulls=1&stargazers=1&theme=Light)



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
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ghost-him/ZeroLaunch-rs)
</div>

<div align="center">

[简体中文](README.md) | [繁體中文](readme-cn2.md) | [English](readme-en.md)

</div>

<div align="center">
    <a href="https://gitee.com/ghost-him/ZeroLaunch-rs" target="_blank">Gitee</a> •
    <a href="https://github.com/ghost-him/ZeroLaunch-rs" target="_blank">GitHub</a> •
    <a href="https://gitcode.com/ghost-him/ZeroLaunch-rs" target="_blank">GitCode</a>
</div>

## 📕 Brief Introduction

ZeroLaunch is an application launcher meticulously crafted for the Windows platform, **helping you instantly find and launch desired applications for the ultimate in efficiency and speed!**

> Currently, the existing program launchers on the market don't quite meet my needs, so I developed this software. I use it daily, so there's no need to worry about me abandoning it (at most, there might be no new updates (～￣▽￣)～).

## 🖥️ Software Interface

[![Main Interface Preview](asset/主界面.png)](asset/picture-en.md)

*Click the image to view the complete set of feature screenshots*

**Customizable background image**

## ✨ Why choose ZeroLaunch-rs / What makes ZeroLaunch-rs unique?

### 🔒 Privacy First
Runs completely offline, no internet connection required. Your data always stays on your device. Adhering to a zero data collection policy, strictly following local processing to ensure your information security.

### ⚡ Efficient Smart Search
Thanks to the optimization of a unique search algorithm, the program boasts excellent spell correction capabilities based on triple matching technology (full name/Pinyin/initials). The program supports mixed Chinese and English queries, combined with a real-time dynamic sorting algorithm and multi-threaded concurrent processing technology, achieving millisecond-level response speeds even on lower-spec devices.

### 🌐 Lightweight and Pure
Focused solely on application search and launch functions, achieving highly specialized application search. Undisturbed by other complex features, ready to use out of the box – that's how pure it is.

## 🔬 Software Features

### Core Features

*   **Application Search**: Quickly retrieve and launch **applications** and **UWP apps**, providing a smooth program access experience. Supports program remarks and aliases, localized name recognition and search.
*   **Application Wake-up**: Intelligently identifies and brings already open windows to the foreground, enabling convenient multi-task switching.
*   **Customizable Interface**: Highly customizable appearance, supporting custom background images, option colors, search font color and size, display font color and size, number of candidates displayed, frosted glass effect, rounded corner size settings, program width and height, and many other items, with convenient interaction buttons for each.
*   **Multi-language support**: Supports Simplified Chinese, Traditional Chinese, and English.
*   **Open File Location**: In the right-click menu, you can open the folder where the target file is located.

---
### More Practical Features / Advanced Play

*   **Fine-tune Search Algorithm**: Supports fine-tuning the search algorithm to meet personalized settings.
*   **Custom Program and File Addition**: Supports adding files and programs using file wildcards or regular expressions, enabling the addition of files and programs. Intelligently identifies file formats and reacts correctly.
*   **Custom Web Search**: Supports adding and using the default browser to launch web pages. No need to enter protocol header (http/https).
*   **Custom Command Search**: Supports custom command addition, enabling functions like system startup, shutdown, and opening specific secondary settings pages.
*   **Intelligent Loading of Program/File/Web Icons**: Loads the correct file icons to the greatest extent possible, and also supports correct loading of Steam game icons.
*   **Custom Configuration File Save Path**: Supports custom local storage and network storage using the WebDAV protocol.
*   **Supports Startup and Silent Launch**: Nothing much to explain, right? ==
*   **Debugging Function**: Allows viewing the program's running status on the current computer (usually no issues), and viewing the search algorithm's results. Support setting log output level.
*   **Gaming Mode**: Allows manually disabling hotkeys to prevent issues during gaming.
*   **Supports Opening Recently Launched Programs**: Hold down the `Alt` key to list recently opened programs in order.
*   **Supports Custom Keybindings**: Allows customizing keyboard mappings to better suit your operating habits.
*   **Supports Call-out Position Following Mouse**: If the mouse is on a secondary screen, the search bar will appear on that secondary screen.
*   **Search result display optimization**: Support setting the search result display threshold. When the number of searches is greater than the threshold, it will automatically switch to scrolling mode.

## 🚀 Quick Start

### Hotkey Quick Reference

| Function                  | Hotkey            |
|---------------------------|-------------------|
| Call out search bar       | `Alt + Space`     |
| Select item up/down       | `↑/↓` or `Ctrl+k/j` |
| Launch selected program   | `Enter`           |
| Launch as administrator (for regular apps only) | `Ctrl + Enter`    |
| Clear search box          | `Esc`             |
| Hide search interface     | Click outside area |
| Open already open window  | `Shift + Enter`   |
| Sort by recent launch time | `Alt`             |

### Implementation of Common Features

For the implementation of program addition, file addition, command addition, search algorithm fine-tuning, and solutions to **common problems**, please refer to the following document: [Usage Guide](doc/Feature_Implementation_Guide_en.md)

Writing documentation is so troublesome, and sometimes I can't describe things well (っ °Д °;)っ. Go check out [DeepWiki](https://deepwiki.com/ghost-him/ZeroLaunch-rs), it seems to explain things pretty well there.

## 🚩 Program Download

*   Gitee: [release](https://gitee.com/ghost-him/ZeroLaunch-rs/releases)
*   Github: [release](https://github.com/ghost-him/ZeroLaunch-rs/releases)
*   Gitcode: [release](https://gitcode.com/ghost-him/ZeroLaunch-rs/releases)

## 🛠️ Developer Guide

> This Rust is quite good, unified package management is very convenient.

### Environment Requirements

*   Rust v1.85.0
*   Node.js v22.11.0
*   Bun v1.2.3

### Build Steps

```bash
# Clone the repository
git clone https://github.com/ghost-him/ZeroLaunch-rs.git

# Install dependencies
bun install

# Development mode
bun run tauri dev

# Production build
bun run tauri build
```

Build artifact path: `./src-tauri/target/release/`

## 📦 Data Directory Structure

The program's configuration files are divided into: **local configuration file** and **remote configuration file**, both of which are JSON format files. The storage location of the local configuration file is as follows, and the local configuration file stores the address of the remote configuration file. The remote configuration file contains the file information generated during program runtime, and its default storage location is also this directory.

```
%APPDATA%\ZeroLaunch-rs\                # E.g.: C:\Users\[username]\AppData\Roaming\ZeroLaunch-rs\
├── logs/                               # Runtime logs
└── ZeroLaunch_local_config.json        # Location of the remote configuration file, defaults to this folder
```

## 📌 Known Limitations

### Short Word Search

⚠️ When input length is < 3 characters, search results may not be precise enough.

## 🌍 Language Support

ZeroLaunch-rs currently supports the following languages:

- 🇨🇳 Simplified Chinese (zh)
- 🇺🇸 English (en)

### Contributing Translations

We welcome community contributions for localization in more languages! Translation files are located in the `src/i18n/locales/` directory:

- `zh.json` - Simplified Chinese translation
- `en.json` - English translation

If you want to add new language support for ZeroLaunch-rs, please:

1. Copy an existing translation file (e.g., `en.json`)
2. Rename it to the corresponding language code (e.g., `fr.json` for French)
3. Translate all text content in the file
4. Submit a Pull Request

Thank you for contributing to ZeroLaunch-rs internationalization! 🙏

## 🤝 Open Source Acknowledgments

This project is built upon the following excellent open-source projects:

*   [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - Core dictionary for Chinese to Pinyin conversion
*   [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP application indexing solution
*   [bootstrap](https://icons.bootcss.com/) - Provided some program icons
*   [icon-icons](https://icon-icons.com/zh/) - Provided some program icons
*   [Follower-v2.0](https://github.com/MrBeanCpp/Follower-v2.0) - Provided the full-screen detection solution

## ❤️ Support the Author

You can support the author in the following ways:

1.  Give a free little star ⭐
2.  Share this project with other interested friends
3.  Propose more suggestions for improvement (ZeroLaunch-rs is positioned as a pure program launcher, so it will only focus on launcher functions and will not add too many irrelevant features, please understand 🥺🙏)

[![Star History Chart](https://api.star-history.com/svg?repos=ghost-him/zerolaunch-rs&type=Date)](https://www.star-history.com/#ghost-him/zerolaunch-rs&Date)

> translated by gemini 2.5 flash thinking