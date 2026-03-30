use super::types::{CandidateId, SearchCandidate};
use crate::plugin_system::types::LaunchMethod;
use dashmap::DashMap;
use dashmap::Entry;
use std::collections::HashSet;

// 保存当前已经缓存的候选数据
pub struct CachedCandidateData {
    // 当前缓存的候选数据
    candidates: Vec<SearchCandidate>,
    // 候选ID到索引的映射
    index: DashMap<CandidateId, usize>,
    // 该方法用于去重，只有没有重复的候选项才会被添加到candidates中，重复的候选项会被丢弃掉
    // 判断的依据：启动方式
    cached_launch_methods: HashSet<LaunchMethod>,
    // 下一个候选项ID
    next_candidate_id: CandidateId,
}

impl Default for CachedCandidateData {
    fn default() -> Self {
        Self::new()
    }
}

impl CachedCandidateData {
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
            index: DashMap::new(),
            cached_launch_methods: HashSet::new(),
            next_candidate_id: 1, // 从1开始，0表示无效ID
        }
    }

    // 添加一个候选人
    pub fn add_candidate(&mut self, mut candidate: SearchCandidate) {
        // 如果已经缓存了这个候选项的启动方式了，就不添加了，避免重复
        if self.has_launch_method(&candidate.launch_method) {
            return;
        }
        // 给这个候选项分配一个新的ID，并添加到candidates中，同时更新index和cached_launch_methods
        let candidate_id = self.next_candidate_id;
        candidate.id = candidate_id;
        self.cached_launch_methods
            .insert(candidate.launch_method.clone());
        self.candidates.push(candidate);
        self.index.insert(candidate_id, self.candidates.len() - 1);
        self.next_candidate_id += 1;
    }

    // 根据id获得指定的一个候选人
    pub fn get_candidate(&self, id: CandidateId) -> Option<&SearchCandidate> {
        match self.index.entry(id) {
            Entry::Occupied(entry) => Some(&self.candidates[*entry.get()]),
            Entry::Vacant(_) => None,
        }
    }

    // 添加多个候选人
    pub fn add_candidates(&mut self, candidates: CachedCandidateData) {
        for candidate in candidates.candidates.iter() {
            self.add_candidate(candidate.clone());
        }
    }

    // 获得原始的数据
    pub fn get_candidates(&self) -> &Vec<SearchCandidate> {
        &self.candidates
    }

    // 获得原始的数据的可变引用
    pub fn get_candidates_mut(&mut self) -> &mut Vec<SearchCandidate> {
        &mut self.candidates
    }

    // 判断是否已经缓存了某个启动方式的候选项了
    fn has_launch_method(&self, launch_method: &LaunchMethod) -> bool {
        self.cached_launch_methods.contains(launch_method)
    }
}
