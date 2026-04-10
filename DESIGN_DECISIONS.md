# 设计决策记录

本文档记录 ZeroLaunch-rs 项目中的重要架构设计决策及其理由。

---

## 1. Configurable Trait 的两种配置存储模式

### 背景

项目中所有插件组件都实现了 `Configurable` trait，但在配置存储方式上存在两种不同的设计模式。

### 两种模式对比

#### 模式 A: RwLock<Inner> (类型化存储)

```rust
struct XxxOptimizerInner {
    priority: i32,
    uses_context: bool,
}

pub struct XxxOptimizer {
    inner: RwLock<XxxOptimizerInner>,
}

impl Configurable for XxxOptimizer {
    fn apply_settings(&self, settings: Value) {
        let mut inner = self.inner.write();
        inner.priority = settings.get("priority")...;  // 解析时转换
    }

    fn get_settings(&self) -> Value {
        let inner = self.inner.read();
        json!({ "priority": inner.priority, ... })  // 需要重新构造
    }
}
```

**使用场景**: `KeywordOptimizer` 系列组件

#### 模式 B: RwLock<Value> (原始 JSON 存储)

```rust
pub struct XxxSource {
    settings: RwLock<serde_json::Value>,  // 存储原始 JSON
}

impl XxxSource {
    fn parse_xxx(&self) -> Vec<XxxConfig> {
        self.settings.read().get("xxx")...  // 按需解析
    }
}

impl Configurable for XxxSource {
    fn apply_settings(&self, settings: Value) {
        *self.settings.write() = settings;  // 直接存储
    }

    fn get_settings(&self) -> Value {
        self.settings.read().clone()  // 直接返回
    }
}
```

**使用场景**: `DataSource` 系列组件

### 多维度分析

| 维度 | 模式 A (Inner) | 模式 B (Value) |
|------|----------------|----------------|
| **类型安全** | ✅ 编译时检查，字段访问有 IDE 支持 | ❌ 运行时解析，可能失败 |
| **性能** | ✅ 解析一次，后续 O(1) 访问 | ❌ 每次使用都需解析 |
| **配置验证时机** | ✅ apply_settings 时立即发现错误 | ❌ 使用时才发现错误 |
| **代码简洁性** | ❌ 需定义 Inner + 构造 JSON | ✅ 直接存储，无需转换 |
| **数据一致性** | ❌ 存在两份数据表示 | ✅ 单一数据源 |
| **灵活性** | ❌ 新增字段需改结构体 | ✅ JSON 结构灵活 |

### 决策：根据组件特性选择不同模式

**不强制统一**，而是根据组件的配置复杂度和使用频率选择合适的模式：

| 组件类型 | 配置复杂度 | 使用频率 | 采用模式 |
|----------|-----------|----------|----------|
| KeywordOptimizer | 简单 (2-3个字段) | 极高 (每次 optimize 调用) | Inner |
| DataSource | 复杂 (嵌套数组/对象) | 低 (仅 fetch_candidates 时) | Value |
| ScoreBooster | 中等 | 高 | Inner |
| Launcher | 无配置 | - | 无状态 |
| SearchEngine | 无配置 | - | 无状态 |

### 选择理由

#### KeywordOptimizer 选择 Inner 模式

1. **高频访问**: `priority` 和 `uses_context` 在每次 `optimize()`、`uses_context()`、`get_priority()` 调用时都要访问
2. **性能敏感**: 如果用 Value 模式，每次调用都要解析 JSON，性能开销不可接受
3. **配置简单**: 字段固定且少（通常 2-3 个），定义 Inner 结构体成本低
4. **类型安全**: 编译期保证字段类型正确

#### DataSource 选择 Value 模式

1. **配置复杂**: 如 `ProgramSource` 的 `directories` 配置是嵌套数组，定义完整 Inner 结构体繁琐
2. **低频使用**: 仅在 `fetch_candidates` 时解析一次，性能影响小
3. **灵活性**: 保持原始 JSON 便于扩展和迁移
4. **代码简洁**: 无需维护 Inner 与 JSON 的双向转换

### 相关文件

**KeywordOptimizer (Inner 模式)**:
- `src/plugin/keyword_optimizer/first_letter_extractor.rs`
- `src/plugin/keyword_optimizer/pinyin_converter.rs`
- `src/plugin/keyword_optimizer/version_number_remover.rs`
- `src/plugin/keyword_optimizer/space_normalizer.rs`
- `src/plugin/keyword_optimizer/space_remover.rs`
- `src/plugin/keyword_optimizer/symbol_remover.rs`
- `src/plugin/keyword_optimizer/upper_case_letter_extractor.rs`
- `src/plugin/keyword_optimizer/lower_case_converter.rs`

**ScoreBooster (Inner 模式)**:
- `src/plugin/score_booster/query_affinity.rs`
- `src/plugin/score_booster/history_booster.rs`

**DataSource (Value 模式)**:
- `src/plugin/data_source/uwp_source.rs`
- `src/plugin/data_source/url_source.rs`
- `src/plugin/data_source/program_source.rs`
- `src/plugin/data_source/bookmark_source.rs`
- `src/plugin/data_source/command_source.rs`

---

## 2. Inner 模式的逻辑委托规范

### 背景

对于采用 `RwLock<Inner>` 模式的组件，存在两种代码组织方式：

1. **逻辑分散型**: 外壳定义辅助方法，Inner 仅存储数据
2. **逻辑委托型**: 外壳仅做委托，Inner 包含所有业务逻辑

### 决策：统一采用逻辑委托型

```rust
// ✅ 正确：外壳只做委托
impl XxxOptimizer {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(XxxOptimizerInner::new()),
        }
    }
}

impl KeywordOptimizer for XxxOptimizer {
    fn optimize(&self, keyword: &str) -> Vec<String> {
        self.inner.read().optimize(keyword)  // 直接委托
    }
}

// ✅ 正确：Inner 包含完整逻辑
impl XxxOptimizerInner {
    fn new() -> Self { ... }
    fn optimize(&self, keyword: &str) -> Vec<String> { /* 完整逻辑 */ }
}
```

### 验收标准

1. 外壳方法签名与 inner 相同
2. 外壳方法体只有一行：`self.inner.read().xxx(...)` 或 `self.inner.write().xxx(...)`
3. 所有业务逻辑集中在 inner 中

### 理由

1. **职责清晰**: 外壳负责并发控制，Inner 负责业务逻辑
2. **代码组织**: 避免逻辑分散在多个 impl 块中
3. **可测试性**: Inner 可独立测试（如果需要）

---

*文档持续更新中...*
