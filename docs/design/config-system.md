# 配置系统架构设计文档

## 一、设计目标

### 新设计目标

1. 配置与组件紧密绑定
2. 统一的配置管理接口
3. 支持前端动态发现和渲染配置界面
4. 简化更新机制

---

## 二、架构设计

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ConfigManager                                   │
│  - 统一管理所有可配置组件                                                      │
│  - 提供配置的 CRUD 操作                                                       │
│  - 负责配置的持久化                                                            │
│  - 向前端提供配置 schema                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ 管理所有 Configurable
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          ConfigurableRegistry                                │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  Map<component_id, Arc<dyn Configurable>>                           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  组件类型：                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Plugin       │  │ DataSource   │  │ SearchEngine │  │ ScoreBooster │    │
│  │ (Configurable)│  │ (Configurable)│  │ (Configurable)│  │ (Configurable)│    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 三、Configurable Trait 设计

### Trait 继承关系

```
┌───────────────────┐
│   Configurable    │  ← 基础配置能力
└─────────┬─────────┘
          │
    ┌─────┴─────┬─────────────────┬───────────────┬──────────────┐
    ▼           ▼                 ▼               ▼              ▼
┌─────────┐ ┌───────────┐ ┌───────────────┐ ┌─────────────┐ ┌─────────┐
│ Plugin  │ │ DataSource│ │ SearchEngine  │ │ ScoreBooster│ │ ActionExecutor│
└─────────┘ └───────────┘ └───────────────┘ └─────────────┘ └─────────┘
```

**设计说明**：
- 使用 `serde_json::Value` 作为配置传递介质，支持复杂嵌套结构
- 保留类型信息（数字、布尔、数组、对象），与前端 JSON 交互更自然
- 可直接使用 `serde_json::from_value` 反序列化到结构体

---

## 四、SettingDefinition 设计

### 核心类型定义

**FieldDefinition**：组件配置项的字段定义，用于描述配置项的核心属性。

**SettingDefinition**：组件配置项的声明式定义，服务于设置存储与动态设置界面生成。

**ArrayUiHint**：数组元素的 UI 渲染提示，用于指导前端如何渲染数组类型的配置项。

**SettingType**：组件设置项的输入控件类型，服务于设置表单渲染与取值校验。

### 设计说明

1. **类型安全**：`ArrayItem` 使用联合类型，编译期阻止 `Primitive` 和 `Object` 同时存在的无效状态
2. **default_value 语义**：
   - `SettingDefinition.field.default_value`：整个设置项的默认值
   - `FieldDefinition.default_value`（在 `ArrayItem::Object` 内）：新增一行对象时的字段默认值模板
3. **UI 渲染提示**：`ArrayUiHint` 指导前端如何渲染数组配置项

---

## 五、ConfigManager 设计

### 职责

1. 注册/注销可配置组件
2. 提供配置的统一访问接口
3. 配置的持久化（加载/保存）
4. 向前端提供配置 Schema

---

## 六、配置组件目录组织规范

**核心原则**：所有**核心程序**（非插件）的 Configurable 组件属于上层业务配置，统一放在 `core/config/components/` 下，按能力维度组织。

```
src-tauri/src/core/config/
├── components/                  ← 按能力维度组织的配置组件
│   ├── mod.rs                   # 组件模块入口，统一导出所有 Configurable 组件
│   ├── hotkey_config.rs         # HotkeyConfigComponent — 快捷键语义化配置
│   └── storage_config.rs        # StorageConfigComponent — 存储后端切换与 WebDAV 配置
├── event.rs                     # 配置事件定义（ConfigEvent）
├── manager.rs                   # ConfigManager — 统一配置管理器核心逻辑
├── models.rs                    # 配置数据模型
├── registry.rs                  # ConfigurableRegistry — 组件注册表
└── store.rs                     # ConfigStore — 配置持久化存储
```

**SDK 层与 Configurable 的职责边界**：
- `sdk/` 只提供**平台原语**（trait + 实现），如 `HotkeyManager`、`StorageService trait`。
- `core/config/components/` 提供**业务配置组件**（实现 `Configurable` trait）。
- SDK 不导出 Configurable 组件，不关心配置如何管理——这是 `core/config` 的职责。

---

## 七、前端交互数据结构

### 获取所有可配置组件列表

```
GET /api/config/components
→ Response: Vec<ComponentInfo>

struct ComponentInfo {
    id: String,                    // 组件ID
    name: String,                  // 显示名称
    component_type: ComponentType, // 组件类型
    enabled: bool,                 // 是否启用
    has_settings: bool,            // 是否有配置项
}
```

### 获取单个组件的配置 Schema

```
GET /api/config/components/{id}/schema
→ Response: ComponentSchema

struct ComponentSchema {
    component_id: String,
    component_name: String,
    groups: Vec<SettingGroup>,     // 分组的配置项
}
```

### 获取组件的当前配置值

```
GET /api/config/components/{id}/settings
→ Response: HashMap<String, String>
```

### 更新组件配置（整体替换）

```
PUT /api/config/components/{id}/settings
Body: HashMap<String, String>
→ Response: Result<(), ConfigError>
```

### 重置为默认配置

```
POST /api/config/components/{id}/reset
→ Response: HashMap<String, String>  // 返回默认值
```

---

## 八、配置持久化设计

### 配置文件结构

```json
{
  "version": "3",
  "components": {
    "program-source": {
      "enabled": true,
      "settings": {
        "target_paths": "[{\"root_path\":\"...\", ...}]",
        "scan_uwp": "true",
        "forbidden_paths": "[\"...\"]"
      }
    },
    "calculator-plugin": {
      "enabled": true,
      "settings": {
        "precision": "10",
        "history_size": "50"
      }
    }
  }
}
```

### 特点

1. 扁平化结构，所有组件配置在同一层级
2. 统一的 enabled 字段管理启用状态
3. settings 内部是 String -> String 映射，具体解析由组件负责
4. 新增组件只需添加新条目，无需修改整体结构

---

## 九、完整架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                  前端                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         Settings Page                                │    │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │    │
│  │  │ Plugin List     │  │ Setting Form    │  │ Preview/Actions │     │    │
│  │  │ (动态渲染)       │  │ (动态渲染)       │  │                 │     │    │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ Tauri Commands
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Tauri Commands                                  │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  get_all_components() → Vec<ComponentInfo>                          │    │
│  │  get_component_schema(id) → ComponentSchema                         │    │
│  │  get_component_settings(id) → HashMap<String, String>               │    │
│  │  apply_component_settings(id, settings) → Result<()>                │    │
│  │  reset_component_settings(id) → Result<()>                          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ 调用
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ConfigManager                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  - 组件注册/注销                                                     │    │
│  │  - 配置读写                                                          │    │
│  │  - 配置验证                                                          │    │
│  │  - 事件发布                                                          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                      │                                       │
│                                      │ 管理                                  │
│                                      ▼                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      ConfigurableRegistry                            │    │
│  │  Map<component_id, Arc<dyn Configurable>>                           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ 持久化
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ConfigStore                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  - 本地 JSON 文件读写                                                │    │
│  │  - 远程同步 (WebDAV)                                                 │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```
