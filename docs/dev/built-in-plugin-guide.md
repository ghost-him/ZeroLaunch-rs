# 内置插件开发指南

## 架构概览

内置插件是 ZeroLaunch-rs 的核心扩展机制。所有插件编译进同一个二进制，通过 `inventory` crate 实现编译期自动发现和注册。

## 组件分类

| 类别 | 目录 | Trait | 说明 |
|------|------|-------|------|
| ActionExecutor | `plugin/executor/` | `ActionExecutor` | 执行器：启动程序、打开 URL、管理员运行等 |
| DataSource | `plugin/data_source/` | `DataSource` | 数据源：采集候选项（程序列表、书签、命令等） |
| KeywordOptimizer | `plugin/keyword_optimizer/` | `KeywordOptimizer` | 关键词优化器：拼音、首字母、去空格等 |
| SearchEngine | `plugin/search_engine/` | `SearchEngine` | 搜索引擎：打分排序算法 |
| ScoreBooster | `plugin/score_booster/` | `ScoreBooster` | 分数增强器：历史频率、查询亲和度 |
| Plugin | `plugin/triggerable/` | `Plugin` | 插件：触发式交互（计算器、Everything 搜索等） |
| Core Component | `core/config/components/` | `Configurable` | 核心配置组件：热键、外观、存储等系统级配置 |

## 第 1 步：选择正确的类别

| 你想做什么 | 选哪个 |
|-----------|--------|
| 启动程序 / 打开文件 / 以管理员运行 | `ActionExecutor` |
| 从某处采集候选项（如浏览器书签、Steam 游戏） | `DataSource` |
| 修改搜索关键词（如加拼音、去符号） | `KeywordOptimizer` |
| 改变搜索排序算法 | `SearchEngine` |
| 对搜索结果加权（如按使用频率） | `ScoreBooster` |
| 创建新的查询/交互模式（如计算器） | `Plugin` |
| 添加新的设置类别（如代理配置） | `Core Component` |

## 第 2 步：创建后端文件

### ActionExecutor 示例

1. 在 `src-tauri/src/plugin/executor/` 下创建文件（如 `my_executor.rs`）
2. 参考 `_template/template_executor.rs` 复制骨架
3. 实现 `Configurable` trait：
   - `component_id()`: 返回唯一 ID (kebab-case)
   - `component_name()`: 返回显示名称
   - `component_type()`: 返回 `ComponentType::ActionExecutor`
4. 实现 `ActionExecutor` trait：
   - `supported_target_types()`: 返回支持的 `TargetType` 列表
   - `supported_actions()`: 返回对外暴露的动作列表
   - `execute()`: 执行动作的具体逻辑
5. 在文件底部添加 `inventory::submit!` 块

### DataSource 示例

创建文件后，实现 `DataSource::fetch_candidates()` 返回 `Vec<SearchCandidate>`。

### KeywordOptimizer 示例

创建文件后，实现 `KeywordOptimizer::optimize(&self, keyword: &str) -> Vec<String>`，返回优化后的关键词列表。

### Plugin 示例

参考 `triggerable/calculator_plugin.rs`，实现 `Plugin::query()` 和 `Plugin::execute_action()`。

## 第 3 步：inventory::submit! 规范

### Handle Key

需要 `PluginHandle` 的组件通过 `InventoryContext::get_handle(key)` 获取。相同 key 的组件共享同一个 handle。

**当前已定义的 key：**

| Key | 用途 |
|-----|------|
| `shell-executor` | 程序启动、文件打开、URL 打开 |
| `app-executor` | UWP 应用启动 |
| `command-executor` | 命令执行 |
| `window-activator` | 窗口激活 |
| `program-source` | 程序列表采集 |
| `app-source` | UWP 应用采集 |
| `url-source` | URL 采集 |
| `bookmark-source` | 书签采集 |
| `command-source` | 命令采集 |

新增的组件可以复用已有 key，或定义新 key（新 key 会自动创建对应的 PluginHandle）。

### Priority

Priority 值决定注册顺序。数值越小越先注册。建议间隔 10 以便插入。

各组件类型的 priority 当前已用范围（非硬上限，新增组件建议取当前最大值 + 10）：
- Executor: 0–50
- DataSource: 0–40
- KeywordOptimizer: 0–70
- SearchEngine: 0–20
- ScoreBooster: 0–10
- Plugin: 0
- Core: 0–40

新组件建议取当前最大值 + 10。

### component_id

`inventory::submit!` 中的 `component_id` 必须与 `Configurable::component_id()` 返回值完全一致。

## 第 4 步：创建前端文件（可选）

如果后端返回 `CustomPanel`（Plugin 类型），需要前端面板渲染。

1. 在 `src-ui/plugins/built-in/<plugin-id>/` 下创建 `index.ts`
2. 实现 `FrontendPlugin` 接口，至少声明 `id`、`name`、`version`、`description`
3. 如需自定义面板，配置 `panelProvider`，`matchType` 匹配后端 `CustomPanel.panel_type`
4. 前端文件通过 `import.meta.glob` 自动发现，无需手动注册

## 第 5 步：验证

1. `cargo build` 零错误
2. 启动应用，在设置中确认新组件出现
3. 验证功能正常工作
4. 如需调试，启用 `cargo build --features inspector` 后在设置的"插件检查器"查看运行时状态

## 配置 Schema 编写

所有 `Configurable` 组件通过 `setting_schema()` 暴露配置 UI。使用 `SchemaBuilder` API：

```rust
fn setting_schema(&self) -> Vec<SettingDefinition> {
    vec![
        SchemaBuilder::text("my_field", "显示名称", "描述")
            .group("分组名")
            .order(0)
            .default("默认值")
            .build(),
        SchemaBuilder::number("my_number", "数值", "描述")
            .group("分组名")
            .order(1)
            .default(1.0)
            .min(0.0)
            .max(100.0)
            .build(),
        SchemaBuilder::boolean("my_bool", "开关", "描述")
            .group("分组名")
            .order(2)
            .default(true)
            .build(),
    ]
}
```

### SchemaBuilder 可用方法

- `text(name, label, desc)` — 文本字段
- `number(name, label, desc)` — 数值字段 (.min, .max, .step)
- `boolean(name, label, desc)` — 布尔开关
- `select(name, label, desc)` — 下拉选择 (.options)
- `color(name, label, desc)` — 颜色选择器
- `path(name, label, desc)` — 路径字段（带文件夹选择按钮）
- `array(name, label, desc)` — 数组/表格 (.object_items, .table_ui)
- `json(name, label, desc)` — JSON 编辑器
- `image(name, label, desc)` — 图片字段

### Settings struct 规范

所有 `apply_settings()` 必须使用强类型 Settings struct：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MySettings {
    #[serde(rename = "my_field", default)]
    my_field: String,
    #[serde(rename = "my_number", default = "default_my_number")]
    my_number: f64,
}

fn default_my_number() -> f64 { 1.0 }
```
