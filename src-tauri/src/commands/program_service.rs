use crate::commands::ui_command::hide_window;
use crate::modules::config::config_manager::PartialRuntimeConfig;
use crate::modules::config::default::ICON_CACHE_DIR;
use crate::modules::config::default::MODELS_DIR;
use crate::modules::program_manager::FallbackReason;
use crate::modules::program_manager::{LaunchMethod, LaunchMethodKind};
use crate::save_config_to_file;
use crate::state::app_state::AppState;
use crate::utils::notify::notify;
use crate::utils::service_locator::ServiceLocator;
use crate::utils::windows::shell_execute_open;
use crate::ProgramManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tauri::Runtime;
use tracing::{debug, info, warn};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramInfo {
    pub name: String,
    pub is_uwp: bool,
    pub bias: f64,
    pub path: String,
    pub history_launch_time: u64,
}

/// 更新搜索窗口

#[derive(Serialize, Debug)]
pub struct SearchResult(u64, String, String);

#[derive(Serialize, Debug)]
pub struct LaunchTemplateInfo {
    template: String,
    kind: LaunchMethodKind,
    placeholder_count: usize,
    show_name: String,
}

/// 执行内置命令
async fn execute_builtin_command(cmd_str: &str) -> Result<(), String> {
    use crate::modules::program_manager::builtin_commands::{
        parse_builtin_command, BuiltinCommandType,
    };

    let cmd_type =
        parse_builtin_command(cmd_str).ok_or_else(|| format!("未知的内置命令: {}", cmd_str))?;

    info!("🔧 执行内置命令: {:?}", cmd_type);

    match cmd_type {
        BuiltinCommandType::OpenSettings => {
            crate::tray::handle_show_settings_window();
        }
        BuiltinCommandType::RefreshDatabase => {
            crate::tray::handle_update_app_setting();
        }
        BuiltinCommandType::RetryRegisterShortcut => {
            crate::tray::handle_register_shortcut();
        }
        BuiltinCommandType::ToggleGameMode => {
            crate::tray::handle_toggle_game_mode();
        }
        BuiltinCommandType::ExitProgram => {
            let state = ServiceLocator::get_state();
            let app_handle = state.get_main_handle();
            crate::tray::handle_exit_program(&app_handle).await;
        }
    }

    Ok(())
}

/// 尝试唤醒已存在的程序窗口
async fn try_activate_window(program_manager: &ProgramManager, program_guid: u64) -> bool {
    debug!("🔍 尝试唤醒现有程序窗口: GUID={}", program_guid);
    let activated = program_manager.activate_target_program(program_guid).await;
    if activated {
        info!("✅ 程序窗口唤醒成功: GUID={}", program_guid);
    } else {
        debug!("⚠️ 程序窗口唤醒失败: GUID={}", program_guid);
    }
    activated
}

/// 启动新程序实例
async fn launch_new_program(
    program_manager: &ProgramManager,
    program_guid: u64,
    is_admin_required: bool,
    override_method: Option<LaunchMethod>,
) {
    debug!(
        "🚀 启动新程序实例: GUID={}, 管理员权限={}, 覆盖方法={}",
        program_guid,
        is_admin_required,
        override_method.is_some()
    );
    program_manager
        .launch_program(program_guid, is_admin_required, override_method)
        .await;
}

/// 统一记录一次“用户使用该程序/命令”的意图：
/// - 由 ProgramManager 统一记录 ranker 与查询关联
/// - 并保存配置
async fn record_full_launch(
    state: &tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
    query_text: String,
) {
    let program_manager = state.get_program_manager();
    debug!(
        "📝 记录使用: query='{}' -> GUID={}",
        query_text, program_guid
    );
    program_manager.record_query_launch(&query_text, program_guid);

    debug!("💾 保存配置文件");
    save_config_to_file(false).await;
    info!("✅ 使用意图统计完成: GUID={}", program_guid);
}

/// 协调程序启动流程并处理可选的覆盖启动方式
async fn launch_program_internal(
    state: tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
    ctrl: bool,
    shift: bool,
    query_text: String,
    override_method: Option<LaunchMethod>,
) -> Result<(), String> {
    info!(
        "🚀 启动程序请求: GUID={}, Ctrl={}, Shift={}, Override={}",
        program_guid,
        ctrl,
        shift,
        override_method.is_some()
    );

    let program_manager = state.get_program_manager();

    // 1. 先隐藏窗口
    if let Err(e) = hide_window() {
        warn!("⚠️ 隐藏窗口失败: {:?}", e);
        return Err(format!("Failed to hide window: {:?}", e));
    }

    // 2. 获取程序信息
    let program = program_manager
        .get_program_by_guid(program_guid)
        .await
        .ok_or_else(|| format!("未找到程序: GUID={}", program_guid))?;

    // 3. 检查是否是内置命令 (这是一个独立的逻辑分支，提前返回是清晰的)
    if let LaunchMethod::BuiltinCommand(ref cmd_str) = program.launch_method {
        let result = execute_builtin_command(cmd_str).await;
        // 无论命令执行是否成功，都记录一次用户意图
        record_full_launch(&state, program_guid, query_text).await;
        return result;
    }

    // 4. 处理普通程序的启动逻辑
    let is_admin_required = ctrl;
    let should_activate_window = shift;

    let need_launch_new = if should_activate_window {
        let activated = try_activate_window(&program_manager, program_guid).await;
        let launch_new_on_failure = state
            .get_runtime_config()
            .get_app_config()
            .get_launch_new_on_failure();

        if activated {
            info!("✅ 窗口唤醒成功，无需启动新实例");
            false
        } else {
            // 唤醒失败，根据配置与 UWP 判断是否启动
            if program_manager.is_uwp_program(program_guid).await {
                debug!("⚠️ UWP 程序窗口唤醒失败，尝试启动新实例");
                true
            } else if launch_new_on_failure {
                debug!("⚠️ 普通程序窗口唤醒失败，配置允许 -> 启动新实例");
                true
            } else {
                info!("⚠️ 窗口唤醒失败，配置禁止启动新实例 -> 不启动");
                false
            }
        }
    } else {
        // 未请求唤醒，直接需要启动
        true
    };

    // 5. 根据决策执行启动动作
    if need_launch_new {
        launch_new_program(
            &program_manager,
            program_guid,
            is_admin_required,
            override_method,
        )
        .await;
    }

    // 6. 统一记录本次用户意图
    // 无论是成功唤醒、启动新实例，还是唤醒失败但不启动，都代表了一次用户意图的完成。
    record_full_launch(&state, program_guid, query_text).await;

    Ok(())
}

#[tauri::command]
pub async fn load_program_icon<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
) -> Result<Vec<u8>, String> {
    let program_manager = state.get_program_manager();
    let result = program_manager.get_icon(&program_guid).await;
    if result.is_empty() {
        warn!("id： {}， 获得图标失败！", program_guid);
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_program_count<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    let program_manager = state.get_program_manager();
    let result = program_manager.get_program_count().await;
    Ok(result)
}

#[tauri::command]
pub async fn command_get_program_url_status<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<bool>, String> {
    let program_manager = state.get_program_manager();
    Ok(program_manager.get_program_is_url_list().await)
}

#[tauri::command]
/// 统一的程序启动接口
///
/// # 参数
/// - `program_guid`: 程序唯一标识
/// - `ctrl`: 是否按下 Ctrl 键(请求管理员权限)
/// - `shift`: 是否按下 Shift 键(唤醒已存在窗口)
/// - `args`: 用户参数数组,无参数时传递空数组 []
pub async fn launch_program(
    state: tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
    ctrl: bool,
    shift: bool,
    args: Vec<String>,
    query_text: String,
) -> Result<(), String> {
    use crate::modules::parameter_resolver::SystemParameterSnapshot;

    // 总是捕获系统参数快照(性能影响可忽略,约 0.1ms)
    let snapshot = SystemParameterSnapshot::capture();

    let program_manager = state.get_program_manager();

    // 使用传入的用户参数数组(可能为空),同时解析系统参数
    let override_method = program_manager
        .build_launch_method_with_args(program_guid, &args, &snapshot)
        .await
        .map_err(|e| format!("Failed to build launch method: {}", e))?;

    launch_program_internal(
        state,
        program_guid,
        ctrl,
        shift,
        query_text,
        Some(override_method),
    )
    .await
}

#[tauri::command]
/// 获取指定程序的启动模板与占位符元数据
pub async fn get_launch_template_info<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
) -> Result<LaunchTemplateInfo, String> {
    let program_manager = state.get_program_manager();
    let (template, kind, placeholder_count, show_name) = program_manager
        .get_launch_template_info(program_guid)
        .await
        .ok_or_else(|| format!("Program GUID {} not found", program_guid))?;

    Ok(LaunchTemplateInfo {
        template,
        kind,
        placeholder_count,
        show_name,
    })
}

#[tauri::command]
pub async fn get_program_info<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<ProgramInfo>, String> {
    let manager = state.get_program_manager();
    let data = manager.get_program_infos().await;
    debug!("{:?}", data);
    let mut program_infos = Vec::new();
    for item in data {
        program_infos.push(ProgramInfo {
            name: item.0,
            is_uwp: item.1,
            bias: item.2,
            path: item.3,
            history_launch_time: item.4,
        })
    }
    Ok(program_infos)
}

#[tauri::command]
pub async fn refresh_program<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
) -> Result<(), String> {
    crate::tray::handle_update_app_setting();
    Ok(())
}

/// 处理前端发来的消息
#[tauri::command]
pub async fn handle_search_text<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    search_text: String,
) -> Result<Vec<SearchResult>, String> {
    use tracing::{debug, info};

    debug!("🔍 处理搜索请求: '{}'", search_text);

    let runtime_config = state.get_runtime_config();

    let result_count = runtime_config.get_app_config().get_search_result_count();
    debug!("📊 搜索配置: 最大结果数={}", result_count);

    // 处理消息
    let program_manager = state.get_program_manager();

    let results = program_manager.update(&search_text, result_count).await;
    debug!("🎯 搜索完成: 找到 {} 个结果", results.len());

    let mut ret = Vec::new();
    for item in results {
        ret.push(SearchResult(item.0, item.1, item.2));
    }

    if search_text.trim().is_empty() {
        debug!("📝 空搜索请求，返回默认结果");
    } else {
        info!("🔍 搜索完成: '{}' -> {} 个结果", search_text, ret.len());
    }

    Ok(ret)
}

/// 启动Everything搜索结果
#[cfg(target_arch = "x86_64")]
#[tauri::command]
pub async fn launch_everything_item<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let everything_manager = state.get_everything_manager();
    everything_manager.launch(&path);
    Ok(())
}

/// 启动Everything搜索结果 (arm64 不支持)
#[cfg(not(target_arch = "x86_64"))]
#[tauri::command]
pub async fn launch_everything_item<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    _state: tauri::State<'_, Arc<AppState>>,
    _path: String,
) -> Result<(), String> {
    Ok(())
}

/// 处理Everything搜索请求
#[cfg(target_arch = "x86_64")]
#[tauri::command]
pub async fn handle_everything_search<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    search_text: String,
) -> Result<Vec<SearchResult>, String> {
    let everything_manager = state.get_everything_manager();
    let results = everything_manager.search(&search_text)?;

    Ok(results
        .into_iter()
        .map(|r| SearchResult(r.id, r.path.clone(), r.path))
        .collect())
}

/// 处理Everything搜索请求 (arm64 不支持)
#[cfg(not(target_arch = "x86_64"))]
#[tauri::command]
pub async fn handle_everything_search<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    _state: tauri::State<'_, Arc<AppState>>,
    _search_text: String,
) -> Result<Vec<SearchResult>, String> {
    Ok(Vec::new())
}

/// 使用记事本打开 Everything 搜索结果
#[cfg(target_arch = "x86_64")]
#[tauri::command]
pub async fn everything_open_with_notepad<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    path: String,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    if let Err(e) = hide_window() {
        warn!("隐藏窗口失败: {:?}", e);
    }

    let result = std::process::Command::new("notepad.exe")
        .arg(&path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    if let Err(e) = result {
        warn!("使用记事本打开文件失败: {:?}, 路径: {}", e, path);
        return Err(format!("Failed to open with notepad: {}", e));
    }

    Ok(())
}

/// 使用记事本打开 Everything 搜索结果 (arm64 不支持)
#[cfg(not(target_arch = "x86_64"))]
#[tauri::command]
pub async fn everything_open_with_notepad<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    _path: String,
) -> Result<(), String> {
    Ok(())
}

/// 在资源管理器中打开 Everything 搜索结果所在文件夹并选中文件
#[cfg(target_arch = "x86_64")]
#[tauri::command]
pub async fn everything_open_file_location<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    path: String,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    if let Err(e) = hide_window() {
        warn!("隐藏窗口失败: {:?}", e);
    }

    let windows_path = path.replace("/", "\\");
    let path_obj = Path::new(&windows_path);
    let spawn_result = if path_obj.is_dir() {
        // 如果目标本身是目录，直接在资源管理器中打开
        std::process::Command::new("explorer.exe")
            .arg(&windows_path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    } else {
        // 如果是文件，打开其所在目录并选中该文件
        // 使用 cmd /C 包裹 explorer 命令，并通过 raw_arg 避免 Rust 对引号的自动转义
        // 确保路径中的空格被正确引号包裹，explorer 能正确解析 /select,"path" 语法
        std::process::Command::new("cmd")
            .raw_arg(format!("/C explorer /select,\"{}\"", windows_path))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    };

    if let Err(e) = spawn_result {
        warn!("打开文件所在位置失败: {:?}, 路径: {}", e, path);
        return Err(format!("Failed to open file location: {}", e));
    }

    Ok(())
}

/// 在资源管理器中打开 Everything 搜索结果所在文件夹 (arm64 不支持)
#[cfg(not(target_arch = "x86_64"))]
#[tauri::command]
pub async fn everything_open_file_location<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    _path: String,
) -> Result<(), String> {
    Ok(())
}

/// 在资源管理器中打开 Everything 搜索结果所在目录
#[cfg(target_arch = "x86_64")]
#[tauri::command]
pub async fn everything_enable_path_match<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    enable: bool,
) -> Result<(), String> {
    let everything_manager = state.get_everything_manager();
    everything_manager.enable_path_match(enable);
    Ok(())
}

#[cfg(not(target_arch = "x86_64"))]
#[tauri::command]
pub async fn everything_enable_path_match<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    enable: bool,
) -> Result<(), String> {
    Ok(())
}

/// 获得最近启动的程序
#[tauri::command]
pub async fn command_get_latest_launch_program(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<SearchResult>, String> {
    let runtime_config = state.get_runtime_config();
    let result_count = runtime_config.get_app_config().get_search_result_count();
    // 处理消息
    let program_manager = state.get_program_manager();
    let results = program_manager
        .get_latest_launch_program(result_count)
        .await;
    let mut ret = Vec::new();
    for item in results {
        ret.push(SearchResult(item.0, item.1, item.2));
    }
    debug!("latest_launch_propgram: {:?}", ret);
    Ok(ret)
}

/// 获取当前搜索状态提示（用于右下角展示）
#[tauri::command]
pub async fn command_get_search_status_tip(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let pm = state.get_program_manager();
    let reason = pm.get_fallback_reason().await;
    // 仅返回原因码，前端基于 i18n 进行本地化展示
    let code = match reason {
        FallbackReason::None => "none",
        FallbackReason::AiDisabled => "ai_disabled",
        FallbackReason::ModelNotReady => "model_not_ready",
    };
    Ok(code.to_string())
}

#[tauri::command]
pub async fn command_load_remote_config<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PartialRuntimeConfig, String> {
    let runtime_config = state.get_runtime_config();
    Ok(runtime_config.to_partial())
}

#[tauri::command]
pub async fn open_target_folder<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
) -> Result<bool, String> {
    let program_manager = state.get_program_manager();
    if let Err(e) = hide_window() {
        return Err(format!("Failed to hide window: {:?}", e));
    }
    let result = program_manager.open_target_folder(program_guid).await;
    if !result {
        notify("ZeroLaunch-rs", "打开文件夹失败，目标类型不支持");
        return Ok(false);
    }
    Ok(true)
}

#[tauri::command]
#[allow(clippy::zombie_processes)]
pub async fn command_open_models_dir<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
) -> Result<(), String> {
    let target_path = Path::new(&*MODELS_DIR);
    shell_execute_open(target_path)
        .map_err(|error| format!("Failed to open models directory: {}", error.to_hresult()))?;
    Ok(())
}

#[tauri::command]
#[allow(clippy::zombie_processes)]
pub async fn command_open_icon_cache_dir<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
) -> Result<(), String> {
    let target_path = Path::new(&*ICON_CACHE_DIR);
    shell_execute_open(target_path).map_err(|error| {
        format!(
            "Failed to open icon cache directory: {}",
            error.to_hresult()
        )
    })?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PathInfo {
    path_type: String, // "file"、"directory" 或 "error"
    original_path: String,
    parent_path: Option<String>,
    filename: Option<String>,
    error_message: Option<String>,
}

#[tauri::command]
pub fn command_get_path_info(path_str: String) -> PathInfo {
    let path = Path::new(&path_str);
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                PathInfo {
                    path_type: "directory".to_string(),
                    original_path: path_str,
                    parent_path: None,
                    filename: None,
                    error_message: None,
                }
            } else if metadata.is_file() {
                let parent = path.parent().and_then(|p| p.to_str()).map(String::from);
                let filename = path.file_name().and_then(|n| n.to_str()).map(String::from);
                PathInfo {
                    path_type: "file".to_string(),
                    original_path: path_str,
                    parent_path: parent,
                    filename,
                    error_message: None,
                }
            } else {
                // 如果已检索到元数据，则不应经常发生，但要进行防御性处理
                PathInfo {
                    path_type: "error".to_string(),
                    original_path: path_str,
                    parent_path: None,
                    filename: None,
                    error_message: Some("Path is neither a file nor a directory.".to_string()),
                }
            }
        }
        Err(e) => PathInfo {
            path_type: "error".to_string(),
            original_path: path_str,
            parent_path: None,
            filename: None,
            error_message: Some(format!("Failed to access path metadata: {}", e)),
        },
    }
}

use crate::modules::icon_manager::IconRequest;
use crate::modules::program_manager::ProgramDisplayInfo;

#[tauri::command]
pub async fn command_search_programs_lightweight(
    keyword: String,
    load_all: Option<bool>,
) -> Result<Vec<ProgramDisplayInfo>, String> {
    let state = ServiceLocator::get_state();
    let program_manager = state.get_program_manager();
    Ok(program_manager
        .search_programs_lightweight(&keyword, load_all.unwrap_or(false))
        .await)
}

#[tauri::command]
pub async fn command_update_program_icon(
    icon_request_json: String,
    new_icon_path: String,
) -> Result<(), String> {
    let state = ServiceLocator::get_state();
    let icon_manager = state.get_icon_manager();

    let icon_request: IconRequest =
        serde_json::from_str(&icon_request_json).map_err(|e| e.to_string())?;

    icon_manager
        .update_program_icon_cache(icon_request, &new_icon_path)
        .await
        .map_err(|e| e.to_string())
}

/// 添加屏蔽路径（用于右键菜单屏蔽功能）
/// 将路径添加到 forbidden_paths 配置中，并保存配置文件
/// 需要用户手动刷新数据库后才能生效
#[tauri::command]
pub async fn command_add_forbidden_path(path: String) -> Result<(), String> {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config();

    // 获取当前的 forbidden_paths
    let program_manager_config = runtime_config.get_program_manager_config();
    let loader_config = program_manager_config.get_loader_config();
    let mut forbidden_paths = loader_config.get_forbidden_paths();

    // 检查是否已存在
    if forbidden_paths.contains(&path) {
        return Ok(());
    }

    // 添加新路径
    forbidden_paths.push(path);

    // 更新配置
    use crate::modules::program_manager::config::program_loader_config::PartialProgramLoaderConfig;
    use crate::modules::program_manager::config::program_manager_config::PartialProgramManagerConfig;
    use tauri::Emitter;

    let partial_loader = PartialProgramLoaderConfig {
        forbidden_paths: Some(forbidden_paths),
        ..Default::default()
    };

    let partial_pm = PartialProgramManagerConfig {
        loader: Some(partial_loader),
        ..Default::default()
    };

    runtime_config.update(PartialRuntimeConfig {
        program_manager_config: Some(partial_pm),
        ..Default::default()
    });

    // 保存配置文件（不触发刷新）
    save_config_to_file(false).await;

    // 通知设置窗口更新配置
    let handle = state.get_main_handle();
    let _ = handle.emit("emit_update_setting_window_config", "");

    Ok(())
}

/// 获取程序的路径（用于屏蔽功能）
#[tauri::command]
pub async fn command_get_program_path(program_guid: u64) -> Result<String, String> {
    let state = ServiceLocator::get_state();
    let program_manager = state.get_program_manager();

    let program = program_manager
        .get_program_by_guid(program_guid)
        .await
        .ok_or_else(|| "Program not found".to_string())?;

    Ok(program.launch_method.get_text().clone())
}
