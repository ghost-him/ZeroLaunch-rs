---
description: 第三方插件 SDK init/t_key 时序 — main 首行必须调 init() 读 ZEROLAUNCH_PLUGIN_ID，t_key() 在握手前调用必 panic
condition: "ZEROLAUNCH_PLUGIN_ID|t_key|\\binit\\(\\)|PLUGIN_ID"
scope: "tool:read(crates/plugin-sdk-rust/**), tool:edit(crates/plugin-sdk-rust/**), tool:write(crates/plugin-sdk-rust/**), tool:read(crates/zerolaunch-rs/src/plugin_framework/plugin_manager.rs), tool:edit(crates/zerolaunch-rs/src/plugin_framework/plugin_manager.rs), tool:write(crates/zerolaunch-rs/src/plugin_framework/plugin_manager.rs)"
---

# SDK init/t_key 时序契约

第三方插件 SDK（`crates/plugin-sdk-rust`）用 `OnceLock<String>` 存插件 id，宿主在 spawn 子进程时经环境变量 `ZEROLAUNCH_PLUGIN_ID` 注入。

## 约束

- 插件 `main()` **首行必须** 调用 SDK 的 `init()`——它读取 `ZEROLAUNCH_PLUGIN_ID` 预置 `PLUGIN_ID`，使 `t_key()` 可在组件元数据构造时（`run()` 之前）安全使用
- `t_key(key)` 在 `PLUGIN_ID` 未设置时 **panic**（`expect`）——这是程序员时序错误（漏调 init / 在 init 前用 t_key），不是可恢复的用户错误
- 宿主侧 `plugin_manager.rs` spawn 子进程时必须注入 `ZEROLAUNCH_PLUGIN_ID`（连同 `ZEROLAUNCH_DATA_DIR`/`ZEROLAUNCH_LOG_DIR`）
- 插件 i18n 键由宿主 `register_plugin_catalog` 做 dotted-key 展开——插件侧只管用 `t_key("组件元数据用键")` 返回 `plugin.<id>.<key>` 形态

## 症状对照

- 插件 stderr 出现 `t_key` panic、或安装失败报 transport closed → 检查插件 main 是否首行调 `init()`、宿主是否注入 env
