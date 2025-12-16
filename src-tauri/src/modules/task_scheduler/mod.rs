//! Windows 任务计划程序模块
//! 用于管理应用程序的开机自启动

use encoding_rs::{GBK, UTF_16LE};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::Builder;
use tracing::{debug, info, warn};
use winreg::enums::*;
use winreg::RegKey;

use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

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

    /// 检查任务是否已存在 (Public Interface)
    pub fn is_enabled(&self) -> Result<bool, String> {
        let task_enabled = self.is_enabled_via_task_scheduler().unwrap_or(false);
        let reg_enabled = self.is_enabled_via_registry().unwrap_or(false);
        Ok(task_enabled || reg_enabled)
    }

    /// 启用自动启动 (Public Interface)
    pub fn enable(&self) -> Result<(), String> {
        // 尝试使用任务计划程序
        match self.enable_via_task_scheduler() {
            Ok(_) => {
                // 如果任务计划程序成功，确保注册表项被清理（避免重复）
                let _ = self.disable_via_registry();
                return Ok(());
            }
            Err(e) => {
                warn!("任务计划程序设置失败: {}。尝试使用注册表方式...", e);
                let _ = self.disable_via_task_scheduler();
            }
        }

        // 如果任务计划程序失败，尝试注册表
        self.enable_via_registry().map_err(|e| {
            format!(
                "自动启动设置完全失败。任务计划程序和注册表均失败。注册表错误: {}",
                e
            )
        })
    }

    /// 禁用自动启动 (Public Interface)
    pub fn disable(&self) -> Result<(), String> {
        let mut errors = Vec::new();

        // 尝试禁用任务计划
        if self.is_enabled_via_task_scheduler().unwrap_or(false) {
            if let Err(e) = self.disable_via_task_scheduler() {
                warn!("禁用任务计划程序失败: {}", e);
                errors.push(format!("任务计划程序: {}", e));
            }
        }

        // 尝试禁用注册表
        if let Err(e) = self.disable_via_registry() {
            warn!("禁用注册表启动项失败: {}", e);
            errors.push(format!("注册表: {}", e));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            // 再次检查是否真的还启用着
            if self.is_enabled().unwrap_or(true) {
                Err(format!("无法完全禁用自动启动: {}", errors.join("; ")))
            } else {
                Ok(())
            }
        }
    }

    /// 检查任务计划程序任务是否存在
    fn is_enabled_via_task_scheduler(&self) -> Result<bool, String> {
        debug!("检查任务是否存在: {}", self.task_name);

        let output = Command::new("schtasks")
            .args(["/Query", "/TN", &self.task_name, "/FO", "LIST"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("执行 schtasks 查询命令失败: {}", e))?;

        // 如果任务存在，命令会返回成功状态码
        Ok(output.status.success())
    }

    /// 通过任务计划程序启用自动启动
    fn enable_via_task_scheduler(&self) -> Result<(), String> {
        info!(
            "正在通过任务计划程序启用自动启动，任务名: {}",
            self.task_name
        );

        // 先删除已存在的任务（如果有）
        if self.is_enabled_via_task_scheduler()? {
            debug!("检测到已存在的任务，先删除");
            self.disable_via_task_scheduler()?;
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
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("执行 schtasks 创建命令失败: {}", e))?;

        // 删除临时文件
        if let Err(err) = temp_path.close() {
            warn!("删除临时 XML 文件失败: {}", err);
        }

        if !output.status.success() {
            let error_msg = Self::decode_system_output(&output.stderr);
            return Err(format!("创建任务计划失败: {}", error_msg));
        }

        info!("任务计划程序自动启动任务创建成功");
        Ok(())
    }

    /// 通过任务计划程序禁用自动启动
    fn disable_via_task_scheduler(&self) -> Result<(), String> {
        info!(
            "正在通过任务计划程序禁用自动启动，任务名: {}",
            self.task_name
        );

        let output = Command::new("schtasks")
            .args(["/Delete", "/TN", &self.task_name, "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("执行 schtasks 删除命令失败: {}", e))?;

        if !output.status.success() {
            let error_msg = Self::decode_system_output(&output.stderr);
            warn!("删除任务计划失败: {}", error_msg);
            return Err(format!("删除任务计划失败: {}", error_msg));
        }

        info!("任务计划程序自动启动任务删除成功");
        Ok(())
    }

    /// 检查注册表启动项是否存在
    fn is_enabled_via_registry(&self) -> Result<bool, String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = Path::new("Software")
            .join("Microsoft")
            .join("Windows")
            .join("CurrentVersion")
            .join("Run");
        let key = hkcu
            .open_subkey_with_flags(&path, KEY_READ)
            .map_err(|e| format!("打开注册表键失败: {}", e))?;

        match key.get_value::<String, _>(&self.task_name) {
            Ok(_) => Ok(true),
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("读取注册表值失败: {}", e)),
        }
    }

    /// 通过注册表启用自动启动
    fn enable_via_registry(&self) -> Result<(), String> {
        info!("尝试通过注册表启用自动启动: {}", self.task_name);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = Path::new("Software")
            .join("Microsoft")
            .join("Windows")
            .join("CurrentVersion")
            .join("Run");
        let (key, _) = hkcu
            .create_subkey(&path)
            .map_err(|e| format!("打开或创建注册表键失败: {}", e))?;

        key.set_value(&self.task_name, &self.exe_path)
            .map_err(|e| format!("写入注册表值失败: {}", e))?;

        info!("注册表自动启动设置成功");
        Ok(())
    }

    /// 通过注册表禁用自动启动
    fn disable_via_registry(&self) -> Result<(), String> {
        info!("尝试通过注册表禁用自动启动: {}", self.task_name);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = Path::new("Software")
            .join("Microsoft")
            .join("Windows")
            .join("CurrentVersion")
            .join("Run");

        // 如果键不存在，直接返回成功
        let key = match hkcu.open_subkey_with_flags(&path, KEY_WRITE) {
            Ok(k) => k,
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("打开注册表键失败: {}", e)),
        };

        match key.delete_value(&self.task_name) {
            Ok(_) => {
                info!("注册表自动启动项删除成功");
                Ok(())
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("删除注册表值失败: {}", e)),
        }
    }

    /// 解码系统命令输出（处理 GBK 编码的中文 Windows）
    fn decode_system_output(bytes: &[u8]) -> String {
        // 先尝试 UTF-8
        if let Ok(s) = std::str::from_utf8(bytes) {
            return s.trim().to_string();
        }
        // 回退到 GBK（中文 Windows 默认编码）
        let (decoded, _, _) = GBK.decode(bytes);
        decoded.trim().to_string()
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
        debug!("任务计划使用的用户标识: {}", user_id_raw);
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
        // 尝试通过 whoami /user 获取 SID
        // 这比用户名更可靠，特别是当用户名包含特殊字符或在不同语言环境下
        // 同时也解决了管理员账户下可能出现的用户名匹配问题
        let output = Command::new("whoami")
            .args(["/user", "/fo", "csv", "/nh"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                // whoami 的输出可能是系统编码，但 SID 仅包含 ASCII 字符
                let stdout = String::from_utf8_lossy(&output.stdout);
                let line = stdout.trim();

                // 解析 CSV: "User Name","SID"
                // 简单的 split 应该足够，因为我们只想要最后一个字段（SID）
                let parts: Vec<&str> = line.split(',').collect();
                if let Some(sid_part) = parts.last() {
                    let sid = sid_part.trim().trim_matches('"');
                    if sid.starts_with("S-1-") {
                        return sid.to_string();
                    }
                }
            }
        }

        // 回退到旧方法
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
