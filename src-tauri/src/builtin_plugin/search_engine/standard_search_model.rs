#![allow(dead_code)]
use std::collections::HashMap;
use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, Configurable};
use zerolaunch_plugin_api::{
    CachedCandidateData, ScoreDetail, ScoredCandidate, SearchCandidate, SearchEngine,
};

/// 标准搜索引擎
///
/// 综合使用最短编辑距离、子集匹配、KMP 首字符/子串匹配等算法，
/// 计算每个候选项与用户查询的匹配分数。
pub struct StandardSearchModel {
    core: ComponentCore,
}

impl StandardSearchModel {
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "standard-search-model".to_string(),
                "标准搜索引擎".to_string(),
                "默认的标准模糊搜索算法".to_string(),
                ComponentType::SearchEngine,
                0,
            ),
        }
    }
}

impl Default for StandardSearchModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Configurable for StandardSearchModel {
    fn core(&self) -> &ComponentCore {
        &self.core
    }
}

impl SearchEngine for StandardSearchModel {
    /// 批量计算候选项与查询的匹配分数
    ///
    /// # Arguments
    /// * `candidates` - 缓存的候选数据
    /// * `query` - 用户输入的搜索字符串（已预处理为小写）
    ///
    /// # Returns
    /// * 按原始数据排列的 `ScoredCandidate` 列表，包含详细评分明细
    fn calculate_scores(
        &self,
        candidates: &CachedCandidateData,
        query: &str,
    ) -> Vec<ScoredCandidate> {
        candidates
            .get_candidates()
            .iter()
            .map(|candidate| calculate_candidate_score(candidate, query))
            .collect()
    }
}

/// 计算单个候选项的匹配分数，生成带明细的 ScoredCandidate
///
/// # Arguments
/// * `candidate` - 搜索候选项
/// * `user_input` - 用户输入的搜索字符串
fn calculate_candidate_score(candidate: &SearchCandidate, user_input: &str) -> ScoredCandidate {
    let mut best_score: f64 = -10000.0;
    let mut best_details: Vec<ScoreDetail> = Vec::new();

    for keyword in &candidate.keywords {
        let input_len = user_input.chars().count();
        let target_len = keyword.chars().count();

        // 条件容错：短关键字（<=2字符）严格匹配，长关键字允许多打1字符
        let tolerance = if target_len <= 2 { 0 } else { 1 };
        if target_len + tolerance < input_len {
            continue;
        }

        let mut details: Vec<ScoreDetail> = Vec::new();

        // 1. 最短编辑距离基础分
        let edit_distance_score = shortest_edit_dis(keyword, user_input);
        details.push(ScoreDetail {
            score: edit_distance_score,
            weight: 1.0,
            description: "编辑距离基础分".to_string(),
        });

        let mut score = edit_distance_score;

        // 2. 长度比率调整
        let input_len_f = input_len as f64;
        let target_len_f = target_len as f64;

        let ratio = if input_len > target_len {
            1.0
        } else {
            input_len_f / target_len_f
        };
        let length_ratio_adjustment = adjust_score_log2(ratio);
        score *= length_ratio_adjustment;
        details.push(ScoreDetail {
            score: length_ratio_adjustment,
            weight: 1.0,
            description: "长度比率调整".to_string(),
        });

        // 3. 动态溢出惩罚
        let overflow_penalty = if input_len > target_len {
            let overflow_ratio = (input_len_f - target_len_f) / target_len_f;
            let penalty = (1.0 - overflow_ratio * 0.3).max(0.7);
            score *= penalty;
            penalty
        } else {
            1.0
        };
        details.push(ScoreDetail {
            score: overflow_penalty,
            weight: 1.0,
            description: "溢出惩罚".to_string(),
        });

        // 4. 子集匹配分
        let subset_score = subset_dis(keyword, user_input);
        score += subset_score;
        details.push(ScoreDetail {
            score: subset_score,
            weight: 1.0,
            description: "子集匹配分".to_string(),
        });

        // 5. KMP 首字符+子串匹配分
        let kmp_score = kmp(keyword, user_input);
        score += kmp_score;
        details.push(ScoreDetail {
            score: kmp_score,
            weight: 1.0,
            description: "KMP匹配分".to_string(),
        });

        // 6. 固定偏移
        if candidate.bias.abs() > f64::EPSILON {
            details.push(ScoreDetail {
                score: candidate.bias,
                weight: 1.0,
                description: "固定偏移".to_string(),
            });
        }
        score += candidate.bias;

        if score > best_score {
            best_score = score;
            best_details = details;
        }
    }

    // 如果没有任何 keyword 匹配（best_score 仍为初始值），仅保留 bias
    if best_score <= -10000.0 {
        best_score = candidate.bias;
        best_details = vec![ScoreDetail {
            score: candidate.bias,
            weight: 1.0,
            description: "固定偏移(无匹配)".to_string(),
        }];
    }

    ScoredCandidate {
        candidate_id: candidate.id,
        score: best_score,
        detailed_score: best_details,
    }
}

/// 得分权重调整公式 log2
///
/// # Arguments
/// * `origin_score` - 原始分数，范围 [0.0, 1.0]
///
/// # Returns
/// * 调整后的分数，经 log2 映射放大
fn adjust_score_log2(origin_score: f64) -> f64 {
    3.0 * ((origin_score + 1.0).log2())
}

/// 子集匹配算法
///
/// 计算 `input_name` 中有多少字符属于 `compare_name` 的子集（含重复计数）。
///
/// # Arguments
/// * `compare_name` - 目标字符串
/// * `input_name` - 用户输入字符串
///
/// # Returns
/// * 匹配的字符数（f64）
fn subset_dis(compare_name: &str, input_name: &str) -> f64 {
    let mut compare_chars = HashMap::with_capacity(compare_name.len());

    for c in compare_name.chars() {
        *compare_chars.entry(c).or_insert(0) += 1;
    }

    let mut result = 0;
    for c in input_name.chars() {
        if let Some(count) = compare_chars.get_mut(&c) {
            if *count > 0 {
                result += 1;
                *count -= 1;
            }
        }
    }

    result as f64
}

/// 权重计算最短编辑距离
///
/// 计算 `compare_name` 的某个后缀与 `input_name` 的最短编辑距离，
/// 并将距离转换为分数（距离越小分数越高）。
///
/// # Arguments
/// * `compare_name` - 目标字符串
/// * `input_name` - 用户输入字符串
///
/// # Returns
/// * 基于最短编辑距离的加权分数
fn shortest_edit_dis(compare_name: &str, input_name: &str) -> f64 {
    let compare_chars: Vec<char> = compare_name.chars().collect();
    let input_chars: Vec<char> = input_name.chars().collect();
    let m = compare_chars.len();
    let n = input_chars.len();

    if n == 0 {
        return 1.0;
    }

    let mut prev = vec![0i32; n + 1];
    let mut current = vec![0i32; n + 1];
    let mut min_operations = n as i32;

    for (j, value) in prev.iter_mut().enumerate() {
        *value = j as i32;
    }

    for i in 1..=m {
        current[0] = 0;
        for j in 1..=n {
            let cost = if compare_chars[i - 1] == input_chars[j - 1] {
                0
            } else {
                1
            };
            current[j] = (prev[j - 1] + cost)
                .min(prev[j] + 1)
                .min(current[j - 1] + 1);
        }
        if current[n] < min_operations {
            min_operations = current[n];
        }
        std::mem::swap(&mut prev, &mut current);
    }

    let value = 1.0 - (min_operations as f64 / n as f64);
    adjust_score_log2(n as f64) * (3.0 * value - 2.0).exp()
}

/// KMP 首字符 + 子串匹配
///
/// 计算首字符串连续匹配长度与子串包含匹配的加和分数。
///
/// # Arguments
/// * `compare_name` - 目标字符串
/// * `input_name` - 用户输入字符串
///
/// # Returns
/// * 首字符匹配分 + 子串匹配分
fn kmp(compare_name: &str, input_name: &str) -> f64 {
    let mut ret: f64 = 0.0;

    for (c1, c2) in compare_name.chars().zip(input_name.chars()) {
        if c1 == c2 {
            ret += 1.0;
        } else {
            break;
        }
    }

    if compare_name.contains(input_name) {
        ret += input_name.chars().count() as f64;
    }

    ret
}

use crate::plugin_framework::builtin_registry::SearchEngineEntry;
use std::sync::Arc;

pub(crate) fn build_standard_search_model() -> (Arc<dyn Configurable>, Arc<dyn SearchEngine>) {
    let engine: Arc<dyn SearchEngine> = Arc::new(StandardSearchModel::new());
    let configurable: Arc<dyn Configurable> = engine.clone();
    (configurable, engine)
}

::inventory::submit! {
    SearchEngineEntry {
        component_id: "standard-search-model",
        priority: 0,
        factory: build_standard_search_model,
    }
}
