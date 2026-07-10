---
description: 禁止在 TypeScript/Vue 代码中使用 any 类型 — 使用 unknown + 类型守卫
condition: ": any|as any"
scope: "tool:edit(*.ts), tool:edit(*.vue), tool:write(*.ts), tool:write(*.vue)"
---

你在 TypeScript 代码中使用了 `any` 类型。`any` 禁用了类型检查，在边界处丧失精度。

改用 `unknown` + 类型守卫：

    // 正确
    function parse(input: unknown): MyType {
      if (isMyType(input)) return input;
      throw new Error('Invalid input');
    }

    // 错误
    function parse(input: any): MyType { return input; }

类型守卫集中在 `utils/schemaTypes.ts`。禁止在组件中内联类型判断。
