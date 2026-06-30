use std::fmt::Debug;
use std::sync::Arc;

/// 焦点事件类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEvent {
    /// 窗口失去焦点（关闭请求或焦点离开且鼠标在窗口外）。
    Lost,
}

/// 焦点监控回调类型。
/// 窗口失去焦点时所有已注册回调被依次调用。
pub type FocusCallback = Arc<dyn Fn(FocusEvent) + Send + Sync>;
