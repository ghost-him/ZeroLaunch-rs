# 🛠️ 开发者指南

感谢您对 ZeroLaunch-rs 的贡献！本文档将引导您如何参与项目的开发。

## 环境要求

在开始开发之前，请确保您已安装以下工具：

* **Rust** v1.90.0 或更高版本
* **Bun** v1.2.22 或更高版本

Github Actions 使用最新稳定版完成构建

## 构建步骤

### 克隆仓库

```bash
git clone https://github.com/ghost-him/ZeroLaunch-rs.git
cd ZeroLaunch-rs
```

### 安装依赖

```bash
bun install
```

### 开发模式

启动开发服务器进行实时开发和调试：

```bash
bun run tauri dev
```

### 生产构建

使用 `xtask` 自动化构建工具进行生产构建。

#### 仅构建安装包（默认启用 AI），x64 版本

```bash
cd xtask
cargo run --bin xtask build-installer --arch x64
```

#### 构建 Lite 版安装包（关闭 AI）

```bash
cd xtask
cargo run --bin xtask build-installer --arch x64 --ai disabled
```

#### 构建所有版本（安装包 + 便携版，所有架构，默认同时含 / 不含 AI）

```bash
cd xtask
cargo run --bin xtask build-all
```

#### 清理构建产物

```bash
cd xtask
cargo run --bin xtask clean
```

## 构建产物

- **安装包**：项目根目录下的 `.msi` 文件
- **便携版**：项目根目录下的 `.zip` 文件
- **详细说明**：请参考 [xtask/README.md](xtask/README.md)

## 数据目录结构

了解程序的数据存储结构有助于调试和开发。

### 本地数据目录

程序分为**安装包版本**与**便携版**两个版本，数据存储位置不同：

- **安装包版本**：`C:\Users\[用户名]\AppData\Roaming\ZeroLaunch-rs\`
- **便携版**：软件所在的目录

### 本地数据目录结构

```
本地数据目录/                              # 安装包版本：C:\Users\[用户名]\AppData\Roaming\ZeroLaunch-rs\
                                          # 便携版：软件所在目录
├── logs/                                 # 运行日志
├── icons/                                # 程序图标缓存
└── ZeroLaunch_local_config.json          # 本地配置文件，存储相关数据以及远程目录路径
```

### 远程目录结构

远程目录用于存放程序的详细运行配置，默认为当前的本地数据目录。通过远程存储可以实现两个机器间的数据同步。

```
远程目录/                                 # 默认与本地数据目录相同
├── background.png                        # 自定义背景图片
└── ZeroLaunch_remote_config.json         # 远程配置文件，存储程序运行配置
```

## 贡献指南

### 问题报告

如果您发现了 bug 或有改进建议，请在 GitHub Issues 中报告。提交 Issue 时，请尽量提供：

- 问题的详细描述
- 复现步骤
- 系统环境信息（Windows 版本、Rust 版本等）
- 相关的日志输出（可在`C:\Users\[username]\AppData\Roaming\ZeroLaunch-rs\logs\`目录下找到）

### 拉取请求（Pull Request）

我们欢迎您的 Pull Request！请遵循以下步骤：

1. Fork 本仓库
2. 创建您的功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交您的更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

### 代码风格

请确保您的代码遵循项目现有的代码风格：

- **Rust 代码**：使用 `cargo fmt` 进行格式化，使用 `cargo clippy` 进行 linting
- **TypeScript/Vue 代码**：遵循现有的代码风格惯例

### 测试

提交 PR 前，请确保：

- 代码能够成功编译（运行 `cargo check`）
- 所有现有功能仍然正常工作
- 新功能包含适当的测试

## 许可证

本项目采用 GPLv3 许可证。参与贡献即表示您同意将您的贡献代码置于相同的许可证下。

## 第三方依赖

本项目使用了以下优秀的开源库和资源：

- [chinese-xinhua](https://github.com/pwxcoo/chinese-xinhua) - 中文转拼音核心词典
- [LaunchyQt](https://github.com/samsonwang/LaunchyQt) - UWP 应用索引方案
- [bootstrap](https://icons.bootcss.com/) - 程序图标
- [icon-icons](https://icon-icons.com/zh/) - 程序图标
- [Follower-v2.0](https://github.com/MrBeanCpp/Follower-v2.0) - 全屏检测方案

### EmbeddingGemma 第三方条款

本项目可选在本地使用 Google 的 EmbeddingGemma 模型，仅用于离线语义检索。

使用与再分发须遵守：

- 《Gemma 使用条款》https://ai.google.dev/gemma/terms
- 《禁止用途政策》https://ai.google.dev/gemma/prohibited_use_policy

如再分发该模型或其衍生物（非托管服务），需：

1. 在您的协议中传递上述限制
2. 向接收方提供 Gemma 条款副本（可用链接）
3. 标注被修改的文件
4. 随附名为 NOTICE 的文本文件，内容为："Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms"

## 联系方式

- **GitHub**: https://github.com/ghost-him/ZeroLaunch-rs
- **Gitee**: https://gitee.com/ghost-him/ZeroLaunch-rs
- **GitCode**: https://gitcode.com/ghost-him/ZeroLaunch-rs
- **官网**: https://zerolaunch.ghost-him.com

感谢您的贡献！🙏
