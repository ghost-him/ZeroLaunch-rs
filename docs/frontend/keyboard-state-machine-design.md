# 按键监听状态机架构设计（草案）

> 状态：设计草案，尚未实现。本文档记录"事二"（按键监听状态机）的背景、需求与设计，供实现前评审。

## 1. 背景

### 1.1 现状

当前按键监听由宿主统一分发：

```
document keydown
  → useKeyboardRouter（按 sessionMode 分发）
    → searchHandler / inlineParamHandler / paramPanelHandler
    → inlinePluginHandler（仅处理 Escape 与 Enter）
    → fullPagePluginHandler（仅处理 Escape，其余按键交给插件面板自绘接管）
```

- 默认搜索面板、行内参数面板、参数面板的按键逻辑由宿主写死
- 行内插件面板（InlinePlugin）的按键逻辑也是宿主写死的：Escape 退出、Enter 触发 `confirmPluginAction`
- 全页面插件（FullPagePlugin）通过"插件自绘接管窗口"变相获得按键权，但没有显式的按键契约

### 1.2 问题

1. **行内插件没有自定义按键能力**。翻译插件需要"Enter 触发查询"语义，只能通过宿主写死的 Enter 处理 + `PanelQueryTrigger::OnEnter` 表达，无法自定义更复杂的按键行为（如 Ctrl+Enter 复制、Tab 切换引擎）。
2. **按键逻辑分散且不可扩展**。宿主 handler 随插件需求膨胀（每个新按键需求都要改宿主代码）。
3. **缺少显式的"监听权移交"模型**。面板切换时按键权的交接是隐式的（依赖 sessionMode 分发），插件无法表达"我要接管所有按键"或"返回默认面板"的意图。

### 1.3 触发背景

`PanelSubmitBehavior`（现 `PanelQueryTrigger`）语义修正过程中，明确了：查询触发策略只属于行内插件模式；而更一般的"面板按键行为"应当由面板自身定义。这引出了按键监听状态机的设计。

## 2. 需求

1. **按键监听是状态机**：每个"面板"是状态机的节点，当前激活的面板持有按键监听权；状态转换（面板切换）时监听权随之移交。
2. **每个面板自己定义按键处理逻辑**：默认搜索列表、各插件面板、插件内部子面板各自声明自己对按键的反应。
3. **插件面板有完全按键权限**：激活期间所有按键（除系统保留键）均由插件面板解释，包括 Enter、方向键、字母数字键、组合键。
4. **插件的跳转路径受限**：插件不感知其他插件存在，只能：
   - 返回默认面板；
   - 跳转到自己定义的其他内部面板（同一插件内的子面板）。
5. **默认面板的查询策略**：默认面板（搜索列表）始终自动查询（`PanelQueryTrigger::OnInput` 语义）；`PanelQueryTrigger` 仅用于行内插件模式（已由事一实现）。
6. **兼容现状**：默认面板、行内参数、参数面板的现有按键行为保持。

## 3. 概念设计

### 3.1 核心模型

```
                    ┌─────────────────────────────┐
                    │        按键监听状态机        │
                    │   （宿主持有当前激活面板）    │
                    └──────────────┬──────────────┘
                                   │ 监听权移交
              ┌────────────────────┼────────────────────┐
              ▼                    ▼                    ▼
    ┌─────────────────┐   ┌───────────────┐   ┌──────────────────┐
    │   默认面板       │   │  插件面板 A    │   │ 插件 A 内部子面板 │
    │（搜索列表）      │   │ （翻译/计算器）│   │（A 自定义）       │
    │ 宿主实现按键逻辑 │   │ 插件实现按键   │   │ 插件实现按键      │
    │ 始终自动查询    │   │ 完全权限       │   │ 完全权限          │
    └─────────────────┘   └───────────────┘   └──────────────────┘
              ▲                    │
              └──── 返回默认面板 ───┘
```

- **面板（Panel）**：状态机节点，拥有按键处理逻辑与展示区域。默认面板、插件面板、插件内部子面板均为面板。
- **监听权（Key Focus）**：任意时刻仅一个面板持有，keydown 事件只交给它。
- **切换（Transition）**：面板通过明确的动作（返回默认 / 跳转内部面板）或宿主路由（查询路由到插件）触发切换。

### 3.2 与现有概念的关系

| 现有概念 | 新模型中的位置 |
|---|---|
| `sessionMode`（search/inline_param/param_panel/inline_plugin/full_page_plugin） | 状态机初始节点的派生：宿主面板 = 默认面板 + 参数面板等宿主面板 |
| `useKeyboardRouter` | 状态机的分发骨架（保留），分发目标从"写死 handler"变为"当前面板的按键处理器" |
| `PanelQueryTrigger`（OnInput/OnEnter） | 行内插件模式下默认面板查询策略的声明，属于插件面板的"默认按键行为"子集 |
| `PluginPanelHost` / 插件自绘 | 插件面板的展示层；按键逻辑独立于展示，由插件声明 |
| `bridge_query` / `route_query` | 面板切换的触发源之一（查询路由到插件 → 进入该插件面板） |

### 3.3 按键处理器的形态

插件面板的按键处理器不应依赖前端代码（第三方插件不是前端代码），因此采用**声明式契约 + 宿主解释执行**：

- 插件在响应中声明按键映射：`key: "Enter"` → `action: "trigger_query" | "execute_action" | "go_back" | ...`
- 宿主把按键事件翻译为动作并执行宿主能力（发起查询、执行动作、切换面板）
- 复杂逻辑（需要插件业务状态参与）通过 `execute_config_action` 式的能力调用回插件

全页面插件（自绘接管）保持现有"窗口接管"模式，作为插件面板的特例（按键权 = 物理接管）。

## 4. 详细设计

### 4.1 状态定义

```
PanelState =
  | HostPanel(HostPanelId)          // 宿主面板：default_search | inline_param | param_panel
  | PluginPanel { plugin_id, panel_id }   // 插件面板（含内部子面板，panel_id 默认 "main"）
```

- `HostPanelId::DefaultSearch` 为初始状态，始终存在
- `PluginPanel.panel_id` 允许插件定义子面板（如 `"main"`、`"settings"`），跳转仅限同一 `plugin_id` 内

### 4.2 状态转换

| 转换 | 触发 | 说明 |
|---|---|---|
| `HostPanel(DefaultSearch) → PluginPanel` | `bridge_query` 路由到插件（现有 `route_query`） | 查询响应为 CustomPanel 时进入 |
| `PluginPanel → HostPanel(DefaultSearch)` | 插件动作 `go_back` / 用户 Escape（若插件未声明） / **输入文本不再匹配插件触发词（强制）** | Escape 优先级需插件声明（`claim_escape` 语义） |
| `PluginPanel{A, p1} → PluginPanel{A, p2}` | 插件动作 `goto_panel(p2)` | 仅限同插件内部 |
| 任意 → `HostPanel(...)` | 会话重置（窗口隐藏/关闭） | 现有 `reset_session` |

### 4.2.1 InlinePlugin 强制退出规则（宿主强制）

行内插件（`InlinePlugin`）有**唯一的强制退出路径**，宿主强制执行、插件不可拦截：

- **输入文本回退到插件触发词之外**（如从 `fy hello` 回退到 `fy`）时，下一次输入查询（`confirm=false`）由 `parse_trigger` 判定不再命中触发词，路由回落默认搜索，前端据响应退出面板。
- 该退出方式的机制基础：**输入变化始终触发非确认查询**（承担路由职责），手动模式下翻译动作由 Enter 触发——面板无动作（ready/失败）时 Enter 发起确认查询（`confirm=true`，翻译或失败后重试），面板已有动作（翻译成功）时 Enter 直接执行默认动作（复制译文）；输入查询与确认查询在后端以 `Query.confirm` 区分。
- 语义保证：手动模式下用户输入任意文本，面板要么显示 ready 预览（仍匹配触发词），要么退出面板回到默认搜索（不再匹配）——不存在"文本已不属于插件但界面仍停留"的状态。
- **退出防抖豁免（退出优先于防抖）**：宿主在进入面板时记录插件**全部触发词**（随 `panel-interaction` 事件推送）。输入时先判定是否退出：无空格，或首词不在触发词集合 → 判定退出 → 立即查询（不受插件防抖延迟）；否则按插件配置防抖。退出判定独立于防抖配置，如 live 模式从 `fy hello` 回退到 `fy` 即时退出；`fy hello → tr hello`（`tr` 同为翻译插件触发词）则正常防抖。

### 4.3 按键分发

```
keydown
  → 系统保留键（Alt+Space 等）直接放行
  → 当前 PanelState 的按键处理器
      ├─ HostPanel：现有宿主 handler（searchHandler 等），行为保持
      └─ PluginPanel：插件声明式按键映射 → 宿主翻译为动作
```

### 4.4 插件 API 契约（终态，2026-08-05 定稿）

```rust
/// 插件面板按键绑定 —— 声明式按键契约的最小单元。
pub struct PanelKeyBinding {
    pub key: String,        // "Enter" / "Ctrl+Enter" / "Escape" / "a"
    pub action: PanelKeyAction,
}

pub enum PanelKeyAction {
    /// 确认当前面板状态（Enter 标准语义，宿主 confirmQuery 三分支）：
    /// 有可执行动作时执行默认动作，否则发起确认查询。
    Confirm,
    /// 执行面板的默认动作或指定动作。
    ExecuteAction { action_id: Option<String> },
    /// 返回默认面板。
    GoBack,
    /// 跳转到同一插件内的子面板。
    GotoPanel { panel_id: String },
    /// 触发插件自定义动作（经 host 调用插件能力）。
    Custom { action: String, args: serde_json::Value },
}
```

- 按键契约挂在 `PanelInteraction.bindings: Vec<PanelKeyBinding>`（随 session-state 事件下发；`PanelKeyMap` 包装结构与 `claim_all` 已删除——**声明即接管，无宿主兜底**：命中绑定由宿主解释执行，未声明的键一律放行交还浏览器/输入框，插件必须声明全部所需按键）
- `PanelKeyAction` 为 internally-tagged 枚举（`serde tag = "kind"`，逐变体 camelCase rename）

### 4.5 与 `PanelQueryTrigger` 的关系

- `PanelQueryTrigger` 决定**查询触发时机**（onInput 自动 / onEnter 手动），与按键契约正交；两种模式均须声明自己的按键绑定
- `OnEnter` ≈ `bindings = [Enter → Confirm]`（宿主解释：动作列表非空则执行默认动作，否则发起确认查询；失败面板可重试，成功面板 Enter 即复制译文）
- `OnInput` ≈ `bindings = [Enter → Confirm, Escape → GoBack]`（翻译由输入防抖自动触发；Enter 仅在有结果时执行复制、失败时重试；宿主 confirmQuery 对在途查询做防重，不会重复触发 LLM）
- 过渡策略已终止：`PanelQueryTrigger` 不再承担按键语义，全部面板按键经声明式 bindings 表达

### 4.6 实现阶段划分

| 阶段 | 内容 | 依赖 |
|---|---|---|
| 1（已完成） | `PanelQueryTrigger` 语义修正（OnInput/OnEnter + 事件推送） | — |
| 2（已完成） | 统一会话重构：session 分发 + 声明式按键绑定（bindings）随 session-state 下发 | 阶段 1 |
| 3 | 插件内部子面板支持：`goto_panel` + 面板状态生命周期（进入/退出回调） | 阶段 2 |

## 5. 待决问题

1. 全页面插件是否统一纳入 `PanelState`（作为"物理接管"特例），还是保持独立路径？
2. 按键契约的返回时机：随 `CustomPanel` 响应内嵌，还是独立接口（类似 `interaction_policy`）？（已决：`interaction_policy` 随 session-state 事件下发）
3. ~~`claim_all` 默认值~~（已决：删除 `claim_all`，声明即接管、未声明即放行，宿主不做任何兜底）
4. 第三方插件按键映射的校验：未知按键格式、未知动作的降级策略（忽略 + 日志）。
