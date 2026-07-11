use std::collections::HashMap;
use zerolaunch_plugin_api::config::{ComponentType, Configurable};
use zerolaunch_plugin_api::{
    CachedCandidateData, ScoreDetail, ScoredCandidate, SearchCandidate, SearchEngine,
};

/// 这个文件是以LaunchyQT的搜索模型为基础进行的改造
/// 项目地址如下：https://github.com/samsonwang/LaunchyQt
/// 但是launchyqt是基于比较进行搜索的，而不是基于分数的
/// 所以我对这个搜索算法做了一些修改，从而可以适应当前的搜索框架
///
/// Launchy 搜索引擎
///
/// 基于 LaunchyQT 搜索算法的评分策略，将多级比较规则转化为数值分数。
///
/// 评分优先级：
/// 1. 精确匹配：获得最高的基础分
/// 2. 连续子串匹配：获得次高的基础分，匹配位置越靠前分数越高
/// 3. 子集匹配：用户输入的字符是程序名称的子集，获得较低的基础分
/// 4. 名称长度惩罚：作为微小的调整项，用于打破平局
pub struct LaunchySearchModel {}

impl Configurable for LaunchySearchModel {
    fn component_id(&self) -> &str {
        "launchy-search-model"
    }

    fn component_name(&self) -> &str {
        "Launchy 搜索引擎"
    }

    fn component_description(&self) -> &str {
        "兼容 Launchy 风格的搜索算法"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::SearchEngine
    }

    fn default_enabled(&self) -> bool {
        false
    }
}

impl SearchEngine for LaunchySearchModel {
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
            .map(|candidate| calculate_launchy_score(candidate, query))
            .collect()
    }
}

/// 计算单个候选项的 Launchy 匹配分数
///
/// # Arguments
/// * `candidate` - 搜索候选项
/// * `user_input` - 用户输入的搜索字符串
fn calculate_launchy_score(candidate: &SearchCandidate, user_input: &str) -> ScoredCandidate {
    let mut best_score: f64 = -1.0;
    let mut best_details: Vec<ScoreDetail> = Vec::new();

    if user_input.is_empty() {
        return ScoredCandidate {
            candidate_id: candidate.id,
            score: 0.0,
            detailed_score: vec![ScoreDetail {
                score: 0.0,
                weight: 1.0,
                description: "空输入".to_string(),
            }],
        };
    }

    for keyword in &candidate.keywords {
        let mut current_score = -1.0;
        let mut details: Vec<ScoreDetail> = Vec::new();

        if keyword.eq_ignore_ascii_case(user_input) {
            const EXACT_MATCH_BASE_SCORE: f64 = 100_000.0;
            current_score = EXACT_MATCH_BASE_SCORE;
            details.push(ScoreDetail {
                score: EXACT_MATCH_BASE_SCORE,
                weight: 1.0,
                description: "精确匹配".to_string(),
            });
        } else if let Some(start_index) = keyword.to_lowercase().find(&user_input.to_lowercase()) {
            const CONTIGUOUS_MATCH_BASE_SCORE: f64 = 10_000.0;
            let position_penalty = (start_index as f64) * 10.0;
            current_score = CONTIGUOUS_MATCH_BASE_SCORE - position_penalty;
            details.push(ScoreDetail {
                score: CONTIGUOUS_MATCH_BASE_SCORE,
                weight: 1.0,
                description: "连续子串匹配".to_string(),
            });
            details.push(ScoreDetail {
                score: -position_penalty,
                weight: 1.0,
                description: "位置惩罚".to_string(),
            });
        } else {
            let mut compare_chars = HashMap::with_capacity(keyword.len());

            for c in keyword.chars() {
                *compare_chars.entry(c).or_insert(0) += 1;
            }

            let mut result = 0;
            for c in user_input.chars() {
                if let Some(count) = compare_chars.get_mut(&c) {
                    if *count > 0 {
                        result += 1;
                        *count -= 1;
                    }
                }
            }

            if result == user_input.len() {
                const SUBSET_MATCH_BASE_SCORE: f64 = 1_000.0;
                current_score = SUBSET_MATCH_BASE_SCORE;
                details.push(ScoreDetail {
                    score: SUBSET_MATCH_BASE_SCORE,
                    weight: 1.0,
                    description: "子集匹配".to_string(),
                });
            }
        }

        if current_score > -1.0 {
            let length_bonus = 10.0 / (keyword.len() as f64 + 1.0);
            current_score += length_bonus;
            details.push(ScoreDetail {
                score: length_bonus,
                weight: 1.0,
                description: "名称长度加成".to_string(),
            });
        }

        if current_score > best_score {
            best_score = current_score;
            best_details = details;
        }
    }

    if best_score <= -1.0 {
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

use crate::plugin_framework::builtin_registry::SearchEngineEntry;
use std::sync::Arc;

pub(crate) fn build_launchy_search_model() -> (Arc<dyn Configurable>, Arc<dyn SearchEngine>) {
    let engine: Arc<dyn SearchEngine> = Arc::new(LaunchySearchModel {});
    let configurable: Arc<dyn Configurable> = engine.clone();
    (configurable, engine)
}

::inventory::submit! {
    SearchEngineEntry {
        component_id: "launchy-search-model",
        priority: 10,
        factory: build_launchy_search_model,
    }
}
