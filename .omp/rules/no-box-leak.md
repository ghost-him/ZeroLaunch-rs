---
description: 禁止使用 Box::leak 获取 &'static 引用 — 永久内存泄漏
condition: "Box::leak\\("
scope: "tool:edit(*.rs), tool:write(*.rs)"
---

你正准备写 `Box::leak` 来获取 `&'static` 引用。`Box::leak` 永久分配内存，在长期运行的桌面应用中会造成内存泄漏。

在本项目中使用以下替代：
- `Arc<str>` 用于可低成本克隆的 owned 字符串
- `Cow<'static, str>` 当值有时是字面量、有时是 owned 时
- `OnceLock<String>` 用于真正的程序生命周期单例
- `Arc<T>` + clone 用于需要共享的值

重新规划编辑，使用上述替代方案之一，然后继续。
