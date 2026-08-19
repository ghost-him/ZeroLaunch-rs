use async_trait::async_trait;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use tempfile::Builder;
use tracing::{debug, info, warn};
use windows::Win32::Globalization::GetACP;
use winreg::enums::*;
use winreg::RegKey;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::autostart::AutoStartManager;

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

        // 编码为 UTF-16LE 并写入 BOM。
        // schtasks 仅接受带 BOM 的 UTF-16 XML，否则按系统 ANSI 代码页解析，
        // 非 ASCII 路径（如中文目录）会乱码，导致任务指向无效路径而无法自启动。
        let mut encoded: Vec<u8> = Vec::with_capacity(xml_content.len() * 2 + 2);
        encoded.extend_from_slice(&[0xFF, 0xFE]);
        for unit in xml_content.encode_utf16() {
            encoded.extend_from_slice(&unit.to_le_bytes());
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

    /// 按指定 Windows ANSI 代码页解码命令输出。
    /// 系统命令（schtasks 等）的输出按系统 ANSI 代码页编码：
    /// 简体中文 GBK、日文 Shift-JIS、韩文 EUC-KR、繁体中文 Big5、西欧 CP1252 等。
    /// UTF-8 字节（如开启了系统 UTF-8 支持）优先识别。
    fn decode_with_codepage(bytes: &[u8], codepage: u32) -> String {
        if let Ok(s) = std::str::from_utf8(bytes) {
            return s.trim().to_string();
        }
        match codepage {
            936 => encoding_rs::GBK.decode(bytes).0.trim().to_string(),
            932 => encoding_rs::SHIFT_JIS.decode(bytes).0.trim().to_string(),
            949 => encoding_rs::EUC_KR.decode(bytes).0.trim().to_string(),
            950 => encoding_rs::BIG5.decode(bytes).0.trim().to_string(),
            874 => encoding_rs::WINDOWS_874.decode(bytes).0.trim().to_string(),
            1250 => encoding_rs::WINDOWS_1250.decode(bytes).0.trim().to_string(),
            1251 => encoding_rs::WINDOWS_1251.decode(bytes).0.trim().to_string(),
            1252 => encoding_rs::WINDOWS_1252.decode(bytes).0.trim().to_string(),
            1253 => encoding_rs::WINDOWS_1253.decode(bytes).0.trim().to_string(),
            1254 => encoding_rs::WINDOWS_1254.decode(bytes).0.trim().to_string(),
            1255 => encoding_rs::WINDOWS_1255.decode(bytes).0.trim().to_string(),
            1256 => encoding_rs::WINDOWS_1256.decode(bytes).0.trim().to_string(),
            1257 => encoding_rs::WINDOWS_1257.decode(bytes).0.trim().to_string(),
            1258 => encoding_rs::WINDOWS_1258.decode(bytes).0.trim().to_string(),
            866 => encoding_rs::IBM866.decode(bytes).0.trim().to_string(),
            _ => encoding_rs::Encoding::for_label(format!("windows-{codepage}").as_bytes())
                .map(|enc| enc.decode(bytes).0.trim().to_string())
                .unwrap_or_else(|| String::from_utf8_lossy(bytes).trim().to_string()),
        }
    }

    /// 解码系统命令输出（按系统 ANSI 代码页，兼容不同语言系统）
    fn decode_system_output(bytes: &[u8]) -> String {
        Self::decode_with_codepage(bytes, unsafe { GetACP() })
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

        let template: &str = include_str!("../assets/task_template.xml");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_ascii_output() {
        let s = "ERROR: The system cannot find the file specified.";
        assert_eq!(
            WindowsAutoStartManager::decode_with_codepage(s.as_bytes(), 936),
            s
        );
    }

    #[test]
    fn test_decode_utf8_output() {
        // 开启系统 UTF-8 支持（ACP=65001）时命令输出为 UTF-8
        let s = "错误: 系统找不到指定的路径。";
        assert_eq!(
            WindowsAutoStartManager::decode_with_codepage(s.as_bytes(), 65001),
            s
        );
    }

    #[test]
    fn test_decode_gbk_output() {
        // 简体中文系统（cp936）
        let (encoded, _, _) = encoding_rs::GBK.encode("错误: 任务 XML 格式错误。");
        assert_eq!(
            WindowsAutoStartManager::decode_with_codepage(&encoded, 936),
            "错误: 任务 XML 格式错误。"
        );
    }

    #[test]
    fn test_decode_shift_jis_output() {
        // 日文系统（cp932）
        let (encoded, _, _) =
            encoding_rs::SHIFT_JIS.encode("エラー: 指定されたファイルが見つかりません。");
        assert_eq!(
            WindowsAutoStartManager::decode_with_codepage(&encoded, 932),
            "エラー: 指定されたファイルが見つかりません。"
        );
    }

    #[test]
    fn test_decode_big5_output() {
        // 繁体中文系统（cp950）
        let (encoded, _, _) = encoding_rs::BIG5.encode("錯誤: 找不到指定的路徑。");
        assert_eq!(
            WindowsAutoStartManager::decode_with_codepage(&encoded, 950),
            "錯誤: 找不到指定的路徑。"
        );
    }

    #[test]
    fn test_decode_windows_1252_output() {
        // 英文/西欧系统（cp1252）
        let mut bytes = vec![0x54, 0x61, 0x73, 0x6B]; // "Task"
        bytes.extend_from_slice(&[0x20, 0xE9, 0x63, 0x68, 0x6F, 0x75]); // " échou"
        assert_eq!(
            WindowsAutoStartManager::decode_with_codepage(&bytes, 1252),
            "Task échou"
        );
    }
}
