---
description: apply_keyword_optimizers 的 sorted 参数必须由调用方按 priority 升序排列，禁止在函数内部排序
condition: "apply_keyword_optimizers"
scope: "tool:edit(*.rs), tool:write(*.rs)"
---

# apply_keyword_optimizers 调用规范

`apply_keyword_optimizers(name, sorted)` 的参数 `sorted` **必须** 由调用方按 `get_priority()` 升序排列后传入。函数内部**不执行**排序。

## 原因

- 该函数是 `collect()` 和 `generate_keywords_for_name()` 的共享实现
- `collect()` 在 per-candidate 循环**外**排序一次，避免重复排序
- 在函数内部排序会破坏调用方的优化（`collect()` 无法复用已排序列表）

## 正确

```rust
// collect() — 循环外排序一次
let mut sorted: Vec<&dyn KeywordOptimizer> = self.keyword_optimizers
    .iter()
    .map(|a| a.as_ref())
    .collect();
sorted.sort_by_key(|op| op.get_priority());

for candidate in candidates.get_candidates_mut() {
    candidate.keywords = Self::apply_keyword_optimizers(&candidate.name, &sorted);
}

// generate_keywords_for_name() — 调试路径，每次自行排序
let mut sorted: Vec<&dyn KeywordOptimizer> = self.keyword_optimizers
    .iter()
    .map(|a| a.as_ref())
    .collect();
sorted.sort_by_key(|op| op.get_priority());
Self::apply_keyword_optimizers(name, &sorted)
```

## 错误

```rust
// ❌ 在 apply_keyword_optimizers 内部排序 — 调用方无法复用已排序列表
fn apply_keyword_optimizers(name: &str, sorted: &[&dyn KeywordOptimizer]) -> Vec<String> {
    // ...内部又 sort 一次...
}

// ❌ 调用 collect() 时每个 candidate 都排序一次
for candidate in candidates.get_candidates_mut() {
    let mut sorted = ...;
    sorted.sort_by_key(...);  // 应提到循环外
    candidate.keywords = Self::apply_keyword_optimizers(&candidate.name, &sorted);
}
```
