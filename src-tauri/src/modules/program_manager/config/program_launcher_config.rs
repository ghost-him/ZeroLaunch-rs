use crate::utils::generate_current_date;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialProgramLauncherConfig {
    pub launch_info: Option<VecDeque<HashMap<String, u64>>>,
    pub history_launch_time: Option<HashMap<String, u64>>,
    pub last_update_data: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramLauncherConfigInner {
    /// 天数,[一个地址的启动次数]
    pub launch_info: VecDeque<HashMap<String, u64>>,
    /// 历史启动次数
    pub history_launch_time: HashMap<String, u64>,
    /// 上次的读取日期
    pub last_update_data: String,
}

impl Default for ProgramLauncherConfigInner {
    fn default() -> Self {
        let mut deque = VecDeque::new();
        deque.push_front(HashMap::new());
        ProgramLauncherConfigInner {
            launch_info: deque,
            history_launch_time: HashMap::new(),
            last_update_data: generate_current_date(),
        }
    }
}

impl ProgramLauncherConfigInner {
    pub fn to_partial(&self) -> PartialProgramLauncherConfig {
        PartialProgramLauncherConfig {
            history_launch_time: Some(self.history_launch_time.clone()),
            last_update_data: Some(self.last_update_data.clone()),
            launch_info: Some(self.launch_info.clone()),
        }
    }
    pub fn update(&mut self, partial_config: PartialProgramLauncherConfig) {
        if let Some(partial_launch_info) = partial_config.launch_info {
            self.launch_info = partial_launch_info;
        }
        if let Some(partial_history_launch_time) = partial_config.history_launch_time {
            self.history_launch_time = partial_history_launch_time;
        }
        if let Some(partial_last_update_data) = partial_config.last_update_data {
            self.last_update_data = partial_last_update_data;
        }
    }
}
#[derive(Debug)]
pub struct ProgramLauncherConfig {
    inner: RwLock<ProgramLauncherConfigInner>,
}

impl Default for ProgramLauncherConfig {
    fn default() -> Self {
        ProgramLauncherConfig {
            inner: RwLock::new(ProgramLauncherConfigInner::default()),
        }
    }
}

impl ProgramLauncherConfig {
    pub fn to_partial(&self) -> PartialProgramLauncherConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_launch_info(&self) -> VecDeque<HashMap<String, u64>> {
        self.inner.read().launch_info.clone()
    }

    pub fn get_history_launch_time(&self) -> HashMap<String, u64> {
        self.inner.read().history_launch_time.clone()
    }

    pub fn get_last_update_data(&self) -> String {
        self.inner.read().last_update_data.clone()
    }
    pub fn update(&self, partial_config: PartialProgramLauncherConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
}
