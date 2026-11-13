use std::sync::Arc;
use zerolaunch_plugin_api::{
    CachedCandidateData, CandidateId, ScoreBooster, ScoredCandidate, SearchCandidate, SearchEngine,
};

/// 搜索管道：单一引擎打分 + 多个增强器顺序修正（与内置引擎/增强器进程内一致；
/// 第三方组件经 RemoteComponent 走 RPC，管道对组件来源无感知）。
/// 引擎可选：未启用任何搜索引擎时候选原样透传（零分），仅由增强器决定排序。
#[derive(Clone)]
pub struct SearchPipeline {
    engine: Option<Arc<dyn SearchEngine>>,
    boosters: Vec<Arc<dyn ScoreBooster>>,
    top_k: usize,
}

impl SearchPipeline {
    pub fn new(
        engine: Arc<dyn SearchEngine>,
        boosters: Vec<Arc<dyn ScoreBooster>>,
        top_k: usize,
    ) -> Self {
        Self {
            engine: Some(engine),
            boosters,
            top_k,
        }
    }

    /// 无引擎管道：候选透传（零分），排序完全由增强器决定。
    pub fn without_engine(boosters: Vec<Arc<dyn ScoreBooster>>, top_k: usize) -> Self {
        Self {
            engine: None,
            boosters,
            top_k,
        }
    }

    /// 执行搜索并截断到 top_k。
    /// 参数：candidates - 候选项缓存；query - 已预处理的查询词。
    /// 返回：按分数降序排列、截断后的 ScoredCandidate 列表。
    pub async fn search(
        &self,
        candidates: &CachedCandidateData,
        query: &str,
    ) -> Vec<ScoredCandidate> {
        self.search_all(candidates, query)
            .await
            .into_iter()
            .take(self.top_k)
            .collect()
    }

    /// 执行全量搜索（不截断 top_k），供调试详情等需要观察完整排序的场景使用。
    /// 参数：candidates - 候选项缓存；query - 已预处理的查询词。
    /// 返回：按分数降序排列的完整 ScoredCandidate 列表。
    pub async fn search_all(
        &self,
        candidates: &CachedCandidateData,
        query: &str,
    ) -> Vec<ScoredCandidate> {
        let mut scored = match &self.engine {
            Some(engine) => engine.calculate_scores(candidates, query).await,
            None => candidates
                .get_candidates()
                .iter()
                .map(|c: &SearchCandidate| ScoredCandidate {
                    candidate_id: c.id,
                    score: 0.0,
                    detailed_score: Vec::new(),
                })
                .collect(),
        };

        for booster in &self.boosters {
            booster.boost(&mut scored, candidates, query).await;
        }

        // total_cmp：NaN 分数（异常引擎/增强器产出）不 panic，按 NaN 最大排序兜底。
        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        scored
    }

    /// 获取当前 top_k 值
    pub fn top_k(&self) -> usize {
        self.top_k
    }

    /// 当前管道使用的搜索引擎（诊断/测试用）；无引擎时返回 None。
    pub fn engine(&self) -> Option<&Arc<dyn SearchEngine>> {
        self.engine.as_ref()
    }

    /// 记录候选项被选中启动，通知所有 ScoreBooster 学习用户习惯
    /// 参数：candidate_id - 被选中的候选项 ID；data - 候选项缓存数据；query - 用户查询词
    pub async fn record(&self, candidate_id: CandidateId, data: &CachedCandidateData, query: &str) {
        for booster in &self.boosters {
            booster.record(candidate_id, data, query).await;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use zerolaunch_plugin_api::config::{
        ComponentCore, ComponentType, Configurable, SettingDefinition,
    };
    use zerolaunch_plugin_api::services::icon_request::IconRequest;
    use zerolaunch_plugin_api::{ExecutionTarget, SearchCandidate};

    /// 无引擎管道：候选零分透传，增强器 boost 仍生效并决定排序。
    #[tokio::test]
    async fn without_engine_passes_candidates_through_and_applies_boosters() {
        struct StubBooster {
            core: ComponentCore,
            boost: Mutex<Option<f64>>,
        }
        #[async_trait]
        impl Configurable for StubBooster {
            fn core(&self) -> &ComponentCore {
                &self.core
            }
            fn setting_schema(&self) -> Vec<SettingDefinition> {
                vec![]
            }
        }
        #[async_trait]
        impl ScoreBooster for StubBooster {
            async fn record(
                &self,
                _candidate_id: CandidateId,
                _data: &CachedCandidateData,
                _query: &str,
            ) {
            }
            async fn boost(
                &self,
                scored: &mut Vec<ScoredCandidate>,
                _data: &CachedCandidateData,
                _query: &str,
            ) {
                let mut guard = self.boost.lock();
                for s in scored.iter_mut() {
                    let add = guard.get_or_insert(0.0);
                    s.score += *add;
                }
            }
        }

        let mut data = CachedCandidateData::new();
        data.add_candidate(SearchCandidate {
            id: 0,
            name: "alpha".to_string(),
            icon: IconRequest::Path("alpha.exe".to_string()),
            target: ExecutionTarget::Path("alpha.exe".to_string()),
            keywords: vec!["alpha".to_string()],
            bias: 0.0,
            trigger_keywords: vec![],
        });
        data.add_candidate(SearchCandidate {
            id: 0,
            name: "beta".to_string(),
            icon: IconRequest::Path("beta.exe".to_string()),
            target: ExecutionTarget::Path("beta.exe".to_string()),
            keywords: vec!["beta".to_string()],
            bias: 0.0,
            trigger_keywords: vec![],
        });

        let booster: Arc<dyn ScoreBooster> = Arc::new(StubBooster {
            core: ComponentCore::new(
                "stub-booster".to_string(),
                "stub".to_string(),
                String::new(),
                ComponentType::ScoreBooster,
                0,
            ),
            boost: Mutex::new(Some(5.0)),
        });
        let pipeline = SearchPipeline::without_engine(vec![booster], 10);
        let result = pipeline.search(&data, "query").await;

        assert_eq!(result.len(), 2, "候选应全部透传");
        assert!(
            result.iter().all(|s| (s.score - 5.0).abs() < 1e-9),
            "增强器加分应生效"
        );
        assert!(result.iter().all(|s| s.detailed_score.is_empty()));
    }
}
