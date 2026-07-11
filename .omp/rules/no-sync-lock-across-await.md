---
description: 锁守卫生命周期 — 分离式加锁导致 TOCTOU 脏读（sync/async 锁均受影响）
condition: "(Mutex|RwLock)"
scope: "tool:edit(*.rs), tool:write(*.rs)"
interruptMode: never
---

这段代码使用了 `Mutex`/`RwLock`。最隐蔽的陷阱是 **分离式加锁 TOCTOU** —— 编译器无法捕获：

```rust
// ❌ 两次独立读锁 — a 和 b 可能来自不同版本（sync/async 锁均存在此问题）
let a = lock.read().await.field_a.clone();
something().await;
let b = lock.read().await.field_b;
```

**正确**：一次加锁 clone 全部所需字段，块作用域隔离守卫：

```rust
// ✅ 一次加锁，一致性保证
let (a, b) = {
    let guard = lock.read().await;   // 或 lock.read()
    (guard.field_a.clone(), guard.field_b)
}; // guard 释放
something().await;
```

**补充**：`parking_lot`/`std::sync` 守卫是 `!Send`，跨 `.await` 时 `tokio::spawn` 内编译器拒绝（`block_on` 内编译通过但阻塞线程池）。`tokio::sync` 守卫是 `Send`，无此问题。无论哪种锁，都应缩短临界区。
