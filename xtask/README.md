# ZeroLaunch 构建工具

`xtask` 负责构建安装包、便携包与 CLI，并同步构建期内嵌的使用手册快照。

## 前置条件

- Rust stable，安装 `x86_64-pc-windows-msvc` / `aarch64-pc-windows-msvc` 目标。
- 构建安装包需要 Inno Setup 6，`iscc` 必须在 `PATH` 中。
- 干净构建前必须同步 wiki 快照（`crates/zerolaunch-rs/assets/wiki` 不入库）。

## 命令

```bash
# 从 ../ZeroLaunch-rs.wiki 同步（不存在时克隆到 target/wiki-cache）
cargo run --bin xtask sync-wiki
cargo run --bin xtask sync-wiki --check

# 构建安装包（Inno Setup）
cargo run --bin xtask build-installer --arch x64
cargo run --bin xtask build-installer --arch arm64

# 构建便携包
cargo run --bin xtask build-portable --arch x64
cargo run --bin xtask build-portable --arch arm64

# 同时构建安装包与便携包
cargo run --bin xtask build-all

# 清理 target 与根目录产物
cargo run --bin xtask clean
```

`--arch` 可取 `x64`、`arm64`、`all`；默认 `all`。

## 产物

- `ZeroLaunch-{version}-{arch}-setup.exe`：Inno Setup 安装包。
- `ZeroLaunch-portable-{version}-{arch}.zip`：解压即用，内含 `ZeroLaunch_{arch}.exe`、`icons/`、`locales/`。
- `zerolaunch-cli_{version}_{arch}.exe`：CLI 客户端。

安装包与便携包都由同一 `zerolaunch-gpui` 可执行文件产生；便携模式通过 Cargo `portable` feature 把数据目录解析到 exe 所在目录。

## 故障排除

- ARM64 构建失败：`rustup target add aarch64-pc-windows-msvc`。
- 提示 `iscc` 不存在：安装 Inno Setup 6 并重新打开终端。
- 构建脚本提示缺少 wiki：先运行 `sync-wiki`。
