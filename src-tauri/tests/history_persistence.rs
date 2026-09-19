//! 组件运行态（启动历史 / 查询亲和）持久化的回归测试。
//!
//! 覆盖：重启后统计保留、跨天日期桶按日期对齐、运行态文件损坏时降级、
//! 运行态不混入用户配置。测试走真实持久化链路
//! （ConfigManager → ConfigStore → 运行态文件 / 配置文件）。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::{Duration, Local};
use serde_json::json;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    CachedCandidateData, ExecutionTarget, ScoreBooster, ScoredCandidate, SearchCandidate,
};
use zerolaunch_rs_lib::builtin_plugin::score_booster::history_booster::HistoryBooster;
use zerolaunch_rs_lib::builtin_plugin::score_booster::query_affinity::QueryAffinityBooster;
use zerolaunch_rs_lib::core::config::ConfigManager;

/// 被记录的启动目标（作为运行态中的键）
const TARGET: &str = "C:/Program Files/Tencent/QQ/QQ.exe";
/// 记录时用户输入的查询
const QUERY: &str = "qq";
/// 候选项基础分：历史增强器的基础分抑制因子在 15 分处为 1.0（不抑制）
const BASE_SCORE: f64 = 15.0;

/// 构造含单个候选（id = 1）的缓存快照
fn cache_with_target() -> CachedCandidateData {
    let mut cache = CachedCandidateData::new();
    cache.add_candidate(SearchCandidate {
        id: 0,
        name: "QQ".to_string(),
        icon: IconRequest::Path(TARGET.to_string()),
        target: ExecutionTarget::Path(TARGET.to_string()),
        keywords: vec![QUERY.to_string()],
        bias: 0.0,
        trigger_keywords: Vec::new(),
    });
    cache
}

/// 构造基础分候选项（与 `cache_with_target` 的候选 id 对应）
fn scored() -> Vec<ScoredCandidate> {
    vec![ScoredCandidate {
        candidate_id: 1,
        score: BASE_SCORE,
        detailed_score: Vec::new(),
    }]
}

/// 读取候选项指定明细项的分值
fn detail_score(candidate: &ScoredCandidate, description: &str) -> f64 {
    candidate
        .detailed_score
        .iter()
        .find(|detail| detail.description == description)
        .unwrap_or_else(|| panic!("候选项缺少明细项: {description}"))
        .score
}

/// 生成相对今天偏移若干天的日期字符串（本地时区）
fn date_offset(days: i64) -> String {
    (Local::now().date_naive() + Duration::days(days))
        .format("%Y-%m-%d")
        .to_string()
}

/// 写入（或合并进）运行态文件，模拟上次会话落盘的内容
fn write_runtime_state(dir: &Path, component_id: &str, state: serde_json::Value) {
    let path = dir.join("runtime_state.json");
    let mut all: HashMap<String, serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default();
    all.insert(component_id.to_string(), state);
    std::fs::write(&path, serde_json::to_string_pretty(&all).unwrap()).unwrap();
}

/// 启动历史：会话内记录 → 重启后仍参与加成
#[tokio::test]
async fn history_booster_state_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_target();

    let cm1 = ConfigManager::new(dir.path().to_path_buf());
    let booster1 = Arc::new(HistoryBooster::new());
    cm1.register(booster1.clone() as Arc<dyn Configurable>)
        .await;
    booster1.record(1, &cache, QUERY).await;
    cm1.flush_runtime_state();
    assert!(
        dir.path().join("runtime_state.json").exists(),
        "记录后应写入运行态文件"
    );

    let mut before = scored();
    booster1.boost(&mut before, &cache, QUERY).await;

    // 会话 2：重启（新实例 + load_from_storage）
    let cm2 = ConfigManager::new(dir.path().to_path_buf());
    let booster2 = Arc::new(HistoryBooster::new());
    cm2.register(booster2.clone() as Arc<dyn Configurable>)
        .await;
    cm2.load_from_storage().await.unwrap();

    let mut after = scored();
    booster2.boost(&mut after, &cache, QUERY).await;

    assert!(
        (after[0].score - before[0].score).abs() < 1e-3,
        "重启后历史加成应与会话内一致: 会话内 {} / 重启后 {}",
        before[0].score,
        after[0].score
    );
    assert!(after[0].score > BASE_SCORE, "重启后应保留历史加分");
    assert!(detail_score(&after[0], "历史启动分数") > 0.0);
    assert!(detail_score(&after[0], "近期习惯分数") > 0.0);
}

/// 查询亲和：会话内记录 → 重启后仍参与加成
#[tokio::test]
async fn query_affinity_state_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_target();

    let cm1 = ConfigManager::new(dir.path().to_path_buf());
    let booster1 = Arc::new(QueryAffinityBooster::new());
    cm1.register(booster1.clone() as Arc<dyn Configurable>)
        .await;
    booster1.record(1, &cache, QUERY).await;
    cm1.flush_runtime_state();

    let mut before = scored();
    booster1.boost(&mut before, &cache, QUERY).await;

    let cm2 = ConfigManager::new(dir.path().to_path_buf());
    let booster2 = Arc::new(QueryAffinityBooster::new());
    cm2.register(booster2.clone() as Arc<dyn Configurable>)
        .await;
    cm2.load_from_storage().await.unwrap();

    let mut after = scored();
    booster2.boost(&mut after, &cache, QUERY).await;

    assert!(
        (after[0].score - before[0].score).abs() < 1e-3,
        "重启后查询亲和加成应与会话内一致: 会话内 {} / 重启后 {}",
        before[0].score,
        after[0].score
    );
    assert!(after[0].score > BASE_SCORE, "重启后应保留查询亲和加分");
}

/// 昨天的日期桶在恢复后落在第 2 位（衰减 1/1.3），过期桶整体作废
#[tokio::test]
async fn date_buckets_are_aligned_on_restore() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_target();
    write_runtime_state(
        dir.path(),
        "history-booster",
        json!({
            "launch_time": [{ "date": date_offset(-1), "counts": { (TARGET): 3 } }],
            "history_launch_time": { (TARGET): 3 },
            "latest_launch_time": {},
        }),
    );

    let cm = ConfigManager::new(dir.path().to_path_buf());
    let booster = Arc::new(HistoryBooster::new());
    cm.register(booster.clone() as Arc<dyn Configurable>).await;
    cm.load_from_storage().await.unwrap();

    // 今天记录一次启动：今日桶 1 次（权重 1.0）+ 昨日桶 3 次（权重 1/1.3）
    booster.record(1, &cache, QUERY).await;
    let mut candidates = scored();
    booster.boost(&mut candidates, &cache, QUERY).await;

    let expected_recent_habit = 1.0 + 3.0 / 1.3;
    let actual = detail_score(&candidates[0], "近期习惯分数");
    assert!(
        (actual - expected_recent_habit).abs() < 1e-6,
        "昨日桶应按第 2 位衰减计入: 期望 {expected_recent_habit} / 实际 {actual}"
    );
}

/// 跨度超过 7 天的桶整体作废，历史总次数仍保留
#[tokio::test]
async fn expired_buckets_are_dropped_on_restore() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_target();
    write_runtime_state(
        dir.path(),
        "history-booster",
        json!({
            "launch_time": [{ "date": date_offset(-30), "counts": { (TARGET): 5 } }],
            "history_launch_time": { (TARGET): 5 },
            "latest_launch_time": {},
        }),
    );

    let cm = ConfigManager::new(dir.path().to_path_buf());
    let booster = Arc::new(HistoryBooster::new());
    cm.register(booster.clone() as Arc<dyn Configurable>).await;
    cm.load_from_storage().await.unwrap();

    let mut candidates = scored();
    booster.boost(&mut candidates, &cache, QUERY).await;
    assert_eq!(
        detail_score(&candidates[0], "近期习惯分数"),
        0.0,
        "超过 7 天的桶不得计入近期习惯分"
    );
    assert!(
        detail_score(&candidates[0], "历史启动分数") > 0.0,
        "历史总次数与日期无关，应保留"
    );
}

/// 运行态文件损坏：告警并降级为空统计，不影响配置项加载
#[tokio::test]
async fn corrupted_runtime_state_degrades_to_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("runtime_state.json"), "{not json").unwrap();
    std::fs::write(
        dir.path().join("zerolaunch_config.json"),
        serde_json::to_string(&json!({
            "version": "3",
            "components": {
                "history-booster": {
                    "enabled": true,
                    "settings": {
                        "history_weight": 2.5,
                        "recent_habit_weight": 1.5,
                        "temporal_weight": 0.5,
                        "temporal_decay": 10800.0,
                    },
                },
            },
        }))
        .unwrap(),
    )
    .unwrap();

    let cm = ConfigManager::new(dir.path().to_path_buf());
    let booster = Arc::new(HistoryBooster::new());
    cm.register(booster.clone() as Arc<dyn Configurable>).await;
    cm.load_from_storage().await.unwrap();

    assert_eq!(
        booster.get_settings()["history_weight"],
        2.5,
        "运行态损坏不得影响用户配置加载"
    );

    let cache = cache_with_target();
    let mut candidates = scored();
    booster.boost(&mut candidates, &cache, QUERY).await;
    assert_eq!(candidates[0].score, BASE_SCORE, "损坏运行态应降级为空统计");
    assert_eq!(detail_score(&candidates[0], "历史启动分数"), 0.0);
}

/// 运行态只写独立文件，不混入用户配置
#[tokio::test]
async fn runtime_state_stays_out_of_user_config() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_target();

    let cm = ConfigManager::new(dir.path().to_path_buf());
    let booster = Arc::new(HistoryBooster::new());
    cm.register(booster.clone() as Arc<dyn Configurable>).await;
    booster.record(1, &cache, QUERY).await;
    cm.save_to_storage().unwrap();
    cm.flush_runtime_state();

    let config_content =
        std::fs::read_to_string(dir.path().join("zerolaunch_config.json")).unwrap();
    assert!(
        !config_content.contains("launch_time"),
        "运行态不得写入用户配置文件"
    );

    let persisted = cm.build_persistent_config();
    let settings = persisted.components["history-booster"]
        .settings
        .as_object()
        .unwrap();
    assert_eq!(
        settings.len(),
        4,
        "组件 settings 应只含权重字段，实际: {:?}",
        settings.keys().collect::<Vec<_>>()
    );

    let runtime_content = std::fs::read_to_string(dir.path().join("runtime_state.json")).unwrap();
    assert!(
        runtime_content.contains("history_launch_time"),
        "运行态应写入独立运行态文件"
    );
}
