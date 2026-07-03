set shell := ["cmd.exe", "/C"]

# Cargo workspace 模式 — 所有 cargo 命令从项目根运行

# 代码风格检查与自动修复
style:
    cargo fmt --all
    cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged

# 快速编译检查（全 workspace）
check:
    cargo check --workspace

# 运行测试（全 workspace）
test:
    cargo test --workspace

# 本地模拟 CI（全量检查）
ci:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo test --workspace

# 构建前端 + release 编译
build:
    bun run build
    cargo build --release
