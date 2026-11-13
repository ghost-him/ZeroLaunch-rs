use std::collections::HashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, RegistrationError, ResultAction, TargetType,
};

/// 执行器注册中心
/// 使用 (TargetType, action_id) 作为复合主键定位 Executor
pub struct ExecutorRegistry {
    /// 复合主键 -> Executor 映射，用于 execute 时 O(1) 查找
    executor_map: HashMap<(TargetType, String), Arc<dyn ActionExecutor>>,

    /// TargetType -> actions 映射，用于 get_actions 查询。
    /// 由 register 按注册顺序追加、由 unregister 按同一复合主键撤销，两者成对维护；
    /// 因复合主键唯一，撤销只会移除被注销执行器自己的动作。
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

        // 聚合 actions 到 target_actions（unregister 按同一主键撤销）
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

    /// 获取某个 TargetType 下所有可用的 actions（按注册顺序）
    pub fn get_actions(&self, target_type: TargetType) -> Vec<ResultAction> {
        self.target_actions
            .get(&target_type)
            .cloned()
            .unwrap_or_default()
    }

    /// 注销指定 component_id 的执行器及其所有 action 映射
    pub fn unregister(&mut self, component_id: &str) {
        // 先取出该执行器的全部复合主键，再按主键同时撤销查表项与动作索引项
        let removed: Vec<(TargetType, String)> = self
            .executor_map
            .iter()
            .filter(|(_, executor)| executor.component_id() == component_id)
            .map(|(key, _)| key.clone())
            .collect();

        for key in &removed {
            self.executor_map.remove(key);
        }

        for (target_type, action_id) in removed {
            let Some(actions) = self.target_actions.get_mut(&target_type) else {
                continue;
            };
            actions.retain(|action| action.id != action_id);
        }
        // 动作列表已空的类型不再保留条目（等价于「该类型无执行器」）
        self.target_actions.retain(|_, actions| !actions.is_empty());
    }
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use zerolaunch_plugin_api::config::{
        ComponentCore, ComponentType, Configurable, SettingDefinition,
    };
    use zerolaunch_plugin_api::services::IconRequest;

    /// 测试用执行器 —— 固定声明目标类型与动作列表，仅限本文件内使用。
    struct StubExecutor {
        /// 组件身份元数据（component_id 用于注销匹配）。
        core: ComponentCore,
        /// 声明支持的目标类型集合。
        target_types: Vec<TargetType>,
        /// 声明支持的动作列表，顺序即注册顺序。
        actions: Vec<ResultAction>,
    }

    impl StubExecutor {
        /// 构造测试执行器。
        /// 参数：component_id - 组件身份；target_types - 支持的目标类型；action_ids - 动作 id 列表。
        /// 返回：测试执行器实例（调用方按 `Arc<dyn ActionExecutor>` 注册）。
        fn new(component_id: &str, target_types: Vec<TargetType>, action_ids: &[&str]) -> Self {
            Self {
                core: ComponentCore::new(
                    component_id.to_string(),
                    component_id.to_string(),
                    String::new(),
                    ComponentType::ActionExecutor,
                    0,
                ),
                target_types,
                actions: action_ids
                    .iter()
                    .map(|id| ResultAction {
                        id: (*id).to_string(),
                        label: (*id).to_string(),
                        icon: IconRequest::Path(String::new()),
                        is_default: false,
                        shortcut_key: String::new(),
                    })
                    .collect(),
            }
        }
    }

    #[async_trait]
    impl Configurable for StubExecutor {
        fn core(&self) -> &ComponentCore {
            &self.core
        }

        fn setting_schema(&self) -> Vec<SettingDefinition> {
            vec![]
        }
    }

    #[async_trait]
    impl ActionExecutor for StubExecutor {
        fn supported_target_types(&self) -> Vec<TargetType> {
            self.target_types.clone()
        }

        fn supported_actions(&self) -> Vec<ResultAction> {
            self.actions.clone()
        }

        async fn execute(
            &self,
            _ctx: &ExecutionContext,
            _action_id: &str,
        ) -> Result<(), ExecutionError> {
            Ok(())
        }
    }

    /// 读取指定目标类型下的动作 id 列表（断言用）。
    fn action_ids(registry: &ExecutorRegistry, target_type: TargetType) -> Vec<String> {
        registry
            .get_actions(target_type)
            .into_iter()
            .map(|action| action.id)
            .collect()
    }

    /// 注销一个执行器后：动作列表只剩其余执行器的动作，不重复且保持注册顺序。
    #[test]
    fn unregister_keeps_remaining_actions_without_duplicates() {
        let mut registry = ExecutorRegistry::new();
        registry
            .register(Arc::new(StubExecutor::new(
                "test.a",
                vec![TargetType::Path],
                &["open", "copy"],
            )))
            .expect("注册 test.a 不应冲突");
        registry
            .register(Arc::new(StubExecutor::new(
                "test.b",
                vec![TargetType::Path],
                &["rename", "delete"],
            )))
            .expect("注册 test.b 不应冲突");

        assert_eq!(
            action_ids(&registry, TargetType::Path),
            ["open", "copy", "rename", "delete"]
        );

        registry.unregister("test.a");

        assert_eq!(
            action_ids(&registry, TargetType::Path),
            ["rename", "delete"]
        );
        assert!(registry.get_actions(TargetType::App).is_empty());
    }

    /// 注销后重新注册同类型执行器：动作列表不残留已注销项，也不重复累积。
    #[test]
    fn unregister_then_register_keeps_action_list_exact() {
        let mut registry = ExecutorRegistry::new();
        registry
            .register(Arc::new(StubExecutor::new(
                "test.a",
                vec![TargetType::Path, TargetType::App],
                &["open"],
            )))
            .expect("注册 test.a 不应冲突");

        registry.unregister("test.a");
        assert!(registry.get_actions(TargetType::Path).is_empty());
        assert!(registry.get_actions(TargetType::App).is_empty());

        registry
            .register(Arc::new(StubExecutor::new(
                "test.b",
                vec![TargetType::Path, TargetType::App],
                &["open", "rename"],
            )))
            .expect("复用已释放的动作键不应冲突");
        assert_eq!(action_ids(&registry, TargetType::Path), ["open", "rename"]);
        assert_eq!(action_ids(&registry, TargetType::App), ["open", "rename"]);
    }
}
