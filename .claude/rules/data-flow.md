---
paths:
  - "src-tauri/src/commands/**"
  - "src-tauri/src/plugin_system/**"
  - "src-tauri/src/core/**"
  - "src-tauri/src/plugin/**"
---

# 数据流规范

> 以下数据流均基于实际代码验证（`src-tauri/src/`）。当文档与代码不一致时，以代码为准。

---

## 搜索数据流

```
用户输入
  → bridge_query (commands/bridge.rs:70)
    → 构造 Query { search_term: raw_query.to_lowercase(), .. }
    → session_router.route_query(&trace_id, &query).await (session_router.rs:116)
        │
        ├─ PluginService.query() 命中插件触发器 (service.rs:54)
        │   → *current_mode = SessionMode::Plugin
        │   → 直接返回 QueryResponse::CustomPanel { .. }
        │
        └─ 未命中 → *current_mode = SessionMode::Search
            → pipeline.search(&cached_candidates, &query.search_term) (search_pipeline.rs:23)
                ├─ engine.calculate_scores(candidates, query)  → Vec<ScoredCandidate>
                ├─ 逐个 booster.boost(&mut scored, candidates, query)
                ├─ scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap())
                └─ scored.into_iter().take(top_k).collect()
            → route_query 将 ScoredCandidate 映射为 ListItem (session_router.rs:141-163)
                ├─ 从 cached_candidates 查找完整 SearchCandidate
                └─ executor_registry 解析可用动作
            → 返回 QueryResponse::List { results }
```

**关键文件**：
- `commands/bridge.rs` — 入口，Tauri command
- `plugin_system/session_router.rs` — 路由：插件 vs 搜索
- `plugin_system/search_pipeline.rs` — 编排：打分 → 提升 → 排序 → 截断
- `plugin_system/types.rs` — SearchEngine、ScoreBooster trait 定义
- `plugin/search_engine/` — 三种搜索引擎实现
- `plugin/score_booster/` — HistoryBooster、QueryAffinityBooster

---

## 候选数据流

```
触发时机：
  - 应用启动时
  - ConfigEvent (DataSource / KeywordOptimizer 配置变更) → handle_config_event (session_router.rs:326)
  - bridge_refresh_candidates 命令主动刷新

SessionRouter.refresh_candidates() (session_router.rs:110)
  → candidate_pipeline.read().await
  → pipeline.collect().await (candidate_pipeline.rs:28)
      ├─ 逐个 data_source 收集候选
      ├─ keyword_optimizers 按 priority 排序
      │   └─ 对每个候选，逐层应用优化器，累加关键词（上下文感知）
      └─ 对每个候选的关键词去重
  → *self.cached_candidates.write() = candidates
```

**关键文件**：
- `plugin_system/candidate_pipeline.rs` — collect() 同时完成采集和关键词优化
- `plugin_system/session_router.rs` — refresh_candidates()、handle_config_event()
- `plugin/data_source/` — 各种数据源实现

---

## 配置数据流

```
前端调用 config_apply_settings (commands/config_file.rs:62)
  → config_manager.apply_settings(&component_id, settings) (manager.rs:140)
      1. validate_settings(&settings)?  ← 失败则在此阻断
      2. apply_settings(settings)?      ← 仅写内部 RwLock
      3. on_settings_changed()          ← 副作用（重建服务、注册回调、启停监听）
      4. 广播 ConfigEvent               ← 无论后续持久化是否成功都会发布
      5. save_to_storage()              ← 本地 JSON + 可选远程 WebDAV

  → ConfigEvent 被 SessionRouter.handle_config_event() 消费
    → DataSource 或 KeywordOptimizer 变更
      → self.refresh_candidates().await   ← 自动重建候选缓存
```

**关键文件**：
- `commands/config_file.rs` — 8 个 config_ 前缀的 Tauri 命令
- `core/config/manager.rs` — apply_settings() 五步流水线
- `core/config/components/` — 各 Configurable 实现
- `plugin_system/session_router.rs` — handle_config_event() 消费事件

---

## 确认执行流

```
bridge_confirm (commands/bridge.rs)
  → session_router.route_confirm() (session_router.rs)
    → executor_registry.execute(ctx, action_id).await
    → search_pipeline.record(candidate_id, query)  ← 学习反馈
        → 逐个 booster.record(id, data, query)      ← HistoryBooster / QueryAffinityBooster
```

**关键文件**：
- `plugin_system/session_router.rs` — route_confirm()
- `plugin_system/executor_registry.rs` — execute() 唯一公开入口
- `plugin_system/search_pipeline.rs` — record() 学习记录
