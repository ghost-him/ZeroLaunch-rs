# ZeroLaunch-rs 🚀

[![GPLv3 License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
![Platform](https://img.shields.io/badge/Platform-Windows-0078d7)
![Rust](https://img.shields.io/badge/Rust-1.72%2B-orange)

现代化的Windows快速启动工具，使用 Rust + Tauri + Vue.js 构建

[![主界面预览](asset/主界面.png)](asset/picture.md)  
*点击图片查看完整功能截图集*

## ✨ 核心特性

- **隐私优先** 🔒
  - 完全本地运行，无需网络连接
  - 不收集任何用户数据

- **极致搜索** 🔍
  - 三重智能匹配：全称/模糊/拼音搜索
  - 支持中英文混合输入
  - 实时结果排序优化
  - 多线程搜索

- **高效运维** ⚙️
  - 配置文件云同步
  - 智能路径管理

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
   进入设置 → 其他 → 选择目标路径（推荐使用网盘同步目录）

2. **自动同步配置**  

```plaintext
    [同步目录]
        ├── config.json      # 程序配置
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

* Rust
* Node.js
* Yarn

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/ghost-him/ZeroLaunch-rs.git

# 安装依赖
yarn install

# 开发模式
yarn tauri dev

# 生产构建
yarn tauri build
```

构建产物路径：./src-tauri/target/release/

## 📦 数据目录结构

```
%APPDATA%\ZeroLaunch-rs\
├── logs/                               # 运行日志
└── ZeroLaunch_local_config.json        # 配置文件的存放地址
```

## 📌 已知限制

### 输入法兼容性

ℹ️ 部分中文输入法（如Rime）可能导致输入延迟，建议临时切换为系统默认输入法

### 短词搜索

⚠️ 输入长度 < 3 字符时，搜索结果可能不够精确

## 🤝 开源致谢

本项目基于以下优秀开源项目构建：

* chinese-xinhua - 中文转拼音核心词典
* Bootstrap Icons - 界面图标资源
* LaunchyQt - UWP应用索引方案