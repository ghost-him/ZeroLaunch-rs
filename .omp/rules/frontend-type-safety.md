---
description: 前端类型安全 — IPC 类型在 contract.ts 定义，类型守卫集中在 schemaTypes.ts，禁止 any 用 unknown+守卫
condition: ": any|as any|contract\\.ts|schemaTypes|isMyType"
scope: "tool:edit(*.ts), tool:edit(*.vue), tool:write(*.ts), tool:write(*.vue)"
interruptMode: tool-only
---

# 前端类型安全

- 所有 IPC 类型在 `bridge/contract.ts` 中定义，与 Rust 字段级 `#[serde(rename = "camelCaseKey")]` 保持同步
- Schema 类型守卫集中在 `utils/schemaTypes.ts`。**禁止** 在组件中内联类型判断
- **禁止** 使用 `any` 类型。使用 `unknown` + 类型守卫：

```typescript
// 正确
function parse(input: unknown): MyType {
  if (isMyType(input)) return input;
  throw new Error('Invalid input');
}

// 错误
function parse(input: any): MyType { return input; }
```
