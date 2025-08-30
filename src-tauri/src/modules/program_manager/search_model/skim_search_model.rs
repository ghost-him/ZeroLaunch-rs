use crate::program_manager::search_model::Scorer;
use crate::program_manager::Program;
use crate::Arc;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
#[derive(Serialize, Deserialize, Default)]
pub struct SkimScorer {
    #[serde(skip)]
    matcher: SkimMatcherV2,
}

impl Scorer for SkimScorer {
    fn calculate_score(&self, program: &Arc<Program>, user_input: &str) -> f64 {
        let mut ret: f64 = -10000.0;
        for name in &program.search_keywords {
            if name.chars().count() < user_input.chars().count() {
                continue;
            }
            let score = self.matcher.fuzzy_match(name, user_input);
            if let Some(s) = score {
                ret = f64::max(ret, s as f64);
            }
        }
        ret
    }
}

impl Debug for SkimScorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkimScorer").finish()
    }
}

impl SkimScorer {
    pub fn new() -> Self {
        SkimScorer {
            matcher: SkimMatcherV2::default(),
        }
    }
}
