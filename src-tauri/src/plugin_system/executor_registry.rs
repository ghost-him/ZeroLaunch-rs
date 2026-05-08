use super::types::{
    ActionExecutor, ExecutionContext, ExecutionError, RegistrationError, ResultAction, TargetType,
};
use std::collections::HashMap;
use std::sync::Arc;

/// 执行器注册中心
/// 使用 (TargetType, action_id) 作为复合主键定位 Executor
pub struct ExecutorRegistry {
    /// 复合主键 -> Executor 映射，用于 execute 时 O(1) 查找
    executor_map: HashMap<(TargetType, String), Arc<dyn ActionExecutor>>,

    /// TargetType -> actions 映射，用于 get_actions 查询
    target_actions: HashMap<TargetType, Vec<ResultAction>>,
}

impl ExecutorRegistry {
    /// 创建一个新的执行器注册中心
    pub fn new() -> Self {
        Self {
            executor_map: HashMap::new(),
            target_actions: HashMap::new(),
        }
    }

    /// 注册执行器
    /// 返回 Result 以优雅处理注册冲突
    pub fn register(&mut self, executor: Arc<dyn ActionExecutor>) -> Result<(), RegistrationError> {
        let target_types = executor.supported_target_types();
        let actions = executor.supported_actions();

        // 先检查所有 key 是否可用
        for target_type in &target_types {
            for action in &actions {
                let key = (*target_type, action.id.clone());
                if self.executor_map.contains_key(&key) {
                    return Err(RegistrationError::ActionConflict {
                        target_type: *target_type,
                        action_id: action.id.clone(),
                    });
                }
            }
        }

        // 确认无冲突后，执行注册
        for target_type in &target_types {
            for action in &actions {
                let key = (*target_type, action.id.clone());
                self.executor_map.insert(key, executor.clone());
            }
        }

        // 聚合 actions 到 target_actions
        for target_type in target_types {
            self.target_actions
                .entry(target_type)
                .or_default()
                .extend(actions.clone());
        }

        Ok(())
    }

    /// 根据上下文和动作 ID 查找执行器，返回 Arc 克隆（同步，无锁持有）
    pub fn resolve(
        &self,
        ctx: &ExecutionContext,
        action_id: &str,
    ) -> Result<Arc<dyn ActionExecutor>, ExecutionError> {
        let target_type = ctx.target.target_type();
        let key = (target_type, action_id.to_string());
        self.executor_map
            .get(&key)
            .cloned()
            .ok_or_else(|| ExecutionError::UnsupportedAction(target_type, action_id.to_string()))
    }

    /// 查找回退执行器，返回 Arc 克隆（同步，无锁持有）
    pub fn resolve_fallback(
        &self,
        ctx: &ExecutionContext,
        fallback_action: &str,
    ) -> Result<Arc<dyn ActionExecutor>, ExecutionError> {
        let target_type = ctx.target.target_type();
        let fallback_key = (target_type, fallback_action.to_string());
        self.executor_map
            .get(&fallback_key)
            .cloned()
            .ok_or_else(|| {
                ExecutionError::Failed(format!(
                    "Fallback action '{}' not found for {:?}",
                    fallback_action, target_type
                ))
            })
    }

    /// 获取某个 TargetType 下所有可用的 actions
    pub fn get_actions(&self, target_type: TargetType) -> Vec<ResultAction> {
        self.target_actions
            .get(&target_type)
            .cloned()
            .unwrap_or_default()
    }

    /// 检查是否已注册指定类型的执行器
    pub fn has_executor(&self, target_type: TargetType) -> bool {
        self.target_actions.contains_key(&target_type)
    }

    /// 获取已注册的执行器数量（按 TargetType 计数）
    pub fn len(&self) -> usize {
        self.target_actions.len()
    }

    /// 检查是否没有注册任何执行器
    pub fn is_empty(&self) -> bool {
        self.target_actions.is_empty()
    }
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
