---
name: plugin-real-verify
description: 第三方插件改动的真机验证流程：源码构建产物、校验 zip 布局、按真实路径安装、真实键鼠操作并核对可观测结果。
argument-hint: "[插件仓库路径]"
---

# plugin-real-verify — 插件改动真机验证

插件改动（Rust / UI 脚本 / i18n / manifest 任一）必须走完 **构建 → 验产物 → 安装 → 真实操作 → 可观测结果核对**。只拷文件进安装目录或只跑离线 harness 不算完成。

宿主侧 UI 验证：修改 GPUI 搜索窗/设置窗时另按 GPUI 真机流程验证。

## 1. 构建产物

```bash
python package.py            # 无系统 Python：uv run python package.py
python package.py --no-build # 仅复用现有产物验证打包
```

产物落在 `dist/`。纯 UI/i18n 改动也必须跑一次打包，暴露 zip 布局回归。

## 2. 校验 zip

```bash
unzip -l dist/<zip>   # manifest.toml 在根；bin/<exe>、ui/、i18n/ 按 manifest 声明
```

确认 `manifest.toml` 与 `[ui]` 入口存在，且插件 UI 脚本是本次构建内容。

## 3. 安装

宿主入口：设置 → 插件管理 → 安装本地插件，选择 `dist/` 下的 zip。原生文件对话框由 GPUI 的 rfd 提供。

自动化回退：按 zip 布局解压到 `%USERPROFILE%/.ZeroLaunch-rs/plugins/<plugin-id>/`，然后重启宿主（插件 UI 模块有缓存）。

## 4. 模拟用户操作

- 用全局热键唤起搜索窗，输入插件触发词，按 Enter 打开面板/执行动作。
- 插件 UI 由 `gpui-shell` 在宿主进程内运行，直接观察面板渲染文本与焦点行为，不使用 WebView/CDP。
- 设置页验证：插件管理列表描述取自已安装 `manifest.toml`，描述变化可证明安装的是新包。

## 5. 可观测结果

- 面板文本、候选描述、动作副作用（资源管理器路径、浏览器标题、剪贴板）。
- 插件日志：`%USERPROFILE%/.ZeroLaunch-rs/plugin-logs/<plugin-id>.log`。
- 宿主日志：`%USERPROFILE%/.ZeroLaunch-rs/logs/`。
- 截图或窗口枚举作为界面证据。

## 6. 汇报判据

- 产物路径、zip 文件清单、安装方式。
- 真实操作后的界面文本与副作用。
- 相邻按键/动作回归一次。
- 离线 harness 只能作为补充证据。

## 7. 清理

1. 关闭本次打开的资源管理器/外部程序，删除临时解包目录。
2. 若动过插件安装目录，还原已发布内容；用户刻意安装的新包保持不动并说明。
3. 停掉本次启动的宿主实例；用户正在使用的实例保留并说明。
4. 仓库不留一次性脚本。
5. 发版时 `manifest.toml`、`Cargo.toml`、git tag 三处版本一致。
