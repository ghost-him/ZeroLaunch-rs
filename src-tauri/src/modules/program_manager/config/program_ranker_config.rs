use crate::utils::generate_current_date;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// 部分程序排序器配置（用于运行时数据导出）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialProgramRankerConfig {
    pub launch_info: Option<VecDeque<DashMap<String, u64>>>,
    pub history_launch_time: Option<DashMap<String, u64>>,
    pub last_update_data: Option<String>,
    pub latest_launch_time: Option<DashMap<String, i64>>,
    /// 查询亲和度存储: 查询词 -> { "launch_method.get_text()" -> QueryAffinityData }
    pub query_affinity_store: Option<HashMap<String, DashMap<String, QueryAffinityData>>>,
    /// 历史总分权重系数 (默认1.0)
    pub history_weight: Option<f64>,
    /// 近期习惯权重系数 (7天内,默认2.0)
    pub recent_habit_weight: Option<f64>,
    /// 短期热度系数 (默认0.5)
    pub temporal_weight: Option<f64>,
    /// 查询亲和系数 (默认3)
    pub query_affinity_weight: Option<f64>,
    /// 查询亲和时间衰减常数(秒) (默认259200 = 3天)
    pub query_affinity_time_decay: Option<i64>,
    /// 查询亲和冷却时间(秒) (默认15秒)
    pub query_affinity_cooldown: Option<i64>,
    /// 短期热度衰减常数(秒) (默认10800 = 3小时)
    pub temporal_decay: Option<i64>,
    /// 是否启用排序算法 (默认true)
    pub is_enable: Option<bool>,
}

/// 查询亲和度数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAffinityData {
    /// 衰减后的有效次数（浮点数，支持衰减累积）
    #[serde(default = "QueryAffinityData::default_effective_count")]
    pub effective_count: f64,
    /// 最后一次启动时间（用于计算时的衰减）
    #[serde(default = "QueryAffinityData::default_last_launch_time")]
    pub last_launch_time: i64,
    /// 最后一次记录计数的时间（用于冷却机制）
    #[serde(default = "QueryAffinityData::default_last_record_time")]
    pub last_record_time: i64,
}

impl QueryAffinityData {
    pub(crate) fn default_effective_count() -> f64 {
        0.0
    }

    pub(crate) fn default_last_launch_time() -> i64 {
        0
    }

    pub(crate) fn default_last_record_time() -> i64 {
        0
    }
}

/// 程序排序器配置内部结构
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ProgramRankerConfigInner {
    /// 天数,[一个地址的启动次数]
    #[serde(default = "ProgramRankerConfigInner::default_launch_info")]
    pub launch_info: VecDeque<DashMap<String, u64>>,
    /// 历史启动次数
    #[serde(default = "ProgramRankerConfigInner::default_history_launch_time")]
    pub history_launch_time: DashMap<String, u64>,
    /// 上次的读取日期
    #[serde(default = "ProgramRankerConfigInner::default_last_update_data")]
    pub last_update_data: String,
    /// 最近一次的启动时间
    #[serde(default = "ProgramRankerConfigInner::default_latest_launch_time")]
    pub latest_launch_time: DashMap<String, i64>,
    /// 查询亲和度存储
    #[serde(default = "ProgramRankerConfigInner::default_query_affinity_store")]
    pub query_affinity_store: HashMap<String, DashMap<String, QueryAffinityData>>,
    /// 历史总分权重系数
    #[serde(default = "ProgramRankerConfigInner::default_history_weight")]
    pub history_weight: f64,
    /// 近期习惯权重系数 (7天内)
    #[serde(default = "ProgramRankerConfigInner::default_recent_habit_weight")]
    pub recent_habit_weight: f64,
    /// 瞬时热度系数
    #[serde(default = "ProgramRankerConfigInner::default_temporal_weight")]
    pub temporal_weight: f64,
    /// 查询亲和系数
    #[serde(default = "ProgramRankerConfigInner::default_query_affinity_weight")]
    pub query_affinity_weight: f64,
    /// 查询亲和时间衰减常数
    #[serde(default = "ProgramRankerConfigInner::default_query_affinity_time_decay")]
    pub query_affinity_time_decay: i64,
    /// 查询亲和冷却时间（秒），防止短时间重复计数
    #[serde(default = "ProgramRankerConfigInner::default_query_affinity_cooldown")]
    pub query_affinity_cooldown: i64,
    /// 短期热度衰减常数
    #[serde(default = "ProgramRankerConfigInner::default_temporal_decay")]
    pub temporal_decay: i64,
    /// 是否启用排序算法
    #[serde(default = "ProgramRankerConfigInner::default_is_enable")]
    pub is_enable: bool,
}

impl Default for ProgramRankerConfigInner {
    fn default() -> Self {
        Self {
            launch_info: Self::default_launch_info(),
            history_launch_time: Self::default_history_launch_time(),
            last_update_data: Self::default_last_update_data(),
            latest_launch_time: Self::default_latest_launch_time(),
            query_affinity_store: Self::default_query_affinity_store(),
            history_weight: Self::default_history_weight(),
            recent_habit_weight: Self::default_recent_habit_weight(),
            temporal_weight: Self::default_temporal_weight(),
            query_affinity_weight: Self::default_query_affinity_weight(),
            query_affinity_time_decay: Self::default_query_affinity_time_decay(),
            query_affinity_cooldown: Self::default_query_affinity_cooldown(),
            temporal_decay: Self::default_temporal_decay(),
            is_enable: Self::default_is_enable(),
        }
    }
}

impl ProgramRankerConfigInner {
    pub(crate) fn default_launch_info() -> VecDeque<DashMap<String, u64>> {
        let mut deque = VecDeque::new();
        deque.push_front(DashMap::new());
        deque
    }

    pub(crate) fn default_history_launch_time() -> DashMap<String, u64> {
        DashMap::new()
    }

    pub(crate) fn default_last_update_data() -> String {
        generate_current_date()
    }

    pub(crate) fn default_latest_launch_time() -> DashMap<String, i64> {
        DashMap::new()
    }

    pub(crate) fn default_query_affinity_store(
    ) -> HashMap<String, DashMap<String, QueryAffinityData>> {
        HashMap::new()
    }

    pub(crate) fn default_history_weight() -> f64 {
        1.0
    }

    pub(crate) fn default_recent_habit_weight() -> f64 {
        2.0
    }

    pub(crate) fn default_temporal_weight() -> f64 {
        0.5
    }

    pub(crate) fn default_query_affinity_weight() -> f64 {
        3.0
    }

    pub(crate) fn default_query_affinity_time_decay() -> i64 {
        259200 // 3 days in seconds
    }

    pub(crate) fn default_query_affinity_cooldown() -> i64 {
        15 // 15 seconds cooldown
    }

    pub(crate) fn default_temporal_decay() -> i64 {
        10800 // 3 hours in seconds
    }

    pub(crate) fn default_is_enable() -> bool {
        true
    }

    pub fn to_partial(&self) -> PartialProgramRankerConfig {
        PartialProgramRankerConfig {
            history_launch_time: Some(self.history_launch_time.clone()),
            last_update_data: Some(self.last_update_data.clone()),
            launch_info: Some(self.launch_info.clone()),
            latest_launch_time: Some(self.latest_launch_time.clone()),
            query_affinity_store: Some(self.query_affinity_store.clone()),
            history_weight: Some(self.history_weight),
            recent_habit_weight: Some(self.recent_habit_weight),
            temporal_weight: Some(self.temporal_weight),
            query_affinity_weight: Some(self.query_affinity_weight),
            query_affinity_time_decay: Some(self.query_affinity_time_decay),
            query_affinity_cooldown: Some(self.query_affinity_cooldown),
            temporal_decay: Some(self.temporal_decay),
            is_enable: Some(self.is_enable),
        }
    }

    pub fn update(&mut self, partial_config: PartialProgramRankerConfig) {
        if let Some(partial_launch_info) = partial_config.launch_info {
            self.launch_info = partial_launch_info;
        }
        if let Some(partial_history_launch_time) = partial_config.history_launch_time {
            self.history_launch_time = partial_history_launch_time;
        }
        if let Some(partial_last_update_data) = partial_config.last_update_data {
            self.last_update_data = partial_last_update_data;
        }
        if let Some(partial_latest_launch_time) = partial_config.latest_launch_time {
            self.latest_launch_time = partial_latest_launch_time;
        }
        if let Some(partial_query_affinity_store) = partial_config.query_affinity_store {
            self.query_affinity_store = partial_query_affinity_store;
        }
        if let Some(weight) = partial_config.history_weight {
            self.history_weight = weight;
        }
        if let Some(weight) = partial_config.recent_habit_weight {
            self.recent_habit_weight = weight;
        }
        if let Some(weight) = partial_config.temporal_weight {
            self.temporal_weight = weight;
        }
        if let Some(weight) = partial_config.query_affinity_weight {
            self.query_affinity_weight = weight;
        }
        if let Some(decay) = partial_config.query_affinity_time_decay {
            self.query_affinity_time_decay = decay;
        }
        if let Some(cooldown) = partial_config.query_affinity_cooldown {
            self.query_affinity_cooldown = cooldown;
        }
        if let Some(decay) = partial_config.temporal_decay {
            self.temporal_decay = decay;
        }
        if let Some(enable) = partial_config.is_enable {
            self.is_enable = enable;
        }
    }
}

/// 程序排序器配置
#[derive(Debug)]
pub struct ProgramRankerConfig {
    inner: RwLock<ProgramRankerConfigInner>,
}

impl Default for ProgramRankerConfig {
    fn default() -> Self {
        ProgramRankerConfig {
            inner: RwLock::new(ProgramRankerConfigInner::default()),
        }
    }
}

impl ProgramRankerConfig {
    pub fn to_partial(&self) -> PartialProgramRankerConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_launch_info(&self) -> VecDeque<DashMap<String, u64>> {
        self.inner.read().launch_info.clone()
    }

    pub fn get_history_launch_time(&self) -> DashMap<String, u64> {
        self.inner.read().history_launch_time.clone()
    }

    pub fn get_last_update_data(&self) -> String {
        self.inner.read().last_update_data.clone()
    }

    pub fn get_latest_launch_time(&self) -> DashMap<String, i64> {
        self.inner.read().latest_launch_time.clone()
    }

    pub fn get_query_affinity_store(&self) -> HashMap<String, DashMap<String, QueryAffinityData>> {
        self.inner.read().query_affinity_store.clone()
    }

    pub fn get_history_weight(&self) -> f64 {
        self.inner.read().history_weight
    }

    pub fn get_recent_habit_weight(&self) -> f64 {
        self.inner.read().recent_habit_weight
    }

    pub fn get_temporal_weight(&self) -> f64 {
        self.inner.read().temporal_weight
    }

    pub fn get_query_affinity_weight(&self) -> f64 {
        self.inner.read().query_affinity_weight
    }

    pub fn get_query_affinity_time_decay(&self) -> i64 {
        self.inner.read().query_affinity_time_decay
    }

    pub fn get_query_affinity_cooldown(&self) -> i64 {
        self.inner.read().query_affinity_cooldown
    }

    pub fn get_temporal_decay(&self) -> i64 {
        self.inner.read().temporal_decay
    }

    pub fn get_is_enable(&self) -> bool {
        self.inner.read().is_enable
    }

    pub fn update(&self, partial_config: PartialProgramRankerConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
}
