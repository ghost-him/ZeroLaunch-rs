---
name: dev-verify-cdp
description: 在 Windows 下启动 ZeroLaunch-rs dev 实例并用 WebView2 CDP 真实驱动做 UI/行为验证的完整流程：清理残留实例、带调试端口启动、双窗口精确目标定位、真实键鼠驱动、断言选择器、面板接管与 OS 副作用取证、收尾清理。改动了 UI/设置页/面板/搜索链路后需要真机验证时使用。
argument-hint: "[验证目标: 搜索窗口 | 设置页 | 插件面板 | 全部]"
---

# dev-verify-cdp — dev 实例 + CDP 真实验证

**前提**：Windows；仓库根目录；已装 Bun 与 Rust 工具链。

**适用**：`.rs` / `src-ui/` 改动后需要真实驱动验证（禁止用离线 mock 替代真机结论）。前端改动经 vite HMR 实时生效，后端 `.rs` 改动需等 cargo 增量重编译。

---

## 1. 启动 dev 实例

### 1.1 先清残留（必做）

`tauri-plugin-single-instance` 会让新实例在旧实例存在时**静默退出（exit 0、无窗口、无提示）**，表现为「启动了但连不上 CDP」。

```bash
tasklist | grep -i zerolaunch            # 应无输出
taskkill /F /IM zerolaunch-rs.exe        # 结束旧实例（先确认不是用户正在用的实例）
netstat -ano | grep LISTENING | grep -E "9222|12345"   # 确认端口已释放
```

### 1.2 带调试端口启动

```bash
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222 --remote-allow-origins=*" bun run tauri dev
```

- 用支持后台服务的终端工具启动时：就绪条件用**端口 9222**（或日志 `应用启动完成`），**不要**在命令里加 `&`、也不要给很短的超时，否则会被判为「进程已立即退出」。
- 就绪 ≠ 可以连接：端口开了之后主窗口 target 可能还没出现；`.rs` 改动触发自动重建重启期间 target 列表会短暂为空。`fetch /json/list` 为空时先用 `curl` 确认端口真实在监听，再重连一次。

---

## 2. 目标定位（双窗口）

ZeroLaunch 有两个 WebView2 page target：

| 窗口 | URL | window label |
|---|---|---|
| 搜索窗口 | `http://localhost:12345/#/` | `main` |
| 设置窗口 | `http://localhost:12345/setting_window.html` | `setting_window` |

```bash
curl -s http://127.0.0.1:9222/json/list     # 直接看原始 JSON，按 url 字段挑目标
```

- **按 URL 精确匹配选 target**，不要按 title 子串：`"ZeroLaunch"` 会先命中 `"ZeroLaunch 设置"`，导致反复连到设置窗口。（下面的 JS 骨架即按 `url` 字段筛选，无需 jq 等额外工具。）
- 两窗口默认都隐藏，操作前先显示：`window.__TAURI_INTERNALS__.invoke('plugin:window|show', { label: 'main' })`（设置窗把 label 换成 `setting_window`，之后等约 2s 让异步内容加载完）。

---

## 3. CDP 驱动骨架

`Runtime.evaluate` 查/改 DOM；`Input.*` 发真实事件。WebSocket 客户端（在 `eval` 里用 JS）：

```js
const t = (await (await fetch('http://127.0.0.1:9222/json/list')).json())
  .find(x => x.type === 'page' && x.url.includes('#/'))     // 设置窗用 'setting_window.html'
const ws = new WebSocket(t.webSocketDebuggerUrl)
await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej })
let id = 0
const pending = new Map()
ws.onmessage = (e) => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id) } }
const send = (method, params = {}) => new Promise((res) => { const i = ++id; pending.set(i, res); ws.send(JSON.stringify({ id: i, method, params })) })
const evaluate = async (expr) => {
  const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true })
  if (r.result?.exceptionDetails) throw new Error(JSON.stringify(r.result.exceptionDetails))
  return r.result?.result?.value
}
```

驱动原语：

```js
// 真实按键（Tab 需要 virtualKeyCode=9；modifiers: Alt=1 Ctrl=2 Meta=4 Shift=8）
const key = (k, code, vk, modifiers = 0) => send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: k, code, windowsVirtualKeyCode: vk, modifiers })
  .then(() => send('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code, windowsVirtualKeyCode: vk, modifiers }))
// Enter=13, Escape=27, Ctrl+A = key('a','KeyA',65,2)
await send('Input.insertText', { text: '查询内容' })        // 真实插入字符
await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 })
await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 })
```

必守三条：

1. **换查询前 Ctrl+A 再 `insertText`**：否则新旧文本拼接，形态/关键词判定会失败。
2. **naive-ui 组件必须真实鼠标事件**（取 `getBoundingClientRect()` 中心）：`element.click()` 对下拉/开关无效；下拉选项展开后才存在于 `.n-base-select-option`。
3. **进插件面板（第三方面板 / Shadow DOM）前开焦点仿真**：`send('Emulation.setFocusEmulationEnabled', { enabled: true })`；否则 `focus()` 不落地、键盘事件打到 DIV 上，面板键盘处理永不触发。
4. `Runtime.evaluate` 返回 `{}` 或空而期望字符串，通常是 IPC 调用静默失败：打印完整响应（含 `exceptionDetails`）重试一次，不要当作「行为没发生」。

---

## 4. 断言选择器

| 目标 | 选择器 |
|---|---|
| 结果条数 | `.result-item`（面板接管时应为 0） |
| 结果标题 / 选中态 | `.item-title` / `.result-item.selected` |
| 搜索框值与容器 | `.search-bar-wrapper input` |
| Footer | `.footer` |
| 插件面板宿主 | `.plugin-panel-host`（沉浸式另有 `--immersive` 修饰类） |
| 智能目标（路径/网址）面板 | `.smart-target-panel` / `.st-title` / `.st-subtitle` / `.st-actions button` |
| 计算器面板 | `.calculator-panel` |
| naive-ui 下拉选项 | `.n-base-select-option`（展开后） |
| 设置页子页签 | `textContent.trim() === '<页签名>'` 的 `SPAN.n-tabs-tab__label` 叶元素（`.n-tab` 不可靠） |
| 错误通知 | `.n-notification` / `.n-message` |

---

## 5. 验证配方

### 5.1 搜索窗口唤出 / 主页 / 清空

- OS 全局热键（如 Alt+Space）**CDP 打不到**。后端 `show_window` 事件（`bootstrap.rs` 的 show 回调与托盘双击同契约）可从页内发射，足以驱动前端 `onShowWindow` 全部逻辑：
  ```js
  await evaluate(`window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event: 'show_window', payload: {} })`)
  ```
- **清空输入框不能用合成 Backspace**（WebView2 实测不触发删除）。用原生 setter + `input` 事件驱动 Vue v-model（走真实 `doQuery('')` 路径）：
  ```js
  const input = document.querySelector('.search-bar-wrapper input')
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, '')
  input.dispatchEvent(new Event('input', { bubbles: true }))
  ```

### 5.2 面板接管类特性 + OS 副作用

面板内按 Enter/点动作执行成功后**窗口会被隐藏**：下一次输入前必须重新 `plugin:window|show`。

- **资源管理器取证**（只读，**绝不要 `.Quit()`**——Win11 单进程多标签，关一个会连带关掉用户其它窗口）：
  ```powershell
  (New-Object -ComObject Shell.Application).Windows() | ForEach-Object { $_.LocationURL }
  ```
- **浏览器导航取证**：只看进程存在不可靠，用**窗口标题**判断是否真的导航（打开可辨识 URL，如 `https://example.com` → 标题变 `Example Domain`）：
  ```powershell
  Get-Process msedge,chrome,vivaldi -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle } | Select-Object ProcessName,MainWindowTitle
  ```
- **裸域名不导航**（实测）：`ShellExecuteW(open, "example.com")` 返回成功但浏览器不导航；`https://example.com` 正常。此类特性必须标题法验证，不能凭返回值判定成功。

### 5.3 实时切换配置与启停（不经设置页 UI）

直接调宿主 IPC（与设置页同一命令），可顺带验证 `config-changed` → 前端刷新链路：

```js
const s = await evaluate(`window.__TAURI_INTERNALS__.invoke('config_get_settings', { componentId: 'window-behavior-config' })`)
await evaluate(`window.__TAURI_INTERNALS__.invoke('config_apply_settings', { componentId: 'window-behavior-config', settings: ${JSON.stringify({ ...s, is_show_home_on_empty_query: true })} })`)
await evaluate(`window.__TAURI_INTERNALS__.invoke('config_set_enabled', { componentId, enabled })`)
await evaluate(`window.__TAURI_INTERNALS__.invoke('plugin_set_enabled', { pluginId, enabled })`)
```

---

## 6. 收尾清理

```bash
# 1) 停后台服务/进程
# 2) 看残留
tasklist | grep -iE "zerolaunch|vite"
# 3) 精确杀：app 按镜像名（安全）；vite(node) 按 PID
taskkill /F /IM zerolaunch-rs.exe
L=$(netstat -ano | grep LISTENING | grep ":12345" | awk '{print $5}' | head -1); [ -n "$L" ] && taskkill /F /PID "$L"
# 4) 校验：无进程 + 9222/12345 无 LISTEN
```

两条禁令：

- **禁止** `taskkill /IM bun.exe|node.exe`：当前会话自身的 JS 运行时与工具链就是 bun/node 进程，按镜像名杀会连带打断本会话（含 eval 内核）。只按 PID 杀已确认归属的进程。
- **禁止** MSYS 下写 `taskkill //F //IM ...`：转义后报「无效参数」，用 `/F /IM`。

另注：停掉后台服务后**子进程可能仍在运行**（app / 插件 / vite），必须再按上面第 2–4 步确认一次，不要留实例给用户自己关。若用户正在使用该实例，则保留并在汇报里说明。

---

## 7. 其他已知陷阱

- **配置位置**：标准模式 `%USERPROFILE%\.ZeroLaunch-rs\config\zerolaunch_config.json`（便携模式改为可执行文件同目录）。`%APPDATA%\ZeroLaunch-rs\zerolaunch_config.json` 可能是**旧版拆分键名的残留**，当前版本不加载它，别被误导；组件键名用 `*-config` 形式（如 `general-config`、`appearance-config`）。
- **msys 路径**：Git Bash 下 `CARGO_TARGET_DIR=/c/...` 会被拼成 `C:\c\...`；隔离构建目录时用完整 Windows 路径。
- **naive-ui tabs 布局**：`.n-tab-pane` 直接挂在 `.n-tabs` 下（无 `.n-tabs-pane-wrapper` 中间层）；组件级 `overflow: hidden` 与外层同特异性规则冲突时按源顺序覆盖，内层滚动会失效——修复靠提高特异性 + `flex: 1; min-height: 0`。
- **截图目视复核**：`Page.captureScreenshot` 落盘后自行查看；注意 msys 路径写法。
- **日志**：宿主 `%USERPROFILE%\.ZeroLaunch-rs\logs\`；判定错误时 grep `ERROR|panic`。
