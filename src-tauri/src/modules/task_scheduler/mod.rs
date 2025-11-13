//! Windows 任务计划程序模块
//! 用于管理应用程序的开机自启动

use encoding_rs::UTF_16LE;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::Builder;
use tracing::{debug, info, warn};

/// 任务计划程序管理器
pub struct TaskScheduler {
    task_name: String,
    exe_path: String,
}

impl TaskScheduler {
    /// 创建新的任务计划程序管理器
    pub fn new(task_name: impl Into<String>, exe_path: impl Into<String>) -> Self {
        Self {
            task_name: task_name.into(),
            exe_path: exe_path.into(),
        }
    }

    /// 检查任务是否已存在
    pub fn is_enabled(&self) -> Result<bool, String> {
        debug!("检查任务是否存在: {}", self.task_name);

        let output = Command::new("schtasks")
            .args(["/Query", "/TN", &self.task_name, "/FO", "LIST"])
            .output()
            .map_err(|e| format!("执行 schtasks 查询命令失败: {}", e))?;

        // 如果任务存在，命令会返回成功状态码
        Ok(output.status.success())
    }

    /// 启用自动启动（创建任务计划）
    pub fn enable(&self) -> Result<(), String> {
        info!("正在启用自动启动，任务名: {}", self.task_name);

        // 先删除已存在的任务（如果有）
        if self.is_enabled()? {
            debug!("检测到已存在的任务，先删除");
            self.disable()?;
        }

        // 创建 XML 配置文件内容
        let xml_content = self.generate_task_xml();

        let mut temp_file = Builder::new()
            .prefix("zerolaunch-task-")
            .suffix(".xml")
            .tempfile()
            .map_err(|e| format!("创建临时 XML 文件失败: {}", e))?;

        // 编码为 UTF-16LE
        let (encoded, _, had_errors) = UTF_16LE.encode(&xml_content);
        if had_errors {
            return Err(String::from("生成任务计划 XML 时出现不可编码字符"));
        }

        // 写入文件：BOM + UTF-16LE 内容
        temp_file
            .write_all(&encoded)
            .and_then(|_| temp_file.flush())
            .map_err(|e| format!("写入临时 XML 文件失败: {}", e))?;

        let temp_path = temp_file.into_temp_path();

        // 使用 schtasks 创建任务
        let output = Command::new("schtasks")
            .args(["/Create", "/TN", &self.task_name, "/XML"])
            .arg(temp_path.as_os_str())
            .output()
            .map_err(|e| format!("执行 schtasks 创建命令失败: {}", e))?;

        // 删除临时文件
        if let Err(err) = temp_path.close() {
            warn!("删除临时 XML 文件失败: {}", err);
        }

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("创建任务计划失败: {}", error_msg));
        }

        info!("自动启动任务创建成功");
        Ok(())
    }

    /// 禁用自动启动（删除任务计划）
    pub fn disable(&self) -> Result<(), String> {
        info!("正在禁用自动启动，任务名: {}", self.task_name);

        let output = Command::new("schtasks")
            .args(["/Delete", "/TN", &self.task_name, "/F"])
            .output()
            .map_err(|e| format!("执行 schtasks 删除命令失败: {}", e))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            warn!("删除任务计划失败: {}", error_msg);
            return Err(format!("删除任务计划失败: {}", error_msg));
        }

        info!("自动启动任务删除成功");
        Ok(())
    }

    /// 生成任务计划的 XML 配置（来自模板替换）
    fn generate_task_xml(&self) -> String {
        // XML 字段转义函数
        fn escape_xml(input: &str) -> String {
            let mut s = String::with_capacity(input.len());
            for ch in input.chars() {
                match ch {
                    '&' => s.push_str("&amp;"),
                    '"' => s.push_str("&quot;"),
                    '\'' => s.push_str("&apos;"),
                    '<' => s.push_str("&lt;"),
                    '>' => s.push_str("&gt;"),
                    _ => s.push(ch),
                }
            }
            s
        }

        let author_name = whoami::username();
        let author = escape_xml(&author_name);

        let user_id_raw = Self::current_user_id();
        let user_id = escape_xml(&user_id_raw);

        let exe_path_escaped = escape_xml(&self.exe_path);
        let working_dir = Path::new(&self.exe_path)
            .parent()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| String::from("C:\\"));
        let working_dir_escaped = escape_xml(&working_dir);

        let template: &str = include_str!("./task_template.xml");
        template
            .replace("${AUTHOR}", &author)
            .replace("${USER_ID}", &user_id)
            .replace("${EXE}", &exe_path_escaped)
            .replace("${WORKDIR}", &working_dir_escaped)
    }

    fn current_user_id() -> String {
        let username = whoami::username();
        let domain = std::env::var("USERDOMAIN").ok();
        match domain {
            Some(ref d) if !d.is_empty() => format!(r"{}\\{}", d, username),
            _ => username,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_scheduler_creation() {
        let scheduler = TaskScheduler::new("TestTask", "C:\\test.exe");
        assert_eq!(scheduler.task_name, "TestTask");
        assert_eq!(scheduler.exe_path, "C:\\test.exe");
    }
}
