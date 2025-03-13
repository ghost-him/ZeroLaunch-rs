

<div align="center">
<!--
    <p align="center">
         <img src="./Web/src/assets/logo.png" height="128" alt="ZeroLaunch-logo"/> 
    </p>
-->
    <h1>🚀 ZeroLaunch-rs 🚀</h1>
</div>

<div align="center"><h3>✨ 極速精準、輕量純粹的 Windows 應用程式啟動器！✨</h3></div>

<div align="center">

[![Gitee star](https://gitee.com/ghost-him/ZeroLaunch-rs/badge/star.svg?theme=dark)](https://gitee.com/ghost-him/ZeroLaunch-rs/stargazers)
[![Gitee fork](https://gitee.com/ghost-him/ZeroLaunch-rs/badge/fork.svg?theme=dark)](https://gitee.com/ghost-him/ZeroLaunch-rs/members)
[![GitHub stars](https://img.shields.io/github/stars/ghost-him/ZeroLaunch-rs.svg?style=social)](https://github.com/ghost-him/ZeroLaunch-rs/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/ghost-him/ZeroLaunch-rs.svg?style=social)](https://github.com/ghost-him/ZeroLaunch-rs/network/members)
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

## 📕 一句話介紹

ZeroLaunch 是一款專為 Windows 平台精心打造的應用程式啟動器，致力於提供極致高效、快捷的搜尋體驗，讓您瞬間找到並啟動所需應用。

> 該項目因個人需要而開發，因此該項目將持續維護與優化，確保其長期穩定運行與功能完善。

## 🖥️ 軟體介面

[![主介面預覽](asset/主界面.png)](asset/picture-cn.md)

*點擊圖片查看完整功能截圖集*

**背景圖片可自定義**

## ✨ 核心特性

### 🔒 隱私至上
完全離線運行，無需網路連接，您的資料始終保留在裝置中。我們堅持零資料採集原則，嚴格遵循本地化處理，確保您的資訊安全。

### ⚡ 智能搜尋
採用四重匹配技術（全稱/模糊/拼音/首字母），支援中英文混合查詢，配合即時動態排序演算法和多線程併發處理，即使是低配電腦也能帶來毫秒級的回應。

### 🌐 輕巧純粹
專注於應用程式搜尋功能，簡潔而不簡單，為您提供精準、快速的結果。

## 🔬 軟體功能

### 主要功能

* **應用程式搜尋**：快速檢索並啟動傳統應用程式及UWP應用，提供流暢的程式存取體驗。
* **應用程式喚醒**：智能識別並將已開啟的視窗置前，實現便捷的多任務切換。
* **自定義外觀介面**：支援自定義背景圖片，選項顏色，搜尋字體顏色與大小，顯示字體顏色與大小，顯示候選個數等。

---
### 次要功能

* 自定義搜尋演算法：支援對搜尋演算法做微調，從而滿足個性化設置。
* 自定義程式添加：支援添加屏蔽字來避免某些程式的載入，支援添加自定義安裝路徑的程式。
* 自定義檔案搜尋：支援自定義添加檔案搜尋，滿足少數常用檔案的搜尋功能。
* 自定義網頁搜尋：支援自定義添加網頁搜尋，滿足少數常用網頁的搜尋功能。
* 自定義命令搜尋：支援自定義添加命令，滿足少數常用命令的搜尋功能。
* 自定義配置檔案的保存路徑：可將配置檔案放至同步網盤從而實現配置資訊的同步。

## 🚀 快速入門

### 快捷鍵速查

| 功能                | 快捷鍵           |
|---------------------|------------------|
| 呼出搜尋欄          | `Alt + Space`    |
| 上下選擇項目        | `↑/↓` 或 `Ctrl+k/j` |
| 啟動選中程式        | `Enter`          |
| 管理員權限啟動      | `Ctrl + Enter`   |
| 清空搜尋框          | `Esc`            |
| 隱藏搜尋介面        | 點擊外部區域      |
| 開啟已開啟的視窗     | `Shift + Enter` |

### 常見功能的實現

程式添加，檔案添加，命令添加，搜索算法微調等功能的實現以及**常見的問題**的解決辦法詳見以下文檔：[使用指南](doc/Feature_Implementation_Guide_cn2.md)

## 🚩 程式下載

* Gitee: [release](https://gitee.com/ghost-him/ZeroLaunch-rs/releases)
* Github: [release](https://github.com/ghost-him/ZeroLaunch-rs/releases)
* Gitcode: [release](https://gitcode.com/ghost-him/ZeroLaunch-rs/releases)

## 🛠️ 開發者指南

### 環境要求

* Rust v1.82.0
* Node.js v22.11.0
* Bun v1.2.3

### 建構步驟

```bash
# 克隆倉庫
git clone https://github.com/ghost-him/ZeroLaunch-rs.git

# 安裝依賴
bun install

# 開發模式
bun run tauri dev

# 生產建構
bun run tauri build
```

建構產物路徑：`./src-tauri/target/release/`

## 📦 資料目錄結構

```
%APPDATA%\ZeroLaunch-rs\
├── logs/                               # 運行日誌
└── ZeroLaunch_local_config.json        # 遠端配置檔案的存放地址，預設為此資料夾
```

## 📌 已知限制

### 短詞搜尋

⚠️ 輸入長度 < 3 字元時，搜尋結果可能不夠精確

## 🤝 開源致謝

本項目基於以下優秀開源項目建構：

* [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - 中文轉拼音核心詞典
* [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP應用索引方案
* [bootstrap](https://icons.bootcss.com/) - 提供了部分的程式圖示
* [icon-icons](https://icon-icons.com/zh/) - 提供了部分的程式圖示

## 🎯 todo

### 軟體目標

* 使用正則表達式來做關鍵字屏蔽與路徑屏蔽
* 暗色主題
* 錯誤處理優化

### 長期目標

> 當以上目標都完成時才開始實現以下功能

* 支援linux系統（wayland優先）

## ❤️ 支持作者

如果這個程式對你有幫助，就給作者點一個 **star** 吧，一個 **star** 就能讓作者開心一整天！

**以上內容由 DeepSeek-R1 完成轉換**