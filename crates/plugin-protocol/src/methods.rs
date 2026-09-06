/// Host-to-plugin method names (plugin/* namespace).
pub mod plugin {
    /// 进程级握手：插件初始化（版本协商、目录、locale 下发）。
    pub const INITIALIZE: &str = "plugin/initialize";
    /// 拉取插件元数据（PluginMetadata）。
    pub const GET_METADATA: &str = "plugin/get_metadata";
    /// 拉取插件实现的全部组件清单（ComponentDescriptor 列表）。
    pub const GET_COMPONENTS: &str = "plugin/get_components";
    /// 拉取指定组件的设置 schema。
    pub const GET_SETTINGS_SCHEMA: &str = "plugin/get_settings_schema";
    /// 拉取指定组件当前的设置值。
    pub const GET_SETTINGS: &str = "plugin/get_settings";
    /// 指定组件应用新的设置值。
    pub const APPLY_SETTINGS: &str = "plugin/apply_settings";
    /// 校验一组设置值是否合法（不实际应用）。
    pub const VALIDATE_SETTINGS: &str = "plugin/validate_settings";
    /// 拉取指定组件的配置动作列表。
    pub const CONFIG_ACTIONS: &str = "plugin/config_actions";
    /// 执行指定组件的配置动作。
    pub const EXECUTE_CONFIG_ACTION: &str = "plugin/execute_config_action";
    /// 插件查询（Plugin 组件）。
    pub const QUERY: &str = "plugin/query";
    /// 执行插件动作（Plugin 组件）。
    pub const EXECUTE_ACTION: &str = "plugin/execute_action";
    /// 插件初始化钩子（注册完成后通知，携带真实查询上下文）。
    pub const INIT: &str = "plugin/init";
    /// 拉取面板交互策略（PanelInteraction，插件级语义）。
    /// 插件级语义：宿主仅对 Plugin 种类组件调用；SDK 恒返回主插件策略。
    pub const INTERACTION_POLICY: &str = "plugin/interaction_policy";
    /// 拉取指定组件的默认启用状态。
    pub const GET_DEFAULT_ENABLED: &str = "plugin/get_default_enabled";
    /// DataSource 组件：采集候选项。
    pub const FETCH_CANDIDATES: &str = "plugin/fetch_candidates";
    /// ActionExecutor 组件：支持的目标类型列表。
    pub const SUPPORTED_TARGET_TYPES: &str = "plugin/supported_target_types";
    /// ActionExecutor 组件：支持的动作列表。
    pub const SUPPORTED_ACTIONS: &str = "plugin/supported_actions";
    /// ActionExecutor 组件：执行动作。
    pub const EXECUTOR_EXECUTE: &str = "plugin/executor_execute";
    /// SearchEngine 组件：对缓存候选计算分数。
    pub const CALCULATE_SCORES: &str = "plugin/calculate_scores";
    /// ScoreBooster 组件：对已计算分数做增强。
    pub const BOOSTER_BOOST: &str = "plugin/booster_boost";
    /// ScoreBooster 组件：记录用户确认（学习用户习惯）。
    pub const BOOSTER_RECORD: &str = "plugin/booster_record";
    /// KeywordOptimizer 组件：拉取声明属性（input_source / priority）。
    pub const KEYWORD_OPTIMIZER_INFO: &str = "plugin/keyword_optimizer_info";
    /// KeywordOptimizer 组件：优化单个关键词。
    pub const KEYWORD_OPTIMIZE: &str = "plugin/keyword_optimize";
    /// KeywordInjector 组件：对单个候选注入关键词。
    pub const KEYWORD_INJECT: &str = "plugin/keyword_inject";
}

/// Plugin-to-host method names (host/* namespace).
pub mod host {
    /// 插件侧日志写入（转发到宿主日志系统）。
    pub const LOG: &str = "host/log";
    /// 插件侧通知弹窗（宿主桌面通知）。
    pub const NOTIFY: &str = "host/notify";
    /// 打开指定路径/URL（系统关联）。
    pub const SHELL_OPEN: &str = "host/shell.open";
    /// 打开资源管理器文件夹。
    pub const SHELL_OPEN_FOLDER: &str = "host/shell.open_folder";
    /// 以提权方式执行命令。
    pub const SHELL_EXECUTE_ELEVATION: &str = "host/shell.execute_elevation";
    /// 执行命令（普通权限）。
    pub const SHELL_EXECUTE_COMMAND: &str = "host/shell.execute_command";
    /// 激活指定进程对应的窗口。
    pub const WINDOW_ACTIVATE_BY_PROCESS: &str = "host/window.activate_by_process";
    /// 获取图标（返回 WebP/PNG base64 字节）。
    pub const ICON_GET: &str = "host/icon.get";
    /// 枚举已安装应用。
    pub const APP_ENUMERATE: &str = "host/app.enumerate";
    /// 解析路径（环境变量展开等）。
    pub const PATH_RESOLVE: &str = "host/path.resolve";
    /// 上传资源文件到插件目录。
    pub const RESOURCE_UPLOAD: &str = "host/resource.upload";
    /// 写入资源文件。
    pub const RESOURCE_PUT: &str = "host/resource.put";
    /// 读取资源文件。
    pub const RESOURCE_GET: &str = "host/resource.get";
    /// 删除资源文件。
    pub const RESOURCE_DELETE: &str = "host/resource.delete";
    /// 列出插件目录资源。
    pub const RESOURCE_LIST: &str = "host/resource.list";
    /// 解析参数模板。
    pub const PARAMETER_RESOLVE: &str = "host/parameter.resolve";
    /// 拉取宿主当前 locale。
    pub const GET_LOCALE: &str = "host/i18n.get_locale";
    /// 拉取宿主当前主题。
    pub const GET_THEME: &str = "host/theme.get";
    /// 拉取全网模型清单（聚合所有提供方）。
    pub const MODEL_LIST: &str = "host/model.list";
    /// 按 model_id 调用文本生成。
    pub const MODEL_CHAT: &str = "host/model.chat";
    /// 按 model_id 调用文本向量化。
    pub const MODEL_EMBEDDING: &str = "host/model.embedding";
    /// 按 model_id 计算查询向量与多个目标向量的相似度。
    pub const MODEL_SIMILARITY: &str = "host/model.similarity";
}

/// Notification-only method names (fire-and-forget, no response expected).
pub mod notify {
    /// 插件进程就绪通知。
    pub const PLUGIN_READY: &str = "plugin/ready";
    /// 插件进程崩溃通知。
    pub const PLUGIN_CRASHED: &str = "plugin/crashed";
}
