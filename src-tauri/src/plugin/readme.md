这个文件夹下保存的目录名是以实现的trait来划分的。比如data_source文件夹下，保存的全是实现data_source trait的类

使用这种方式可以提升代码的可读性

toolName: view_files
            
status: success
          
            
filePath: c:\Users\ghost\ZeroLaunch\ZeroLaunch-rs\src-tauri\src\plugin_system\types.rs
          
## types.rs Trait 分析

### Trait 继承关系图

```
                    ┌───────────────────┐
                    │   Configurable    │  ← 基础配置能力（所有组件都必须实现）
                    │   (基础 trait)     │
                    └─────────┬─────────┘
                              │
        ┌─────────┬───────────┼───────────┬─────────────┬─────────────┐
        ▼         ▼           ▼           ▼             ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐ ┌─────────────┐ ┌───────────┐ ┌─────────┐
│DataSource │ │KeywordOpt │ │SearchEng  │ │ScoreBooster │ │  Launcher  │ │ Plugin  │
│(数据源)    │ │(关键字优化)│ │(搜索引擎)  │ │(分数提升器)  │ │ (启动器)   │ │(插件)   │
└───────────┘ └───────────┘ └───────────┘ └─────────────┘ └───────────┘ └─────────┘
```

---

## 各 Trait 详细分析

### 1. Configurable（基础配置能力）

**作用**：所有可配置组件的基础契约，提供统一的配置管理能力。

```rust
pub trait Configurable: Send + Sync {
    // === 标识信息 ===
    fn component_id(&self) -> &str;      // 唯一标识符，如 "program-source"
    fn component_name(&self) -> &str;    // 显示名称，如 "程序数据源"
    fn component_type(&self) -> ComponentType;  // 组件类型枚举

    // === 配置定义 ===
    fn setting_schema(&self) -> Vec<SettingDefinition> { vec![] }  // 配置项 Schema
    
    // === 配置读写 ===
    fn get_settings(&self) -> serde_json::Value { serde_json::Value::Object(serde_json::Map::new()) }
    fn apply_settings(&mut self, settings: serde_json::Value) -> Result<(), ConfigError>;
    
    // === 可选回调 ===
    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> { Ok(()) }
    fn on_settings_changed(&self) {}  // 配置变更后的回调
}
```

**设计目的**：
- 前端可根据 `setting_schema()` 动态渲染配置界面
- 统一的配置管理，新增组件无需修改核心代码
- 支持配置验证和变更通知
- 使用 `serde_json::Value` 作为配置传递介质，支持复杂嵌套结构

---

### 2. DataSource（数据源）

**作用**：提供搜索候选项的数据来源。

```rust
pub trait DataSource: Configurable {
    fn fetch_candidates(&self) -> CachedCandidateData;
}
```

**设计目的**：
- 将数据获取逻辑从搜索逻辑中分离
- 支持多种数据源：程序、书签、URL、文件等
- 数据源可以独立配置和启用/禁用

**使用场景**：
| 实现类           | 数据来源         |
| ---------------- | ---------------- |
| `ProgramSource`  | 已安装的程序列表 |
| `BookmarkSource` | 浏览器书签       |
| `UrlSource`      | 用户保存的 URL   |
| `FileSource`     | 文件系统扫描     |

---

### 3. KeywordOptimizer（关键字优化器）

**作用**：对候选项的关键字进行扩展和优化，提高搜索召回率。

```rust
pub trait KeywordOptimizer: Configurable {
    // 根据关键词优化出一组新关键词
    fn optimize(&self, keyword: &str) -> Vec<String>;
    
    // 是否对所有已累积的关键词进行优化（true），还是只对原始名称优化
    fn uses_context(&self) -> bool { false }
    
    // 获得优先级，优先级小的优化器会先被调用
    fn get_priority(&self) -> i32;
    fn set_priority(&mut self, priority: i32);
}
```

**设计目的**：
- 将"微信"扩展为 ["微信", "wechat", "weixin"]
- 将拼音 "weixin" 转换为 "微信"
- 支持多种优化策略组合，按优先级链式调用

**链式优化流程**：

```
原始名称: "微信"
    │
    ▼
┌─────────────────────────────────┐
│ Optimizer A (priority=10)        │
│ uses_context=false               │
│ optimize("微信")                  │
│ 输出: ["weixin"]                  │
└─────────────────────────────────┘
    │
    ▼ 累积关键词: ["微信", "weixin"]
    │
┌─────────────────────────────────┐
│ Optimizer B (priority=20)        │
│ uses_context=true                │
│ 对每个关键词调用 optimize:        │
│   optimize("微信") → []           │
│   optimize("weixin") → ["wx"]    │
│ 输出: ["wx"]                      │
└─────────────────────────────────┘
    │
    ▼ 最终关键词: ["微信", "weixin", "wx"]
```

**使用场景**：
| 实现类             | 功能                   |
| ------------------ | ---------------------- |
| `PinyinConverter`  | 中文转拼音、拼音转中文 |
| `AliasExpander`    | 添加常用别名           |
| `InitialExtractor` | 提取首字母（如 "wx"）  |

---

### 4. SearchEngine（搜索引擎）

**作用**：计算候选项与用户查询的匹配分数。

```rust
pub trait SearchEngine: Configurable {
    fn calculate_scores(
        &self,
        candidates: &CachedCandidateData,
        query: &str,
    ) -> Vec<ScoredCandidate>;
}
```

**设计目的**：
- 计算单个候选项与查询的相关性
- 支持多种搜索算法：模糊搜索、语义搜索等
- 可替换的搜索策略

**使用场景**：
| 实现类                 | 算法                  |
| ---------------------- | --------------------- |
| `StandardSearchEngine` | 模糊匹配 + 关键字匹配 |
| `SemanticSearchEngine` | AI 语义相似度搜索     |

---

### 5. ScoreBooster（分数提升器）

**作用**：基于用户行为对搜索结果进行个性化排序优化。

```rust
pub trait ScoreBooster: Configurable {
    fn record(&self, candidate: &mut ScoredCandidate, query: &str);  // 记录用户选择
    fn boost(&self, candidates: &mut Vec<ScoredCandidate>);          // 批量调整分数
}
```

**设计目的**：
- 区别于 SearchEngine：SearchEngine 计算单个候选项，ScoreBooster 处理所有候选项
- 支持基于历史行为的个性化排序
- 可组合多个 Booster

**使用场景**：
| 实现类                 | 功能                 |
| ---------------------- | -------------------- |
| `HistoryBooster`       | 基于启动次数提升分数 |
| `QueryAffinityBooster` | 基于查询关联提升分数 |

---

### 6. Launcher（启动器）

**作用**：执行候选项的实际启动操作。

```rust
pub trait Launcher: Send + Sync {
    fn supported_method(&self) -> LaunchMethodType;  // 支持的启动类型
    fn launch(&self, method: &LaunchMethod) -> Result<(), LaunchError>;
}
```

**设计目的**：
- 每种启动方式一个 Launcher，职责单一
- 通过 `LaunchMethodType` 匹配对应的 Launcher
- 支持扩展新的启动方式

**使用场景**：
| 实现类            | LaunchMethodType    |
| ----------------- | ------------------- |
| `PathLauncher`    | `Path`              |
| `UwpLauncher`     | `PackageFamilyName` |
| `UrlLauncher`     | `Url`               |
| `CommandLauncher` | `Command`           |

---

### 7. Plugin（插件）

**作用**：完整的独立功能单元，支持触发词、查询、动作执行。

```rust
#[async_trait]
pub trait Plugin: Configurable {
    fn metadata(&self) -> &PluginMetadata;
    
    async fn init(&self, ctx: &PluginContext, api: Arc<dyn PluginAPI>) -> Result<(), PluginError>;
    async fn query(&self, ctx: &PluginContext, query: &Query) -> Result<QueryResponse, PluginError>;
    async fn execute_action(&self, ctx: &PluginContext, action_id: &str, payload: serde_json::Value) -> Result<(), PluginError>;
}
```

**设计目的**：
- 独立的功能模块，通过触发词激活
- 支持异步操作
- 可以返回自定义 UI（List、CustomPanel、WebView）

**使用场景**：
| 实现类             | 触发词 | 功能     |
| ------------------ | ------ | -------- |
| `CalculatorPlugin` | `=`    | 计算器   |
| `EverythingPlugin` | `f`    | 文件搜索 |

---

## 如果我要写一个 DataSource，需要实现哪些 Trait？

### 答案：只需要实现 `DataSource` trait

因为 `DataSource: Configurable`，所以你需要实现两个 trait：

```rust
pub struct ProgramSource {
    config: ProgramSourceConfig,
    programs: Vec<Arc<Program>>,
}

// 1. 实现 Configurable（必须，因为 DataSource 继承了它）
impl Configurable for ProgramSource {
    fn component_id(&self) -> &str { "program-source" }
    fn component_name(&self) -> &str { "程序数据源" }
    fn component_type(&self) -> ComponentType { ComponentType::DataSource }
    
    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SettingDefinition {
                field: FieldDefinition {
                    key: "scan_uwp".to_string(),
                    label: "扫描 UWP 应用".to_string(),
                    description: "是否扫描 Windows Store 应用".to_string(),
                    setting_type: SettingType::Boolean,
                    default_value: serde_json::json!(true),
                    visible: true,
                    editable: true,
                },
                group: None,
                order: 0,
            },
        ]
    }
    
    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&self.config).unwrap_or(serde_json::Value::Null)
    }
    
    fn apply_settings(&mut self, settings: serde_json::Value) -> Result<(), ConfigError> {
        self.config = serde_json::from_value(settings)
            .map_err(|e| ConfigError::ApplyFailed(e.to_string()))?;
        Ok(())
    }
}

// 2. 实现 DataSource（核心功能）
impl DataSource for ProgramSource {
    fn fetch_candidates(&self) -> CachedCandidateData {
        let candidates = self.programs.iter().map(|p| {
            SearchCandidate {
                id: p.program_guid,
                name: p.show_name.clone(),
                icon: p.icon_request_json.clone(),
                launch_method: p.launch_method.clone().into(),
                keywords: p.search_keywords.clone(),
                bias: p.stable_bias,
            }
        }).collect();
        
        CachedCandidateData::from_candidates(candidates)
    }
}
```

---

## 快速参考表

| 我想写...    | 需要实现的 Trait                      | 核心方法                         |
| ------------ | ------------------------------------- | -------------------------------- |
| 数据源       | `DataSource` (+ `Configurable`)       | `fetch_candidates()`             |
| 关键字优化器 | `KeywordOptimizer` (+ `Configurable`) | `optimize()`, `uses_context()`   |
| 搜索引擎     | `SearchEngine` (+ `Configurable`)     | `calculate_scores()`             |
| 分数提升器   | `ScoreBooster` (+ `Configurable`)     | `record()`, `boost()`            |
| 启动器       | `Launcher`                            | `supported_method()`, `launch()` |
| 完整插件     | `Plugin` (+ `Configurable`)           | `query()`, `execute_action()`    |
