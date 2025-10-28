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

ZeroLaunch is a Windows smart launcher that understands your typing. It excels at Pinyin and fuzzy matching, and can optionally add AI semantic understanding—so typos and vague queries still get instant results. Clean, offline, and built for speed.

> Currently, the existing program launchers on the market don't quite meet my needs, so I developed this software. I use it daily, so there's no need to worry about me abandoning it (at most, there might be no new updates (～￣▽￣)～).

## 🖥️ Software Interface

[![Main Interface Preview](asset/主界面.png)](asset/picture-en.md)

*Click the image to view the complete set of feature screenshots*

**Customizable background image**

## ✨ Why choose ZeroLaunch-rs / What makes ZeroLaunch-rs unique?

### 🔒 Privacy First
Runs completely offline, no internet connection required. Your data always stays on your device. Adhering to a zero data collection policy, strictly following local processing to ensure your information security.

### ⚡ Efficient Smart Search
With optional on-device AI semantic search (EmbeddingGemma‑300m/ONNX), you get natural‑language and multilingual intent retrieval. Even without AI, our self‑developed matching algorithm (triple matching: full name/Pinyin/initials + spell correction) delivers high efficiency, high match quality, and high fault tolerance, with real‑time ranking.

We’ve implemented systemic performance optimizations end‑to‑end: hot‑path and data‑structure refactors, layered caching and on‑demand loading, a tuned concurrency model, and incremental index updates — all to minimize compute and I/O. You can expect stable millisecond‑level responses even on mid‑/low‑spec machines.

### 🌐 Lightweight and Pure
Laser‑focused on “quickly and accurately launching what you need.” Zero configuration required — the default setup already fits most users and scenarios. Power users still get ample customization (appearance, behavior, indexing strategy). No unrelated bloat: out‑of‑the‑box, lightweight, and pure.

## 🔬 Software Features

### Core Features

*   **AI semantic retrieval (optional)**: Powered by the latest EmbeddingGemma‑300m (ONNX) local embedding model — lightweight, efficient, and accurate. With AI, multi‑language search is supported, and you can use natural intent keywords (e.g., “music app”, “image editor”) to quickly surface relevant applications. All inference runs locally for privacy.
*   **Application Search**: Quickly retrieve and launch **applications** and **UWP apps**, providing a smooth program access experience. Supports program remarks and aliases, localized name recognition and search.
*   **Application Wake-up**: Intelligently identifies and brings already open windows to the foreground, enabling convenient multi-task switching.
*   **Customizable Interface**: Highly customizable appearance, supporting custom background images, option colors, search font color and size, display font color and size, number of candidates displayed, frosted glass effect, rounded corner size settings, program width and height, and many other items, with convenient interaction buttons for each.
*   **Multi-language support**: Supports Simplified Chinese, Traditional Chinese, and English. On startup, the application will attempt to detect your system language and automatically select the matching UI language. If detection fails, English will be used as the fallback default.
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
| Call out and hide search bar       | `Alt + Space`     |
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

### Editions (AI / Lite)

We provide two editions to fit different resource budgets and feature needs:

- AI edition (default, recommended): Supports local semantic search (requires downloading the EmbeddingGemma ONNX model separately) for smarter retrieval.
    - Runtime memory: when AI semantic search is enabled, about 500 ~ 550 MB; if you use the traditional search algorithm, the usage is the same as Lite.
    - Filenames: do not contain `lite`, e.g.:
        - `zerolaunch-rs_0.x.x_x64-setup.exe`, `zerolaunch-rs_0.x.x_x64_en-US.msi`
        - `ZeroLaunch-portable-0.x.x-x64.zip`

- Lite (no AI): No semantic search, smaller footprint and lower memory usage.
    - Memory usage: about 60 ~ 70 MB
    - Filenames: contain `lite`, e.g.:
        - `zerolaunch-rs_lite_0.x.x_x64-setup.exe`, `zerolaunch-rs_lite_0.x.x_x64_en-US.msi`
        - `ZeroLaunch-portable-lite-0.x.x-x64.zip`

Build tip (for developers): enable the `ai` feature for AI edition; omit it for Lite (see tasks or Cargo feature configuration). When using the xtask helper: `build-installer` / `build-portable` default to the AI edition; pass `--ai disabled` to build the Lite edition.

## 🛠️ Developer Guide

> This Rust is quite good, unified package management is very convenient.

### Environment Requirements

* Rust v1.90.0
* Bun v1.2.22

### Build Steps

```bash
# Clone the repository
git clone https://github.com/ghost-him/ZeroLaunch-rs.git

# Install dependencies
bun install

# Development mode
bun run tauri dev

# Use xtask automation build tool for production builds
cd xtask

# Build installer (AI edition by default), x64 only
cargo run --bin xtask build-installer --arch x64

# Build Lite installer (disable AI)
cargo run --bin xtask build-installer --arch x64 --ai disabled

# Build all versions (installer + portable, all architectures, both AI modes by default)
cargo run --bin xtask build-all

# Clean build artifacts
cargo run --bin xtask clean
```

Build artifacts:
- Installer: `.msi` files in project root directory
- Portable: `.zip` files in project root directory
- For detailed instructions, see [xtask/README.md](xtask/README.md)

## 📦 Data Directory Structure

The program comes in two versions: **Installer Version** and **Portable Version**:

- **Installer Version**: Uses `C:\Users\[username]\AppData\Roaming\ZeroLaunch-rs\` as the local data directory
- **Portable Version**: Uses the software's installation directory as the local data directory

### Local Data Directory Structure

The local data directory contains the following files:

```
Local Data Directory/                   # Installer Version: C:\Users\[username]\AppData\Roaming\ZeroLaunch-rs\
                                        # Portable Version: Software installation directory
├── logs/                               # Runtime logs
├── icons/                              # Program icon cache
└── ZeroLaunch_local_config.json        # Local configuration file, stores related data and remote directory path
```

### Remote Directory Structure

The remote directory is used to store detailed runtime configurations of the program. By default, it's the same as the local data directory. Remote storage enables data synchronization between two machines.

```
Remote Directory/                       # Default: same as local data directory
├── background.png                      # Custom background image
└── ZeroLaunch_remote_config.json       # Remote configuration file, stores program runtime configuration
```

## 📌 Known Limitations

### Short Word Search

⚠️ When input length is < 3 characters, search results may not be precise enough.

## 🧠 Semantic Search & Memory Usage

When the "Semantic Search" feature is enabled, this project loads Google's EmbeddingGemma model locally (ONNX version under `src-tauri/EmbeddingGemma-300m/`). Approximate memory usage by mode:

- Regular algorithm: about 60 ~ 70 MB
- Semantic search algorithm (EmbeddingGemma): about 500 ~ 550 MB

Tip: If you prefer lower memory usage, run without the AI/Semantic Search feature (build without the `ai` feature flag).

## 🌍 Language Support

ZeroLaunch-rs currently supports the following languages:

- 🇨🇳 Simplified Chinese (zh-Hans)
- 🇹🇼 Traditional Chinese (zh-Hant) - Translated by Gemini 2.5 Pro
- 🇺🇸 English (en) - Translated by Gemini 2.5 Pro

### Changing Language

You can change the application's display language through the following methods:

![Language Selection Demo](asset/select_language.png)

*Language selection interface demonstration: Simple and intuitive language switching experience*

1. **Through Settings Interface**:
   - Open ZeroLaunch-rs settings window (as shown in the image above)
   - Click on the 「General」 option in the left navigation bar
   - Find the 「Interface language」 dropdown menu in the 「Language Settings」 section
   - Click the dropdown menu and select your preferred language from the available options (supports Chinese, Traditional Chinese, English)
   - Save the settings「Save Config」to apply the new language settings

> 💡 **Helpful Tip**: The language switching feature is designed to be simple and clear. No matter which language you choose, the entire interface will completely switch to the corresponding language, providing a native user experience for users of different language backgrounds.


### Contributing Translations

We welcome community contributions for localization in more languages! Translation files are located in the `src/i18n/locales/` directory:

- `zh-Hans.json` - Simplified Chinese translation
- `zh-Hant.json` - Traditional Chinese translation
- `en.json` - English translation

If you want to add new language support for ZeroLaunch-rs, please:

1. Copy an existing translation file (e.g., `en.json`)
2. Rename it to the corresponding language code (e.g., `fr.json` for French)
3. Translate all text content in the file
4. Submit a Pull Request

Thank you for contributing to ZeroLaunch-rs internationalization! 🙏

## 📄 Third‑party Terms — EmbeddingGemma

- This project may optionally use Google’s EmbeddingGemma model locally for offline semantic search.
- Use and redistribution are subject to the Gemma Terms of Use https://ai.google.dev/gemma/terms and the Prohibited Use Policy https://ai.google.dev/gemma/prohibited_use_policy.
- If you redistribute the model or any derivatives (outside a hosted service), you must: (1) pass through these restrictions in your terms; (2) provide recipients a copy of the Gemma Terms (a link is fine); (3) mark modified files as modified; and (4) include a text file named NOTICE containing: “Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms”.

## 🤝 Open Source Acknowledgments

This project is built upon the following excellent open-source projects:

*   [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - Core dictionary for Chinese to Pinyin conversion
*   [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP application indexing solution
*   [bootstrap](https://icons.bootcss.com/) - Provided some program icons
*   [icon-icons](https://icon-icons.com/zh/) - Provided some program icons
*   [Follower-v2.0](https://github.com/MrBeanCpp/Follower-v2.0) - Provided the full-screen detection solution

## Roadmap

Planned future goals:

1. Use a database to manage stored information (remote configuration data) and reduce information redundancy.
2. Refactor the front-end pages to improve maintainability of the application.

These are high-level goals — implementation details (database choice, migration strategy, front-end architecture) will be decided in follow-up design discussions.

## ❤️ Support the Author

You can support the author in the following ways:

1.  Give a free little star ⭐
2.  Share this project with other interested friends
3.  Propose more suggestions for improvement (ZeroLaunch-rs is positioned as a pure program launcher, so it will only focus on launcher functions and will not add too many irrelevant features, please understand 🥺🙏)

[![Star History Chart](https://api.star-history.com/svg?repos=ghost-him/zerolaunch-rs&type=Date)](https://www.star-history.com/#ghost-him/zerolaunch-rs&Date)

> translated by gemini 2.5 flash thinking