![zerolaunch-rs](https://socialify.git.ci/ghost-him/zerolaunch-rs/image?custom_description=%F0%9F%9A%80%E6%9E%81%E9%80%9F%E7%B2%BE%E5%87%86%E3%80%81%E8%BD%BB%E9%87%8F%E7%BA%AF%E7%B2%B9%E7%9A%84+Windows+%E5%BA%94%E7%94%A8%E7%A8%8B%E5%BA%8F%E5%90%AF%E5%8A%A8%E5%99%A8%EF%BC%81%E6%8B%BC%E9%9F%B3%E6%A8%A1%E7%B3%8A%E5%8C%B9%E9%85%8D+%2B+%E6%80%A5%E9%80%9F%E5%93%8D%E5%BA%94%EF%BC%8C%E5%9F%BA%E4%BA%8E+Rust+%2B+Tauri+%2B+Vue.js+%E6%9E%84%E5%BB%BA%EF%BC%81&description=1&font=Bitter&forks=1&issues=1&language=1&logo=https%3A%2F%2Fgithub.com%2Fghost-him%2FZeroLaunch-rs%2Fblob%2Fmain%2Fsrc-tauri%2Ficons%2FSquare310x310Logo.png%3Fraw%3Dtrue&name=1&owner=1&pattern=Floating+Cogs&pulls=1&stargazers=1&theme=Light)

<div align="center">

![Platform](https://img.shields.io/badge/Platform-Windows_11-0078d7?logo=windows11&logoColor=white)
[![GPLv3 License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ghost-him/ZeroLaunch-rs)
[![GitHub downloads](https://img.shields.io/github/downloads/ghost-him/ZeroLaunch-rs/total)](https://github.com/ghost-him/ZeroLaunch-rs/releases)
[![Release Build](https://github.com/ghost-him/ZeroLaunch-rs/actions/workflows/release.yml/badge.svg)](https://github.com/ghost-him/ZeroLaunch-rs/actions/workflows/release.yml)
[![CI](https://github.com/ghost-him/ZeroLaunch-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ghost-him/ZeroLaunch-rs/actions/workflows/ci.yml)

</div>

<div align="center">


[![Gitee star](https://gitee.com/ghost-him/ZeroLaunch-rs/badge/star.svg?theme=dark)](https://gitee.com/ghost-him/ZeroLaunch-rs/stargazers)
[![Gitee fork](https://gitee.com/ghost-him/ZeroLaunch-rs/badge/fork.svg?theme=dark)](https://gitee.com/ghost-him/ZeroLaunch-rs/members)
[![GitHub stars](https://img.shields.io/github/stars/ghost-him/ZeroLaunch-rs.svg?style=social)](https://github.com/ghost-him/ZeroLaunch-rs/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/ghost-him/ZeroLaunch-rs.svg?style=social)](https://github.com/ghost-him/ZeroLaunch-rs/network/members)
[![GitCode stars](https://gitcode.com/ghost-him/ZeroLaunch-rs/star/badge.svg)](https://gitcode.com/ghost-him/ZeroLaunch-rs/stargazers)

</div>

<div align="center">

[簡體中文](README.md) | [繁體中文](readme-cn2.md) | [English](readme-en.md)

</div>


<div align="center">
    <a href="https://gitee.com/ghost-him/ZeroLaunch-rs" target="_blank">Gitee</a> •
    <a href="https://github.com/ghost-him/ZeroLaunch-rs" target="_blank">GitHub</a> •
    <a href="https://gitcode.com/ghost-him/ZeroLaunch-rs" target="_blank">GitCode</a>
</div>

## 📕 一句話介紹

ZeroLaunch：一款懂你輸入習慣的 Windows 智能啟動器，精通拼音與模糊搜尋，並可選配 AI 語義理解，讓錯字、搜詞都能秒速響應。純淨、離線，為高效而生。

> 目前市面上的程式啟動器都有點不合我的需求，所以我才開發了這款軟體。現在我每天都會使用，所以不需要擔心我會棄坑（頂多是沒東西可更新(～￣▽￣)～

## 🖥️ 軟體介面

[![主介面預覽](asset/主界面.png)](asset/picture-cn.md)

*點擊圖片查看完整功能截圖集*

**背景圖片可自定義**

## ✨ 為什麼選擇 ZeroLaunch-rs / ZeroLaunch-rs 的獨特之處 ?

### 🔒 隱私至上
完全離線運行，無需網路連線，您的資料始終保留在裝置中。堅持零資料收集原則，嚴格遵循本地化處理，確保您的資訊安全。

### ⚡ 高效智能搜尋
在可選的本地 AI 語義搜尋（EmbeddingGemma‑300m/ONNX）加持下，支援自然語言/多語言意圖檢索；即便不啟用 AI，我們基於自研的搜尋匹配演算法（全稱/拼音/首字母三重匹配 + 拼寫糾錯），同樣具備高效性、高匹配性與高容錯性，並提供即時排序。

我們對程式做了系統性的效能優化：從資料結構與熱路徑重構，到分層快取與按需載入、並發模型優化與索引的增量更新，盡可能降低運算與 I/O 開銷；在中低配裝置上也能穩定獲得毫秒級響應。

### 🌐 輕巧純粹
始終專注於「快速、準確地啟動所需內容」。無需折騰即可上手——即使不做任何設定，預設配置也能覆蓋大多數人的使用習慣與情境；同時為進階使用者保留充足的自訂空間（外觀、行為、索引策略皆可細調）。不摻雜與啟動無關的功能，開箱即用、輕巧純粹。

## 🔬 軟體功能

### 核心功能

*   **AI 語義檢索（可選）**：採用最新 EmbeddingGemma‑300m（ONNX）本地向量模型，輕量、高效、精準；在 AI 加持下，支援多語言檢索，亦可透過自然語言意圖關鍵詞（如「音樂軟體」「圖片編輯」）快速定位相關應用。所有推理皆於本地完成，隱私無虞。
*   **應用程式搜尋**：快速檢索並啟動**應用程式**及**UWP應用**，提供流暢的程式存取體驗。支援程式備註和別名，本地化名稱識別與搜尋。
*   **應用程式喚醒**：智能識別並將已開啟的視窗置前，實現便捷的多任務切換。
*   **自定義外觀介面**：外觀高度自定義化，支援自定義背景圖片、選項顏色、搜尋字體顏色與大小、顯示字體顏色與大小、顯示候選個數、毛玻璃效果、圓角大小設定、程式的寬度與高度等多項內容，並且每一項都做了方便交互的按鈕。
*   **多語言支援**：支援簡體中文、繁體中文、英文三種語言。程序啟動時會嘗試讀取系統當前使用的語言，並自動選擇對應的介面語言；若偵測失敗，則預設使用英文。
*   **開啟檔案所在的目錄**：在右鍵選單中，可以開啟目標檔案所在的資料夾。

---
### 更多實用功能 / 進階玩法

*   **微調搜尋演算法**：支援對搜尋演算法做微調，從而滿足個性化設定。
*   **自定義程式與檔案添加**：支援使用檔案萬用字元或正規表達式添加檔案與程式，從而實現對檔案與程式的添加。智能識別檔案的格式並做出正確的反應。
*   **自定義網頁搜尋**：支援添加並使用預設的瀏覽器啟動網頁。無需輸入協議頭（http/https）。
*   **自定義命令搜尋**：支援自定義添加命令，可以實現開機、關機、開啟指定的設定二級頁面的功能。
*   **智能載入程式/檔案/網頁的圖示**：盡最大的可能載入正確的檔案圖示，同時支援 Steam 遊戲圖示的正確載入。
*   **自定義配置檔案的儲存路徑**：支援自定義本地儲存與使用 WebDAV 協定實現網路儲存。
*   **支援開機自啟動與靜默啟動**：沒什麼好解釋的吧==
*   **偵錯功能**：可以查看程式在當前電腦上的運行情況（基本都沒問題），查看搜尋演算法的運行結果。支援設定日誌輸出級別。
*   **遊戲模式**：可以手動關閉快捷鍵，防止在遊戲時誤觸。
*   **支援開啟最近啟動程式**：按住 `Alt` 鍵就可以順序列出最近開啟的程式。
*   **支援自定義按鍵**：可以自定義鍵盤上的映射，可設定成更符合自己的操作方式。
*   **支援呼出位置跟隨滑鼠而動**：如果滑鼠在副螢幕上，則搜尋欄呼出在副螢幕上。
*   **搜尋結果顯示優化**：支援設定搜尋結果顯示閾值，當搜尋的數量大於閾值後會自動切換成滾動模式。

## 🚀 快速入門

### 快捷鍵速查

| 功能                | 快捷鍵           |
|---------------------|------------------|
| 呼出與隱藏搜尋欄          | `Alt + Space`    |
| 上下選擇項目        | `↑/↓` 或 `Ctrl+k/j` |
| 啟動選中程式        | `Enter`          |
| 管理員權限啟動（僅限普通應用）      | `Ctrl + Enter`   |
| 清空搜尋框          | `Esc`            |
| 隱藏搜尋介面        | 點擊外部區域      |
| 開啟已開啟的視窗     | `Shift + Enter` |
| 以最近啟動時間排序  | `Alt` |

### 常見功能的實現

程式添加、檔案添加、命令添加、搜尋演算法微調等功能的實現以及**常見問題**的解決辦法詳見以下文件：[使用指南](docs/Feature_Implementation_Guide_cn2.md)

寫文件好麻煩，有時候也不會描述(っ °Д °;)っ，去 [DeepWiki](https://deepwiki.com/ghost-him/ZeroLaunch-rs) 上看看吧，那個上面講的看起來也不錯。

## 🚩 程式下載

### 使用 WinGet 安裝（推薦）

執行下列任意一個命令即可完成安裝：

```
winget install zerolaunch
```

```
winget install ZeroLaunch-rs
```

```
winget install ghost-him.ZeroLaunch-rs
```

### 從發布頁獲取

本專案已實現全自動構建與發布流程（CI/CD）。每當發布新版本時，GitHub Actions 會自動構建所有變體（AI / Lite 版，x64 / arm64 架構），並同步發布到以下平台。您可以選擇訪問速度最快的鏡像源進行下載：

*   **GitHub Releases:** [https://github.com/ghost-him/ZeroLaunch-rs/releases](https://github.com/ghost-him/ZeroLaunch-rs/releases) (全球用戶推薦)
*   **Gitee Releases:** [https://gitee.com/ghost-him/ZeroLaunch-rs/releases](https://gitee.com/ghost-him/ZeroLaunch-rs/releases) (中國大陸用戶推薦)
*   **GitCode Releases:** [https://gitcode.com/ghost-him/ZeroLaunch-rs/releases](https://gitcode.com/ghost-him/ZeroLaunch-rs/releases) (中國大陸用戶推薦)

### 版本說明（AI / Lite）

本專案提供兩個版本，以滿足不同的資源占用與功能需求：

- 含 AI（預設，推薦）：支援本地語義搜尋（需要另外下載 EmbeddingGemma ONNX 模型），檢索更聰明。
    - 執行時記憶體：啟用 AI 語義搜尋後約 500 ~ 550 MB；若使用傳統搜尋演算法，則與 Lite 版占用一致。
    - 檔名規則：安裝包與便攜包檔名不含 `lite` 標識，例如：
        - `zerolaunch-rs_0.x.x_x64-setup.exe`、`zerolaunch-rs_0.x.x_x64_en-US.msi`
        - `ZeroLaunch-portable-0.x.x-x64.zip`

- 輕量版 Lite（無 AI）：不包含語義搜尋，體積更小、占用更低。
    - 記憶體占用：約 60 ~ 70 MB
    - 檔名規則：檔名包含 `lite` 標識，例如：
        - `zerolaunch-rs_lite_0.x.x_x64-setup.exe`、`zerolaunch-rs_lite_0.x.x_x64_en-US.msi`
        - `ZeroLaunch-portable-lite-0.x.x-x64.zip`

建置提示（開發者）：啟用 AI 功能需加入 `ai` feature；Lite 版請移除該 feature（見 tasks 或 Cargo feature 設定）。使用 `xtask` 時：`build-installer` / `build-portable` 預設即為啟用 AI；若需 Lite 版請加入 `--ai disabled`。

## 🛠️ 開發者指南

詳細的開發指南、環境配置、建置步驟以及貢獻指南，請參考 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 📦 資料目錄結構

程式分為**安裝包版本**與**便捷版**兩個版本：

- **安裝包版本**：將 `C:\Users\[username]\AppData\Roaming\ZeroLaunch-rs\` 作為本地資料目錄
- **便捷版**：將軟體所在的目錄作為本地資料目錄

### 本地資料目錄結構

本地資料目錄中存放以下檔案：

```
本地資料目錄/                            # 安裝包版本：C:\Users\[使用者名稱]\AppData\Roaming\ZeroLaunch-rs\
                                        # 便捷版：軟體所在目錄
├── logs/                               # 運行日誌
├── icons/                              # 程式圖示快取
└── ZeroLaunch_local_config.json        # 本地配置檔案，儲存相關資料以及遠端目錄路徑
```

### 遠端目錄結構

遠端目錄用於存放程式的詳細運行配置，預設為當前的本地資料目錄。透過遠端儲存可以實現兩個機器間的資料同步。

```
遠端目錄/                               # 預設與本地資料目錄相同
├── background.png                      # 自訂背景圖片
└── ZeroLaunch_remote_config.json       # 遠端配置檔案，儲存程式運行配置
```

## 📌 已知限制

### 短詞搜尋

⚠️ 輸入長度 < 3 字元時，搜尋結果可能不夠精確

## 🧠 語義搜尋與記憶體占用

本專案在啟用「語義搜尋」功能時，會在本地載入 Google 的 EmbeddingGemma 模型（ONNX 版本，見 `src-tauri/EmbeddingGemma-300m/`）。不同搜尋模式對記憶體占用約為：

- 普通演算法：約 60 ~ 70 MB
- 語義搜尋演算法（EmbeddingGemma）：約 500 ~ 550 MB

提示：若您希望更低的記憶體占用，可在未啟用 AI/語義搜尋功能的模式下運行（建置時不包含 `ai` 功能特性）。

## 🌍 語言支援

當前 ZeroLaunch-rs 支援以下語言：

- 🇨🇳 簡體中文 (zh-Hans)
- 🇹🇼 繁體中文 (zh-Hant) - 由 Gemini 2.5 Pro 翻譯
- 🇺🇸 English (en) - 由 Gemini 2.5 Pro 翻譯

### 更改語言

您可以透過以下方式更改應用程式的顯示語言：

![語言選擇演示](asset/select_language.png)

*語言選擇介面演示：簡潔直觀的語言切換體驗*

1. **透過設定介面**：
   - 開啟 ZeroLaunch-rs 設定視窗（如上圖所示）
   - 點擊左側導航欄中的「General」選項
   - 在「Language Settings」區域找到「Interface language」下拉選單
   - 點擊下拉選單，從可選語言清單中選擇您偏好的語言（支援中文、繁體中文、English）
   - 儲存設定「Save Config」以套用新的語言設定

> 💡 **貼心提示**：語言切換功能設計簡潔明瞭，無論您選擇哪種語言，整個介面都會完整地切換到對應語言，為不同語言背景的使用者提供原生化的使用體驗。


### 貢獻翻譯

我們歡迎社群貢獻更多語言的本地化翻譯！翻譯檔案位於 `src/i18n/locales/` 目錄下：

- `zh-Hans.json` - 簡體中文翻譯
- `zh-Hant.json` - 繁體中文翻譯
- `en.json` - 英文翻譯

如果您想為 ZeroLaunch-rs 添加新的語言支援，請：

1. 複製現有的翻譯檔案（如 `en.json`）
2. 重新命名為對應的語言代碼（如 `fr.json` 表示法語）
3. 翻譯檔案中的所有文字內容
4. 提交 Pull Request

感謝您為 ZeroLaunch-rs 的國際化做出貢獻！🙏

## 📄 第三方條款 — EmbeddingGemma

- 本專案可選在本地使用 Google 的 EmbeddingGemma 模型，僅用於離線語義檢索。
- 使用與再散布須遵守《Gemma 使用條款》https://ai.google.dev/gemma/terms 與《禁止用途政策》https://ai.google.dev/gemma/prohibited_use_policy。
- 如再散布該模型或其衍生物（非託管服務），需：(1) 在您的協議中傳遞上述限制；(2) 向接收方提供 Gemma 條款副本（可用連結）；(3) 標註被修改的檔案；(4) 隨附名為 NOTICE 的文字檔，內容為：「Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms」。

## ✍️ 代碼簽署策略

免費代碼簽署由 [SignPath.io](https://signpath.io) 提供，憑證由 [SignPath Foundation](https://signpath.org) 提供。

### 團隊角色

專案維護和代碼簽署的職責分配如下：

*   **提交者和審核者：** [ghost-him](https://github.com/ghost-him)
*   **批准者：** [ghost-him](https://github.com/ghost-him)

### 隱私政策

除非使用者或安裝/操作該程式的人員特別要求，否則該程式不會向其他聯網系統傳輸任何資訊。有關更多詳細資訊，請參閱我們完整的[隱私政策](PRIVACY.md)。

## 🤝 開源致謝

本專案基於以下優秀開源專案建置：

*   [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - 中文轉拼音核心詞典
*   [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP應用索引方案
*   [bootstrap](https://icons.bootcss.com/) - 提供了部分的程式圖示
*   [icon-icons](https://icon-icons.com/zh/) - 提供了部分的程式圖示
*   [Follower-v2.0](https://github.com/MrBeanCpp/Follower-v2.0) - 提供了全螢幕偵測的方案

## 未來目標

計劃中的未來目標：

1. 使用資料庫來管理儲存資訊（遠端配置資訊），以減少資訊冗餘。
2. 重構前端頁面，提高程式的可維護性。

以上為高層目標，具體的實作方案（資料庫選型、遷移策略、前端架構方案等）將透過後續的設計討論決定。

## ❤️ 支持作者

可以透過以下的方式支持作者：

1.  點一個免費的小星星⭐
2.  把這個專案分享給其他感興趣的朋友
3.  提出更多改進的建議（ZeroLaunch-rs 的定位就是純粹的程式啟動器，所以只會專注於啟動器的功能，不會添加太多無關的功能哦，請諒解🥺🙏）

[![Star History Chart](https://api.star-history.com/svg?repos=ghost-him/zerolaunch-rs&type=Date)](https://www.star-history.com/#ghost-him/zerolaunch-rs&Date)

> 由 gemini 2.5 flash thinking 翻译