use crate::plugin_system::types::{
    ComponentType, Configurable, ScoreDetail, ScoredCandidate, SearchEngine,
};
use crate::plugin_system::CachedCandidateData;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use parking_lot::RwLock;

/// Skim 搜索引擎
///
/// 基于 Skim 模糊匹配算法的评分策略，使用 SkimMatcherV2 进行模糊匹配。
/// 适用于需要灵活模糊匹配的场景。
pub struct SkimSearchModel {
    matcher: RwLock<SkimMatcherV2>,
}

impl SkimSearchModel {
    pub fn new() -> Self {
        SkimSearchModel {
            matcher: RwLock::new(SkimMatcherV2::default()),
        }
    }
}

impl Default for SkimSearchModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Configurable for SkimSearchModel {
    fn component_id(&self) -> &str {
        "skim-search-model"
    }

    fn component_name(&self) -> &str {
        "Skim 搜索引擎"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::SearchEngine
    }

    fn default_enabled(&self) -> bool {
        false
    }
}

impl SearchEngine for SkimSearchModel {
    /// 批量计算候选项与查询的匹配分数
    ///
    /// # Arguments
    /// * `candidates` - 缓存的候选数据
    /// * `query` - 用户输入的搜索字符串
    ///
    /// # Returns
    /// * 按原始数据排列的 `ScoredCandidate` 列表
    fn calculate_scores(
        &self,
        candidates: &CachedCandidateData,
        query: &str,
    ) -> Vec<ScoredCandidate> {
        candidates
            .get_candidates()
            .iter()
            .map(|candidate| calculate_skim_score(&self.matcher, candidate, query))
            .collect()
    }
}

/// 计算单个候选项的 Skim 模糊匹配分数
///
/// # Arguments
/// * `matcher` - Skim 模糊匹配器
/// * `candidate` - 搜索候选项
/// * `user_input` - 用户输入的搜索字符串
fn calculate_skim_score(
    matcher: &RwLock<SkimMatcherV2>,
    candidate: &crate::plugin_system::types::SearchCandidate,
    user_input: &str,
) -> ScoredCandidate {
    let mut best_score: f64 = -10000.0;
    let mut best_details: Vec<ScoreDetail> = Vec::new();

    let input_len = user_input.chars().count();

    for keyword in &candidate.keywords {
        let target_len = keyword.chars().count();

        if target_len < input_len {
            continue;
        }

        let mut details: Vec<ScoreDetail> = Vec::new();

        let score = matcher.read().fuzzy_match(keyword, user_input);
        if let Some(s) = score {
            let fuzzy_score = s as f64;
            details.push(ScoreDetail {
                score: fuzzy_score,
                weight: 1.0,
                description: "Skim 模糊匹配分".to_string(),
            });

            if fuzzy_score > best_score {
                best_score = fuzzy_score;
                best_details = details;
            }
        }
    }

    if best_score <= -10000.0 {
        best_score = candidate.bias;
        best_details = vec![ScoreDetail {
            score: candidate.bias,
            weight: 1.0,
            description: "固定偏移(无匹配)".to_string(),
        }];
    } else if candidate.bias.abs() > f64::EPSILON {
        best_details.push(ScoreDetail {
            score: candidate.bias,
            weight: 1.0,
            description: "固定偏移".to_string(),
        });
        best_score += candidate.bias;
    }

    ScoredCandidate {
        candidate_id: candidate.id,
        score: best_score,
        detailed_score: best_details,
    }
}

use crate::plugin_system::builtin_registry::SearchEngineEntry;
use std::sync::Arc;

pub(crate) fn build_skim_search_model() -> (Arc<dyn Configurable>, Arc<dyn SearchEngine>) {
    let engine: Arc<dyn SearchEngine> = Arc::new(SkimSearchModel::new());
    let configurable: Arc<dyn Configurable> = engine.clone();
    (configurable, engine)
}

::inventory::submit! {
    SearchEngineEntry {
        component_id: "skim-search-model",
        priority: 20,
        factory: build_skim_search_model,
    }
}
