use serde::{Deserialize, Serialize};

/// 命令执行方式 —— 决定命令跑在后台还是可见控制台窗口中。
///
/// 使用范围：`ShellExecutor` / `PluginHost` / `PluginHandle` 的 `shell_execute_command`
/// 参数，以及 `host/shell.execute_command` 的线上载荷（载荷缺省时按 `Background` 处理，
/// 与旧版"后台执行"语义一致）。四个消费层共用这一份定义，不再按呈现方式各自开方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CommandRunMode {
    /// 后台静默执行：不创建控制台窗口，命令跑完即结束（缺省值）。
    #[default]
    #[serde(rename = "background")]
    Background,
    /// 在可见控制台窗口中执行：窗口对用户可见、可交互。
    #[serde(rename = "console")]
    Console {
        /// 命令结束后是否保留窗口：true 保留（cmd /K），false 结束后自动关闭（cmd /C）。
        #[serde(rename = "keepOpen")]
        keep_open: bool,
    },
}
