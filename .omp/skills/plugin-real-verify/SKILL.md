---
name: plugin-real-verify
description: 第三方插件（ZeroLaunch-plugin-* 与 plugin-template）改动的真机验证流程：源码构建产物、校验 zip 布局、按真实路径安装、真实键鼠模拟并核对可观测结果，含 CDP 面板驱动要点、原生窗口取证片段与清理清单。改插件 Rust/UI/i18n/manifest 任一环节后使用。
argument-hint: "[插件仓库路径]"
---

# plugin-real-verify — 插件改动真机验证

**铁律**：插件改动（Rust / UI / i18n / manifest 任一）必须走完 **构建 → 验产物 → 安装 → 真实操作 → 可观测结果核对** 五步。只把改动文件拷进已安装目录、不打包不安装，或只有离线 harness（happy-dom 之类）结论，都不算验证完成。

宿主侧（ZeroLaunch-rs 本体）UI 验证见 `dev-verify-cdp`。

---

## 1. 构建产物

在插件仓库根目录执行打包脚本（内部会 `cargo build --release` 后打 zip）：

```bash
python package.py            # 无系统 Python 时：uv run python package.py
python package.py --no-build # 只复用现有产物、仅验证打包时
```

产物落在 `dist/`。**纯 UI / i18n 改动也必须真跑一次打包**——它是唯一能暴露打包脚本与 zip 布局回归的环节。

## 2. 校验产物（构建与安装之间最容易漏的一步）

```bash
unzip -l dist/<zip>                                  # 期望：manifest.toml 在根、bin/<exe>、ui/、i18n/，DLL 在根
unzip -o -q dist/<zip> -d /tmp/zipcheck && grep -c <本次新增的符号> /tmp/zipcheck/ui/<面板文件>
```

符号计数用于证明「装进去的确实是这次构建的产物」，而不是上一次的包。

## 3. 安装（真实路径）

宿主侧入口：**设置 → 插件管理 → 从本地文件安装**（zip；旁边另有「从目录安装」）。链路是：`@tauri-apps/plugin-dialog` 原生文件对话框（窗口类 `#32770`）→ 应用内预检确认弹窗 → `plugin_install_local`；已安装同名插件时会再弹「是否覆盖」。

- **不要让脚本用 SendKeys 驱动原生对话框**：实测 AppActivate 返回 True 但按键落点不可控（一次静默失败、一次把对话框关掉）。也不要仅凭 UIA「找不到该窗口」就断言对话框没开——UIA 的 `TreeScope::Children` 在该对话框上有盲区，而 `EnumWindows` 能枚举到它。
- 优先让用户自己点、自己装；若必须自动化，用下面的**等价回退**并在汇报里写明用的是回退路径：

```bash
# 回退：按 zip 原布局解包到插件目录
unzip -o dist/<zip> -d "$USERPROFILE/.ZeroLaunch-rs/plugins/<plugin-id>/"
```

- 生效方式：宿主按 URL 缓存插件 ESM 模块。**`Page.reload` 搜索窗口即可清掉面板模块缓存**，比重启宿主快。拖拽安装走 webview 级 `onDragDropEvent`（OS 级事件，CDP 造不出来）。

## 4. 模拟用户操作（真实事件）

CDP 连接与启动流程见 `dev-verify-cdp`。插件面板相关的额外要点：

- **必须开焦点仿真**：`Emulation.setFocusEmulationEnabled { enabled: true }`，否则 `focus()` 不落地、键盘事件打到 DIV 上，面板键盘处理永不触发。
- 进面板：全局热键（OS 级）CDP 打不到，用**触发词**唤醒——把 `value` 写入搜索框 + 派发 `input` 事件 → 等防抖 → 在候选列表上按**真实 Enter**。
- 面板在 Shadow DOM：`.third-party-panel-host` → `shadowRoot` → 面板内部选择器（如 `#<前缀>-input`、`.ev-item` 这类由插件约定的类名，以其前端源码为准）。
- 面板内输入用 `Input.insertText`（等价真实键入）；尽量不用「改 value + 派发事件」。
- **不要钩 `window.__TAURI_INTERNALS__.invoke`**：该属性不可写，赋值静默失效，会让你误判「动作没触发」。**断言副作用，不要断言调用**。
- 焦点相关行为必须实测 `document.activeElement`：真实点击不可聚焦的列表项后焦点会回到 `BODY`，挂在输入框上的键盘处理整片失效——「点击后快捷键无效」这类问题只能靠实测定位。

## 5. 可观测结果核对

- 面板渲染文本（插件面板提示语、条目内容）与动作执行后的副作用。
- 资源管理器：先取基线，只核对/清理自己新开的窗口：
  ```powershell
  (New-Object -ComObject Shell.Application).Windows() | ForEach-Object { $_.LocationURL }
  ```
  只读，**绝不要 `.Quit()`**（Win11 单进程多标签会连带关掉用户其它窗口）。
- 浏览器：用**窗口标题变化**判断是否真的导航，不要只看进程是否存在。
- 日志：插件 `%USERPROFILE%\.ZeroLaunch-rs\plugin-logs\<plugin-id>.log`；宿主 `%USERPROFILE%\.ZeroLaunch-rs\logs\`。
- 安装生效的旁证：候选列表里插件的**描述文案**取自已安装包的 `manifest.toml`，描述变了即说明装的是新包。

## 6. 原生窗口取证（UIA 看不见时用它）

```powershell
Add-Type @"
using System;using System.Text;using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
}
"@
$fg = [W]::GetForegroundWindow()
$cb = [W+EnumWindowsProc]{ param($h,$l)
  if ([W]::IsWindowVisible($h)) {
    $t = New-Object System.Text.StringBuilder 512; [void][W]::GetWindowText($h,$t,512)
    $c = New-Object System.Text.StringBuilder 256; [void][W]::GetClassName($h,$c,256)
    if ($t.ToString() -or $c.ToString() -eq '#32770') { "hwnd=$($h.ToInt64()) class=$($c.ToString()) title='$($t.ToString())' fg=$($h -eq $fg)" }
  }
  return $true
}
[void][W]::EnumWindows($cb,[IntPtr]::Zero)
# 关闭自己打开的对话框（WM_CLOSE=0x0010）：SendMessage($h, 0x0010, 0, 0)
```

跑 PowerShell 时建议 `-NoProfile -WindowStyle Hidden`（必要时 `-EncodedCommand` 传 UTF-16LE base64），避免控制台抢前台干扰被测窗口。

## 7. 汇报判据

- **产物**：打包脚本输出路径、zip 内文件清单（含新增符号计数）、安装方式（用户手动 / UI 装 / 解压回退）。
- **真机现象**：面板渲染文本、真实按键后的副作用（资源管理器路径、浏览器标题）、对应日志行。
- **回归**：改动相邻的既有按键/动作各跑一次（例：动了 Ctrl+Enter 必须复验 Enter）。
- 离线 harness 结论只算补充证据，不能替代真机验证。

## 8. 清理清单

1. 关掉自己打开的资源管理器窗口与外部程序；删临时标记目录与 `/tmp` 解包目录。
2. 若动过插件安装目录：还原为已发布内容；若是用户刻意安装新包，**保持安装态不动**并在汇报里说明。
3. 停掉本次启动的 dev 实例（按 `dev-verify-cdp` 的清理顺序，确认无残留进程与端口占用）；用户正在使用的实例保留并说明。
4. 仓库里不留一次性脚本；`dist/` 产物可留（已 gitignore）。
5. 发版提醒：版本号需在 `manifest.toml`、`Cargo.toml`、git tag 三处一致。
