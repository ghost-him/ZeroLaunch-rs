//! 路径类型查询 IPC 命令。
//!
//! 拖放事件只给出路径字符串，前端无法区分文件与目录（不直接访问文件系统）；
//! 设置页拖放路由经此命令拿到类型判定后，才决定写入路径还是父目录 + 文件模式。

use serde::Serialize;
use std::path::Path;

/// 拖入路径的类型判定结果 — 跨 IPC 序列化，仅供设置页拖放路由消费。
#[derive(Debug, Clone, Serialize)]
pub struct PathKindInfo {
    /// 原始路径字符串，与入参一一对应且保持顺序。
    #[serde(rename = "path")]
    pub path: String,
    /// 该路径的判定结果。
    #[serde(rename = "kind")]
    pub kind: PathKind,
}

/// 路径类型 — 前端按此决定投放语义（目录新增条目，文件取父目录 + 扩展名模式）。
#[derive(Debug, Clone, Copy, Serialize)]
pub enum PathKind {
    /// 普通文件。
    #[serde(rename = "file")]
    File,
    /// 目录（含符号链接指向的目录）。
    #[serde(rename = "directory")]
    Directory,
    /// 路径不存在、无权限，或既非文件也非目录。
    #[serde(rename = "unavailable")]
    Unavailable,
}

/// 批量查询路径类型。
///
/// 参数：`paths` — 待判定的绝对路径（拖放事件给出的原始字符串）。
/// 返回：与入参同序的判定结果；单个路径失败只标记该项为 `Unavailable`，不影响其余项。
#[tauri::command]
pub fn path_kind(paths: Vec<String>) -> Vec<PathKindInfo> {
    paths
        .into_iter()
        .map(|path| {
            let kind = match std::fs::metadata(Path::new(&path)) {
                Ok(metadata) if metadata.is_dir() => PathKind::Directory,
                Ok(metadata) if metadata.is_file() => PathKind::File,
                _ => PathKind::Unavailable,
            };
            PathKindInfo { path, kind }
        })
        .collect()
}
