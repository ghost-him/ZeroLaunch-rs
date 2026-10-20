# ZeroLaunch-rs 自动化构建工具

这是一个用于自动化构建 ZeroLaunch-rs 项目的强大工具，支持构建安装包版本和便携版本，包括 x64 和 ARM64 架构，并提供灵活的架构选择功能。

## 🚀 功能特性

- ✨ **架构选择**：支持选择性构建特定架构（x64、ARM64 或全部）
- 🧠 **AI 特性开关**：支持构建完全体（启用 AI）或精简版（关闭 AI）
- 📦 **多版本支持**：安装包版本和便携版本
- 🏗️ **多架构支持**：x64 和 ARM64 架构
- 📁 **自动打包**：便携版本自动打包为 ZIP 文件
- 🧹 **智能清理**：清理构建产物和临时文件
- ⚡ **高效构建**：可选择性构建，节省时间

## 📋 使用方法

### 基本命令

#### 构建所有版本（默认：所有架构 & 同时构建 启用/关闭 AI 两种模式）
```bash
cargo run --bin xtask build-all
```

#### 仅构建启用 AI 的完全体（只生成含 AI 版本）
```bash
cargo run --bin xtask build-all --ai enabled
```

#### 仅构建关闭 AI 的精简版
```bash
cargo run --bin xtask build-all --ai disabled
```

#### 构建安装包（默认：启用 AI）
```bash
cargo run --bin xtask build-installer
```

#### 构建安装包版本（显式关闭 AI）
```bash
cargo run --bin xtask build-installer --ai disabled
```

#### 构建安装包版本（默认：所有架构，默认启用 AI）
```bash
cargo run --bin xtask build-installer
```

#### 构建便携版本（默认：所有架构，默认启用 AI）
```bash
cargo run --bin xtask build-portable
```

#### 构建便携版本（关闭 AI）
```bash
cargo run --bin xtask build-portable --ai disabled
```

#### 清理构建产物
```bash
cargo run --bin xtask clean
```

#### 同步使用手册快照
`src-tauri/assets/wiki` 是构建期内嵌的 wiki 快照，不入库；干净构建前必须同步一次。
```bash
# 从 ../ZeroLaunch-rs.wiki 同步（不存在时克隆到 target/wiki-cache）
cargo run --bin xtask sync-wiki

# 只校验快照与源是否一致
cargo run --bin xtask sync-wiki --check
```

### 🎯 架构选择功能

#### 仅构建 x64 架构
```bash
# 构建所有版本的 x64 架构
cargo run --bin xtask build-all --arch x64

# 仅构建 x64 安装包
cargo run --bin xtask build-installer --arch x64

# 仅构建 x64 便携版
cargo run --bin xtask build-portable --arch x64
```

#### 仅构建 ARM64 架构
```bash
# 构建所有版本的 ARM64 架构
cargo run --bin xtask build-all --arch arm64

# 仅构建 ARM64 安装包
cargo run --bin xtask build-installer --arch arm64

# 仅构建 ARM64 便携版
cargo run --bin xtask build-portable --arch arm64
```

#### 构建所有架构（显式指定）
```bash
# 明确指定构建所有架构
cargo run --bin xtask build-all --arch all
```

### 🤖 AI 特性开关

当前默认策略：

* build-installer / build-portable 不加 `--ai` 参数时 = 启用 AI（完全体）
* 如需精简 Lite 版，加 `--ai disabled`
* build-all 默认同时构建 enabled + disabled 两种产物，可改为单一模式

```bash
# 构建默认完全体（启用 AI）
cargo run --bin xtask build-installer

# 构建精简版（关闭 AI）
cargo run --bin xtask build-installer --ai disabled

# 便携版默认同样启用 AI
cargo run --bin xtask build-portable

# 构建便携版精简版
cargo run --bin xtask build-portable --ai disabled

# build-all 默认同时构建启用与关闭 AI 的两个版本，可显式限制
cargo run --bin xtask build-all --ai both
```

### 📖 参数说明

| 参数 | 简写 | 可选值 | 默认值 | 说明 |
|------|------|--------|--------|---------|
| `--arch` | `-a` | `x64`, `arm64`, `all` | `all` | 指定构建的目标架构 |
| `--ai` | - | `enabled`, `disabled`, `both`(仅 `build-all`) | `enabled`（`build-all` 默认 `both`） | 是否启用 AI 特性：安装包/便携版默认构建启用 AI；Lite 需显式 `--ai disabled` |

### 💡 使用场景示例

```bash
# 快速构建：仅构建当前平台的 x64 便携版
cargo run --bin xtask build-portable -a x64

# 发布准备：构建所有版本的所有架构（包含启用/关闭 AI）
cargo run --bin xtask build-all

# 测试构建：仅构建 ARM64 安装包
cargo run --bin xtask build-installer --arch arm64

# 清理后重新构建 x64 版本
cargo run --bin xtask clean
cargo run --bin xtask build-all -a x64
```

## 📦 构建产物

### 安装包版本
构建完成后，安装包会自动移动到项目根目录：
- `zerolaunch-rs_0.5.1_x64-setup.exe` / `ZeroLaunch_0.5.1_x64_en-US.msi` 等 —— 启用 AI 的完全体命名（示例）
- `zerolaunch-rs_lite_0.5.1_x64-setup.exe` / `ZeroLaunch_lite_0.5.1_x64_en-US.msi` 等 —— 关闭 AI 的精简版命名（示例）
- ARM64 架构的安装包命名同理，会在架构字段前插入 `_lite`

### 便携版本
便携版会打包成 ZIP 文件并放置在项目根目录：
- `ZeroLaunch-portable-0.5.1-x64.zip` - 启用 AI 的 x64 便携版 ZIP 包
- `ZeroLaunch-portable-lite-0.5.1-x64.zip` - 关闭 AI 的 x64 便携版 ZIP 包
- `ZeroLaunch-portable-0.5.1-arm64.zip` / `ZeroLaunch-portable-lite-0.5.1-arm64.zip` - ARM64 架构同理

便携版 ZIP 包包含：
- 主程序可执行文件
- `icons/` 文件夹（图标资源）
- `locales/` 文件夹（国际化文件，如果存在）

## 🔧 故障排除

### 常见问题

1. **ARM64 构建失败**
   ```bash
   # 安装 ARM64 目标平台
   rustup target add aarch64-pc-windows-msvc
   ```

2. **构建时间过长**
   - 使用架构选择功能仅构建需要的架构
   - 确保系统有足够的内存和存储空间

3. **权限问题**
   - 确保对项目目录有写权限
   - 在 Windows 上可能需要管理员权限

## 📄 许可证

本项目遵循与 ZeroLaunch-rs 主项目相同的许可证。

---

**提示**：首次构建可能需要较长时间，因为需要下载依赖和编译。建议使用架构选择功能来加速开发过程中的构建。