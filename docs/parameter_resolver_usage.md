> 目前由ai写，后续会由人工重写，现在属于临时看看的那种，后续会全部重写

# 参数解析器功能使用说明

## 概述

参数解析器模块 (`parameter_resolver`) 为 ZeroLaunch-rs 提供了强大的参数处理能力,支持以下参数类型:

- `{}` - 用户提供的位置参数
- `{clip}` - 剪贴板内容
- `{hwnd}` - 当前活动窗口句柄
- `{selection}` - 唤醒前活动窗口的选中文本

**重要**: 参数解析功能对**所有类型的启动方式**都有效,包括:
- ✅ 命令行程序
- ✅ 网址 (URL)
- ✅ 自定义命令
- ✅ 文件路径

## 功能特性

### 1. 位置参数 `{}`

传统的位置参数,按顺序由用户输入填充。

**命令示例:**
```
cmd /c echo {}
```
用户输入: `Hello World`
实际执行: `cmd /c echo Hello World`

**网址示例:**
```
https://www.bing.com/search?q={}
```
用户输入: `Rust programming`
实际访问: `https://www.bing.com/search?q=Rust programming`

### 2. 剪贴板参数 `{clip}`

自动获取剪贴板中的文本内容。

**命令示例:**
```
notepad {clip}
```
如果剪贴板包含 `C:\test.txt`
实际执行: `notepad C:\test.txt`

**网址示例:**
```
https://www.bing.com/search?q={clip}
```
如果剪贴板包含 `Python tutorial`
实际访问: `https://www.bing.com/search?q=Python tutorial`

**特性:**
- 在用户按下回车确认启动时捕获剪贴板内容
- 如果剪贴板为空或无法访问,使用空字符串
- 剪贴板内容在参数收集过程中不会改变

### 3. 窗口句柄参数 `{hwnd}`

获取**按下快捷键唤醒搜索栏之前**的前台窗口句柄值(十进制格式)。

**重要**: `{hwnd}` 捕获的是**唤醒 ZeroLaunch-rs 之前的活动窗口**,而不是搜索栏窗口本身。

**命令示例:**
```
window-tool --hwnd {hwnd}
```
**使用场景:**
1. 在记事本窗口中按下 Alt+Space 唤醒 ZeroLaunch-rs
2. 执行上述命令
3. 实际执行: `window-tool --hwnd 1234567` (记事本的窗口句柄)

**特性:**
- 在用户按下快捷键唤醒搜索栏时捕获前台窗口句柄
- 如果无法获取窗口句柄,返回 `0`
- 窗口句柄在参数收集过程中不会改变
- **不会**捕获搜索栏本身的句柄

### 4. 选中文本参数 `{selection}`

获取**按下快捷键唤醒搜索栏之前**活动窗口中选中的文本内容。

**重要**: `{selection}` 捕获的是**唤醒 ZeroLaunch-rs 之前活动窗口中选中的文本**，不会污染剪贴板。

**命令示例:**
```
translate-tool --text {selection}
```
**使用场景:**
1. 在记事本或浏览器中选中一段文本
2. 按下 Alt+Space 唤醒 ZeroLaunch-rs
3. 执行上述命令
4. 实际执行: `translate-tool --text "选中的文本内容"`

**网址示例:**
```
https://www.bing.com/search?q={selection}
```
选中任何文本 → 唤醒 ZeroLaunch-rs → 执行搜索 → 直接搜索选中的内容

**特性:**
- 在用户按下快捷键唤醒搜索栏时捕获选中文本
- 使用 Windows UI Automation API 获取，不污染剪贴板
- 如果没有选中文本或无法访问，返回空字符串
- 选中文本在参数收集过程中不会改变
- 支持大多数支持 UI Automation 的应用程序

## 混合使用示例

可以同时使用多种参数类型:

**命令:**
```
program {} {clip} {} {hwnd} {selection}
```

**网址:**
```
https://search.example.com/?q={}&source={clip}&window={hwnd}&selected={selection}
```

这个命令需要:
- 用户输入2个参数(两个 `{}`)
- 自动获取剪贴板内容
- 自动获取窗口句柄
- 自动获取选中文本

**示例执行流程:**
1. 用户选择这个程序
2. 系统提示输入第1个参数: 用户输入 `arg1`
3. 系统提示输入第2个参数: 用户输入 `arg2`
4. 用户按回车确认启动
5. **此时捕获剪贴板和窗口句柄** (重要!)
6. 填充模板: `program arg1 clipboard_content arg2 window_handle`
7. 执行程序或打开网址

## 实际应用场景

### 场景1: 快速搜索剪贴板内容

配置网页程序:
```
名称: Bing 搜索剪贴板
URL: https://www.bing.com/search?q={clip}
```

使用流程:
1. 复制任何文本到剪贴板
2. 启动 ZeroLaunch-rs
3. 搜索并选择"Bing 搜索剪贴板"
4. 按回车 → 自动在浏览器中搜索剪贴板内容！

### 场景2: 快速编辑剪贴板文件

```
notepad {clip}
```
复制文件路径 → 启动此命令 → 直接打开文件

### 场景3: 带参数的搜索

配置网页程序:
```
名称: Google 搜索
URL: https://www.google.com/search?q={}
```

使用流程:
1. 启动 ZeroLaunch-rs
2. 搜索并选择"Google 搜索"
3. 输入搜索词
4. 按回车 → 在浏览器中打开搜索结果

### 场景4: 窗口操作工具

```
window-manager --action minimize --hwnd {hwnd}
```
在某个窗口前台时启动 → 自动获取该窗口句柄 → 最小化该窗口

### 场景5: 快速翻译选中文本

```
https://translate.google.com/?sl=auto&tl=zh-CN&text={selection}
```
在任何应用中选中文本 → 唤醒 ZeroLaunch-rs → 执行此命令 → 直接翻译选中内容

### 场景6: 组合使用

```
search-tool --query {} --clipboard {clip} --window {hwnd} --selected {selection}
```
用户输入搜索词 → 同时附带剪贴板内容、窗口句柄和选中文本

## 技术细节

### 参数捕获时机

**系统参数捕获的三个关键时刻:**

1. **窗口句柄 `{hwnd}`**: 在用户**按下快捷键(如 Alt+Space)唤醒搜索栏**时捕获
   - 捕获的是唤醒前的前台窗口句柄
   - 不是搜索栏窗口本身
   - 不会因为后续操作改变

2. **选中文本 `{selection}`**: 在用户**按下快捷键唤醒搜索栏**时捕获
   - 与窗口句柄同时捕获
   - 使用 Windows UI Automation API 获取
   - 不污染剪贴板

3. **剪贴板内容 `{clip}`**: 在用户**按下回车确认启动**时捕获
   - 允许用户在参数输入过程中复制内容
   - 使用的是最终启动时刻的剪贴板状态

**示例时间线:**
```
T0: 用户在记事本窗口工作，选中了一段文本 (窗口句柄 = 12345)
T1: 用户按下 Alt+Space 唤醒搜索栏 
    → 🎯 此时捕获窗口句柄 12345
    → 🎯 此时捕获选中文本
    → 搜索栏显示 (窗口句柄 = 67890, 但不使用)
T2: 用户选择命令,开始输入参数
T3: 用户复制文本到剪贴板
T4: 用户按回车确认启动
    → 🎯 此时捕获剪贴板内容
    → 使用 T1 捕获的窗口句柄 12345
    → 使用 T1 捕获的选中文本
T5: 启动程序,参数为: args[] + clip + hwnd(12345) + selection
```

这意味着:
- ✅ 用户可以在输入参数期间复制内容到剪贴板
- ✅ 窗口句柄始终是唤醒前的窗口
- ✅ 选中文本始终是唤醒前的选中内容
- ✅ 参数收集过程中切换窗口不会影响最终的窗口句柄和选中文本
- ✅ 不会因为获取选中文本而污染剪贴板

### 参数统计

只有位置参数 `{}` 会计入需要用户输入的参数数量。

```
program {} {clip} {}     // 用户需要输入2个参数
program {clip} {hwnd}    // 用户不需要输入参数,直接启动
```

### 架构设计

模块采用高内聚、低耦合的设计:

```
parameter_resolver/
├── parameter_types.rs    # 参数类型定义
├── providers.rs          # 参数提供者(获取系统参数)
├── template_parser.rs    # 模板解析器
├── resolver.rs          # 核心解析器
└── mod.rs               # 模块导出
```

**核心概念:**
- `ParameterProvider` trait: 定义参数获取接口
- `SystemParameterSnapshot`: 系统参数快照(捕获时刻的状态)
- `ParameterResolver`: 核心解析器(填充模板)
- `TemplateParser`: 模板解析器(识别参数占位符)

## 使用场景示例

### 场景1: 快速编辑剪贴板文件

```
notepad {clip}
```
复制文件路径 → 启动此命令 → 直接打开文件

### 场景2: 窗口操作工具

```
window-manager --action minimize --hwnd {hwnd}
```

**使用流程:**
1. 在某个窗口前台时(比如浏览器)
2. 按下 Alt+Space 唤醒 ZeroLaunch-rs
3. 搜索并选择"window-manager"命令
4. 按回车执行
5. **结果**: 自动获取浏览器的窗口句柄并最小化浏览器窗口

**注意**: 获取的是**步骤1中的浏览器窗口**,不是搜索栏窗口

### 场景3: 混合参数

```
search-tool --query {} --clipboard {clip} --window {hwnd}
```
用户输入搜索词 → 同时附带剪贴板内容和窗口句柄

## 扩展性

架构设计支持未来添加更多系统参数类型:

```rust
pub enum SystemParameter {
    Clipboard,           // 已实现
    WindowHandle,        // 已实现
    // 未来可扩展:
    // CurrentTime,      // 当前时间
    // SelectedText,     // 选中的文本
    // MousePosition,    // 鼠标位置
    // ...
}
```

添加新参数只需:
1. 在 `SystemParameter` 添加枚举值
2. 在 `SystemParameter::from_name()` 添加解析规则
3. 在 `SystemParameterSnapshot::capture()` 添加捕获逻辑
4. 在 `SystemParameterSnapshot::get()` 添加获取逻辑

## 注意事项

1. **剪贴板内容限制**: 只支持文本内容,不支持图片等其他格式
2. **窗口句柄格式**: 返回十进制数字字符串,如 `1234567`
3. **参数捕获时机**: 在用户按回车确认时,不是最后一个参数输入完成时
4. **错误处理**: 如果无法获取系统参数,使用默认值(空字符串或0)而不是报错

## 兼容性

- 旧的 `{}` 占位符完全兼容
- 新旧代码可以并存
- `LaunchMethod::placeholder_count()` 保留但标记为 deprecated
- 使用新方法 `LaunchMethod::user_parameter_count(resolver)` 获取准确的用户参数数量


```
                    ┌─────────────────┐
                    │   AppState      │
                    │ (存储窗口句柄)   │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │ ui_controller   │
                    │  (唤醒时保存)    │
                    └─────────────────┘
                             
┌─────────────────────────────────────────────────┐
│        parameter_resolver 模块                   │
│                                                  │
│  ┌──────────────┐      ┌────────────────────┐  │
│  │ providers.rs │      │   resolver.rs      │  │
│  │  (纯工具)    │      │                    │  │
│  │              │      │  Snapshot::capture │  │
│  │ ✅ 零依赖    │◄─────│  (读取 AppState)   │  │
│  └──────────────┘      └────────────────────┘  │
│                                                  │
└─────────────────────────────────────────────────┘
```