use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::core::storage::utils::read_str;
use crate::core::storage::windows_utils::get_default_remote_data_dir_path;
use crate::error::ResultExt;
use crate::modules::config::default::LOG_DIR;
use crate::modules::version_checker::VersionChecker;
use crate::utils::font_database::get_fonts;
use tracing::{error, info};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

/// 获得当前程序的最新版本
#[tauri::command]
pub async fn command_get_latest_release_version() -> String {
    let result = VersionChecker::get_latest_release_version().await;
    match result {
        Ok(data) => data,
        Err(e) => e.to_string(),
    }
}

/// 获得默认的远程配置文件保存地址
#[tauri::command]
pub fn command_get_default_remote_data_dir_path() -> String {
    get_default_remote_data_dir_path()
}

/// 获取当前的字体
/// 虽然后端向前端发送的是HashSet，但是 tauri 会在传输的过程中将这个变量一个普通的数组，所以前端可以使用string[]来接收
#[tauri::command]
pub fn command_get_system_fonts() -> HashSet<String> {
    get_fonts()
}

#[tauri::command]
pub async fn command_read_file(path: String) -> String {
    let result = tauri::async_runtime::spawn_blocking(move || {
        // 在这里可以安全地执行阻塞代码
        read_str(&path).expect_programming(&format!("读取当前文件失败：{}", &path))
    })
    .await;
    result.expect_programming("读取文件失败")
}

/// 导出日志文件
///
/// 将 logs 文件夹中的所有日志文件打包成 ZIP 压缩包，并保存到用户指定的位置
///
/// # 参数
/// * `save_path` - 用户选择的保存路径（完整的文件路径，包含文件名）
///
/// # 返回
/// * `Result<(), String>` - 成功返回 Ok(())，失败返回错误信息
#[tauri::command]
pub async fn command_export_logs(save_path: String) -> Result<(), String> {
    info!("开始导出日志到: {}", save_path);

    let log_dir = LOG_DIR.clone();
    let save_path_clone = save_path.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        export_logs_internal(&log_dir, &save_path_clone)
    })
    .await;

    match result {
        Ok(Ok(())) => {
            info!("日志导出成功: {}", save_path);
            Ok(())
        }
        Ok(Err(e)) => {
            error!("日志导出失败: {}", e);
            Err(e)
        }
        Err(e) => {
            let error_msg = format!("导出日志任务执行失败: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 内部函数：执行日志导出的实际逻辑
fn export_logs_internal(log_dir: &str, save_path: &str) -> Result<(), String> {
    let log_path = Path::new(log_dir);

    // 检查日志目录是否存在
    if !log_path.exists() {
        return Err(format!("日志目录不存在: {}", log_dir));
    }

    // 创建 ZIP 文件
    let file = File::create(save_path).map_err(|e| format!("创建ZIP文件失败: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // 使用 walkdir 遍历日志目录并添加所有文件到 ZIP
    for entry in WalkDir::new(log_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // 只处理文件，跳过目录
        if !path.is_file() {
            continue;
        }

        // 获取相对路径
        let name = path
            .strip_prefix(log_path)
            .map_err(|e| format!("处理路径失败: {}", e))?;

        // 转换路径分隔符为正斜杠（ZIP 标准格式）
        let file_name = name
            .to_str()
            .ok_or_else(|| format!("无效的文件名: {:?}", name))?
            .replace('\\', "/");

        // 添加文件到 ZIP
        zip.start_file(&file_name, options)
            .map_err(|e| format!("添加文件到ZIP失败 '{}': {}", file_name, e))?;

        // 读取文件内容并写入 ZIP
        let mut file =
            File::open(path).map_err(|e| format!("打开文件失败 '{}': {}", path.display(), e))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("读取文件失败 '{}': {}", path.display(), e))?;

        zip.write_all(&buffer)
            .map_err(|e| format!("写入文件到ZIP失败 '{}': {}", file_name, e))?;
    }

    // 完成 ZIP 文件写入
    zip.finish()
        .map_err(|e| format!("完成ZIP文件写入失败: {}", e))?;

    Ok(())
}
