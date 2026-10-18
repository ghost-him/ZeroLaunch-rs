/// Host-to-plugin method names (plugin/* namespace).
pub mod plugin {
    /// 进程级握手：插件初始化（版本协商、目录、locale 下发）。
    pub const INITIALIZE: &str = "plugin/initialize";
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
    /// 查询匹配裁决（Plugin 组件，插件级语义）：宿主在路由阶段询问插件是否接管当前输入。
    /// 响应为布尔值（true = 接管）；旧插件未实现时宿主按 METHOD_NOT_FOUND 容错，
    /// 用同一份框架关键词判定兜底。
    pub const MATCH_QUERY: &str = "plugin/match_query";
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
    /// 执行命令（普通权限；呈现方式由载荷的 mode 决定）。
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
    /// 查询宿主平台能力集（插件据此决定是否降级）。
    pub const PLATFORM_CAPABILITIES: &str = "host/platform.capabilities";
    /// 更新本插件的 SDK 配置（如图标缓存等级），立即生效。
    pub const SDK_CONFIG_UPDATE: &str = "host/sdk_config.update";
    /// 提取图标；提取失败返回错误（与 host/icon.get 的默认图标回退不同）。
    pub const ICON_EXTRACT: &str = "host/icon.extract";
    /// 强制从磁盘提取图标并写回缓存（跳过缓存读取）。
    pub const ICON_REFRESH: &str = "host/icon.refresh";
    /// 用自定义图标文件覆盖指定图标请求的缓存条目。
    pub const ICON_OVERRIDE_CACHE: &str = "host/icon.override_cache";
    /// 按进程 PID 激活已存在的窗口。
    pub const WINDOW_ACTIVATE_BY_PID: &str = "host/window.activate_by_pid";
    /// 按窗口标题片段激活已存在的窗口。
    pub const WINDOW_ACTIVATE_BY_TITLE: &str = "host/window.activate_by_title";
    /// 将文本写入系统剪贴板。
    pub const CLIPBOARD_SET_TEXT: &str = "host/clipboard.set_text";
    /// 启动已安装应用，返回进程 PID。
    pub const APP_LAUNCH: &str = "host/app.launch";
    /// 查询内置应用图标资源的文件系统路径。
    pub const APP_ICON_PATH: &str = "host/app.icon_path";
    /// 解析 .lnk 快捷方式文件的目标路径。
    pub const PATH_RESOLVE_LNK: &str = "host/path.resolve_lnk";
    /// 解析目录下 desktop.ini 的本地化文件名。
    pub const PATH_LOCALIZED_NAMES: &str = "host/path.localized_names";
    /// 查询系统主题（未经宿主显式 light/dark 配置解析）。
    pub const THEME_GET_SYSTEM: &str = "host/theme.get_system";
    /// 统计参数模板中需要用户输入的参数数量。
    pub const PARAMETER_COUNT_USER: &str = "host/parameter.count_user";
    /// 判断参数模板是否包含系统参数。
    pub const PARAMETER_HAS_SYSTEM: &str = "host/parameter.has_system";
    /// 写入插件本地缓存（可再生的本地数据，不经存储后端）。
    pub const CACHE_PUT: &str = "host/cache.put";
    /// 读取插件本地缓存；不存在时返回 null。
    pub const CACHE_GET: &str = "host/cache.get";
    /// 删除插件本地缓存条目。
    pub const CACHE_DELETE: &str = "host/cache.delete";
    /// 按容量上限清理插件本地缓存域。
    pub const CACHE_CLEANUP: &str = "host/cache.cleanup";
    /// 创建一次性定时器，到期由宿主推送 `plugin/timer.fired`。
    pub const TIMER_SET_TIMEOUT: &str = "host/timer.set_timeout";
    /// 创建重复定时器，每次到期由宿主推送 `plugin/timer.fired`。
    pub const TIMER_SET_INTERVAL: &str = "host/timer.set_interval";
    /// 取消指定的本插件定时器。
    pub const TIMER_CANCEL: &str = "host/timer.cancel";
    /// 取消本插件的全部定时器。
    pub const TIMER_CANCEL_ALL: &str = "host/timer.cancel_all";
    /// 订阅按键事件，宿主推送 `plugin/hotkey.event`。
    pub const HOTKEY_SUBSCRIBE: &str = "host/hotkey.subscribe";
    /// 退订按键事件。
    pub const HOTKEY_UNSUBSCRIBE: &str = "host/hotkey.unsubscribe";
    /// 订阅安装监控事件，宿主推送 `plugin/installation.event`。
    pub const INSTALLATION_SUBSCRIBE: &str = "host/installation.subscribe";
    /// 退订安装监控事件。
    pub const INSTALLATION_UNSUBSCRIBE: &str = "host/installation.unsubscribe";
    /// 订阅窗口焦点事件，宿主推送 `plugin/focus.event`。
    pub const FOCUS_SUBSCRIBE: &str = "host/focus.subscribe";
    /// 退订窗口焦点事件。
    pub const FOCUS_UNSUBSCRIBE: &str = "host/focus.unsubscribe";
}

/// Host-to-plugin push methods (fire-and-forget, no response expected).
///
/// 与 `plugin/*` 请求的区别：这些方法由宿主单向推送给插件进程，用于把
/// `host/timer.*` / `host/*.subscribe` 注册的事件送达插件；插件不应答。
pub mod event {
    /// 定时器到期（参数：`{ id }`，id 为 host/timer.set_* 返回的定时器 ID）。
    pub const TIMER_FIRED: &str = "plugin/timer.fired";
    /// 按键事件（参数：`{ event }`，HotkeyEvent 序列化形态）。
    pub const HOTKEY_EVENT: &str = "plugin/hotkey.event";
    /// 安装监控事件（参数：`{ event }`，InstallationEvent 序列化形态）。
    pub const INSTALLATION_EVENT: &str = "plugin/installation.event";
    /// 窗口焦点事件（参数：`{ event }`，FocusEvent 序列化形态）。
    pub const FOCUS_EVENT: &str = "plugin/focus.event";
}

/// Notification-only method names (fire-and-forget, no response expected).
pub mod notify {
    /// 插件进程就绪通知。
    pub const PLUGIN_READY: &str = "plugin/ready";
    /// 插件进程崩溃通知。
    pub const PLUGIN_CRASHED: &str = "plugin/crashed";
}
