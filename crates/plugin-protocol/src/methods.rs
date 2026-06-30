/// Host-to-plugin method names (plugin/* namespace).
pub mod plugin {
    pub const INITIALIZE: &str = "plugin/initialize";
    pub const SHUTDOWN: &str = "plugin/shutdown";
    pub const GET_METADATA: &str = "plugin/get_metadata";
    pub const GET_COMPONENTS: &str = "plugin/get_components";
    pub const GET_SETTINGS_SCHEMA: &str = "plugin/get_settings_schema";
    pub const GET_SETTINGS: &str = "plugin/get_settings";
    pub const APPLY_SETTINGS: &str = "plugin/apply_settings";
    pub const VALIDATE_SETTINGS: &str = "plugin/validate_settings";
    pub const CONFIG_ACTIONS: &str = "plugin/config_actions";
    pub const EXECUTE_CONFIG_ACTION: &str = "plugin/execute_config_action";
    pub const QUERY: &str = "plugin/query";
    pub const EXECUTE_ACTION: &str = "plugin/execute_action";
    pub const FETCH_CANDIDATES: &str = "plugin/fetch_candidates";
    pub const SUPPORTED_TARGET_TYPES: &str = "plugin/supported_target_types";
    pub const SUPPORTED_ACTIONS: &str = "plugin/supported_actions";
    pub const EXECUTOR_EXECUTE: &str = "plugin/executor_execute";
}

/// Plugin-to-host method names (host/* namespace).
pub mod host {
    pub const LOG: &str = "host/log";
    pub const NOTIFY: &str = "host/notify";
    pub const SHELL_OPEN: &str = "host/shell.open";
    pub const SHELL_OPEN_FOLDER: &str = "host/shell.open_folder";
    pub const SHELL_EXECUTE_ELEVATION: &str = "host/shell.execute_elevation";
    pub const SHELL_EXECUTE_COMMAND: &str = "host/shell.execute_command";
    pub const WINDOW_ACTIVATE_BY_PROCESS: &str = "host/window.activate_by_process";
    pub const ICON_GET: &str = "host/icon.get";
    pub const APP_ENUMERATE: &str = "host/app.enumerate";
    pub const PATH_RESOLVE: &str = "host/path.resolve";
    pub const RESOURCE_UPLOAD: &str = "host/resource.upload";
    pub const RESOURCE_PUT: &str = "host/resource.put";
    pub const RESOURCE_GET: &str = "host/resource.get";
    pub const RESOURCE_DELETE: &str = "host/resource.delete";
    pub const RESOURCE_LIST: &str = "host/resource.list";
    pub const PARAMETER_RESOLVE: &str = "host/parameter.resolve";
}

/// Notification-only method names (fire-and-forget, no response expected).
pub mod notify {
    pub const PLUGIN_READY: &str = "plugin/ready";
    pub const PLUGIN_CRASHED: &str = "plugin/crashed";
}
