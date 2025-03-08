use crate::modules::config::default::APP_PIC_PATH;

use super::pinyin_mapper::PinyinMapper;
use super::search_model::*;
use super::LaunchMethod;
use crate::program_manager::config::program_loader_config::PartialProgramLoaderConfig;
use crate::program_manager::config::program_loader_config::ProgramLoaderConfig;
/// 这个类用于加载电脑上程序，通过扫描路径或使用系统调用接口
use crate::program_manager::Program;
use crate::utils::defer::defer;
use crate::utils::windows::get_u16_vec;
use core::ffi::c_void;
use core::time::Duration;
use image::ImageReader;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};
use windows::Win32::Foundation::{PROPERTYKEY, S_OK};
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitialize, CoUninitialize, IPersistFile,
    StructuredStorage::{PropVariantClear, PROPVARIANT},
    CLSCTX_INPROC_SERVER, STGM,
};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PSGetPropertyKeyFromName};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, IShellLinkW, SHCreateItemFromParsingName,
    ShellLink, SIGDN_NORMALDISPLAY,
};
use windows_core::Interface;
use windows_core::PCWSTR;
#[derive(Debug)]
struct GuidGenerator {
    next_id: u64,
}

impl GuidGenerator {
    pub fn new() -> Self {
        GuidGenerator { next_id: 0 }
    }
    pub fn get_guid(&mut self) -> u64 {
        let ret = self.next_id;
        self.next_id += 1;
        ret
    }
}
#[derive(Debug)]
pub struct ProgramLoaderInner {
    /// 要扫描的路径(路径，遍历的深度)
    target_paths: Vec<(String, u32)>,
    /// 不扫描的路径
    forbidden_paths: Vec<String>,
    /// 禁止的程序关键字（当程序的名字中有与其完全一致的子字符串时，不注册）
    forbidden_program_key: Vec<String>,
    /// 设置程序的固定权重偏移（当程序的名字中有与其完全一致的子字符串时，才会添加）
    program_bias: HashMap<String, (f64, String)>,
    /// guid生成器
    guid_generator: GuidGenerator,
    /// 判断一个程序有没有被添加
    program_name_hash: HashSet<String>,
    /// 拼音转换器
    pinyin_mapper: PinyinMapper,
    /// 是否要扫描uwp
    is_scan_uwp_programs: bool,
    /// 索引的单体文件的地址
    index_file_paths: Vec<String>,
    /// 索引的网页
    index_web_pages: Vec<(String, String)>,
    /// 添加的自定义命令
    custom_command: Vec<(String, String)>,
    /// 加载耗时
    loading_time: Option<Duration>,
}

impl ProgramLoaderInner {
    /// 创建
    pub fn new() -> Self {
        ProgramLoaderInner {
            target_paths: Vec::new(),
            forbidden_paths: Vec::new(),
            forbidden_program_key: Vec::new(),
            program_bias: HashMap::new(),
            guid_generator: GuidGenerator::new(),
            program_name_hash: HashSet::new(),
            pinyin_mapper: PinyinMapper::new(),
            is_scan_uwp_programs: true,
            index_file_paths: Vec::new(),
            index_web_pages: Vec::new(),
            custom_command: Vec::new(),
            loading_time: None,
        }
    }

    pub fn to_partial(&self) -> PartialProgramLoaderConfig {
        PartialProgramLoaderConfig {
            target_paths: Some(self.target_paths.clone()),
            forbidden_paths: Some(self.forbidden_paths.clone()),
            forbidden_program_key: Some(self.forbidden_program_key.clone()),
            program_bias: Some(self.program_bias.clone()),
            is_scan_uwp_programs: Some(self.is_scan_uwp_programs),
            index_file_paths: Some(self.index_file_paths.clone()),
            index_web_pages: Some(self.index_web_pages.clone()),
            custom_command: Some(self.custom_command.clone()),
        }
    }

    /// 使用配置文件初始化
    pub fn load_from_config(&mut self, config: &ProgramLoaderConfig) {
        self.target_paths = config.get_target_paths();
        self.forbidden_paths = config.get_forbidden_paths();
        self.forbidden_program_key = config.get_forbidden_program_key();
        self.program_bias = config.get_program_bias();
        self.is_scan_uwp_programs = config.get_is_scan_uwp_programs();
        self.guid_generator = GuidGenerator::new();
        self.program_name_hash = HashSet::new();
        self.index_file_paths = config.get_index_file_paths();
        self.index_web_pages = config.get_index_web_pages();
        self.custom_command = config.get_custom_command();
    }
    /// 添加目标路径
    pub fn add_target_path(&mut self, path: String, depth: u32) {
        self.target_paths.push((path, depth));
    }
    /// 添加不扫描的路径
    pub fn add_forbidden_path(&mut self, path: String) {
        self.forbidden_paths.push(path);
    }
    /// 添加禁止的程序关键字
    pub fn add_forbidden_program_key(&mut self, key: String) {
        self.forbidden_program_key.push(key);
    }
    /// 设置程序的固定权重偏移
    pub fn add_program_bias(&mut self, key: &str, value: f64, note: String) {
        self.program_bias.insert(key.to_string(), (value, note));
    }
    /// 获得程序的固定权重偏移
    pub fn get_program_bias(&self, key: &str) -> f64 {
        let mut result: f64 = 0.0;
        for item in &self.program_bias {
            if key.contains(item.0) {
                result += item.1 .0;
            }
        }
        result
    }
    /// 预处理名字（完整的名字），返回处理过的别名
    pub fn convert_search_keywords(&self, full_name: &str) -> Vec<String> {
        let removed_version_name = remove_version_number(full_name);
        // 经过过滤的名字
        let filtered_name = remove_repeated_space(&removed_version_name);

        // 以大写首字母开头的名字
        let uppercase_name = get_upper_case_latter(&filtered_name).to_lowercase();

        // 小写名字
        let lower_name = filtered_name.to_lowercase();

        // 分隔开的名字
        let mut split_name = self.pinyin_mapper.convert(&lower_name);

        if split_name.is_empty() {
            split_name = lower_name.clone();
        }

        let first_latter_name = get_first_letters(&split_name);
        let pinyin_name = remove_string_space(&split_name);
        vec![lower_name, pinyin_name, first_latter_name, uppercase_name]
    }
    /// 判断一个程序是不是已经添加了
    fn check_program_is_exist(&mut self, full_name: &str) -> bool {
        // 用于判断的名字
        let unique_name = full_name.to_lowercase();
        // 判断当前的程序有没有被添加过
        if self.program_name_hash.contains(&unique_name.to_string()) {
            return true;
        }
        self.program_name_hash.insert(unique_name.to_string());
        false
    }

    /// 获取当前电脑上所有的程序
    pub fn load_program(&mut self) -> Vec<Arc<Program>> {
        // 开始计时
        let start = Instant::now();
        let mut result = Vec::new();
        if self.is_scan_uwp_programs {
            info!("添加uwp 应用");
            let uwp_infos = self.load_uwp_program();
            result.extend(uwp_infos);
        }
        // 添加普通的程序
        let program_infos = self.load_program_from_path();
        result.extend(program_infos);
        // 添加单体文件
        let file_infos = self.load_file_from_path();
        result.extend(file_infos);
        let web_infos = self.load_web();
        result.extend(web_infos);
        let command_infos = self.load_custom_command();
        result.extend(command_infos);
        // 结束计时
        self.loading_time = Some(start.elapsed());
        result
    }
    /// 获得加载程序的耗时
    pub fn get_loading_time(&self) -> f64 {
        if self.loading_time.is_some() {
            return self.loading_time.as_ref().unwrap().as_secs_f64() * 1000.0;
        }
        -1.0
    }

    /// 所有的网页
    fn load_web(&mut self) -> Vec<Arc<Program>> {
        let mut result = Vec::new();
        let web_pages = self.index_web_pages.clone();
        for (show_name, url) in &web_pages {
            if url.is_empty() || show_name.is_empty() {
                continue;
            }
            let check_name = "[网页]".to_string() + &show_name;
            if self.check_program_is_exist(&check_name) {
                continue;
            }
            let guid = self.guid_generator.get_guid();
            let alias: Vec<String> = self.convert_search_keywords(&show_name);
            let unique_name = check_name.to_lowercase();
            let stable_bias = self.get_program_bias(&unique_name);
            let program = Arc::new(Program {
                program_guid: guid,
                show_name: show_name.clone(),
                launch_method: LaunchMethod::File(url.clone()),
                search_keywords: alias,
                stable_bias,
                icon_path: APP_PIC_PATH.get("web_page").unwrap().value().clone(),
            });
            result.push(program);
        }
        result
    }

    /// 获取所有的单体文件
    fn load_file_from_path(&mut self) -> Vec<Arc<Program>> {
        let mut result = Vec::new();
        let files = self.index_file_paths.clone();
        for file_path in &files {
            // 判断文件的路径是不是有效
            let path = Path::new(&file_path);
            if path.is_file() {
                let show_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(String::from)
                    .unwrap_or_default();
                let check_name = "[文件]".to_string() + &show_name;
                if self.check_program_is_exist(&check_name) {
                    continue;
                }

                let guid = self.guid_generator.get_guid();
                let alias: Vec<String> = self.convert_search_keywords(&show_name);
                let unique_name = check_name.to_lowercase();
                let stable_bias = self.get_program_bias(&unique_name);
                let program = Arc::new(Program {
                    program_guid: guid,
                    show_name,
                    launch_method: LaunchMethod::File(file_path.clone()),
                    search_keywords: alias,
                    stable_bias,
                    icon_path: file_path.clone(),
                });
                result.push(program);
            }
        }
        result
    }

    /// 获取所有的程序
    fn load_program_from_path(&mut self) -> Vec<Arc<Program>> {
        // todo完成程序的加载
        // 遍历所有的目标路径
        let mut program_path: Vec<String> = Vec::new();
        for path_var in &self.target_paths {
            let path_str = &path_var.0;
            let depth = path_var.1;
            let path = Path::new(path_str);
            program_path.extend(self.recursive_visit_dir(path, depth as usize).unwrap());
        }
        let mut result: Vec<Arc<Program>> = Vec::new();

        // 添加通过地址找到的文件
        for path_str in program_path {
            // let target_path = if path_str.ends_with(".lnk") {
            //     self.resolve_shortcut(&path_str)
            // } else {
            //     path_str.to_string()
            // };
            let target_path = path_str;
            let path = Path::new(&target_path);

            let show_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
                .unwrap_or_default();
            // 判断当前的文件是不是被禁止的
            if self.check_program_is_forbidden(&show_name) {
                continue;
            }
            if self.check_program_is_exist(&show_name) {
                continue;
            }

            let guid = self.guid_generator.get_guid();

            let alias: Vec<String> = self.convert_search_keywords(&show_name);
            let unique_name = show_name.to_lowercase();
            let stable_bias = self.get_program_bias(&unique_name);
            let program = Arc::new(Program {
                program_guid: guid,
                show_name,
                launch_method: LaunchMethod::Path(target_path.clone()),
                search_keywords: alias,
                stable_bias,
                icon_path: target_path,
            });
            result.push(program);
        }
        // 添加通过uwp找到的文件
        result
    }

    /// 添加所有的自定义命令
    fn load_custom_command(&mut self) -> Vec<Arc<Program>> {
        let mut result = Vec::new();
        let custom_command = self.custom_command.clone();
        for (key, command) in &custom_command {
            if key.is_empty() || command.is_empty() {
                continue;
            }

            let show_name = key;
            // 不判断是不是被禁止的
            let check_name = "[命令]".to_string() + &show_name;
            if self.check_program_is_exist(&check_name) {
                continue;
            }

            let guid = self.guid_generator.get_guid();

            let alias = self.convert_search_keywords(&show_name);
            let unique_name = show_name.to_lowercase();
            let stable_bias = self.get_program_bias(&unique_name);
            let program = Arc::new(Program {
                program_guid: guid,
                show_name: show_name.clone(),
                launch_method: LaunchMethod::Command(command.clone()),
                search_keywords: alias,
                stable_bias,
                icon_path: APP_PIC_PATH.get("terminal").unwrap().value().clone(),
            });
            result.push(program);
        }
        result
    }

    fn check_program_is_forbidden(&self, name: &str) -> bool {
        let lower_case_name = name.to_lowercase();
        self.forbidden_program_key
            .iter()
            .any(|key| lower_case_name.contains(key))
    }

    fn prop_variant_to_string(&self, pv: &PROPVARIANT) -> String {
        pv.to_string()
    }

    fn load_uwp_program(&mut self) -> Vec<Arc<Program>> {
        let mut ret: Vec<Arc<Program>> = Vec::new();

        unsafe {
            // Initialize COM library
            if CoInitialize(None).is_err() {
                warn!("Failed to initialize COM library");
                return ret;
            }
            defer(|| {
                CoUninitialize();
            });

            // Create Shell item for AppsFolder
            let tmp = get_u16_vec("shell:AppsFolder");
            let app_folder: IShellItem =
                match SHCreateItemFromParsingName(PCWSTR::from_raw(tmp.as_ptr()), None) {
                    Ok(item) => item,
                    Err(e) => {
                        warn!("UWPApp::get_catalog, fail to open shell:AppsFolder {}", e);
                        return ret;
                    }
                };

            // Bind to IEnumShellItems
            let enum_shell_items: IEnumShellItems =
                match app_folder.BindToHandler(None, &BHID_EnumItems) {
                    Ok(enumerator) => enumerator,
                    Err(e) => {
                        warn!("UWPApp::get_catalog, fail to bind to handler {}", e);
                        return ret;
                    }
                };

            // Define PROPERTYKEYs
            let tmp = get_u16_vec("System.Launcher.AppState");
            let mut pk_launcher_app_state = PROPERTYKEY::default();
            match PSGetPropertyKeyFromName(
                PCWSTR::from_raw(tmp.as_ptr()),
                &mut pk_launcher_app_state,
            ) {
                Ok(pk) => pk,
                Err(e) => {
                    warn!(
                        "Failed to get PROPERTYKEY for System.Launcher.AppState{}",
                        e
                    );
                    return ret;
                }
            };
            let tmp = get_u16_vec("System.Tile.SmallLogoPath");
            let mut pk_small_logo_path = PROPERTYKEY::default();
            match PSGetPropertyKeyFromName(PCWSTR::from_raw(tmp.as_ptr()), &mut pk_small_logo_path)
            {
                Ok(pk) => pk,
                Err(e) => {
                    warn!(
                        "Failed to get PROPERTYKEY for System.Tile.SmallLogoPath {}",
                        e
                    );
                    return ret;
                }
            };
            let tmp = get_u16_vec("System.AppUserModel.ID");
            let mut pk_app_user_model_id = PROPERTYKEY::default();
            match PSGetPropertyKeyFromName(
                PCWSTR::from_raw(tmp.as_ptr()),
                &mut pk_app_user_model_id,
            ) {
                Ok(pk) => pk,
                Err(e) => {
                    warn!("Failed to get PROPERTYKEY for System.AppUserModel.ID {}", e);
                    return ret;
                }
            };
            let tmp = get_u16_vec("System.AppUserModel.PackageInstallPath");
            let mut pk_install_path = PROPERTYKEY::default();
            match PSGetPropertyKeyFromName(PCWSTR::from_raw(tmp.as_ptr()), &mut pk_install_path) {
                Ok(pk) => pk,
                Err(e) => {
                    warn!(
                        "Failed to get PROPERTYKEY for System.AppUserModel.PackageInstallPath {}",
                        e
                    );
                    return ret;
                }
            };

            // Enumerate Shell Items

            let mut items: Vec<Option<IShellItem>> = Vec::new();
            items.resize(200, None);

            // 定义一个变量来存储实际检索到的项目数量
            let mut fetched: u32 = 0;
            match enum_shell_items.Next(&mut items, Some(&mut fetched as *mut u32)) {
                Ok(()) => {
                    for shell_item in &items {
                        if shell_item.is_none() {
                            continue;
                        }
                        let shell_item = shell_item.clone().unwrap();

                        // Bind to IPropertyStore
                        let property_store: IPropertyStore = match shell_item
                            .BindToHandler(None, &windows::Win32::UI::Shell::BHID_PropertyStore)
                        {
                            Ok(store) => store,
                            Err(e) => {
                                warn!("error: {}", e);
                                continue;
                            }
                        };

                        // Get Launcher.AppState
                        let mut pv_launcher = PROPVARIANT::default();
                        if let Ok(value) = property_store.GetValue(&pk_launcher_app_state) {
                            pv_launcher = value.clone();
                        }

                        PropVariantClear(&mut pv_launcher).unwrap();

                        // Get Display Name
                        let short_name = match shell_item.GetDisplayName(SIGDN_NORMALDISPLAY) {
                            Ok(name) => name.to_string().unwrap(),
                            Err(_) => String::new(),
                        };

                        // Get AppUserModel.ID
                        let mut pv_app_id = PROPVARIANT::default();
                        if let Ok(value) = property_store.GetValue(&pk_app_user_model_id) {
                            pv_app_id = value.clone();
                        };

                        let app_id = self.prop_variant_to_string(&pv_app_id);
                        PropVariantClear(&mut pv_app_id).unwrap();

                        // Get PackageInstallPath
                        let mut pv_install = PROPVARIANT::default();
                        if let Ok(value) = property_store.GetValue(&pk_install_path) {
                            pv_install = value.clone();
                        };
                        let install_path = self.prop_variant_to_string(&pv_install);
                        if install_path.is_empty() {
                            continue;
                        }
                        PropVariantClear(&mut pv_install).unwrap();

                        // Get SmallLogoPath

                        let mut pv_icon = PROPVARIANT::default();
                        if let Ok(value) = property_store.GetValue(&pk_small_logo_path) {
                            pv_icon = value.clone();
                        };
                        let path = self.prop_variant_to_string(&pv_icon);
                        PropVariantClear(&mut pv_icon).unwrap();

                        let mut full_icon_path = PathBuf::from(&install_path);
                        full_icon_path.push(&path);
                        let icon_path =
                            self.validate_icon_path(full_icon_path.to_string_lossy().into_owned());

                        if self.check_program_is_exist(&short_name) {
                            continue;
                        }

                        let alias_name = self.convert_search_keywords(&short_name);
                        let guid = self.guid_generator.get_guid();
                        let unique_name = short_name.to_lowercase();
                        let stable_bias = self.get_program_bias(&unique_name);

                        ret.push(Arc::new(Program {
                            program_guid: guid,
                            show_name: short_name,
                            launch_method: LaunchMethod::PackageFamilyName(app_id),
                            search_keywords: alias_name,
                            stable_bias,
                            icon_path,
                        }));
                    }
                }
                Err(e) => {
                    warn!("error: {}", e);
                }
            }
        }
        ret
    }
    /// 验证一个图标的路径并返回分辨率最大的图标
    fn validate_icon_path(&self, icon_path: String) -> String {
        // 定义缩放后缀列表，按照分辨率从高到低排序
        let scales = [
            ".scale-400.",
            ".scale-300.",
            ".targetsize-256.",
            ".scale-200.",
            ".targetsize-48.",
            ".scale-100.",
            ".targetsize-24.",
            ".targetsize-16.",
        ];

        let path = Path::new(&icon_path);

        // 分离基础路径和扩展名
        let extension = match path.extension().and_then(OsStr::to_str) {
            Some(ext) => ext,
            None => return String::new(),
        };

        let stem = match path.file_stem().and_then(OsStr::to_str) {
            Some(s) => s,
            None => return String::new(),
        };

        let parent = match path.parent() {
            Some(p) => p,
            None => return String::new(),
        };

        // 首先检查缩放后的图标文件是否存在（按照预设的分辨率顺序）
        for scale in &scales {
            let new_stem = format!("{}{}.", stem, scale);
            let mut new_path = PathBuf::from(parent);
            new_path.push(format!("{}.{}", new_stem, extension));

            if new_path.exists() {
                return new_path.to_string_lossy().into_owned();
            }
        }

        // 如果没有匹配的缩放图标，寻找所有匹配的图标文件并比较它们的实际分辨率
        let icon_prefix = stem;

        let entries = match fs::read_dir(parent) {
            Ok(entries) => entries,
            Err(_) => return String::new(),
        };

        // 存储所有匹配的图标及其分辨率信息
        let mut matching_icons: Vec<(PathBuf, u64)> = Vec::new();

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
                        if ext.eq_ignore_ascii_case("png") {
                            if let Some(file_stem) = path.file_stem().and_then(OsStr::to_str) {
                                if file_stem.starts_with(icon_prefix) {
                                    // 使用图像元数据获取分辨率
                                    if let Some(resolution) = self.get_image_resolution(&path) {
                                        matching_icons.push((path.clone(), resolution));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 按分辨率从高到低排序
        matching_icons.sort_by(|a, b| b.1.cmp(&a.1));

        // 返回分辨率最高的图标
        if let Some((highest_res_path, _)) = matching_icons.first() {
            return highest_res_path.to_string_lossy().into_owned();
        }

        String::new()
    }

    /// 获取图像的分辨率（宽 × 高）
    fn get_image_resolution(&self, path: &Path) -> Option<u64> {
        match ImageReader::open(path) {
            Ok(reader) => {
                match reader.with_guessed_format() {
                    Ok(format_reader) => {
                        match format_reader.into_dimensions() {
                            Ok((width, height)) => {
                                // 使用宽×高作为分辨率指标
                                Some(width as u64 * height as u64)
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }
    /// 判断是不是一个有效的路径
    /// 1. 路径本身有效
    /// 2. 没有被屏蔽
    fn is_valid_path(&self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        for str in &self.forbidden_paths {
            let temp = Path::new(&str);
            // 如果当前的路径以禁止路径开头
            if path.starts_with(temp) {
                return false;
            }
        }
        true
    }

    /// 判断一个目标文件是不是想要的
    fn is_target_file(&self, path: &Path) -> bool {
        if !path.is_file() && !path.is_symlink() {
            return false;
        }

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        if !["url", "exe", "lnk"].contains(&extension) {
            return false;
        }

        path.file_stem()
            .and_then(|name| name.to_str())
            .map(|name| {
                !self
                    .forbidden_program_key
                    .iter()
                    .any(|key| name.contains(key))
            })
            .unwrap_or(false)
    }

    /// 递归遍历一个文件夹
    /// 会自动跳过不可遍历的文件夹
    /// 返回文件夹中所有的文件
    pub fn recursive_visit_dir(&self, dir: &Path, depth: usize) -> io::Result<Vec<String>> {
        if depth == 0 || !self.is_valid_path(dir) {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();

        if dir.is_dir() {
            match fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => {
                                let path = entry.path();
                                if path.is_dir() {
                                    match self.recursive_visit_dir(&path, depth - 1) {
                                        Ok(sub_result) => result.extend(sub_result),
                                        Err(e) => warn!(
                                            "Error accessing directory {}: {}",
                                            path.display(),
                                            e
                                        ),
                                    }
                                } else if self.is_target_file(&path) {
                                    if let Some(path_str) = path.to_str() {
                                        result.push(path_str.to_string());
                                    }
                                }
                            }
                            Err(e) => warn!("Error reading directory entry: {}", e),
                        }
                    }
                }
                Err(e) => warn!("Error reading directory {}: {}", dir.display(), e),
            }
        } else if self.is_valid_path(dir) {
            if let Some(dir_str) = dir.to_str() {
                result.push(dir_str.to_string());
            }
        }

        Ok(result)
    }

    /// 将.lnk文件的路径转成.exe文件的路径
    /// 如果转换失败了，则还是返回的.lnk文件的路径
    fn resolve_shortcut(&self, lnk_path: &str) -> String {
        debug!("开始转换：{lnk_path}");
        unsafe {
            // 初始化 COM 库
            let hr = CoInitialize(Some(std::ptr::null() as *const c_void));
            if !hr.is_ok() && hr != S_OK {
                return lnk_path.to_string();
            }

            // 创建 IShellLink 对象
            let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("CoCreateInstance failed: {:?}", e))
                .unwrap();

            // 获取 IPersistFile 接口
            let persist_file: IPersistFile = shell_link
                .cast()
                .map_err(|e| format!("Failed to cast to IPersistFile: {:?}", e))
                .unwrap();

            // 将 lnk_path 转换为 wide string
            let wide: Vec<u16> = get_u16_vec(lnk_path);

            // 加载快捷方式文件
            persist_file
                .Load(PCWSTR(wide.as_ptr()), STGM(0))
                .map_err(|e| format!("IPersistFile::Load failed: {:?}", e))
                .unwrap();

            // 准备接收目标路径
            let mut target_path = [0u16; 260];
            let mut find_data: WIN32_FIND_DATAW = std::mem::zeroed();
            // 获取目标路径
            let hr = shell_link.GetPath(&mut target_path, &mut find_data, 0);
            if hr.is_err() {
                return lnk_path.to_string();
            }

            // 将 wide string 转换为 Rust String
            let len = target_path
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(target_path.len());
            let path = std::ffi::OsString::from_wide(&target_path[..len])
                .to_string_lossy()
                .into_owned();
            path
        }
    }
}
#[derive(Debug)]
pub struct ProgramLoader {
    inner: RwLock<ProgramLoaderInner>,
}

impl ProgramLoader {
    /// 创建一个新的 `ProgramLoader` 实例
    pub fn new() -> Self {
        ProgramLoader {
            inner: RwLock::new(ProgramLoaderInner::new()),
        }
    }

    /// 从配置文件中加载配置
    pub fn load_from_config(&self, config: &ProgramLoaderConfig) {
        self.inner.write().load_from_config(config);
    }

    /// 添加目标路径
    pub fn add_target_path(&self, path: String, depth: u32) {
        self.inner.write().add_target_path(path, depth);
    }

    /// 添加不扫描的路径
    pub fn add_forbidden_path(&self, path: String) {
        self.inner.write().add_forbidden_path(path);
    }

    /// 添加禁止的程序关键字
    pub fn add_forbidden_program_key(&self, key: String) {
        self.inner.write().add_forbidden_program_key(key);
    }

    /// 设置程序的固定权重偏移
    pub fn add_program_bias(&self, key: &str, value: f64, note: String) {
        self.inner.write().add_program_bias(key, value, note);
    }

    /// 获取程序的固定权重偏移
    pub fn get_program_bias(&self, key: &str) -> f64 {
        self.inner.read().get_program_bias(key)
    }

    /// 获取当前电脑上所有的程序
    pub fn load_program(&self) -> Vec<Arc<Program>> {
        self.inner.write().load_program()
    }

    /// 获得加载时间
    pub fn get_loading_time(&self) -> f64 {
        self.inner.read().get_loading_time()
    }

    /// 递归遍历一个文件夹
    pub fn recursive_visit_dir(&self, dir: &Path, depth: usize) -> io::Result<Vec<String>> {
        self.inner.read().recursive_visit_dir(dir, depth)
    }

    /// 将 `.lnk` 文件的路径转成 `.exe` 文件的路径
    pub fn resolve_shortcut(&self, lnk_path: &str) -> String {
        self.inner.read().resolve_shortcut(lnk_path)
    }

    /// 将 `ProgramLoaderInner` 转换为 `PartialProgramLoaderConfig`
    pub fn to_partial(&self) -> PartialProgramLoaderConfig {
        self.inner.read().to_partial()
    }

    /// 获得一个程序的关键字
    pub fn convert_search_keywords(&self, show_name: &str) -> Vec<String> {
        self.inner.write().convert_search_keywords(show_name)
    }
}
