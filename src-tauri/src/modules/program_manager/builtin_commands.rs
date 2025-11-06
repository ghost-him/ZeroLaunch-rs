use serde::{Deserialize, Serialize};

/// 内置命令的前缀开头
pub const PREFIX: &str = "zerolaunch-builtin:";

/// 内置命令类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BuiltinCommandType {
    OpenSettings,
    RefreshDatabase,
    RetryRegisterShortcut,
    ToggleGameMode,
    ExitProgram,
}

/// 内置命令元数据
#[derive(Debug, Clone)]
pub struct BuiltinCommandMeta {
    pub cmd_type: BuiltinCommandType,
    pub name_key: String,
    pub unique_key: String,
    pub description_key: String,
    pub icon: String,
    pub default_keywords: Vec<String>,
}

/// 获取所有内置命令的元数据
pub fn get_all_builtin_commands() -> Vec<BuiltinCommandMeta> {
    vec![
        BuiltinCommandMeta {
            cmd_type: BuiltinCommandType::OpenSettings,
            name_key: "builtin.open_settings".to_string(),
            unique_key: format!("{}open_settings", PREFIX),
            description_key: "builtin.open_settings_desc".to_string(),
            icon: "settings".to_string(),
            default_keywords: vec!["settings".to_string(), "设置".to_string()],
        },
        BuiltinCommandMeta {
            cmd_type: BuiltinCommandType::RefreshDatabase,
            name_key: "builtin.refresh_database".to_string(),
            unique_key: format!("{}refresh_database", PREFIX),
            description_key: "builtin.refresh_database_desc".to_string(),
            icon: "refresh".to_string(),
            default_keywords: vec!["refresh".to_string(), "刷新数据".to_string()],
        },
        BuiltinCommandMeta {
            cmd_type: BuiltinCommandType::RetryRegisterShortcut,
            name_key: "builtin.retry_register_shortcut".to_string(),
            unique_key: format!("{}retry_register_shortcut", PREFIX),
            description_key: "builtin.retry_register_shortcut_desc".to_string(),
            icon: "register".to_string(),
            default_keywords: vec!["register".to_string(), "重新注册".to_string()],
        },
        BuiltinCommandMeta {
            cmd_type: BuiltinCommandType::ToggleGameMode,
            name_key: "builtin.toggle_game_mode".to_string(),
            unique_key: format!("{}toggle_game_mode", PREFIX),
            description_key: "builtin.toggle_game_mode_desc".to_string(),
            icon: "game".to_string(),
            default_keywords: vec!["game".to_string(), "游戏模式".to_string()],
        },
        BuiltinCommandMeta {
            cmd_type: BuiltinCommandType::ExitProgram,
            name_key: "builtin.exit_program".to_string(),
            unique_key: format!("{}exit_program", PREFIX),
            description_key: "builtin.exit_program_desc".to_string(),
            icon: "exit".to_string(),
            default_keywords: vec!["exit".to_string(), "退出".to_string()],
        },
    ]
}

/// 从字符串解析内置命令类型
pub fn parse_builtin_command(cmd_str: &str) -> Option<BuiltinCommandType> {
    if !cmd_str.starts_with(PREFIX) {
        return None;
    }

    let type_str = cmd_str.strip_prefix(PREFIX)?;
    match type_str {
        "OpenSettings" => Some(BuiltinCommandType::OpenSettings),
        "RefreshDatabase" => Some(BuiltinCommandType::RefreshDatabase),
        "RetryRegisterShortcut" => Some(BuiltinCommandType::RetryRegisterShortcut),
        "ToggleGameMode" => Some(BuiltinCommandType::ToggleGameMode),
        "ExitProgram" => Some(BuiltinCommandType::ExitProgram),
        _ => None,
    }
}
