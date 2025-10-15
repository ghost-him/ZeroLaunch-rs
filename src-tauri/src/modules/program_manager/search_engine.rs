use crate::error::ResultExt;
use crate::modules::program_manager::program_ranker::ProgramRanker;
use crate::modules::program_manager::search_model::Scorer;
use crate::modules::program_manager::semantic_manager::SemanticManager;
use crate::program_manager::remove_repeated_space;
use crate::program_manager::Program;
use crate::program_manager::SearchMatchResult;
use crate::program_manager::SearchModel;
use crate::Arc;
use rayon::prelude::*;

pub(crate) trait SearchEngine: std::fmt::Debug + Send + Sync {
    /// 执行搜索操作
    ///
    /// # Arguments
    /// * `user_input` - 用户输入的搜索字符串。
    /// * `programs` - 可供搜索的程序列表。
    /// * `program_ranker` - 程序排序器实例，用于计算排序分数。
    ///
    /// # Returns
    /// * 一个包含搜索结果的向量，按匹配度按原始数据排列（无排序）。
    fn perform_search(
        &self,
        user_input: &str,
        programs: &[Arc<Program>],
        program_ranker: &ProgramRanker,
    ) -> Vec<SearchMatchResult>;
}

#[derive(Debug)]
pub struct TraditionalSearchEngine {
    search_model: Arc<SearchModel>,
}

impl TraditionalSearchEngine {
    pub fn new(search_model: Arc<SearchModel>) -> Self {
        Self { search_model }
    }
}

impl Default for TraditionalSearchEngine {
    fn default() -> Self {
        Self {
            search_model: Arc::new(SearchModel::default()),
        }
    }
}

impl SearchEngine for TraditionalSearchEngine {
    fn perform_search(
        &self,
        user_input: &str,
        programs: &[Arc<Program>],
        program_ranker: &ProgramRanker,
    ) -> Vec<SearchMatchResult> {
        // 预处理用户输入
        let user_input = user_input.to_lowercase();
        let user_input = remove_repeated_space(&user_input);

        let search_model = self.search_model.clone();
        // 计算所有程序的匹配分数
        programs
            .par_iter()
            .map(|program| {
                // 基础匹配分数
                let base_score = search_model.calculate_score(program, &user_input) + program.stable_bias;
                
                // 应用智能排序增强评分
                let score = program_ranker.calculate_final_score(
                    base_score,
                    program.program_guid,
                    &user_input,
                );

                SearchMatchResult {
                    score,
                    program_guid: program.program_guid,
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct SemanticSearchEngine {
    semantic_model: Arc<SemanticManager>,
}

impl SemanticSearchEngine {
    pub fn new(semantic_model: Arc<SemanticManager>) -> Self {
        Self { semantic_model }
    }
}

impl SearchEngine for SemanticSearchEngine {
    fn perform_search(
        &self,
        user_input: &str,
        programs: &[Arc<Program>],
        program_ranker: &ProgramRanker,
    ) -> Vec<SearchMatchResult> {
        let user_input = user_input.to_lowercase();
        let user_input = remove_repeated_space(&user_input);

        let user_embedding = self
            .semantic_model
            .generate_embedding_for_manager(&user_input)
            .expect_programming("Failed to generate user embedding");

        // 计算所有程序的匹配分数
        programs
            .par_iter()
            .map(|program| {
                let base_score = self
                    .semantic_model
                    .compute_similarity(&user_embedding, &program.embedding)
                    as f64;
                
                // 应用智能排序增强评分
                let score = program_ranker.calculate_final_score(
                    base_score,
                    program.program_guid,
                    &user_input,
                );

                SearchMatchResult {
                    score,
                    program_guid: program.program_guid,
                }
            })
            .collect()
    }
}
