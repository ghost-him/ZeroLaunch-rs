pub trait StateAccess {
    type Target;

    // 安全访问方法
    fn with_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Self::Target) -> R;
}
