set shell := ["cmd.exe", "/C"]

# Cargo workspace 模式 — 所有 cargo 命令从项目根运行

# 代码风格检查与自动修复
style:
    cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged
    cargo fmt --all
    just package-check

# 打包内容检查（发布前校验 crates.io 产物）
# 仅校验 api：其余 4 个 crate 依赖 api，api 发布后可改为循环全量校验
package-check:
    cargo package -p zerolaunch-plugin-api --allow-dirty --no-verify

# 快速编译检查（全 workspace）
check:
    cargo check --workspace

# 运行测试（全 workspace）
test:
    cargo test --workspace

# 本地模拟 CI（全量检查）
ci:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo fmt --all -- --check
    cargo test --workspace

# 构建 release 主程序
build:
    cargo build --release -p zerolaunch-gpui
