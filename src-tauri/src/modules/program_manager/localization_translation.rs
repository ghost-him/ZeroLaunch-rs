use crate::error::{OptionExt, ResultExt};
use crate::utils::windows::expand_environment_variables;
use crate::Path;
use ini::inistr;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use tracing::warn;
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::System::LibraryLoader::{LoadLibraryExW, LOAD_LIBRARY_AS_DATAFILE};
use windows::Win32::UI::WindowsAndMessaging::LoadStringW;
use windows_core::{PCWSTR, PWSTR};
/// 解析形如 "@C:\path\to\file.dll,-12345" 的资源字符串。
///
/// # Arguments
/// * `resource_ref` - 从 ini 文件读取到的原始值。
///
/// # Returns
/// `Some(String)` 如果成功解析出本地化字符串，否则 `None`。
fn resolve_resource_string(resource_ref: &str) -> Option<String> {
    // 1. 验证和解析输入字符串
    // 确保以 '@' 开头，并且至少还有一个 ',' 和一个数字
    if !resource_ref.starts_with('@') {
        return None;
    }

    // 去掉开头的 '@'
    let s = &resource_ref[1..];

    // 找到最后一个逗号来分离路径和ID
    let comma_pos = s.rfind(',')?;

    let (path_part, id_part_with_comma) = s.split_at(comma_pos);

    // 展开环境变量，如 %SystemRoot%
    let expanded_path = match expand_environment_variables(path_part) {
        Some(path) => path,
        None => {
            return None;
        }
    };

    let resource_id = match id_part_with_comma[1..].parse::<i32>() {
        Ok(id) => id.unsigned_abs(),
        Err(e) => {
            warn!("Failed to parse resource ID: {}", e);
            return None;
        }
    };

    // 2. 调用 Windows API
    unsafe {
        let wide_path: Vec<u16> = OsStr::new(&expanded_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let lib_handle = match LoadLibraryExW(
            PCWSTR::from_raw(wide_path.as_ptr()),
            None,
            LOAD_LIBRARY_AS_DATAFILE,
        ) {
            Ok(handle) => handle,
            Err(e) => {
                warn!("Failed to load library {}: {:?}", expanded_path, e);
                return None;
            }
        };

        let mut buffer: [u16; 512] = [0; 512];
        let length = LoadStringW(
            Some(lib_handle.into()),
            resource_id,
            PWSTR::from_raw(buffer.as_mut_ptr()),
            buffer.len() as i32,
        );

        let _ = FreeLibrary(lib_handle);

        if length > 0 {
            return Some(String::from_utf16_lossy(&buffer[..length as usize]));
        }
    }

    None
}
/// 解析指定目录下的 desktop.ini 文件，提取 [LocalizedFileNames] 部分。
/// 它会自动解析 DLL 资源引用。
/// 返回一个从原始文件名到本地化名称的映射。
pub fn parse_localized_names_from_dir(dir_path: &Path) -> HashMap<String, String> {
    let ini_path = dir_path.join("desktop.ini");
    if !ini_path.exists() {
        return HashMap::new();
    }

    // 处理 desktop.ini 可能的 UTF-16 编码
    let content = match std::fs::read(&ini_path) {
        Ok(bytes) => {
            // 检查 UTF-16LE BOM (FF FE)
            if bytes.starts_with(&[0xFF, 0xFE]) && bytes.len() >= 2 {
                // 跳过 BOM (2个字节)
                let u16_bytes = &bytes[2..];

                // 确保字节数是偶数，否则最后一个字节会被忽略
                if u16_bytes.len() % 2 != 0 {
                    // 或者返回错误，或者记录日志
                    return HashMap::new();
                }

                let utf16_values: Vec<u16> = u16_bytes
                    .chunks_exact(2) // 将切片按每2个字节分组
                    .map(|chunk| {
                        u16::from_le_bytes(
                            chunk
                                .try_into()
                                .expect_programming("Chunk should be exactly 2 bytes"),
                        )
                    }) // 将 [u8; 2] 转换为小端序 u16
                    .collect();

                widestring::U16Str::from_slice(&utf16_values).to_string_lossy()
            } else {
                String::from_utf8_lossy(&bytes).to_string()
            }
        }
        Err(e) => {
            warn!("Failed to read desktop.ini file: {}", e);
            return HashMap::new();
        }
    };

    let conf = inistr!(&content);
    let mut localized_map = HashMap::new();
    if let Some(section) = conf.get("localizedfilenames") {
        for (key, value) in section.iter() {
            if value.is_none() {
                continue;
            }
            let value = value
                .clone()
                .expect_programming("Value should not be None after is_none check");
            if value.starts_with('@') {
                if let Some(resolved_name) = resolve_resource_string(&value) {
                    localized_map.insert(key.to_string(), resolved_name);
                }
            } else {
                localized_map.insert(key.to_string(), value.to_string());
            }
        }
    }

    localized_map
}
