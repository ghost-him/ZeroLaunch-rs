use crate::sdk::autostart::AutoStartManager;
use crate::sdk::host_api::HostApiError;
use async_trait::async_trait;
use encoding_rs::{GBK, UTF_16LE};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use tempfile::Builder;
use tracing::{debug, info, warn};
use winreg::enums::*;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Windows 自启动管理器实现。
/// 使用任务计划程序（优先）和注册表（备选）两种方式管理自启动。
pub struct WindowsAutoStartManager;

impl Default for WindowsAutoStartManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAutoStartManager {
    pub fn new() -> Self {
        Self
    }

    /// 检查任务计划程序任务是否存在
    fn is_enabled_via_task_scheduler(&self, task_name: &str) -> Result<bool, HostApiError> {
        debug!("检查任务是否存在: {}", task_name);

        let output = Command::new("schtasks")
            .args(["/Query", "/TN", task_name, "/FO", "LIST"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| HostApiError::AutoStartFailed {
                reason: format!("执行 schtasks 查询命令失败: {}", e),
            })?;

        Ok(output.status.success())
    }

    /// 通过任务计划程序启用自动启动
    fn enable_via_task_scheduler(
        &self,
        task_name: &str,
        exe_path: &str,
    ) -> Result<(), HostApiError> {
        info!("正在通过任务计划程序启用自动启动，任务名: {}", task_name);

        if self.is_enabled_via_task_scheduler(task_name)? {
            debug!("检测到已存在的任务，先删除");
            self.disable_via_task_scheduler(task_name)?;
        }

        let xml_content = self.generate_task_xml(exe_path);

        let mut temp_file = Builder::new()
            .prefix("zerolaunch-task-")
            .suffix(".xml")
            .tempfile()
            .map_err(|e| HostApiError::AutoStartFailed {
                reason: format!("创建临时 XML 文件失败: {}", e),
            })?;

        let (encoded, _, had_errors) = UTF_16LE.encode(&xml_content);
        if had_errors {
            return Err(HostApiError::AutoStartFailed {
                reason: "生成任务计划 XML 时出现不可编码字符".to_string(),
            });
        }

        temp_file
            .write_all(&encoded)
            .and_then(|_| temp_file.flush())
            .map_err(|e| HostApiError::AutoStartFailed {
                reason: format!("写入临时 XML 文件失败: {}", e),
            })?;

        let temp_path = temp_file.into_temp_path();

        let output = Command::new("schtasks")
            .args(["/Create", "/TN", task_name, "/XML"])
            .arg(temp_path.as_os_str())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| HostApiError::AutoStartFailed {
                reason: format!("执行 schtasks 创建命令失败: {}", e),
            })?;

        if let Err(err) = temp_path.close() {
            warn!("删除临时 XML 文件失败: {}", err);
        }

        if !output.status.success() {
            let error_msg = Self::decode_system_output(&output.stderr);
            return Err(HostApiError::AutoStartFailed {
                reason: format!("创建任务计划失败: {}", error_msg),
            });
        }

        info!("任务计划程序自动启动任务创建成功");
        Ok(())
    }

    /// 通过任务计划程序禁用自动启动
    fn disable_via_task_scheduler(&self, task_name: &str) -> Result<(), HostApiError> {
        info!("正在通过任务计划程序禁用自动启动，任务名: {}", task_name);

        let output = Command::new("schtasks")
            .args(["/Delete", "/TN", task_name, "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| HostApiError::AutoStartFailed {
                reason: format!("执行 schtasks 删除命令失败: {}", e),
            })?;

        if !output.status.success() {
            let error_msg = Self::decode_system_output(&output.stderr);
            warn!("删除任务计划失败: {}", error_msg);
            return Err(HostApiError::AutoStartFailed {
                reason: format!("删除任务计划失败: {}", error_msg),
            });
        }

        info!("任务计划程序自动启动任务删除成功");
        Ok(())
    }

    /// 检查注册表启动项是否存在
    fn is_enabled_via_registry(&self, task_name: &str) -> Result<bool, HostApiError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = Path::new("Software")
            .join("Microsoft")
            .join("Windows")
            .join("CurrentVersion")
            .join("Run");
        let key = hkcu.open_subkey_with_flags(&path, KEY_READ).map_err(|e| {
            HostApiError::AutoStartFailed {
                reason: format!("打开注册表键失败: {}", e),
            }
        })?;

        match key.get_value::<String, _>(task_name) {
            Ok(_) => Ok(true),
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(HostApiError::AutoStartFailed {
                reason: format!("读取注册表值失败: {}", e),
            }),
        }
    }

    /// 通过注册表启用自动启动
    fn enable_via_registry(&self, task_name: &str, exe_path: &str) -> Result<(), HostApiError> {
        info!("尝试通过注册表启用自动启动: {}", task_name);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = Path::new("Software")
            .join("Microsoft")
            .join("Windows")
            .join("CurrentVersion")
            .join("Run");
        let (key, _) = hkcu
            .create_subkey(&path)
            .map_err(|e| HostApiError::AutoStartFailed {
                reason: format!("打开或创建注册表键失败: {}", e),
            })?;

        key.set_value(task_name, &exe_path)
            .map_err(|e| HostApiError::AutoStartFailed {
                reason: format!("写入注册表值失败: {}", e),
            })?;

        info!("注册表自动启动设置成功");
        Ok(())
    }

    /// 通过注册表禁用自动启动
    fn disable_via_registry(&self, task_name: &str) -> Result<(), HostApiError> {
        info!("尝试通过注册表禁用自动启动: {}", task_name);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = Path::new("Software")
            .join("Microsoft")
            .join("Windows")
            .join("CurrentVersion")
            .join("Run");

        let key = match hkcu.open_subkey_with_flags(&path, KEY_WRITE) {
            Ok(k) => k,
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                return Err(HostApiError::AutoStartFailed {
                    reason: format!("打开注册表键失败: {}", e),
                })
            }
        };

        match key.delete_value(task_name) {
            Ok(_) => {
                info!("注册表自动启动项删除成功");
                Ok(())
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(HostApiError::AutoStartFailed {
                reason: format!("删除注册表值失败: {}", e),
            }),
        }
    }

    /// 解码系统命令输出（处理 GBK 编码的中文 Windows）
    fn decode_system_output(bytes: &[u8]) -> String {
        if let Ok(s) = std::str::from_utf8(bytes) {
            return s.trim().to_string();
        }
        let (decoded, _, _) = GBK.decode(bytes);
        decoded.trim().to_string()
    }

    /// 生成任务计划的 XML 配置（来自模板替换）
    fn generate_task_xml(&self, exe_path: &str) -> String {
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

        let author_name = whoami::username().unwrap_or_else(|_| "unknown_user".to_string());
        let author = escape_xml(&author_name);

        let user_id_raw = Self::current_user_id();
        debug!("任务计划使用的用户标识: {}", user_id_raw);
        let user_id = escape_xml(&user_id_raw);

        let exe_path_escaped = escape_xml(exe_path);
        let working_dir = Path::new(exe_path)
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

    /// 获取当前用户标识（优先使用 SID，回退到域名\\用户名）
    fn current_user_id() -> String {
        let output = Command::new("whoami")
            .args(["/user", "/fo", "csv", "/nh"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let line = stdout.trim();

                let parts: Vec<&str> = line.split(',').collect();
                if let Some(sid_part) = parts.last() {
                    let sid = sid_part.trim().trim_matches('"');
                    if sid.starts_with("S-1-") {
                        return sid.to_string();
                    }
                }
            }
        }

        let username = whoami::username().unwrap_or_else(|_| "unknown_user".to_string());
        let domain = std::env::var("USERDOMAIN").ok();
        match domain {
            Some(ref d) if !d.is_empty() => format!(r"{}\\{}", d, username),
            _ => username,
        }
    }
}

#[async_trait]
impl AutoStartManager for WindowsAutoStartManager {
    async fn enable(&self, task_name: &str, exe_path: &str) -> Result<(), HostApiError> {
        match self.enable_via_task_scheduler(task_name, exe_path) {
            Ok(_) => {
                let _ = self.disable_via_registry(task_name);
                Ok(())
            }
            Err(e) => {
                warn!("任务计划程序设置失败: {}。尝试使用注册表方式...", e);
                let _ = self.disable_via_task_scheduler(task_name);
                self.enable_via_registry(task_name, exe_path)
            }
        }
    }

    async fn disable(&self, task_name: &str) -> Result<(), HostApiError> {
        let mut errors = Vec::new();

        if self
            .is_enabled_via_task_scheduler(task_name)
            .unwrap_or(false)
        {
            if let Err(e) = self.disable_via_task_scheduler(task_name) {
                warn!("禁用任务计划程序失败: {}", e);
                errors.push(format!("任务计划程序: {}", e));
            }
        }

        if let Err(e) = self.disable_via_registry(task_name) {
            warn!("禁用注册表启动项失败: {}", e);
            errors.push(format!("注册表: {}", e));
        }

        if errors.is_empty() {
            Ok(())
        } else if self.is_enabled(task_name).await.unwrap_or(true) {
            Err(HostApiError::AutoStartFailed {
                reason: format!("无法完全禁用自动启动: {}", errors.join("; ")),
            })
        } else {
            Ok(())
        }
    }

    async fn is_enabled(&self, task_name: &str) -> Result<bool, HostApiError> {
        let task_enabled = self
            .is_enabled_via_task_scheduler(task_name)
            .unwrap_or(false);
        let reg_enabled = self.is_enabled_via_registry(task_name).unwrap_or(false);
        Ok(task_enabled || reg_enabled)
    }

    fn default_task_name(&self) -> String {
        let username = whoami::username().unwrap_or_else(|_| "unknown_user".to_string());
        format!("ZeroLaunch-rs\\autostart ({})", username)
    }
}
