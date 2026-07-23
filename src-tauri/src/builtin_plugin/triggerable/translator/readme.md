# 翻译插件开发指南

本目录是内置 **Plugin**（触发式插件），对应 `builtin_plugin/readme.md` 中的 `Plugin` trait：位于 `triggerable/`，实现 `Configurable` + `Plugin`，经 `inventory::submit!` 自动注册。

翻译引擎（Provider）是**插件内部**策略对象，**不是**框架层组件类型；不要放到 `data_source/` / `executor/` 等目录，也不要为其单独 `inventory::submit!`。

## 目录结构

```
translator/
├── mod.rs
├── plugin.rs           # TranslatorPlugin：Configurable + Plugin + inventory
├── provider.rs         # TranslationProvider / LanguageSupport / 统一结果类型
├── query_parser.rs     # 查询解析（语言码目录随启用引擎变化）
├── registry.rs         # ProviderRegistry：并行、聚合、语言并集
├── providers/
│   ├── mod.rs
│   ├── openai_compatible.rs   # LLM翻译：OpenAI 兼容 chat/completions
│   └── mock.rs                # 模拟翻译：模拟示例
└── readme.md
```

前端：`src-ui/plugins/built-in/translator-panel/`（`panelProvider.matchType: "translator"`，`settingsProvider.matchComponentId: "translator"`）。宿主 `ComponentConfigLoader` 有自定义设置页则用之，否则回退 DynamicForm。

## 与规范的对应关系

| 规范要求 | 本插件做法 |
| -------- | ---------- |
| 按 trait 分目录 | `triggerable/`，实现 `Plugin` |
| `Configurable` | `plugin.rs` 的 schema / get / apply |
| `inventory::submit!` | `component_id: "translator"`，与 metadata id 一致 |
| 触发词注册期固定 | `fy` / `tr` / `翻译` |
| CustomPanel | `panel_type: "translator"` |
| 平台能力 | HTTP 直连或经 `PluginHandle`；勿反向依赖 `plugin_framework` |

## 统一结果契约

所有 Provider（含内置 LLM）共用同一结构；面板 JSON 用 camelCase。

```rust
pub struct SenseEntry {
    pub text: String,
    pub label: Option<String>,
}

pub struct TranslationResult {
    pub provider_id: String,
    pub provider_name: String,
    pub text: String,
    pub phonetic: Option<String>,
    pub computer_sense: Option<String>,
    pub more_senses: Vec<SenseEntry>,  // ≤4，normalize 截断
    pub detected_source: Option<LanguageCode>,
    pub error: Option<String>,
}
```

| 字段 | 面板 | 展示 |
| ---- | ---- | ---- |
| `text` | `text` | 主译大字 |
| `phonetic` | `phonetic` | 主译下小号 |
| `computer_sense` | `computerSense` | `[计算机]` 常显 |
| `more_senses` | `moreSenses` | 浅色小号常显，不折叠 |

成功用 `TranslationResult::ok(...).normalize_senses()`；失败用 `err(...)`，`error` 为中文。

## 运行时数据流

```
fy …
  → SessionRouter → TranslatorPlugin::query
  → enabled_providers 的 language_support 并集 → LangCatalog
  → parse_search_term
  → on_enter 且同文首次：status=ready（不调引擎）
  → 否则 ProviderRegistry::translate_all
  → CustomPanel { panel_type: "translator", … }
```

- **启用引擎**：勾选清单 + 拖拽调序（靠前优先）；`openai-compatible`（OpenAI 兼容，勾选后展开 LLM 配置）与 `mock`（模拟示例：镜像其它引擎成功结果，否则固定占位）。
- **翻译触发**：`live` 输入即译；`on_enter` 同文第二次 query 才真正翻译（插件内部门控，不改框架 `Query`）。
- **默认目标语**：无语言码时用设置中的 `default_target`；与源语相同时回退到另一常用语。
- **语言能力**：各 Provider 的 `language_support()` 须反映当前配置；解析与校验基于启用引擎并集。

## 如何新增 Provider

词典 API、本地引擎、规则翻译等均按同一套路；**不必**走 LLM。

### 1. 实现 `TranslationProvider`

在 `providers/` 新增文件（如 `foo.rs`）：

```rust
#[async_trait]
impl TranslationProvider for FooProvider {
    fn id(&self) -> &str { "foo" }
    fn name(&self) -> &str { "Foo 翻译" }

    fn language_support(&self) -> LanguageSupport {
        LanguageSupport::bilingual(&["zh", "en", "ja"])
    }

    async fn translate(&self, req: &TranslateRequest) -> TranslationResult {
        TranslationResult::ok(
            "foo",
            "Foo 翻译",
            main_text,
            Some("/fəʊ/".into()),
            Some("IT 释义".into()),
            vec![SenseEntry { text: "…".into(), label: Some("n.".into()) }],
            Some(req.source.clone()),
        )
        .normalize_senses()
    }
}
```

- `id()` kebab-case，与 `enabled_providers` 的 value 一致。
- 用户可见错误用**中文**；直接填契约字段，无需 JSON 解析。
- 在 `providers/mod.rs` 中 `mod` + `pub use`。

### 2. 注册

在 `TranslatorPlugin::new()` 加入 `ProviderRegistry::new(vec![...])`。有运行时配置时用 `Arc<RwLock<...>>` 注入（参考内置 `OpenAiCompatibleProvider` 的 `LlmConfig`）。

### 3. 设置

1. `TranslatorSettings` 增加字段。  
2. 更新 `setting_schema()`（DynamicForm 回退路径）与前端 `TranslatorSettings.vue`（自定义设置页）。  
3. `apply_settings` / `normalize` / `init` 同步到 Provider。  
4. 更新本文档。

### 4. 测试

```bash
cargo test -p zerolaunch-rs translator
```

覆盖：缺凭证/非法参数、语言并集、不支持语言对早退、并行聚合。

## 内置示例：`openai-compatible`

当前默认启用的一个 Provider，实现上用 OpenAI 兼容 `POST {base_url}/chat/completions`（`stream: false`），把模型输出解析进统一契约。系统提示词在代码常量里，**不**暴露给用户。

配置键：`llm_base_url` / `llm_api_key` / `llm_model`（及设置页厂商预设，仅便于填 URL）。缺配置时中文报错且不发请求。

扩展新引擎时**不必**复制这套 HTTP/提示词逻辑；按上一节直接填 `TranslationResult` 即可。

## UI 约定

**设置页**（`TranslatorSettings.vue`）：无「启用」项（用宿主统一开关）；分组为基础 / 引擎 / LLM 服务（LLM 分组是内置引擎的专属配置，其他引擎按同样方式加自己的分组）。

**结果面板**（`TranslationPanel.vue`）：元信息 → 主译 → 音标 → 计算机释义 → 更多释义常显 → 其他引擎折叠 → 复制主译。

## 查询语法

| 输入 | 含义 |
| ---- | ---- |
| `hello` | 源语自动检测，目标为默认目标语（相同则回退） |
| `en 你好` | 目标 `en`，源自动检测 |
| `zh en hello` | 源 `zh` → 目标 `en` |
| `zh-TR hello` | 目标繁体（需启用引擎支持） |

## 检查清单

- [ ] 文件在 `providers/`，未进框架 crate  
- [ ] 实现 `id` / `name` / `language_support` / `translate`  
- [ ] 成功结果 `normalize_senses()`；错误中文  
- [ ] 已加入 `ProviderRegistry`；设置与前端选项已更新  
- [ ] 密钥不明文打日志；`cargo test -p zerolaunch-rs translator` 通过  
