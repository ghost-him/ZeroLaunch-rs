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
[![Release Build](https://github.com/ghost-him/ZeroLaunch-rs/actions/workflows/release.yml/badge.svg)](https://github.com/ghost-him/ZeroLaunch-rs/actions/workflows/release.yml)
[![CI](https://github.com/ghost-him/ZeroLaunch-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ghost-him/ZeroLaunch-rs/actions/workflows/ci.yml)
</div>

<div align="center">

[简体中文](README.md) | [繁體中文](readme-cn2.md) | [English](readme-en.md)

</div>


<div align="center">
    <a href="https://gitee.com/ghost-him/ZeroLaunch-rs" target="_blank">Gitee</a> •
    <a href="https://github.com/ghost-him/ZeroLaunch-rs" target="_blank">GitHub</a> •
    <a href="https://gitcode.com/ghost-him/ZeroLaunch-rs" target="_blank">GitCode</a> •
    <a href="https://zerolaunch.ghost-him.com" target="_blank">官网</a>
</div>

## 📕 一句话介绍

ZeroLaunch：一款懂你输入习惯的 Windows 智能启动器，精通拼音与模糊搜索，更能选配 AI 语义理解，让错字、搜词都能秒速响应。纯净、离线，为高效而生。

> 目前市面上的程序启动器都有点不合我的需求，所以我就搞了这个软件。现在每天都会使用，所以不需要担心我会跑路（最多是没东西更新(～￣▽￣)～

## 🖥️ 软件界面

[![主界面预览](asset/主界面.png)](asset/picture.md)

*点击图片查看完整功能截图集*

**背景图片可自定义**

## ✨ 为什么选择 ZeroLaunch-rs / ZeroLaunch-rs 的独特之处 ?

### 🔒 隐私至上
完全离线运行，无需网络连接，您的数据始终保留在设备中。坚持零数据采集原则，严格遵循本地化处理，确保您的信息安全。

### ⚡ 高效智能搜索
在可选的本地 AI 语义搜索（EmbeddingGemma‑300m/ONNX）加持下，支持自然语言/多语言意图检索；即便不启用 AI，我们基于自研的搜索匹配算法（全称/拼音/首字母三重匹配 + 拼写纠错），同样具备高效性、高匹配性与高容错性，并提供实时排序。

我们对程序做了系统性的性能优化：从数据结构与热路径重构，到分层缓存与按需加载、并发模型优化与索引的增量更新，尽可能降低计算与 I/O 开销；在中低配设备上也能稳定获得毫秒级响应。

### 🌐 轻巧纯粹
始终专注于“快速、准确地启动所需内容”。无需折腾即可上手——即使不做任何配置，默认设置也能覆盖大多数人的使用习惯与场景；同时为进阶用户保留充分的个性化空间（外观、行为、索引策略均可细调）。不夹杂与启动无关的功能，开箱即用、轻巧纯粹。

## 🔬 软件功能

### 核心功能

* **AI 语义检索（可选）**：基于最新 EmbeddingGemma-300m（ONNX）本地向量模型，轻量、高效、准确；在 AI 的加持下，支持多语言检索，也可使用自然语言意图关键词（如“音乐软件”“图片编辑”）快速定位相关应用。所有推理均在本地完成，隐私无忧。
* **应用程序搜索**：快速检索并启动**应用程序**及**UWP应用**，支持程序备注与别名，实现对程序本地化名称的识别与搜索，提供流畅的程序访问体验。
* **应用程序唤醒**：智能识别并将已打开的窗口置前，实现便捷的多任务切换。
* **自定义外观界面**：外观高度自定义化，支持自定义背景图片，选项颜色，搜索字体颜色与大小，显示字体颜色与大小，显示候选个数，毛玻璃效果，圆角大小设置，程序的宽度与高度等多项内容，并且每一项都做了方便交互的按钮。
* **多语言支持**：支持简体中文与英文，可以自由切换。程序启动时会尝试读取系统当前使用的语言并自动选择对应的界面语言；若检测失败，则改用英文作为默认语言。
* **打开文件所在的目录**：在右键菜单中，可以打开目标文件所在的文件夹。

---
### 更多实用功能 / 进阶玩法

* **微调搜索算法**：支持对搜索算法做微调，从而满足个性化设置。
* **自定义程序与文件添加**：支持使用文件通配符或正则表达式添加文件与程序，从而实现对文件与程序的添加。智能识别文件的格式并做出正确的反映。
* **自定义网页搜索**：支持添加并使用默认的浏览器启动网页，无需输入 `http://` 或 `https://`。
* **自定义命令搜索**：支持自定义添加命令，可以实现开机、关机、打开指定的设置二级页面的功能。
* **智能加载程序/文件/网页的图标**：尽最大的可能加载正确的文件图标，同时支持steam游戏图标的正确加载。
* **自定义配置文件的保存路径**：支持自定义本地存储与使用 WebDAV 协议实现网络存储。
* **支持开机自启动与静默启动**：没啥好解释的吧==
* **调试功能**：可以查看程序在当前电脑上的运行情况（基本都没问题），查看搜索算法的运行结果，并设置日志输出级别。
* **游戏模式**：可以手动关闭快捷键，防止在游戏时寄掉。
* **支持打开最近启动程序**：按住 `Alt` 键就可以顺序列出最近打开的程序。
* **支持自定义按键**：可以自定义键盘上的映射，可设置成更符合自己的操作方式。
* **支持呼出位置跟随鼠标而动**：如果鼠标在副屏上，则搜索栏呼出在副屏上。
* **搜索结果显示优化**：支持设置搜索结果显示阈值，当搜索的数量大于阈值后会自动切换成滚动模式。

## 🚀 快速入门

### 快捷键速查

| 功能                | 快捷键           |
|---------------------|------------------|
| 呼出与隐藏搜索栏          | `Alt + Space`    |
| 上下选择项目        | `↑/↓` 或 `Ctrl+k/j` |
| 启动选中程序        | `Enter`          |
| 管理员权限启动（仅限普通应用）      | `Ctrl + Enter`   |
| 清空搜索框          | `Esc`            |
| 隐藏搜索界面        | 点击外部区域      |
| 打开已打开的窗口     | `Shift + Enter` |
| 以最近启动时间排序  | `Alt` |

### 常见功能的实现

程序添加，文件添加，命令添加，搜索算法微调等功能的实现以及**常见的问题**的解决办法详见以下文档：[使用指南](doc/Feature_Implementation_Guide_cn.md)

写文档好麻烦，有的时候也不会描述(っ °Д °;)っ，去 [DeepWiki](https://deepwiki.com/ghost-him/ZeroLaunch-rs) 上看看吧，那个上面讲的看起来也不错。

## 🚩 程序下载

### 使用 WinGet 安装（推荐）

运行以下任意一个命令即可完成安装

```
winget install zerolaunch
```

```
winget install ZeroLaunch-rs
```

```
winget install ghost-him.ZeroLaunch-rs
```

### 从发布页获取

本项目已实现全自动构建与发布流程（CI/CD）。每当发布新版本时，GitHub Actions 会自动构建所有变体（AI / Lite 版，x64 / arm64 架构），并同步发布到以下平台。您可以选择访问速度最快的镜像源进行下载：

*   **GitHub Releases:** [https://github.com/ghost-him/ZeroLaunch-rs/releases](https://github.com/ghost-him/ZeroLaunch-rs/releases) (全球用户推荐)
*   **Gitee Releases:** [https://gitee.com/ghost-him/ZeroLaunch-rs/releases](https://gitee.com/ghost-him/ZeroLaunch-rs/releases) (中国大陆用户推荐)
*   **GitCode Releases:** [https://gitcode.com/ghost-him/ZeroLaunch-rs/releases](https://gitcode.com/ghost-him/ZeroLaunch-rs/releases) (中国大陆用户推荐)

### 版本说明（AI / Lite）

本项目提供两个版本，满足不同资源占用与功能需求：

- 含 AI（默认，推荐）：支持本地语义搜索（需要额外下载EmbeddingGemma ONNX模型），检索更智能。
    - 运行时内存占用：启用ai语义搜索后，约 500 ~ 550 MB；若使用传统搜索算法，则与lite版内存占用一致。
    - 文件命名：安装包与便携包文件名不包含 `lite` 标识，例如：
        - `zerolaunch-rs_0.x.x_x64-setup.exe`、`zerolaunch-rs_0.x.x_x64_en-US.msi`
        - `ZeroLaunch-portable-0.x.x-x64.zip`

- 轻量版 Lite（无 AI）：不包含语义搜索，体积更小、占用更低。
    - 内存占用：约 60 ~ 70 MB
    - 文件命名：文件名包含 `lite` 标识，例如：
        - `zerolaunch-rs_lite_0.x.x_x64-setup.exe`、`zerolaunch-rs_lite_0.x.x_x64_en-US.msi`
        - `ZeroLaunch-portable-lite-0.x.x-x64.zip`

构建提示（开发者）：启用 AI 功能需包含特性 `ai`；Lite 版请移除该特性（参见 tasks 或 Cargo feature 配置）。使用 `xtask` 时：`build-installer` / `build-portable` 默认即构建含 AI 版本，如需 Lite 版本请添加 `--ai disabled`。

## 🛠️ 开发者指南

详细的开发指南、环境配置、构建步骤以及贡献指南，请参考 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 📦 数据目录结构

程序分为**安装包版本**与**便捷版**两个版本：

- **安装包版本**：将 `C:\Users\[username]\AppData\Roaming\ZeroLaunch-rs\` 作为本地数据目录
- **便捷版**：将软件所在的目录作为本地数据目录

### 本地数据目录结构

本地数据目录中存放以下文件：

```
本地数据目录/                            # 安装包版本：C:\Users\[用户名]\AppData\Roaming\ZeroLaunch-rs\
                                        # 便捷版：软件所在目录
├── logs/                               # 运行日志
├── icons/                              # 程序图标缓存
└── ZeroLaunch_local_config.json        # 本地配置文件，存储相关数据以及远程目录路径
```

### 远程目录结构

远程目录用于存放程序的详细运行配置，默认为当前的本地数据目录。通过远程存储可以实现两个机器间的数据同步。

```
远程目录/                               # 默认与本地数据目录相同
├── background.png                      # 自定义背景图片
└── ZeroLaunch_remote_config.json       # 远程配置文件，存储程序运行配置
```

## 📌 已知限制

### 短词搜索

⚠️ 输入长度 < 3 字符时，搜索结果可能不够精确

## 🌍 语言支持

当前 ZeroLaunch-rs 支持以下语言：

- 🇨🇳 简体中文 (zh-Hans)
- 🇹🇼 繁体中文 (zh-Hant) - 由 Gemini 2.5 Pro 翻译
- 🇺🇸 English (en) - 由 Gemini 2.5 Pro 翻译

### 更改语言

您可以通过以下方式更改应用程序的显示语言：

![语言选择演示](asset/select_language.png)

*语言选择界面演示：简洁直观的语言切换体验*

1. **通过设置界面**：
   - 打开 ZeroLaunch-rs 设置窗口（如上图所示）
   - 点击左侧导航栏中的「General」选项
   - 在「Language Settings」区域找到「Interface language」下拉菜单
   - 点击下拉菜单，从可选语言列表中选择您偏好的语言（支持中文、繁体中文、English）
   - 保存设置「Save Config」以应用新的语言设置

> 💡 **贴心提示**：语言切换功能设计简洁明了，无论您选择哪种语言，整个界面都会完整地切换到对应语言，为不同语言背景的用户提供原生化的使用体验。

### 贡献翻译

我们欢迎社区贡献更多语言的本地化翻译！翻译文件位于 `src/i18n/locales/` 目录下：

- `zh-Hans.json` - 简体中文翻译
- `zh-Hant.json` - 繁体中文翻译
- `en.json` - 英文翻译

如果您想为 ZeroLaunch-rs 添加新的语言支持，请：

1. 复制现有的翻译文件（如 `en.json`）
2. 重命名为对应的语言代码（如 `fr.json` 表示法语）
3. 翻译文件中的所有文本内容
4. 提交 Pull Request

感谢您为 ZeroLaunch-rs 的国际化做出贡献！🙏

## 📄 第三方条款 — EmbeddingGemma

- 本项目可选在本地使用 Google 的 EmbeddingGemma 模型，仅用于离线语义检索。
- 使用与再分发须遵守《Gemma 使用条款》https://ai.google.dev/gemma/terms 及《禁止用途政策》https://ai.google.dev/gemma/prohibited_use_policy。
- 如再分发该模型或其衍生物（非托管服务），需：(1) 在您的协议中传递上述限制；(2) 向接收方提供 Gemma 条款副本（可用链接）；(3) 标注被修改的文件；(4) 随附名为 NOTICE 的文本文件，内容为："Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms"。

## ✍️ 代码签名策略

免费代码签名由 [SignPath.io](https://signpath.io) 提供，证书由 [SignPath Foundation](https://signpath.org) 提供。

### 团队角色

项目维护和代码签名的职责分配如下：

*   **提交者和审核者：** [ghost-him](https://github.com/ghost-him)
*   **批准者：** [ghost-him](https://github.com/ghost-him)

### 隐私政策

除非用户或安装/操作该程序的人员特别要求，否则该程序不会向其他联网系统传输任何信息。有关更多详细信息，请参阅我们完整的[隐私政策](PRIVACY.md)。

## 🤝 开源致谢

本项目基于以下优秀开源项目构建：

* [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - 中文转拼音核心词典
* [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP应用索引方案
* [bootstrap](https://icons.bootcss.com/) - 提供了部分的程序图标
* [icon-icons](https://icon-icons.com/zh/) - 提供了部分的程序图标
* [Follower-v2.0](https://github.com/MrBeanCpp/Follower-v2.0) - 提供了全屏检测的方案

## 未来目标

计划中的未来目标：

1. 使用数据库来管理存储信息（远程配置信息），以减少信息冗余。
2. 重构前端页面，提高程序的可维护性。

以上为高层目标，具体的实现方案（数据库选型、迁移策略、前端架构方案等）将通过后续的设计讨论确定。

## ❤️ 支持作者

可以通过以下的方式支持作者：

1. 点一个免费的小星星⭐
2. 把这个项目分享给其他感兴趣的朋友
3. 提出更多改进的建议（ZeroLaunch-rs 的定位就是纯粹的程序启动器，所以只会专注于启动器的功能，不会添加太多无关的功能哦，请谅解🥺🙏）

[![Star History Chart](https://api.star-history.com/svg?repos=ghost-him/zerolaunch-rs&type=Date)](https://www.star-history.com/#ghost-him/zerolaunch-rs&Date)