use super::types::{LaunchError, LaunchMethod, LaunchMethodType, Launcher, ResultAction};
use std::collections::HashMap;
use std::sync::Arc;

/// 启动器注册中心
/// 管理所有启动器的注册和查找
pub struct LauncherRegistry {
    launchers: HashMap<LaunchMethodType, Arc<dyn Launcher>>,
}

impl LauncherRegistry {
    /// 创建一个新的启动器注册中心
    pub fn new() -> Self {
        Self {
            launchers: HashMap::new(),
        }
    }

    /// 注册一个启动器
    /// 参数：launcher - 要注册的启动器实例
    pub fn register(&mut self, launcher: Arc<dyn Launcher>) {
        self.launchers.insert(launcher.supported_method(), launcher);
    }

    /// 根据启动方法和动作 ID 执行对应操作
    /// 参数：method - 启动方法；action_id - 动作 ID
    /// 返回：成功返回 Ok(())，失败返回 LaunchError
    pub fn execute(&self, method: &LaunchMethod, action_id: &str) -> Result<(), LaunchError> {
        let method_type = method.method_type();

        self.launchers
            .get(&method_type)
            .ok_or(LaunchError::NotFound(method_type))?
            .execute(method, action_id)
    }

    /// 获取指定启动方法类型的 Launcher 所支持的动作列表
    /// 参数：method_type - 启动方法类型
    /// 返回：该 Launcher 支持的动作列表，若未注册对应 Launcher 则返回空列表
    pub fn get_actions(&self, method_type: LaunchMethodType) -> Vec<ResultAction> {
        self.launchers
            .get(&method_type)
            .map(|l| l.supported_actions())
            .unwrap_or_default()
    }

    /// 检查是否已注册指定类型的启动器
    /// 参数：method_type - 启动方法类型
    /// 返回：已注册返回 true，否则返回 false
    pub fn has_launcher(&self, method_type: LaunchMethodType) -> bool {
        self.launchers.contains_key(&method_type)
    }

    /// 获取已注册的启动器数量
    pub fn len(&self) -> usize {
        self.launchers.len()
    }

    /// 检查是否没有注册任何启动器
    pub fn is_empty(&self) -> bool {
        self.launchers.is_empty()
    }
}

impl Default for LauncherRegistry {
    fn default() -> Self {
        Self::new()
    }
}
