# ZeroLaunch-rs 自动化构建工具

这是一个用于自动化构建 ZeroLaunch-rs 项目的强大工具，支持构建安装包版本和便携版本，包括 x64 和 ARM64 架构，并提供灵活的架构选择功能。

## 🚀 功能特性

- ✨ **架构选择**：支持选择性构建特定架构（x64、ARM64 或全部）
- 📦 **多版本支持**：安装包版本和便携版本
- 🏗️ **多架构支持**：x64 和 ARM64 架构
- 📁 **自动打包**：便携版本自动打包为 ZIP 文件
- 🧹 **智能清理**：清理构建产物和临时文件
- ⚡ **高效构建**：可选择性构建，节省时间

## 📋 使用方法

### 基本命令

#### 构建所有版本（默认：所有架构）
```bash
cargo run --bin xtask build-all
```

#### 构建安装包版本（默认：所有架构）
```bash
cargo run --bin xtask build-installer
```

#### 构建便携版本（默认：所有架构）
```bash
cargo run --bin xtask build-portable
```

#### 清理构建产物
```bash
cargo run --bin xtask clean
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

### 📖 参数说明

| 参数 | 简写 | 可选值 | 默认值 | 说明 |
|------|------|--------|--------|---------|
| `--arch` | `-a` | `x64`, `arm64`, `all` | `all` | 指定构建的目标架构 |

### 💡 使用场景示例

```bash
# 快速构建：仅构建当前平台的 x64 便携版
cargo run --bin xtask build-portable -a x64

# 发布准备：构建所有版本的所有架构
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
- `ZeroLaunch_x.x.x_x64_en-US.msi` - x64 安装包
- `ZeroLaunch_x.x.x_arm64_en-US.msi` - ARM64 安装包

### 便携版本
便携版会打包成 ZIP 文件并放置在项目根目录：
- `ZeroLaunch-portable-x64.zip` - x64 便携版 ZIP 包
- `ZeroLaunch-portable-arm64.zip` - ARM64 便携版 ZIP 包

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