use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, enabled, Level};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    CachedCandidateData, ScoreDetail, ScoreDetailKind, ScoredCandidate, SearchCandidate,
    SearchEngine,
};

use crate::core::config::setting_builders::SchemaBuilder;

/// 标准搜索引擎
///
/// 综合使用最短编辑距离、BM25 子集匹配、KMP 首字符/子串匹配等算法，
/// 计算每个候选项与用户查询的匹配分数。
pub struct StandardSearchModel {
    core: ComponentCore,
    /// 用户可调配置（BM25 参数）；仅在 `apply_settings` 时写入。
    settings: RwLock<StandardSearchSettings>,
    /// 语料统计缓存；`None` 表示本次进程内尚未构建过。
    /// 需要内部可变性是因为 `calculate_scores` 只拿到 `&self`。
    stats: RwLock<Option<Arc<CorpusStats>>>,
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
            settings: RwLock::new(StandardSearchSettings::default()),
            stats: RwLock::new(None),
        }
    }

    /// 取候选缓存当前世代对应的语料统计：命中缓存直接复用，未命中则构建。
    ///
    /// 统计构建成本与「全部 keyword 的总字符数」成正比，只在候选缓存刷新后发生一次；
    /// 查询路径本身只查表。
    ///
    /// 并发安全（`calculate_scores` 会被线程池并发调用，管道是 clone 出守卫后再 await 的）：
    /// - 命中路径只读缓存，不写共享状态；
    /// - 未命中时在写锁内二次判定，并发首查只发生一次真实构建，其余线程复用其结果；
    /// - 缓存只被「不更旧」的世代覆盖 —— 世代单调递增（`bump_generation` 只自增），
    ///   仍持有旧世代快照的线程照常拿到与自身候选列表匹配的统计，但不会顶掉新世代缓存，
    ///   否则当前世代的查询会被反复重建（A/B 抖动）。
    fn corpus_stats(&self, candidates: &CachedCandidateData) -> Arc<CorpusStats> {
        let generation = candidates.generation();
        {
            let cached = self.stats.read();
            if let Some(stats) = cached.as_ref() {
                if stats.generation == generation {
                    return Arc::clone(stats);
                }
            }
        }
        let mut cache = self.stats.write();
        if let Some(stats) = cache.as_ref() {
            if stats.generation == generation {
                return Arc::clone(stats);
            }
        }
        let built = Arc::new(CorpusStats::build(candidates.get_candidates(), generation));
        let should_store = match cache.as_ref() {
            Some(stats) => stats.generation < generation,
            None => true,
        };
        if should_store {
            *cache = Some(Arc::clone(&built));
        }
        built
    }
}

impl Default for StandardSearchModel {
    fn default() -> Self {
        Self::new()
    }
}

/// 标准搜索引擎的强类型配置结构。
///
/// 反序列化自配置文件中 "standard-search-model" 组件的 settings 段；
/// 经 `ConfigManager::apply_settings` 写入后，`SettingsChanged` 事件驱动
/// `SessionDispatcher` 重建搜索管道使新参数生效。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardSearchSettings {
    /// BM25 词频饱和系数 k1（越大词频收益越晚饱和）；默认 1.2。
    #[serde(rename = "bm25_k1", default = "default_bm25_k1")]
    pub bm25_k1: f64,
    /// BM25 文档长度归一化系数 b（0 = 关闭归一化，1 = 完全归一化）；默认 0.3。
    #[serde(rename = "bm25_b", default = "default_bm25_b")]
    pub bm25_b: f64,
}

impl Default for StandardSearchSettings {
    fn default() -> Self {
        Self {
            bm25_k1: default_bm25_k1(),
            bm25_b: default_bm25_b(),
        }
    }
}

/// 默认词频饱和系数：BM25 经典取值。
fn default_bm25_k1() -> f64 {
    1.2
}

/// 默认长度归一化系数：低于 BM25 经典的 0.75 —— 应用名短、长度方差小，
/// 且长度已被长度比率调整与溢出惩罚连罚两轮。
fn default_bm25_b() -> f64 {
    0.3
}

#[async_trait]
impl Configurable for StandardSearchModel {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "bm25_k1",
                t_key!("standard-search-model", "fields.bm25_k1.label"),
                t_key!("standard-search-model", "fields.bm25_k1.desc"),
            )
            .group(t_key!("standard-search-model", "groups.bm25"))
            .order(0)
            .default(1.2)
            .min(0.0)
            .max(5.0)
            .step(0.1)
            .build(),
            SchemaBuilder::number(
                "bm25_b",
                t_key!("standard-search-model", "fields.bm25_b.label"),
                t_key!("standard-search-model", "fields.bm25_b.desc"),
            )
            .group(t_key!("standard-search-model", "groups.bm25"))
            .order(1)
            .default(0.3)
            .min(0.0)
            .max(1.0)
            .step(0.05)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: StandardSearchSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }
}

#[async_trait]
impl SearchEngine for StandardSearchModel {
    /// 批量计算候选项与查询的匹配分数
    ///
    /// # Arguments
    /// * `candidates` - 缓存的候选数据
    /// * `query` - 用户输入的搜索字符串（已预处理为小写）
    ///
    /// # Returns
    /// * 按原始数据排列的 `ScoredCandidate` 列表，包含详细评分明细
    async fn calculate_scores(
        &self,
        candidates: &CachedCandidateData,
        query: &str,
    ) -> Vec<ScoredCandidate> {
        let (k1, b) = {
            let settings = self.settings.read();
            (settings.bm25_k1, settings.bm25_b)
        };
        let stats = self.corpus_stats(candidates);
        let mut scorer = QueryScorer::new(query, &stats, k1, b);
        log_corpus_stats(query, &scorer, &stats);
        candidates
            .get_candidates()
            .iter()
            .map(|candidate| scorer.score(candidate))
            .collect()
    }
}

/// 语料/查询项明细日志（仅在 DEBUG 级别构建字符串）。
///
/// BM25 引入语料依赖后分数不再是 `(query, keyword)` 的纯函数，失败案例需要
/// df/idf 明细才能归因；明细不进 `ScoreDetail.description`——前端按 description
/// 去重生成动态分数列（`DebugTools.vue`），动态串会让列集合随查询漂移。
fn log_corpus_stats(query: &str, scorer: &QueryScorer<'_>, stats: &CorpusStats) {
    if !enabled!(Level::DEBUG) {
        return;
    }
    let mut terms: Vec<(String, u32, f64)> = scorer
        .weights
        .iter()
        .map(|(term, weight)| {
            (
                term_label(*term),
                stats.df.get(term).copied().unwrap_or(0),
                *weight,
            )
        })
        .collect();
    terms.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let detail = terms
        .iter()
        .map(|(label, df, weight)| format!("{label}(df={df},idf={weight:.2})"))
        .collect::<Vec<_>>()
        .join(" ");
    debug!(
        "BM25 子集分: N={}, avgdl={:.2}, 查询 \"{}\" 的 {} 个 term: {}",
        stats.doc_count,
        stats.avg_len,
        query,
        terms.len(),
        detail
    );
}

/// 词频计数缓冲：单次查询内跨 keyword 复用，避免每个 keyword 重新分配计数表。
/// 仅限本文件内使用。
struct TermCounts {
    /// 已计入的 term 与词频（前 `len` 项有效）。
    entries: Vec<(u64, u32)>,
    /// 本轮有效条目数。
    len: usize,
}

impl TermCounts {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            len: 0,
        }
    }

    /// 清空本轮计数（保留已分配容量）。
    fn reset(&mut self) {
        self.len = 0;
    }

    /// 累加一次词频；表内条目数不超过查询项数，线性查找成本可忽略。
    fn bump(&mut self, term: u64) {
        for i in 0..self.len {
            if self.entries[i].0 == term {
                self.entries[i].1 += 1;
                return;
            }
        }
        if self.len == self.entries.len() {
            self.entries.push((term, 1));
        } else {
            self.entries[self.len] = (term, 1);
        }
        self.len += 1;
    }

    /// 本轮已计入的 term 与词频。
    fn iter(&self) -> impl Iterator<Item = (u64, u32)> + '_ {
        self.entries[..self.len].iter().copied()
    }
}

/// 单次查询的打分上下文：查询项权重、语料统计、BM25 参数与复用缓冲。
///
/// 仅限本文件内使用，在 `calculate_scores` 中按查询构建一次，随后逐个候选项复用：
/// 查询项 idf 与查询长度只算一次，逐 keyword 只做一次字符窗扫描。
struct QueryScorer<'a> {
    /// 候选缓存对应的语料统计（df / N / avgdl）。
    stats: &'a CorpusStats,
    /// 查询 bigram term → idf。
    weights: HashMap<u64, f64>,
    /// 查询项 idf 之和（全部命中时的上界权重）。
    ideal: f64,
    /// 查询字符数（长度比率与子集分量纲归一化用）。
    input_len: usize,
    /// 查询词原文（编辑距离与 KMP 用）。
    query: &'a str,
    /// BM25 词频饱和系数 k1。
    k1: f64,
    /// BM25 文档长度归一化系数 b。
    b: f64,
    /// 词频计数缓冲（跨 keyword 复用）。
    counts: TermCounts,
}

impl<'a> QueryScorer<'a> {
    /// 构建查询上下文：查询项权重只算一次，供全部候选复用。
    fn new(query: &'a str, stats: &'a CorpusStats, k1: f64, b: f64) -> Self {
        let mut weights: HashMap<u64, f64> = HashMap::new();
        let mut ideal = 0.0;
        for_each_bigram(query, |term| {
            if weights.contains_key(&term) {
                return;
            }
            let weight = stats.idf(term);
            weights.insert(term, weight);
            ideal += weight;
        });
        Self {
            stats,
            weights,
            ideal,
            input_len: query.chars().count(),
            query,
            k1,
            b,
            counts: TermCounts::new(),
        }
    }

    /// 计算单个候选项的匹配分数，生成带明细的 ScoredCandidate
    ///
    /// # Arguments
    /// * `candidate` - 搜索候选项
    ///
    /// # Returns
    /// * 候选项取其全部 keyword 的最高分；所有 keyword 都被容错门过滤时只保留固定偏移
    fn score(&mut self, candidate: &SearchCandidate) -> ScoredCandidate {
        let mut best_score: f64 = -10000.0;
        let mut best_details: Vec<ScoreDetail> = Vec::new();

        for keyword in &candidate.keywords {
            let input_len = self.input_len;
            let target_len = keyword.chars().count();

            // 条件容错：短关键字（<=2字符）严格匹配，长关键字允许多打1字符
            let tolerance = if target_len <= 2 { 0 } else { 1 };
            if target_len + tolerance < input_len {
                continue;
            }

            let mut details: Vec<ScoreDetail> = Vec::new();

            // 1. 最短编辑距离基础分
            let edit_distance_score = shortest_edit_dis(keyword, self.query);
            details.push(ScoreDetail {
                score: edit_distance_score,
                weight: 1.0,
                description: "编辑距离基础分".to_string(),
                kind: ScoreDetailKind::Add,
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
                kind: ScoreDetailKind::Multiply,
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
                kind: ScoreDetailKind::Multiply,
            });

            // 4. BM25 子集匹配分（命中 bigram 的稀有度加权，取代旧的字符多重集计数）
            let bm25_subset_score = self.subset_score(keyword);
            score += bm25_subset_score;
            details.push(ScoreDetail {
                score: bm25_subset_score,
                weight: 1.0,
                description: "BM25子集分".to_string(),
                kind: ScoreDetailKind::Add,
            });

            // 5. KMP 首字符+子串匹配分
            let kmp_score = kmp(keyword, self.query);
            score += kmp_score;
            details.push(ScoreDetail {
                score: kmp_score,
                weight: 1.0,
                description: "KMP匹配分".to_string(),
                kind: ScoreDetailKind::Add,
            });

            // 6. 固定偏移
            if candidate.bias.abs() > f64::EPSILON {
                details.push(ScoreDetail {
                    score: candidate.bias,
                    weight: 1.0,
                    description: "固定偏移".to_string(),
                    kind: ScoreDetailKind::Add,
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
                kind: ScoreDetailKind::Add,
            }];
        }

        ScoredCandidate {
            candidate_id: candidate.id,
            score: best_score,
            detailed_score: best_details,
        }
    }

    /// BM25 子集分：查询 bigram 在 keyword 文档上的加权覆盖率 × 查询长度。
    ///
    /// 上界为 `input_len × (k1 + 1) / (1 + k1·(1 - b))`（词频为 1、文档长度趋 0 时取到），
    /// 与旧子集分同量纲：都是「按查询长度缩放的绝对分」，故下游增强器的既有标定不受影响。
    /// 空查询（无 term）或 keyword 无字符时返回 0。
    fn subset_score(&mut self, keyword: &str) -> f64 {
        if self.ideal <= 0.0 {
            return 0.0;
        }
        let (k1, b, avg_len, ideal, input_len) = (
            self.k1,
            self.b,
            self.stats.avg_len,
            self.ideal,
            self.input_len as f64,
        );
        let weights = &self.weights;
        let counts = &mut self.counts;
        counts.reset();
        let doc_len = for_each_bigram(keyword, |term| {
            if weights.contains_key(&term) {
                counts.bump(term);
            }
        });
        if doc_len == 0 {
            return 0.0;
        }
        let norm = 1.0 - b + b * doc_len as f64 / avg_len;
        let mut raw = 0.0;
        for (term, freq) in counts.iter() {
            let weight = weights[&term];
            let freq = freq as f64;
            raw += weight * freq * (k1 + 1.0) / (freq + k1 * norm);
        }
        raw / ideal * input_len
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

/// 文档/查询 bigram 的起始哨兵（不出现在普通文本中）。
const TERM_START: char = '\u{1}';
/// 文档/查询 bigram 的结束哨兵（不出现在普通文本中）。
const TERM_END: char = '\u{2}';

/// 语料统计：以「全部候选的全部 keyword」为语料，每个 keyword 视为一篇文档，
/// term 为带首尾边界哨兵的字符二元组（bigram）。
///
/// 仅限本文件内使用，由 `StandardSearchModel::corpus_stats` 按候选缓存世代构建。
struct CorpusStats {
    /// bigram term → 含该 term 的文档数（df）。
    df: HashMap<u64, u32>,
    /// 文档总数 N。
    doc_count: f64,
    /// 平均文档长度 avgdl（文档长度 = bigram 数），文档数为 0 时取 1 避免除零。
    avg_len: f64,
    /// 统计对应的候选缓存世代，用于复用判定。
    generation: u64,
}

impl CorpusStats {
    /// 遍历全部候选的全部 keyword 构建统计；剔除空白后无字符的 keyword 不计入语料。
    fn build(candidates: &[SearchCandidate], generation: u64) -> Self {
        let mut df: HashMap<u64, u32> = HashMap::new();
        let mut doc_terms: HashSet<u64> = HashSet::new();
        let mut doc_count = 0u64;
        let mut total_len = 0u64;
        for candidate in candidates {
            for keyword in &candidate.keywords {
                doc_terms.clear();
                let len = for_each_bigram(keyword, |term| {
                    doc_terms.insert(term);
                });
                if len == 0 {
                    continue;
                }
                doc_count += 1;
                total_len += len as u64;
                for term in &doc_terms {
                    *df.entry(*term).or_insert(0) += 1;
                }
            }
        }
        Self {
            df,
            doc_count: doc_count as f64,
            avg_len: if doc_count == 0 {
                1.0
            } else {
                total_len as f64 / doc_count as f64
            },
            generation,
        }
    }

    /// BM25 平滑 IDF（恒正）：`ln(1 + (N - df + 0.5) / (df + 0.5))`。
    ///
    /// 不用经典 `ln(N / df)`：后者在 `df > N/2` 时为负，会让「命中常见 bigram」反而扣分，
    /// 在应用名这类短文本语料下极易触发。
    fn idf(&self, term: u64) -> f64 {
        let df = self.df.get(&term).copied().unwrap_or(0) as f64;
        (1.0 + (self.doc_count - df + 0.5) / (df + 0.5)).ln()
    }
}

/// 打包 bigram 为 u64 键：高 32 位前一字符、低 32 位后一字符。
fn pack_bigram(left: char, right: char) -> u64 {
    ((left as u64) << 32) | right as u64
}

/// 按字符相邻对遍历 bigram，返回 bigram 数（不分配内存）。
///
/// 先剔除空白字符，再补首尾边界哨兵：剔除空白让「nodejscmd」这类无空格输入与
/// 「Node.js Command Prompt」这类含空格名对齐；边界哨兵让首字符与末字符位置携带位置信息
/// （`^p` / `p$` 与中间的 `ps` 是不同 term），弥补字符多重集丢失的顺序信息。
fn for_each_bigram(text: &str, mut emit: impl FnMut(u64)) -> usize {
    let mut prev: Option<char> = None;
    let mut last: Option<char> = None;
    let mut count = 0;
    for c in text.chars() {
        if c.is_whitespace() {
            continue;
        }
        emit(match prev {
            Some(p) => pack_bigram(p, c),
            None => pack_bigram(TERM_START, c),
        });
        count += 1;
        prev = Some(c);
        last = Some(c);
    }
    if let Some(l) = last {
        emit(pack_bigram(l, TERM_END));
        count += 1;
    }
    count
}

/// term 的可读形式（日志用）：起始哨兵显示为 `^`、结束哨兵显示为 `$`。
fn term_label(term: u64) -> String {
    let render = |c: char| match c {
        TERM_START => '^',
        TERM_END => '$',
        other => other,
    };
    let left = char::from_u32((term >> 32) as u32).unwrap_or('?');
    let right = char::from_u32((term & 0xFFFF_FFFF) as u32).unwrap_or('?');
    format!("{}{}", render(left), render(right))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::thread;
    use zerolaunch_plugin_api::services::icon_request::IconRequest;
    use zerolaunch_plugin_api::ExecutionTarget;

    /// 构造候选项：keywords 即本次测试关注的关键词集合。
    fn candidate(name: &str, keywords: &[&str]) -> SearchCandidate {
        SearchCandidate {
            id: 0,
            name: name.to_string(),
            icon: IconRequest::Path(format!("{name}.exe")),
            target: ExecutionTarget::Path(format!("{name}.exe")),
            keywords: keywords.iter().map(|k| k.to_string()).collect(),
            bias: 0.0,
            trigger_keywords: vec![],
        }
    }

    /// 用 50 个填充候选 + 目标候选构造语料统计，避免 N 过小时 IDF 失真。
    fn stats_with(target: SearchCandidate) -> CorpusStats {
        let mut candidates: Vec<SearchCandidate> = (0..50)
            .map(|i| {
                candidate(
                    &format!("filler-app-{i}"),
                    &[&format!("filler{i}"), &format!("fa{i}")],
                )
            })
            .collect();
        candidates.push(target);
        CorpusStats::build(&candidates, 1)
    }

    /// 子集分不再奖励「字符多重集」噪声：`snipaste` 对查询 `ps` 由 2.0 降为 0。
    #[test]
    fn bm25_subset_rejects_letter_bag_matches() {
        let stats = stats_with(candidate("Snipaste", &["snipaste"]));
        let mut scorer = QueryScorer::new("ps", &stats, 1.2, 0.3);
        assert_eq!(scorer.subset_score("snipaste"), 0.0);

        let mut scorer = QueryScorer::new("ps", &stats, 1.2, 0.3);
        let matched = scorer.subset_score("ps");
        assert!(matched >= 2.0, "命中全部 bigram 应接近满分，实际 {matched}");
    }

    /// 子集分非负且不超过文档化上界 `input_len × (k1 + 1) / (1 + k1·(1 - b))`。
    #[test]
    fn bm25_subset_stays_within_documented_bound() {
        let stats = stats_with(candidate(
            "Visual Studio Code",
            &["visualstudiocode", "vsc", "v"],
        ));
        let cases: [(&str, &[&str]); 4] = [
            ("vsc", &["visualstudiocode", "vsc", "v"]),
            ("visualstudiocode", &["visual studio code"]),
            ("ps", &["wps", "ps"]),
            ("", &["vsc"]),
        ];
        for (query, keywords) in cases {
            let input_len = query.chars().count() as f64;
            let bound = input_len * (1.2 + 1.0) / (1.0 + 1.2 * (1.0 - 0.3)) + 1e-9;
            let mut scorer = QueryScorer::new(query, &stats, 1.2, 0.3);
            for keyword in keywords {
                let score = scorer.subset_score(keyword);
                assert!(
                    (0.0..=bound).contains(&score),
                    "query={query:?} keyword={keyword:?} score={score} 超出上界 {bound}"
                );
            }
        }
    }

    /// 空查询：子集分恒 0，总分有限（不会因除零产生 NaN 被排序顶到首位）。
    #[test]
    fn empty_query_yields_zero_subset() {
        let stats = stats_with(candidate("Snipaste", &["snipaste"]));
        let mut scorer = QueryScorer::new("", &stats, 1.2, 0.3);
        assert_eq!(scorer.subset_score("snipaste"), 0.0);
        let scored = scorer.score(&candidate("Snipaste", &["snipaste"]));
        assert!(scored.score.is_finite());
    }

    /// 单字符查询与 CJK 查询按 bigram 正常计分；无空格输入与含空格关键词对齐。
    #[test]
    fn bigram_handles_short_cjk_and_spacing() {
        let stats = stats_with(candidate("Visual Studio Code", &["visual studio code"]));
        let mut scorer = QueryScorer::new("v", &stats, 1.2, 0.3);
        assert!(scorer.subset_score("visual studio code") > 0.0);

        let cjk = stats_with(candidate("微信", &["微信"]));
        let mut scorer = QueryScorer::new("微", &cjk, 1.2, 0.3);
        assert!(scorer.subset_score("微信") > 0.0);

        let mut scorer = QueryScorer::new("visualstudiocode", &stats, 1.2, 0.3);
        let spaced = scorer.subset_score("visual studio code");
        assert!(
            spaced > 0.7 * "visualstudiocode".chars().count() as f64,
            "含空格关键词应覆盖多数 bigram，实际 {spaced}"
        );
    }

    /// IDF 恒正：`df > N/2` 的常见 term 也不产生负权重（否则命中常见 bigram 反而扣分）。
    #[test]
    fn idf_is_positive_for_common_terms() {
        let candidates: Vec<SearchCandidate> = (0..10)
            .map(|i| candidate(&format!("app-{i}"), &["common"]))
            .collect();
        let stats = CorpusStats::build(&candidates, 1);
        let common = pack_bigram('c', 'o');
        assert_eq!(stats.df.get(&common).copied(), Some(10));
        assert!(stats.idf(common) > 0.0);
    }

    /// 候选缓存世代变化后重建语料统计：新装应用计入语料，不读旧缓存。
    #[test]
    fn corpus_stats_refresh_on_generation_change() {
        let model = StandardSearchModel::new();
        let mut data = CachedCandidateData::new();
        data.add_candidate(candidate("Snipaste", &["snipaste"]));
        data.add_candidate(candidate("Visual Studio Code", &["visualstudiocode"]));
        let before = model.corpus_stats(&data);
        assert_eq!(before.doc_count, 2.0);

        data.add_candidate(candidate("QQ音乐", &["qq音乐"]));
        data.bump_generation();
        let after = model.corpus_stats(&data);
        assert_eq!(after.doc_count, 3.0);
        assert!(!Arc::ptr_eq(&before, &after));
    }

    /// 子集分明细的 description 是前端分数列的列 key，必须稳定为固定文案。
    #[test]
    fn subset_detail_description_is_stable() {
        let stats = stats_with(candidate("Snipaste", &["snipaste"]));
        let mut scorer = QueryScorer::new("snipaste", &stats, 1.2, 0.3);
        let scored = scorer.score(&candidate("Snipaste", &["snipaste"]));
        let detail = scored
            .detailed_score
            .iter()
            .find(|detail| detail.description == "BM25子集分")
            .expect("应存在 BM25 子集分明细项");
        assert!(matches!(detail.kind, ScoreDetailKind::Add));
    }

    /// 旧世代快照的查询不会顶掉新世代缓存，且返回与自身候选列表匹配的统计。
    #[test]
    fn stale_generation_does_not_replace_cached_stats() {
        let model = StandardSearchModel::new();
        let mut data = CachedCandidateData::new();
        data.add_candidate(candidate("Snipaste", &["snipaste"]));
        let stale = data.clone();
        data.add_candidate(candidate("Zed", &["zed"]));
        data.bump_generation();

        let fresh = model.corpus_stats(&data);
        assert_eq!(fresh.doc_count, 2.0);
        let old = model.corpus_stats(&stale);
        assert_eq!(old.doc_count, 1.0);
        let again = model.corpus_stats(&data);
        assert!(Arc::ptr_eq(&fresh, &again), "旧世代查询不应顶掉新世代缓存");
    }

    /// 并发调用共享同一份语料统计且世代一致（构建由写锁串行化，不产生分裂视图）。
    #[test]
    fn concurrent_corpus_stats_calls_share_one_snapshot() {
        let model = Arc::new(StandardSearchModel::new());
        let mut data = CachedCandidateData::new();
        for i in 0..20 {
            data.add_candidate(candidate(
                &format!("app-{i}"),
                &[&format!("app{i}"), "common"],
            ));
        }
        let data = Arc::new(data);
        let barrier = Arc::new(Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let model = Arc::clone(&model);
                let data = Arc::clone(&data);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    model.corpus_stats(&data)
                })
            })
            .collect();
        let stats: Vec<Arc<CorpusStats>> = handles
            .into_iter()
            .map(|handle| handle.join().expect("语料统计构建不应 panic"))
            .collect();
        assert!(stats
            .iter()
            .all(|snapshot| Arc::ptr_eq(snapshot, &stats[0])));
        assert_eq!(stats[0].doc_count, 40.0);
        assert_eq!(stats[0].generation, data.generation());
    }

    /// 并发交替使用新旧世代快照查询后，缓存恒为最新世代（旧世代的写入被拒绝，不回退）。
    #[test]
    fn concurrent_access_never_regresses_cached_generation() {
        let model = Arc::new(StandardSearchModel::new());
        let mut base = CachedCandidateData::new();
        base.add_candidate(candidate("Snipaste", &["snipaste"]));
        let old = Arc::new(base.clone());
        let newest = {
            let mut data = base;
            data.add_candidate(candidate("Zed", &["zed"]));
            data.bump_generation();
            Arc::new(data)
        };
        let barrier = Arc::new(Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let model = Arc::clone(&model);
                let barrier = Arc::clone(&barrier);
                let old = Arc::clone(&old);
                let newest = Arc::clone(&newest);
                thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..50 {
                        let data = if i % 2 == 0 { &old } else { &newest };
                        model.corpus_stats(data);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("并发查询不应 panic");
        }
        let cached = model.stats.read();
        assert_eq!(
            cached.as_ref().expect("缓存应已填充").generation,
            newest.generation()
        );
    }
}
