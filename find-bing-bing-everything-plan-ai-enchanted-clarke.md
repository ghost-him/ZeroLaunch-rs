# ZeroLaunch-rs 参数系统与插件触发系统 — 完整重构计划

## Context（为什么需要此变更）

旧系统的参数系统支持 `{}` 用户自定义参数、`{clip}`/`{hwnd}`/`{selection}` 系统参数、转义空格等功能。新系统已搭建好骨架（SDK 层的 `template_parser`、`default_resolver`、`ParameterSnapshot` 已完成），但：

1. **BUG**：`CommandExecutor` 和 `UrlExecutor` 不调用 `resolve_parameters()`，直接将含 `{}` 的模板传给 shell
2. **缺失**：无行内参数输入模式（精确匹配+空格→输入参数）
3. **缺失**：无参数面板模式（Enter→弹出参数填写面板）
4. **缺失**：无全页面插件模式（插件接管整个窗口）
5. **缺失**：候选项没有独立的触发关键词字段（当前名称即触发词）
6. **缺失**：前端无键盘事件路由器，无法按模式分发按键

本次重构的目标是在功能性上完全恢复旧系统的参数能力，并扩展支持新的插件触发架构。

---

## 一、后端：SessionMode 扩展

**文件**：`src-tauri/src/plugin_system/session_router.rs`

### 1.1 新的 SessionMode 枚举

替换当前的 `SessionMode { None, Plugin(String), Search }` 为：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    /// 空闲状态
    None,
    /// 普通搜索模式
    Search,
    /// 行内参数输入模式：精确匹配触发词+空格后，用户在搜索栏内直接输入参数
    InlineParam {
        candidate_id: CandidateId,
        trigger_keyword: String,
    },
    /// 参数面板模式：用户按 Enter 后弹出参数面板，逐个填写
    ParamPanel {
        candidate_id: CandidateId,
    },
    /// 行内插件模式：插件保留搜索栏，控制结果区域（如计算器）
    InlinePlugin(String),
    /// 全页面插件模式：插件接管整个窗口，管理所有按键
    FullPagePlugin(String),
}

impl SessionMode {
    /// 返回前端可识别的模式字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionMode::None => "none",
            SessionMode::Search => "search",
            SessionMode::InlineParam { .. } => "inline_param",
            SessionMode::ParamPanel { .. } => "param_panel",
            SessionMode::InlinePlugin(_) => "inline_plugin",
            SessionMode::FullPagePlugin(_) => "full_page_plugin",
        }
    }

    /// 是否为插件模式（行内或全页面）
    pub fn is_plugin_mode(&self) -> bool {
        matches!(self, SessionMode::InlinePlugin(_) | SessionMode::FullPagePlugin(_))
    }
}
```

### 1.2 状态转换规则（状态机）

```
┌─────────────────────────────────────────────────────────────────┐
│                        状态转换图                                 │
├──────────────┬────────────────────┬─────────────────────────────┤
│ From         │ To                 │ Trigger                     │
├──────────────┼────────────────────┼─────────────────────────────┤
│ None         │ Search             │ 用户输入任意字符             │
│ Search       │ InlineParam        │ Space + 查询==选中项触发词   │
│              │                    │ + 该项有 {} 参数             │
│ Search       │ InlinePlugin       │ 插件触发匹配 + keep_search  │
│              │                    │ _bar=true                   │
│ Search       │ FullPagePlugin     │ 插件触发匹配 + keep_search  │
│              │                    │ _bar=false                  │
│ Search       │ ParamPanel         │ Enter + 选中项有必填 {} 参数│
│ Search       │ None               │ 执行（无参数项）→隐藏窗口    │
│ InlineParam  │ None               │ 执行成功 → 隐藏窗口          │
│ InlineParam  │ Search             │ Escape / 删除所有参数文本    │
│ ParamPanel   │ None               │ 执行成功 → 隐藏窗口          │
│ ParamPanel   │ Search             │ Escape                      │
│ InlinePlugin │ Search             │ Escape / 清空查询            │
│ FullPagePlugin│ None              │ 插件发送退出信号             │
│ Any          │ None               │ 窗口隐藏 / bridge_reset     │
└──────────────┴────────────────────┴─────────────────────────────┘
```

### 1.3 需要更新的模式匹配

`session_router.rs` 中所有 `match &*self.current_mode.read()` 的位置都需要扩展覆盖新的变体。关键位置：
- `route_query()` 中的模式判断
- `route_confirm()` 中的分发逻辑
- `reset_session()` 中的清理逻辑

---

## 二、后端：触发关键词（Trigger Keywords）

### 2.1 数据模型变更

**`src-tauri/src/plugin/data_source/command_source.rs`** — `CommandEntry` 新增字段：
```rust
/// 触发关键词，逗号分隔。为空时使用 name 作为默认触发词
#[serde(rename = "triggerKeywords", default)]
pub trigger_keywords: String,
```

**`src-tauri/src/plugin/data_source/url_source.rs`** — `UrlEntry` 新增字段：
```rust
/// 触发关键词，逗号分隔。为空时使用 name 作为默认触发词
#[serde(rename = "triggerKeywords", default)]
pub trigger_keywords: String,
```

### 2.2 SearchCandidate 扩展

**`src-tauri/src/plugin_system/types.rs`** — `SearchCandidate` 新增字段：
```rust
/// 触发关键词列表，用于行内模式的精确匹配
pub trigger_keywords: Vec<String>,
```

### 2.3 DataSource 构建逻辑

在 `command_source.rs` 和 `url_source.rs` 的 `fetch_candidates()` 中：
```rust
// 解析触发关键词：逗号分隔，去空白，过滤空值
let trigger_keywords: Vec<String> = if entry.trigger_keywords.is_empty() {
    // 默认使用名称的小写形式作为触发词
    vec![entry.name.to_lowercase()]
} else {
    entry.trigger_keywords
        .split(',')
        .map(|s| s.trim().to_lowercase().to_string())
        .filter(|s| !s.is_empty())
        .collect()
};
```

其他 DataSource（`app_source.rs`, `program_source.rs`, `bookmark_source.rs`）：
- `trigger_keywords: vec![]`（路径/应用程序通常不需要触发词，除非将来扩展）

### 2.4 配置 Schema 变更

在 `command_source.rs` 和 `url_source.rs` 的 Configurable 实现中，为 object_items 数组的 schema 添加：
```rust
SchemaBuilder::text("triggerKeywords", "触发关键词")
    .description("逗号分隔的触发词列表。输入触发词+空格可进入参数模式。为空时默认使用名称。")
    .default("")
    .build_field(),
```

---

## 三、后端：搜索结果中的参数元数据

### 3.1 扩展 ListItem

**`src-tauri/src/plugin_system/types.rs`** — `ListItem` 结构体新增：
```rust
/// 用户参数 {} 的数量
#[serde(rename = "userArgCount")]
pub user_arg_count: usize,

/// 是否包含系统参数（{clip}, {hwnd}, {selection}）
#[serde(rename = "hasSystemParams")]
pub has_system_params: bool,

/// 触发关键词列表
#[serde(rename = "triggerKeywords")]
pub trigger_keywords: Vec<String>,
```

### 3.2 TemplateParser 工具方法

确保 `src-tauri/src/sdk/parameter/template_parser.rs` 暴露以下方法（可能已存在）：
```rust
impl TemplateParser {
    /// 统计模板中 {} 用户参数的数量
    pub fn count_user_args(template: &str) -> usize;
    
    /// 检查模板是否包含系统参数
    pub fn has_system_params(template: &str) -> bool;
}
```

### 3.3 在 route_query 中填充元数据

`session_router.rs` 中构建 `ListItem` 时：
```rust
use crate::sdk::parameter::template_parser::TemplateParser;

let template_str = candidate.target.payload(); // 获取模板字符串
let user_arg_count = TemplateParser::count_user_args(template_str);
let has_system_params = TemplateParser::has_system_params(template_str);
let trigger_keywords = candidate.trigger_keywords.clone();
```

### 3.4 确保 ExecutionTarget 有 payload() 方法

**`types.rs`** 中为 `ExecutionTarget` 添加：
```rust
impl ExecutionTarget {
    /// 获取目标的模板字符串
    pub fn payload(&self) -> &str {
        match self {
            ExecutionTarget::Path(s) => s,
            ExecutionTarget::App(s) => s,
            ExecutionTarget::File(s) => s,
            ExecutionTarget::Url(s) => s,
            ExecutionTarget::Command(s) => s,
            ExecutionTarget::BuiltinCommand(s) => s,
        }
    }
}
```

---

## 四、后端：修复 Executor 参数解析（优先级最高）

### 4.1 CommandExecutor

**`src-tauri/src/plugin/executor/command_executor.rs`**

修改 `execute()` 方法：
```rust
async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
    let template = match &ctx.target {
        ExecutionTarget::Command(cmd) => cmd.as_str(),
        _ => return Err(ExecutionError::NotFound(ctx.target.target_type())),
    };

    // 解析模板，替换参数占位符
    let resolved_command = self.plugin_handle
        .resolve_parameters(template, &ctx.user_args, &ctx.parameter_snapshot)
        .await
        .map_err(|e| ExecutionError::Failed(format!("命令参数解析失败: {}", e)))?;

    match action_id {
        "execute" => {
            self.plugin_handle.shell_execute_command(&resolved_command).await
                .map_err(|e| ExecutionError::Failed(e.to_string()))
        }
        _ => Err(ExecutionError::UnsupportedAction(
            ctx.target.target_type(), action_id.to_string()
        )),
    }
}
```

### 4.2 UrlExecutor

**`src-tauri/src/plugin/executor/url_executor.rs`**

修改 `execute()` 方法：
```rust
async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError> {
    let url_template = match &ctx.target {
        ExecutionTarget::Url(url) => url.as_str(),
        _ => return Err(ExecutionError::NotFound(ctx.target.target_type())),
    };

    // 解析模板，替换参数占位符
    let resolved_url = self.plugin_handle
        .resolve_parameters(url_template, &ctx.user_args, &ctx.parameter_snapshot)
        .await
        .map_err(|e| ExecutionError::Failed(format!("URL参数解析失败: {}", e)))?;

    match action_id {
        "execute" => {
            self.plugin_handle.shell_open(OpenTarget::Url(resolved_url)).await
                .map_err(|e| ExecutionError::Failed(e.to_string()))
        }
        _ => Err(ExecutionError::UnsupportedAction(
            ctx.target.target_type(), action_id.to_string()
        )),
    }
}
```

### 4.3 PathExecutor 注意

`PathExecutor` 通常不需要参数解析（路径中不含 `{}`），但如果用户自定义 Path 类型带参数，也应加上解析。建议在所有执行器中统一添加 `resolve_parameters()` 调用。

---

## 五、后端：行内模式与参数面板的会话处理

### 5.1 SessionRouter 新增方法

```rust
/// 进入行内参数输入模式
/// 前端检测到精确匹配+空格后调用
pub fn enter_inline_param_mode(&self, candidate_id: CandidateId, trigger_keyword: String) {
    *self.current_mode.write() = SessionMode::InlineParam { candidate_id, trigger_keyword };
}

/// 进入参数面板模式
/// 前端按 Enter 且候选项有必填参数时调用
pub fn enter_param_panel_mode(&self, candidate_id: CandidateId) {
    *self.current_mode.write() = SessionMode::ParamPanel { candidate_id };
}

/// 退出当前非搜索模式，回到搜索模式
pub fn exit_current_mode(&self) {
    let current = self.current_mode.read().clone();
    match current {
        SessionMode::InlineParam { .. } | SessionMode::ParamPanel { .. } => {
            *self.current_mode.write() = SessionMode::Search;
        }
        SessionMode::InlinePlugin(_) => {
            *self.current_mode.write() = SessionMode::Search;
        }
        SessionMode::FullPagePlugin(_) => {
            *self.current_mode.write() = SessionMode::None;
        }
        _ => {}
    }
}
```

### 5.2 提取公共执行方法

```rust
/// 统一的候选项执行逻辑，被 InlineParam/ParamPanel/Search 模式共用
async fn execute_candidate(
    &self,
    candidate_id: CandidateId,
    action_id: &str,
    user_args: Vec<String>,
    query_text: &str,
) -> Result<(), String> {
    // 1. 查找候选项
    let candidate = self.cached_candidates.read()
        .get_by_id(candidate_id)
        .ok_or("候选项不存在")?
        .clone();

    // 2. 构建执行上下文
    let snapshot = self.parameter_snapshot.lock().clone();
    let exec_ctx = ExecutionContext {
        target: candidate.target.clone(),
        display_name: candidate.name.clone(),
        user_args,
        parameter_snapshot: snapshot,
    };

    // 3. 记录使用（学习反馈）
    if let Some(pipeline) = self.search_pipeline.read().as_ref() {
        pipeline.record(candidate_id, &candidate, query_text);
    }

    // 4. 解析并执行
    let executor = self.executor_registry.read()
        .resolve(&exec_ctx, action_id)
        .map_err(|e| e.to_string())?;
    
    match executor.execute(&exec_ctx, action_id).await {
        Ok(()) => Ok(()),
        Err(ExecutionError::ActivationFailed { fallback_action }) => {
            // 激活失败时的回退逻辑
            let fallback_executor = self.executor_registry.read()
                .resolve(&exec_ctx, &fallback_action)
                .map_err(|e| e.to_string())?;
            fallback_executor.execute(&exec_ctx, &fallback_action).await
                .map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}
```

### 5.3 更新 route_confirm

```rust
pub async fn route_confirm(&self, trace_id: &str, action_id: &str, payload: &Value) -> Result<(), String> {
    let mode = self.current_mode.read().clone();
    
    match mode {
        SessionMode::InlinePlugin(ref pid) | SessionMode::FullPagePlugin(ref pid) => {
            // 插件模式：委托给插件服务
            self.plugin_service.execute_action(pid, action_id, payload).await
                .map_err(|e| e.to_string())
        }
        SessionMode::InlineParam { candidate_id, .. } => {
            let user_args = Self::extract_user_args(payload);
            let query_text = payload.get("queryText").and_then(|v| v.as_str()).unwrap_or("");
            self.execute_candidate(candidate_id, action_id, user_args, query_text).await
        }
        SessionMode::ParamPanel { candidate_id } => {
            let user_args = Self::extract_user_args(payload);
            let query_text = payload.get("queryText").and_then(|v| v.as_str()).unwrap_or("");
            self.execute_candidate(candidate_id, action_id, user_args, query_text).await
        }
        SessionMode::Search => {
            let candidate_id = payload.get("candidate_id")
                .and_then(|v| v.as_f64())
                .map(|v| v as u64)
                .ok_or("缺少 candidate_id")?;
            let user_args = Self::extract_user_args(payload);
            let query_text = payload.get("queryText").and_then(|v| v.as_str()).unwrap_or("");
            self.execute_candidate(candidate_id, action_id, user_args, query_text).await
        }
        SessionMode::None => Err("当前无活动会话".to_string()),
    }
}

/// 从 payload 中提取 user_args
fn extract_user_args(payload: &Value) -> Vec<String> {
    payload.get("user_args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}
```

### 5.4 新增 Bridge 命令

**`src-tauri/src/commands/bridge.rs`** 新增：

```rust
/// 进入行内参数模式
#[tauri::command]
pub async fn bridge_enter_inline_mode(
    state: tauri::State<'_, Arc<AppState>>,
    candidate_id: u64,
    trigger_keyword: String,
) -> Result<(), String> {
    state.session_router().enter_inline_param_mode(candidate_id, trigger_keyword);
    Ok(())
}

/// 进入参数面板模式
#[tauri::command]
pub async fn bridge_enter_param_panel(
    state: tauri::State<'_, Arc<AppState>>,
    candidate_id: u64,
) -> Result<(), String> {
    state.session_router().enter_param_panel_mode(candidate_id);
    Ok(())
}

/// 退出当前模式（回到 Search 或 None）
#[tauri::command]
pub async fn bridge_exit_mode(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.session_router().exit_current_mode();
    Ok(())
}
```

别忘了在 `lib.rs` 的 `invoke_handler` 中注册这三个命令。

---

## 六、后端：全页面插件模式

### 6.1 QueryResponse 扩展

修改 `CustomPanel` 变体，确保有 `keep_search_bar` 字段区分行内与全页面：
```rust
QueryResponse::CustomPanel {
    panel_type: String,
    data: Value,
    actions: Vec<ResultAction>,
    keep_search_bar: bool,       // true=行内插件, false=全页面插件
    capture_all_keys: bool,      // 新增：全页面插件是否需要捕获所有键盘事件
}
```

### 6.2 键盘事件类型

**`src-tauri/src/plugin_system/types.rs`** 新增：
```rust
/// 前端转发的键盘事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginKeyEvent {
    pub key: String,        // e.g., "Enter", "a", "Escape"
    pub code: String,       // e.g., "KeyA", "Enter"
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
}

/// 插件对键盘事件的响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginKeyEventResponse {
    /// 插件是否处理了该事件
    pub handled: bool,
    /// 是否请求退出全页面模式
    pub exit_plugin: bool,
    /// 可选的 UI 更新数据（插件自定义）
    pub panel_update: Option<Value>,
}
```

### 6.3 Plugin trait 扩展

**`src-tauri/src/plugin_system/types.rs`** — `Plugin` trait 新增默认方法：
```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    // ... 已有方法 ...

    /// 处理键盘事件（仅全页面模式使用）
    /// 默认实现不处理任何事件
    async fn handle_key_event(
        &self,
        _ctx: &PluginContext,
        _event: &PluginKeyEvent,
    ) -> Result<PluginKeyEventResponse, PluginError> {
        Ok(PluginKeyEventResponse {
            handled: false,
            exit_plugin: false,
            panel_update: None,
        })
    }
}
```

### 6.4 SessionRouter 键盘事件路由

```rust
/// 将键盘事件路由给全页面插件
pub async fn route_key_event(&self, event: &PluginKeyEvent) -> Result<PluginKeyEventResponse, String> {
    let mode = self.current_mode.read().clone();
    match mode {
        SessionMode::FullPagePlugin(ref plugin_id) => {
            let response = self.plugin_service
                .handle_key_event(plugin_id, event)
                .await
                .map_err(|e| e.to_string())?;
            
            // 如果插件请求退出
            if response.exit_plugin {
                *self.current_mode.write() = SessionMode::None;
            }
            
            Ok(response)
        }
        _ => Err("当前不在全页面插件模式".to_string()),
    }
}
```

### 6.5 新增 Bridge 命令

```rust
/// 转发键盘事件给全页面插件
#[tauri::command]
pub async fn bridge_plugin_key_event(
    state: tauri::State<'_, Arc<AppState>>,
    event: PluginKeyEvent,
) -> Result<PluginKeyEventResponse, String> {
    state.session_router().route_key_event(&event).await
}
```

### 6.6 route_query 中的插件模式区分

在 `route_query()` 中，当 `PluginService` 返回匹配时：
```rust
if let Some((plugin_id, response)) = self.plugin_service.query(&ctx, &query).await? {
    match &response {
        QueryResponse::CustomPanel { keep_search_bar, .. } => {
            if *keep_search_bar {
                *self.current_mode.write() = SessionMode::InlinePlugin(plugin_id);
            } else {
                *self.current_mode.write() = SessionMode::FullPagePlugin(plugin_id);
            }
        }
        _ => {
            *self.current_mode.write() = SessionMode::InlinePlugin(plugin_id);
        }
    }
    return Ok(response);
}
```

---

## 七、前端：键盘事件路由器

### 7.1 新建 `src-ui-new/composables/useKeyboardRouter.ts`

这是核心调度器，替换现有的 `useKeyboard.ts`：

```typescript
import { computed, onMounted, onUnmounted } from 'vue'
import { useSearchStore } from '@/stores/search-store'
import { handleSearchModeKey } from './keyboard/searchHandler'
import { handleInlineParamKey } from './keyboard/inlineParamHandler'
import { handleParamPanelKey } from './keyboard/paramPanelHandler'
import { handleInlinePluginKey } from './keyboard/inlinePluginHandler'
import { handleFullPagePluginKey } from './keyboard/fullPagePluginHandler'

export type UIMode = 'none' | 'search' | 'inline_param' | 'param_panel' | 'inline_plugin' | 'full_page_plugin'

export function useKeyboardRouter() {
  const store = useSearchStore()

  // 根据当前状态计算 UI 模式
  const uiMode = computed<UIMode>(() => {
    if (store.inlineParamState) return 'inline_param'
    if (store.paramPanelState) return 'param_panel'
    if (store.sessionMode === 'full_page_plugin') return 'full_page_plugin'
    if (store.sessionMode === 'inline_plugin') return 'inline_plugin'
    if (store.sessionMode === 'search') return 'search'
    return 'none'
  })

  // 全局键盘事件处理器
  function onKeyDown(e: KeyboardEvent) {
    // Alt+Space 始终保留给系统（唤醒/隐藏窗口）
    if (e.altKey && e.code === 'Space') return

    switch (uiMode.value) {
      case 'search':
        handleSearchModeKey(e, store)
        break
      case 'inline_param':
        handleInlineParamKey(e, store)
        break
      case 'param_panel':
        handleParamPanelKey(e, store)
        break
      case 'inline_plugin':
        handleInlinePluginKey(e, store)
        break
      case 'full_page_plugin':
        handleFullPagePluginKey(e, store)
        break
      case 'none':
        // 空闲模式不处理按键
        break
    }
  }

  onMounted(() => document.addEventListener('keydown', onKeyDown))
  onUnmounted(() => document.removeEventListener('keydown', onKeyDown))

  return { uiMode }
}
```

### 7.2 `composables/keyboard/searchHandler.ts`

```typescript
import type { SearchStore } from '@/stores/search-store'

export function handleSearchModeKey(e: KeyboardEvent, store: SearchStore) {
  switch (e.key) {
    case 'ArrowUp':
      e.preventDefault()
      store.selectPrev()
      break
    case 'ArrowDown':
      e.preventDefault()
      store.selectNext()
      break
    case 'Tab':
      e.preventDefault()
      store.cycleAction()
      break
    case 'Enter':
      e.preventDefault()
      store.handleEnterInSearchMode()  // 新方法：检查是否需要进入参数面板
      break
    case ' ':  // 空格键
      // 检查是否应进入行内参数模式
      if (store.tryEnterInlineParamMode()) {
        e.preventDefault()  // 阻止空格写入输入框
      }
      // 否则正常输入空格到搜索栏
      break
    case 'Escape':
      e.preventDefault()
      store.handleEscape()
      break
    case 'Home':
      e.preventDefault()
      store.selectFirst()
      break
    case 'End':
      e.preventDefault()
      store.selectLast()
      break
    default:
      // Ctrl+数字快捷执行
      if (e.ctrlKey && e.key >= '1' && e.key <= '9') {
        e.preventDefault()
        store.executeActionByIndex(parseInt(e.key) - 1)
      }
      break
  }
}
```

### 7.3 `composables/keyboard/inlineParamHandler.ts`

```typescript
export function handleInlineParamKey(e: KeyboardEvent, store: SearchStore) {
  switch (e.key) {
    case 'Enter':
      e.preventDefault()
      store.confirmInlineParam()  // 解析参数并执行
      break
    case 'Escape':
      e.preventDefault()
      store.exitInlineParamMode()  // 回到搜索模式
      break
    case 'Backspace':
      // 如果参数输入为空，退出行内模式
      if (store.inlineParamState?.paramInput === '') {
        e.preventDefault()
        store.exitInlineParamMode()
      }
      // 否则正常删除字符（不阻止默认行为）
      break
    // 其他键正常输入到参数区域
  }
}
```

### 7.4 `composables/keyboard/paramPanelHandler.ts`

```typescript
export function handleParamPanelKey(e: KeyboardEvent, store: SearchStore) {
  switch (e.key) {
    case 'Enter':
      e.preventDefault()
      store.confirmParamPanel()  // 收集所有字段值，执行
      break
    case 'Escape':
      e.preventDefault()
      store.exitParamPanelMode()  // 回到搜索模式
      break
    case 'Tab':
      e.preventDefault()
      if (e.shiftKey) {
        store.paramPanelFocusPrev()
      } else {
        store.paramPanelFocusNext()
      }
      break
    // 其他键正常输入到当前焦点字段
  }
}
```

### 7.5 `composables/keyboard/inlinePluginHandler.ts`

```typescript
export function handleInlinePluginKey(e: KeyboardEvent, store: SearchStore) {
  // 行内插件保留搜索栏，所以大部分键给输入框处理
  switch (e.key) {
    case 'Escape':
      e.preventDefault()
      store.exitPluginMode()
      break
    case 'Enter':
      e.preventDefault()
      store.confirmPluginAction()
      break
    // 其他键不阻止，让搜索栏正常处理输入
  }
}
```

### 7.6 `composables/keyboard/fullPagePluginHandler.ts`

```typescript
import { bridgePluginKeyEvent } from '@/bridge/commands'

export async function handleFullPagePluginKey(e: KeyboardEvent, store: SearchStore) {
  e.preventDefault()  // 全页面模式下阻止所有默认行为

  // 将所有键盘事件转发给后端插件
  const response = await bridgePluginKeyEvent({
    key: e.key,
    code: e.code,
    ctrlKey: e.ctrlKey,
    shiftKey: e.shiftKey,
    altKey: e.altKey,
    metaKey: e.metaKey,
  })

  if (response.exitPlugin) {
    store.exitFullPagePlugin()
  }
  if (response.panelUpdate) {
    store.updatePluginPanel(response.panelUpdate)
  }
}
```

### 7.7 替换现有 useKeyboard

在 `SearchView.vue` 中：
- 移除 `import { useKeyboard } from '@/composables/useKeyboard'`
- 替换为 `import { useKeyboardRouter } from '@/composables/useKeyboardRouter'`
- 调用 `const { uiMode } = useKeyboardRouter()`

---

## 八、前端：行内参数输入模式

### 8.1 Store 状态

**`src-ui-new/stores/search-store.ts`** 新增：

```typescript
// 行内参数模式状态
interface InlineParamState {
  candidateId: number
  triggerKeyword: string    // 已匹配的触发词（显示为前缀）
  paramInput: string        // 用户正在输入的参数文本
  userArgCount: number      // 需要的 {} 参数数量
}

const inlineParamState = ref<InlineParamState | null>(null)
```

### 8.2 触发检测方法

```typescript
/**
 * 尝试进入行内参数模式
 * 当用户按下空格时调用，检查当前查询是否精确匹配选中项的触发词
 * @returns true 如果成功进入行内模式（调用者应阻止空格输入）
 */
function tryEnterInlineParamMode(): boolean {
  const selectedItem = results.value[selectedIndex.value]
  if (!selectedItem) return false
  if (selectedItem.userArgCount === 0) return false

  const currentQuery = query.value.trim().toLowerCase()
  
  // 检查是否精确匹配任一触发词
  const matchedKeyword = selectedItem.triggerKeywords
    .find(kw => kw.toLowerCase() === currentQuery)
  
  if (!matchedKeyword) return false

  // 进入行内参数模式
  inlineParamState.value = {
    candidateId: selectedItem.id,
    triggerKeyword: matchedKeyword,
    paramInput: '',
    userArgCount: selectedItem.userArgCount,
  }
  
  // 通知后端
  bridgeEnterInlineMode(selectedItem.id, matchedKeyword)
  return true
}
```

### 8.3 转义序列解析

```typescript
/**
 * 解析行内参数输入，支持转义：
 * - 未转义空格 = 参数分隔符
 * - \空格 = 字面空格
 * - \\ = 字面反斜杠
 */
function parseInlineArgs(input: string): string[] {
  const args: string[] = []
  let current = ''
  let i = 0

  while (i < input.length) {
    if (input[i] === '\\' && i + 1 < input.length) {
      if (input[i + 1] === ' ') {
        current += ' '   // 转义空格 → 字面空格
        i += 2
      } else if (input[i + 1] === '\\') {
        current += '\\'  // 转义反斜杠 → 字面反斜杠
        i += 2
      } else {
        current += input[i]
        i++
      }
    } else if (input[i] === ' ') {
      if (current.length > 0) {
        args.push(current)
        current = ''
      }
      i++
    } else {
      current += input[i]
      i++
    }
  }
  if (current.length > 0) {
    args.push(current)
  }
  return args
}
```

### 8.4 确认执行

```typescript
/**
 * 确认行内参数输入，解析并执行
 */
async function confirmInlineParam() {
  if (!inlineParamState.value) return
  
  const { candidateId, paramInput, userArgCount } = inlineParamState.value
  const args = parseInlineArgs(paramInput)
  
  // 验证参数数量（至少需要 userArgCount 个参数）
  if (args.length < userArgCount) {
    // 可选：显示提示信息
    console.warn(`需要 ${userArgCount} 个参数，实际输入 ${args.length} 个`)
    return
  }

  await bridgeConfirm({
    candidateId,
    actionId: 'execute',
    queryText: inlineParamState.value.triggerKeyword,
    userArgs: args,
  })

  // 执行成功后清理
  inlineParamState.value = null
  // 隐藏窗口等后续操作...
}
```

### 8.5 SearchBar.vue 变更

搜索栏需要在行内参数模式下呈现不同的 UI：

```vue
<template>
  <div class="search-bar-container">
    <!-- 行内参数模式：显示触发词前缀 + 参数输入 -->
    <template v-if="store.inlineParamState">
      <span class="trigger-prefix">{{ store.inlineParamState.triggerKeyword }}</span>
      <input
        ref="paramInputRef"
        v-model="store.inlineParamState.paramInput"
        class="param-input"
        :placeholder="`输入 ${store.inlineParamState.userArgCount} 个参数（空格分隔，\\ 转义空格）`"
        @input="onParamInput"
      />
    </template>
    <!-- 正常搜索模式 -->
    <template v-else>
      <NInput
        v-model:value="store.query"
        ...existing props...
      />
    </template>
  </div>
</template>
```

---

## 九、前端：参数面板模式

### 9.1 Store 状态

```typescript
interface ParamField {
  index: number        // 第几个 {} 参数
  label: string        // 显示标签："参数 1"、"参数 2"
  value: string        // 用户输入的值
}

interface ParamPanelState {
  candidateId: number
  candidateItem: ListItem    // 完整的候选项信息
  fields: ParamField[]       // 参数字段列表
  focusedFieldIndex: number  // 当前焦点字段索引
}

const paramPanelState = ref<ParamPanelState | null>(null)
```

### 9.2 触发逻辑

```typescript
/**
 * 在搜索模式下处理 Enter 键
 * 如果选中项有必填 {} 参数，进入参数面板；否则直接执行
 */
async function handleEnterInSearchMode() {
  const selectedItem = results.value[selectedIndex.value]
  if (!selectedItem) return

  if (selectedItem.userArgCount > 0) {
    // 有必填参数 → 进入参数面板模式
    enterParamPanelMode(selectedItem)
  } else {
    // 无参数 → 直接执行
    await doConfirm()
  }
}

function enterParamPanelMode(item: ListItem) {
  const fields: ParamField[] = Array.from({ length: item.userArgCount }, (_, i) => ({
    index: i,
    label: `参数 ${i + 1}`,
    value: '',
  }))

  paramPanelState.value = {
    candidateId: item.id,
    candidateItem: item,
    fields,
    focusedFieldIndex: 0,
  }

  // 通知后端
  bridgeEnterParamPanel(item.id)
}
```

### 9.3 新建 `src-ui-new/components/search/ParamPanel.vue`

```vue
<template>
  <div class="param-panel" v-if="store.paramPanelState">
    <!-- 候选项信息 -->
    <div class="param-panel-header">
      <span class="candidate-name">{{ store.paramPanelState.candidateItem.name }}</span>
      <span class="param-hint">请填写参数后按 Enter 执行</span>
    </div>

    <!-- 参数字段列表 -->
    <div class="param-fields">
      <div
        v-for="field in store.paramPanelState.fields"
        :key="field.index"
        class="param-field"
      >
        <label>{{ field.label }}</label>
        <input
          :ref="el => setFieldRef(el, field.index)"
          v-model="field.value"
          :placeholder="`输入第 ${field.index + 1} 个参数`"
          @keydown.tab.prevent="focusNextField"
          @keydown.shift.tab.prevent="focusPrevField"
          @keydown.enter.prevent="store.confirmParamPanel()"
          @keydown.escape.prevent="store.exitParamPanelMode()"
        />
      </div>
    </div>

    <!-- 实时预览（可选） -->
    <div class="param-preview" v-if="previewEnabled">
      <span class="preview-label">预览：</span>
      <code>{{ resolvedPreview }}</code>
    </div>
  </div>
</template>
```

### 9.4 确认执行

```typescript
async function confirmParamPanel() {
  if (!paramPanelState.value) return

  const { candidateId, fields } = paramPanelState.value
  const userArgs = fields.map(f => f.value)

  // 验证所有字段非空
  if (userArgs.some(arg => arg.trim() === '')) {
    // 可选：高亮空字段
    return
  }

  await bridgeConfirm({
    candidateId,
    actionId: 'execute',
    queryText: query.value,
    userArgs,
  })

  // 清理
  paramPanelState.value = null
}
```

### 9.5 实时预览（可选功能）

在前端做简单的模板替换预览（非精确，仅用于用户参考）：
```typescript
const resolvedPreview = computed(() => {
  if (!paramPanelState.value) return ''
  const target = paramPanelState.value.candidateItem.target // 需要后端传递模板
  let result = target
  for (const field of paramPanelState.value.fields) {
    result = result.replace('{}', field.value || '___')
  }
  return result
})
```

> 注意：实时预览需要前端知道模板字符串。可以在 ListItem 中增加一个可选的 `template` 字段，或者通过单独的 bridge 命令获取。

---

## 十、前端：全页面插件模式

### 10.1 工作原理

当 `route_query()` 返回 `QueryResponse::CustomPanel { keep_search_bar: false, ... }` 时：
1. 前端收到响应，检测 `keepSearchBar === false`
2. 设置 `sessionMode = 'full_page_plugin'`
3. 隐藏搜索栏和结果列表
4. 渲染插件的自定义面板（通过 `panel_type` 找到对应的前端插件组件）
5. 所有键盘事件通过 `fullPagePluginHandler` → `bridgePluginKeyEvent()` → 后端插件

### 10.2 SearchView.vue 布局调整

```vue
<template>
  <div class="search-view">
    <!-- 全页面插件模式：隐藏所有默认 UI -->
    <template v-if="uiMode === 'full_page_plugin'">
      <PluginFullPageHost
        :panelType="store.pluginPanelType"
        :panelData="store.pluginPanelData"
        :actions="store.pluginActions"
      />
    </template>

    <!-- 其他模式：显示搜索栏 + 结果 -->
    <template v-else>
      <SearchBar />
      <ParamPanel v-if="uiMode === 'param_panel'" />
      <ResultList v-if="uiMode === 'search' || uiMode === 'inline_param'" />
      <PluginInlineHost
        v-if="uiMode === 'inline_plugin'"
        :panelType="store.pluginPanelType"
        :panelData="store.pluginPanelData"
      />
    </template>
  </div>
</template>
```

### 10.3 退出机制

插件通过 `handle_key_event()` 返回 `{ exit_plugin: true }` 来请求退出。前端收到后：
```typescript
function exitFullPagePlugin() {
  sessionMode.value = 'none'
  pluginPanelData.value = null
  bridgeExitMode()
  // 可选：隐藏窗口 或 回到搜索模式
}
```

---

## 十一、IPC 契约变更

### 11.1 `src-ui-new/bridge/contract.ts` 新增/修改

```typescript
// ListItem 扩展
interface ListItem {
  id: number
  name: string
  icon: string        // base64
  actions: ResultAction[]
  // 新增字段：
  userArgCount: number
  hasSystemParams: boolean
  triggerKeywords: string[]
}

// 新增：进入行内模式的 payload
interface EnterInlineModePayload {
  candidateId: number
  triggerKeyword: string
}

// 新增：进入参数面板的 payload
interface EnterParamPanelPayload {
  candidateId: number
}

// 新增：插件键盘事件
interface PluginKeyEvent {
  key: string
  code: string
  ctrlKey: boolean
  shiftKey: boolean
  altKey: boolean
  metaKey: boolean
}

// 新增：插件键盘事件响应
interface PluginKeyEventResponse {
  handled: boolean
  exitPlugin: boolean
  panelUpdate: any | null
}
```

### 11.2 `src-ui-new/bridge/commands.ts` 新增

```typescript
export async function bridgeEnterInlineMode(candidateId: number, triggerKeyword: string): Promise<void> {
  return invoke('bridge_enter_inline_mode', { candidateId, triggerKeyword })
}

export async function bridgeEnterParamPanel(candidateId: number): Promise<void> {
  return invoke('bridge_enter_param_panel', { candidateId })
}

export async function bridgeExitMode(): Promise<void> {
  return invoke('bridge_exit_mode')
}

export async function bridgePluginKeyEvent(event: PluginKeyEvent): Promise<PluginKeyEventResponse> {
  return invoke('bridge_plugin_key_event', { event })
}
```

---

## 十二、配置变更

### 12.1 CommandSource 的 Configurable schema

在 `command_source.rs` 的 `settings_schema()` 中，为 commands 数组的每个 object_item 新增 `triggerKeywords` 字段：

```rust
FieldDefinition {
    key: "triggerKeywords".into(),
    label: "触发关键词".into(),
    description: Some("逗号分隔的触发词列表。输入触发词+空格进入参数模式。为空时默认使用名称。".into()),
    setting_type: SettingType::Text,
    default: Value::String("".into()),
    ..Default::default()
}
```

### 12.2 UrlSource 的 Configurable schema

同样在 `url_source.rs` 中新增 `triggerKeywords` 字段配置。

### 12.3 向后兼容

- `#[serde(default)]` 确保旧配置文件（无 `triggerKeywords` 字段）能正常加载
- 默认值为空字符串，此时使用名称作为触发词

---

## 实施顺序

| 阶段 | 范围 | 风险 | 依赖 |
|------|------|------|------|
| **Phase 1** | 修复 Executor 调用 resolve_parameters()（第四节） | 低 | 无 |
| **Phase 2** | 后端数据模型扩展：trigger_keywords + 参数元数据（第二、三节） | 低 | Phase 1 |
| **Phase 3** | 后端 SessionMode 重构 + 新 bridge 命令（第一、五、六节） | 中 | Phase 2 |
| **Phase 4** | 前端类型契约更新（第十一节） | 低 | Phase 3 |
| **Phase 5** | 前端键盘路由器（第七节） | 中 | Phase 4 |
| **Phase 6** | 前端行内参数模式（第八节） | 中 | Phase 5 |
| **Phase 7** | 前端参数面板（第九节） | 中 | Phase 5 |
| **Phase 8** | 前端全页面插件模式（第十节） | 低 | Phase 5 |

---

## 验证方案

### Phase 1 验证
```bash
cargo check  # 零错误
```
手动测试：配置一个带 `{}` 参数的命令，通过前端传入 userArgs（可临时硬编码），确认参数被正确替换并执行。

### Phase 2 验证
- 配置 CommandSource/UrlSource 的项目带有 `triggerKeywords` 字段
- `bridge_query` 返回的结果中包含 `userArgCount`、`hasSystemParams`、`triggerKeywords` 字段
- `cargo check` 零错误

### Phase 3 验证
- `bridge_get_session_mode` 正确返回新模式字符串
- `bridge_enter_inline_mode`、`bridge_enter_param_panel`、`bridge_exit_mode` 命令正常响应
- 模式转换符合状态机规则
- `cargo check` 零错误

### Phase 5-8 验证
- 搜索模式：正常搜索、选择、执行
- 行内参数模式：输入触发词+空格→进入→输入参数→Enter 执行
- 参数面板模式：选中带参数项→Enter→面板弹出→填写→执行
- 全页面插件模式：触发全页面插件→UI 接管→键盘路由→退出
- 各模式下 Escape 正确退出

### 端到端测试场景
1. 配置 URL `https://google.com/search?q={}` 触发词 `g`，输入 `g` 空格 `hello world` Enter → 打开浏览器搜索
2. 配置命令 `echo {} {clip}` 触发词 `echo`，先复制文本，输入 `echo` 空格 `test` Enter → 输出 `test <剪贴板内容>`
3. 输入 `= 2+3` → 计算器插件行内显示结果
4. 选中带 2 个参数的命令按 Enter → 参数面板显示 2 个输入框

---

## 关键文件清单

### 后端需修改
| 文件 | 变更类型 |
|------|----------|
| `src-tauri/src/plugin_system/session_router.rs` | 重构：SessionMode + 新方法 |
| `src-tauri/src/plugin_system/types.rs` | 扩展：ListItem 字段 + 新类型 |
| `src-tauri/src/commands/bridge.rs` | 扩展：新增 4 个命令 |
| `src-tauri/src/plugin/executor/command_executor.rs` | 修复：添加 resolve_parameters |
| `src-tauri/src/plugin/executor/url_executor.rs` | 修复：添加 resolve_parameters |
| `src-tauri/src/plugin/data_source/command_source.rs` | 扩展：trigger_keywords |
| `src-tauri/src/plugin/data_source/url_source.rs` | 扩展：trigger_keywords |
| `src-tauri/src/lib.rs` | 注册新命令 |

### 前端需修改
| 文件 | 变更类型 |
|------|----------|
| `src-ui-new/bridge/contract.ts` | 扩展：新类型 + 字段 |
| `src-ui-new/bridge/commands.ts` | 扩展：新命令函数 |
| `src-ui-new/stores/search-store.ts` | 重构：新状态 + 新方法 |
| `src-ui-new/views/SearchView.vue` | 重构：布局 + 模式切换 |
| `src-ui-new/components/search/SearchBar.vue` | 重构：行内参数 UI |

### 前端需新建
| 文件 | 用途 |
|------|------|
| `src-ui-new/composables/useKeyboardRouter.ts` | 键盘事件中央路由器 |
| `src-ui-new/composables/keyboard/searchHandler.ts` | 搜索模式按键处理 |
| `src-ui-new/composables/keyboard/inlineParamHandler.ts` | 行内参数模式按键处理 |
| `src-ui-new/composables/keyboard/paramPanelHandler.ts` | 参数面板模式按键处理 |
| `src-ui-new/composables/keyboard/inlinePluginHandler.ts` | 行内插件模式按键处理 |
| `src-ui-new/composables/keyboard/fullPagePluginHandler.ts` | 全页面插件模式按键处理 |
| `src-ui-new/components/search/ParamPanel.vue` | 参数面板组件 |

---

## 需复用的现有基础设施

| 模块 | 路径 | 复用方式 |
|------|------|----------|
| TemplateParser | `src-tauri/src/sdk/parameter/template_parser.rs` | 统计 {} 数量、检测系统参数 |
| DefaultParameterResolver | `src-tauri/src/sdk/parameter/default_resolver.rs` | 参数替换核心逻辑 |
| ParameterSnapshot | `src-tauri/src/sdk/parameter/types.rs` | 系统参数快照 |
| PluginHandle::resolve_parameters | `src-tauri/src/sdk/host_api.rs` | Executor 调用入口 |
| PluginHandle::capture_parameter_snapshot | `src-tauri/src/sdk/host_api.rs` | wake 时捕获 |
| ConfirmPayload.userArgs | `src-ui-new/bridge/contract.ts` | 前端已有字段，只需填充 |
