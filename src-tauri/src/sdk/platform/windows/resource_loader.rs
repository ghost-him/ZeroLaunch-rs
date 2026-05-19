use crate::sdk::shell::resource_loader::ResourceLoader;
use crate::utils::windows::expand_environment_variables;
use ini::inistr;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use tracing::warn;
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::System::LibraryLoader::{LoadLibraryExW, LOAD_LIBRARY_AS_DATAFILE};
use windows::Win32::UI::WindowsAndMessaging::LoadStringW;
use windows_core::{PCWSTR, PWSTR};

/// Windows 平台资源加载器实现。
/// 通过 Windows API 加载 PE 文件中的本地化字符串资源，并解析 desktop.ini 文件。
pub struct WindowsResourceLoader;

impl Default for WindowsResourceLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsResourceLoader {
    pub fn new() -> Self {
        Self
    }
}

impl ResourceLoader for WindowsResourceLoader {
    /// 解析指定目录下的 desktop.ini 文件，提取 [LocalizedFileNames] 部分。
    /// 支持 UTF-16LE 和 UTF-8 编码的 desktop.ini，自动解析 DLL 资源引用。
    /// 参数：dir_path - 要解析的目录路径。
    /// 返回：从原始文件名到本地化名称的映射。
    fn parse_localized_names_from_dir(&self, dir_path: &Path) -> HashMap<String, String> {
        parse_localized_names_from_dir(dir_path)
    }
}

/// 解析形如 "@C:\path\to\file.dll,-12345" 的资源字符串。
/// 参数：resource_ref - 从 ini 文件读取到的原始值。
/// 返回：成功解析出本地化字符串返回 Some，否则返回 None。
fn resolve_resource_string(resource_ref: &str) -> Option<String> {
    if !resource_ref.starts_with('@') {
        return None;
    }

    let s = &resource_ref[1..];
    let comma_pos = s.rfind(',')?;

    let (path_part, id_part_with_comma) = s.split_at(comma_pos);

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
/// 支持 UTF-16LE 和 UTF-8 编码的 desktop.ini，自动解析 DLL 资源引用。
/// 参数：dir_path - 要解析的目录路径。
/// 返回：从原始文件名到本地化名称的映射。
fn parse_localized_names_from_dir(dir_path: &Path) -> HashMap<String, String> {
    let ini_path = dir_path.join("desktop.ini");
    if !ini_path.exists() {
        return HashMap::new();
    }

    let content = match std::fs::read(&ini_path) {
        Ok(bytes) => {
            if bytes.starts_with(&[0xFF, 0xFE]) && bytes.len() >= 2 {
                let u16_bytes = &bytes[2..];

                if u16_bytes.len() % 2 != 0 {
                    return HashMap::new();
                }

                let utf16_values: Vec<u16> = u16_bytes
                    .chunks_exact(2)
                    .map(|chunk| {
                        u16::from_le_bytes(
                            chunk.try_into().expect("Chunk should be exactly 2 bytes"),
                        )
                    })
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
                .expect("Value should not be None after is_none check");
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
