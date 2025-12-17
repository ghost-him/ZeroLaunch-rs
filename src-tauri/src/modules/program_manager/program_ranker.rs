use crate::error::OptionExt;
use crate::program_manager::config::program_ranker_config::PartialProgramRankerConfig;
use crate::program_manager::config::program_ranker_config::ProgramRankerConfig;
use crate::program_manager::config::program_ranker_config::QueryAffinityData;
use crate::program_manager::remove_repeated_space;
use crate::program_manager::LaunchMethod;
use crate::utils::dashmap_to_hashmap;
use crate::utils::hashmap_to_dashmap;
use crate::utils::is_date_current;
use crate::utils::{generate_current_date, get_current_time};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::BTreeSet;
use std::collections::{HashMap, VecDeque};
use tracing::debug;

/// 程序排序器内部实现
#[derive(Debug)]
struct ProgramRankerInner {
    /// 程序映射表（用于获取启动方式的文本）
    launch_store: DashMap<u64, LaunchMethod>,
    /// 最近7天的启动次数记录
    launch_time: VecDeque<DashMap<String, u64>>,
    /// 历史总启动次数
    history_launch_time: DashMap<String, u64>,
    /// 上次更新日期
    last_update_data: String,
    /// 最近一次启动时间（时间戳）
    latest_launch_time: DashMap<String, i64>,
    /// 运行时的启动时间排序集合: (上一次启动的时间, 目标程序的guid)
    runtime_latest_launch_time: BTreeSet<(i64, u64)>,
    /// 查询亲和度映射: (查询词, launch_method_text) -> QueryAffinityData
    query_affinity_map: DashMap<(String, String), QueryAffinityData>,
    /// 权重配置
    history_weight: f64,
    recent_habit_weight: f64,
    temporal_weight: f64,
    query_affinity_weight: f64,
    query_affinity_time_decay: i64,
    query_affinity_cooldown: i64,
    temporal_decay: i64,
    /// 是否启用排序算法
    is_enable: bool,
}

impl ProgramRankerInner {
    fn new() -> Self {
        let mut deque = VecDeque::new();
        deque.push_front(DashMap::new());
        ProgramRankerInner {
            launch_store: DashMap::new(),
            launch_time: deque,
            history_launch_time: DashMap::new(),
            last_update_data: generate_current_date(),
            latest_launch_time: DashMap::new(),
            runtime_latest_launch_time: BTreeSet::new(),
            query_affinity_map: DashMap::new(),
            history_weight: 0.8,
            recent_habit_weight: 1.5,
            temporal_weight: 0.5,
            query_affinity_weight: 3.0,
            query_affinity_time_decay: 259200,
            query_affinity_cooldown: 60,
            temporal_decay: 10800,
            is_enable: true,
        }
    }

    fn load_from_config(&mut self, config: &ProgramRankerConfig) {
        self.launch_time.clear();
        self.launch_store.clear();
        let launch_info = config.get_launch_info();
        launch_info.iter().for_each(|k| {
            let dash_map = hashmap_to_dashmap(k);
            self.launch_time.push_back(dash_map);
        });

        self.last_update_data = config.get_last_update_data();
        self.history_launch_time = hashmap_to_dashmap(&config.get_history_launch_time());
        self.update_launch_info();

        // 维护最近启动次数
        self.latest_launch_time.clear();
        self.latest_launch_time = hashmap_to_dashmap(&config.get_latest_launch_time());

        self.runtime_latest_launch_time.clear();

        // 加载查询亲和度数据
        self.query_affinity_map.clear();
        let query_affinity_store = config.get_query_affinity_store();
        for (query, method_map) in query_affinity_store {
            for (method_text, data) in method_map {
                self.query_affinity_map
                    .insert((query.clone(), method_text), data);
            }
        }

        // 加载权重配置
        self.history_weight = config.get_history_weight();
        self.recent_habit_weight = config.get_recent_habit_weight();
        self.temporal_weight = config.get_temporal_weight();
        self.query_affinity_weight = config.get_query_affinity_weight();
        self.query_affinity_time_decay = config.get_query_affinity_time_decay();
        self.query_affinity_cooldown = config.get_query_affinity_cooldown();
        self.temporal_decay = config.get_temporal_decay();
        self.is_enable = config.get_is_enable();
    }

    fn get_runtime_data(&mut self) -> PartialProgramRankerConfig {
        self.update_launch_info();

        let mut launch_info_data: VecDeque<HashMap<String, u64>> = VecDeque::new();
        for item in &self.launch_time {
            launch_info_data.push_back(dashmap_to_hashmap(item));
        }

        // 导出查询亲和度数据
        let mut query_affinity_store: HashMap<String, HashMap<String, QueryAffinityData>> =
            HashMap::new();
        for entry in self.query_affinity_map.iter() {
            let (query, method_text) = entry.key();
            let data = entry.value();
            query_affinity_store
                .entry(query.clone())
                .or_default()
                .insert(method_text.clone(), data.clone());
        }

        PartialProgramRankerConfig {
            launch_info: Some(launch_info_data),
            history_launch_time: Some(dashmap_to_hashmap(&self.history_launch_time)),
            last_update_data: Some(generate_current_date()),
            latest_launch_time: Some(dashmap_to_hashmap(&self.latest_launch_time)),
            query_affinity_store: Some(query_affinity_store),
            history_weight: None,
            recent_habit_weight: None,
            temporal_weight: None,
            query_affinity_weight: None,
            query_affinity_time_decay: None,
            query_affinity_cooldown: None,
            temporal_decay: None,
            is_enable: None,
        }
    }

    fn register_program(&mut self, program_guid: u64, launch_method: LaunchMethod) {
        debug!("register: {} {}", program_guid, launch_method.get_text());
        let key = launch_method.get_text();
        self.launch_store.insert(program_guid, launch_method);

        self.latest_launch_time.entry(key.clone()).or_insert(0);

        self.latest_launch_time
            .entry(key)
            .and_modify(|latest_launch_time| {
                self.runtime_latest_launch_time
                    .insert((*latest_launch_time, program_guid));
            });
    }

    /// 记录程序启动，更新所有统计数据
    fn record_launch(&mut self, program_guid: u64) {
        let launch_method = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");

        let method_text = launch_method.get_text();

        // 更新今日启动次数
        self.launch_time[0]
            .entry(method_text.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // 更新历史总启动次数
        self.history_launch_time
            .entry(method_text.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // 更新启动的时间
        self.latest_launch_time
            .entry(method_text)
            .and_modify(|last_launch_time| {
                // 去除之前老的数据
                assert!(self
                    .runtime_latest_launch_time
                    .remove(&(*last_launch_time, program_guid)));
                let current_time = get_current_time();
                *last_launch_time = current_time;
                self.runtime_latest_launch_time
                    .insert((current_time, program_guid));
            });
    }

    /// 获得启动器维护的数据
    pub fn get_latest_launch_program(&self, program_count: u32) -> Vec<u64> {
        let mut result = Vec::new();
        for (_, program_guid) in self
            .runtime_latest_launch_time
            .iter()
            .rev()
            .take(program_count as usize)
        {
            result.push(*program_guid);
        }
        result
    }

    /// 计算近期习惯分数 (基于最近7天的启动次数，带衰减)
    fn calculate_recent_habit_score(&self, program_guid: u64) -> f64 {
        let program_string = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");
        let mut result: f64 = 0.0;
        let mut k: f64 = 1.0;
        self.launch_time.iter().for_each(|day| {
            if let Some(time) = day.get(&program_string.get_text()) {
                result += (*time as f64) * k;
            }
            k /= 1.3
        });
        result
    }

    fn program_history_launch_time(&mut self, program_guid: u64) -> u64 {
        let program_string = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");
        let count = self
            .history_launch_time
            .entry(program_string.get_text())
            .or_insert(0);
        *count
    }

    /// 记录查询-程序启动关联（衰减累积 + 冷却机制）
    fn record_query_launch(&mut self, query: &str, program_guid: u64) {
        // 预处理查询词，确保与搜索时的一致性
        let query = query.to_lowercase();
        let query = remove_repeated_space(&query);

        let current_time = get_current_time();
        let launch_method = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");
        let method_text = launch_method.get_text();
        let key = (query, method_text);

        self.query_affinity_map
            .entry(key)
            .and_modify(|data| {
                // 冷却机制：检查距离上次记录是否超过冷却时间
                let time_since_last_record = current_time - data.last_record_time;
                if time_since_last_record >= self.query_affinity_cooldown {
                    // 衰减累积：先对旧的有效次数进行衰减，再加上新的一次
                    let time_diff = current_time - data.last_launch_time;
                    let decay =
                        (-(time_diff as f64) / (self.query_affinity_time_decay as f64 + 1.0)).exp();

                    data.effective_count = data.effective_count * decay + 1.0;
                    data.last_record_time = current_time;
                }
                // 无论是否在冷却时间内，都更新最后启动时间
                data.last_launch_time = current_time;
            })
            .or_insert(QueryAffinityData {
                effective_count: 1.0,
                last_launch_time: current_time,
                last_record_time: current_time,
            });
    }

    /// 计算查询亲和分数（对数缩放 + 时间衰减）
    fn calculate_query_affinity_score(&self, query: &str, program_guid: u64) -> f64 {
        let launch_method = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");
        let method_text = launch_method.get_text();
        let key = (query.to_string(), method_text);

        if let Some(data) = self.query_affinity_map.get(&key) {
            let current_time = get_current_time();
            let time_diff = current_time - data.last_launch_time;

            // 时间衰减因子: exp(-(时间差/时间常数))
            let decay_factor =
                (-(time_diff as f64) / (self.query_affinity_time_decay as f64 + 1.0)).exp();

            // 计算当前有效次数（应用衰减）
            let current_effective_count = data.effective_count * decay_factor;

            // 使用对数缩放，避免分数过大
            // ln(1 + effective_count) * 系数
            (current_effective_count).ln_1p() * 10.0
        } else {
            0.0
        }
    }

    /// 计算近期热度分数
    fn calculate_temporal_score(&self, program_guid: u64) -> f64 {
        let program_string = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");

        if let Some(last_launch_time) = self.latest_launch_time.get(&program_string.get_text()) {
            let current_time = get_current_time();
            let time_diff = current_time - *last_launch_time;

            // 热度公式: K / (1 + 时间差/时间单位)
            let k = 6.0;
            k / (1.0 + (time_diff as f64) / (self.temporal_decay as f64 + 1.0))
        } else {
            0.0
        }
    }

    /// 计算历史总分 (基于所有历史启动次数)
    fn calculate_history_score(&self, program_guid: u64) -> f64 {
        let program_string = self
            .launch_store
            .get(&program_guid)
            .expect_programming("Program GUID should exist in launch store");

        if let Some(count) = self.history_launch_time.get(&program_string.get_text()) {
            // 使用对数函数避免历史数据过大，log(1 + count) 提供递减收益
            (*count as f64).ln_1p()
        } else {
            0.0
        }
    }

    /// 计算最终排序分数（基础分数 + 智能增强）
    fn calculate_score_details(
        &self,
        base_score: f64,
        program_guid: u64,
        query: &str,
    ) -> crate::program_manager::unit::ScoreDetails {
        use crate::program_manager::unit::ScoreDetails;

        if !self.is_enable {
            return ScoreDetails {
                base_score,
                final_score: base_score,
                ..Default::default()
            };
        }

        let history_score = self.calculate_history_score(program_guid);
        let recent_habit_score = self.calculate_recent_habit_score(program_guid);
        let temporal_score = self.calculate_temporal_score(program_guid);
        let query_affinity = self.calculate_query_affinity_score(query, program_guid);

        // 动态权重总和
        let dynamic_score = self.history_weight * history_score
            + self.recent_habit_weight * recent_habit_score
            + self.temporal_weight * temporal_score;

        // 基础分抑制因子：如果基础匹配分过低（说明文本匹配度差），则抑制历史习惯等权重的加成
        // 只有当 base_score >= 15.0 时，才能获得 100% 的动态权重加成
        // 这样可以避免"高频使用的无关程序"挤占"低频使用的精准匹配程序"
        // 当用户没有输入时，不启用抑制因子，以便显示最近使用的程序
        let suppression_factor = if query.is_empty() {
            1.0
        } else {
            (base_score / 15.0).clamp(0.0, 1.0)
        };

        let final_score = base_score
            + dynamic_score * suppression_factor
            + self.query_affinity_weight * query_affinity;

        ScoreDetails {
            base_score,
            history_score,
            recent_habit_score,
            temporal_score,
            query_affinity_score: query_affinity,
            history_weight: self.history_weight,
            recent_habit_weight: self.recent_habit_weight,
            temporal_weight: self.temporal_weight,
            query_affinity_weight: self.query_affinity_weight,
            suppression_factor,
            final_score,
        }
    }

    /// 获取历史权重系数
    fn get_history_weight(&self) -> f64 {
        self.history_weight
    }

    /// 获取近期习惯权重系数
    fn get_recent_habit_weight(&self) -> f64 {
        self.recent_habit_weight
    }

    /// 获取瞬时权重系数
    fn get_temporal_weight(&self) -> f64 {
        self.temporal_weight
    }

    /// 获取查询亲和权重系数
    fn get_query_affinity_weight(&self) -> f64 {
        self.query_affinity_weight
    }

    fn update_launch_info(&mut self) {
        if !is_date_current(&self.last_update_data) {
            self.launch_time.push_front(DashMap::new());
            if self.launch_time.len() > 7 {
                self.launch_time.pop_back();
            }
            self.last_update_data = generate_current_date();
        }
    }
}

/// 程序排序器 - 负责程序的排序统计和计算
#[derive(Debug)]
pub struct ProgramRanker {
    inner: RwLock<ProgramRankerInner>,
}

impl Default for ProgramRanker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramRanker {
    pub fn new() -> Self {
        ProgramRanker {
            inner: RwLock::new(ProgramRankerInner::new()),
        }
    }

    pub fn load_from_config(&self, config: &ProgramRankerConfig) {
        self.inner.write().load_from_config(config);
    }

    pub fn get_runtime_data(&self) -> PartialProgramRankerConfig {
        self.inner.write().get_runtime_data()
    }

    pub fn register_program(&self, program_guid: u64, launch_method: LaunchMethod) {
        self.inner
            .write()
            .register_program(program_guid, launch_method);
    }

    /// 记录程序启动
    pub fn record_launch(&self, program_guid: u64) {
        self.inner.write().record_launch(program_guid);
    }

    /// 计算近期习惯分数 (基于最近7天的启动次数，带衰减)
    pub fn calculate_recent_habit_score(&self, program_guid: u64) -> f64 {
        self.inner.read().calculate_recent_habit_score(program_guid)
    }

    pub fn program_history_launch_time(&self, program_guid: u64) -> u64 {
        self.inner.write().program_history_launch_time(program_guid)
    }

    /// 记录查询-程序启动关联
    pub fn record_query_launch(&self, query: &str, program_guid: u64) {
        self.inner.write().record_query_launch(query, program_guid);
    }

    /// 计算查询亲和分数
    pub fn calculate_query_affinity_score(&self, query: &str, program_guid: u64) -> f64 {
        self.inner
            .read()
            .calculate_query_affinity_score(query, program_guid)
    }

    /// 计算瞬时分数
    pub fn calculate_temporal_score(&self, program_guid: u64) -> f64 {
        self.inner.read().calculate_temporal_score(program_guid)
    }

    /// 计算历史总分
    pub fn calculate_history_score(&self, program_guid: u64) -> f64 {
        self.inner.read().calculate_history_score(program_guid)
    }

    /// 计算详细的排序分数
    pub fn calculate_score_details(
        &self,
        base_score: f64,
        program_guid: u64,
        query: &str,
    ) -> crate::program_manager::unit::ScoreDetails {
        self.inner
            .read()
            .calculate_score_details(base_score, program_guid, query)
    }

    /// 获取历史权重系数
    pub fn get_history_weight(&self) -> f64 {
        self.inner.read().get_history_weight()
    }

    /// 获取近期习惯权重系数
    pub fn get_recent_habit_weight(&self) -> f64 {
        self.inner.read().get_recent_habit_weight()
    }

    /// 获取短期热度权重系数
    pub fn get_temporal_weight(&self) -> f64 {
        self.inner.read().get_temporal_weight()
    }

    /// 获取查询亲和权重系数
    pub fn get_query_affinity_weight(&self) -> f64 {
        self.inner.read().get_query_affinity_weight()
    }

    pub fn get_latest_launch_program(&self, program_count: u32) -> Vec<u64> {
        self.inner.read().get_latest_launch_program(program_count)
    }

    pub fn load_and_register_programs(
        &self,
        config: &ProgramRankerConfig,
        programs: &[(u64, LaunchMethod)],
    ) {
        let mut inner = self.inner.write(); // 获取一次写锁
        inner.load_from_config(config); // 加载配置
        for (program_guid, launch_method) in programs {
            inner.register_program(*program_guid, launch_method.clone()); // 注册程序
        }
    }
}
