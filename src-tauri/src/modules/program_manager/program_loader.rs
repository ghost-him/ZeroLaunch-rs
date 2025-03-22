use super::config::program_loader_config::DirectoryConfig;
use super::pinyin_mapper::PinyinMapper;
use super::search_model::*;
use super::LaunchMethod;
use crate::error;
use crate::modules::config::default::APP_PIC_PATH;
use crate::program_manager::config::program_loader_config::PartialProgramLoaderConfig;
use crate::program_manager::config::program_loader_config::ProgramLoaderConfig;
/// 这个类用于加载电脑上程序，通过扫描路径或使用系统调用接口
use crate::program_manager::Program;
use crate::utils::defer::defer;
use crate::utils::notify;
use crate::utils::notify::notify;
use crate::utils::windows::get_u16_vec;
use core::time::Duration;
use dashmap::DashSet;
use globset::GlobSetBuilder;
use globset::{Glob, GlobSet};
use image::ImageReader;
use parking_lot::RwLock;
use regex::RegexSet;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::{
    CoInitialize, CoUninitialize,
    StructuredStorage::{PropVariantClear, PROPVARIANT},
};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PSGetPropertyKeyFromName};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, SHCreateItemFromParsingName, SIGDN_NORMALDISPLAY,
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

/// 路径检查器，用于判断某一个路径是不是想要的路径
#[derive(Debug)]
struct PathChecker {
    glob: Option<GlobSet>,
    regex: Option<RegexSet>,
    excluded_keys: Vec<String>,
    is_glob: bool,
}

impl PathChecker {
    pub fn new(
        patterns: &Vec<String>,
        pattern_type: &String,
        excluded_keys: &Vec<String>,
    ) -> Result<PathChecker, String> {
        match pattern_type.as_str() {
            "Wildcard" => {
                let mut builder = GlobSetBuilder::new();
                patterns.iter().for_each(|pattern| {
                    builder.add(
                        Glob::new(pattern)
                            .map_err(|e| format!("添加通配符失败：{:?}", e.to_string()))
                            .unwrap(),
                    );
                });

                Ok(PathChecker {
                    glob: Some(
                        builder
                            .build()
                            .map_err(|e| format!("编译通配符检查器失败：{:?}", e.to_string()))
                            .unwrap(),
                    ),
                    regex: None,
                    excluded_keys: excluded_keys.clone(),
                    is_glob: true,
                })
            }
            "Regex" => {
                let regex = RegexSet::new(patterns)
                    .map_err(|e| format!("编译正则表达式失败：{:?}", e.to_string()))
                    .unwrap();

                Ok(PathChecker {
                    glob: None,
                    regex: Some(regex),
                    excluded_keys: excluded_keys.clone(),
                    is_glob: false,
                })
            }
            _ => Err(format!("无当前该匹配项：{}", pattern_type)),
        }
    }

    pub fn is_match(&self, path: &str) -> bool {
        if self.excluded_keys.iter().any(|item| path.contains(item)) {
            return false;
        }

        if self.is_glob {
            // 使用glob模式匹配
            if let Some(ref glob_set) = self.glob {
                return glob_set.is_match(path);
            }
        } else {
            // 使用正则表达式匹配
            if let Some(ref regex_set) = self.regex {
                let ret = regex_set.is_match(path);
                return ret;
            }
        }
        false
    }
}

#[derive(Debug)]
pub struct ProgramLoaderInner {
    /// 要扫描的路径(路径，遍历的深度)
    target_paths: Vec<DirectoryConfig>,
    /// 设置程序的固定权重偏移（当程序的名字中有与其完全一致的子字符串时，才会添加）
    program_bias: HashMap<String, (f64, String)>,
    /// guid生成器
    guid_generator: GuidGenerator,
    /// 判断一个程序有没有被添加
    program_name_hash: DashSet<String>,
    /// 拼音转换器
    pinyin_mapper: PinyinMapper,
    /// 是否要扫描uwp
    is_scan_uwp_programs: bool,
    /// 索引的网页
    index_web_pages: Vec<(String, String)>,
    /// 添加的自定义命令
    custom_command: Vec<(String, String)>,
    /// 加载耗时
    loading_time: Option<Duration>,
    /// 不扫描的路径
    forbidden_paths: Vec<String>,
}

impl ProgramLoaderInner {
    /// 创建
    pub fn new() -> Self {
        ProgramLoaderInner {
            target_paths: Vec::new(),
            program_bias: HashMap::new(),
            guid_generator: GuidGenerator::new(),
            program_name_hash: DashSet::new(),
            pinyin_mapper: PinyinMapper::new(),
            is_scan_uwp_programs: true,
            index_web_pages: Vec::new(),
            custom_command: Vec::new(),
            loading_time: None,
            forbidden_paths: Vec::new(),
        }
    }

    pub fn to_partial(&self) -> PartialProgramLoaderConfig {
        PartialProgramLoaderConfig {
            target_paths: Some(self.target_paths.clone()),
            forbidden_paths: Some(self.forbidden_paths.clone()),
            program_bias: Some(self.program_bias.clone()),
            is_scan_uwp_programs: Some(self.is_scan_uwp_programs),
            index_web_pages: Some(self.index_web_pages.clone()),
            custom_command: Some(self.custom_command.clone()),
        }
    }

    /// 使用配置文件初始化
    pub fn load_from_config(&mut self, config: &ProgramLoaderConfig) {
        self.target_paths = config.get_target_paths();
        self.forbidden_paths = config.get_forbidden_paths();
        self.program_bias = config.get_program_bias();
        self.is_scan_uwp_programs = config.get_is_scan_uwp_programs();
        self.guid_generator = GuidGenerator::new();
        self.program_name_hash = DashSet::new();
        self.index_web_pages = config.get_index_web_pages();
        self.custom_command = config.get_custom_command();
    }
    /// 添加目标路径
    pub fn add_target_path(&mut self, directory_config: DirectoryConfig) {
        self.target_paths.push(directory_config);
    }
    /// 设置程序的固定权重偏移
    pub fn add_program_bias(&mut self, key: &str, value: f64, note: String) {
        self.program_bias.insert(key.to_string(), (value, note));
    }
    /// 添加不扫描的路径
    pub fn add_forbidden_path(&mut self, path: String) {
        self.forbidden_paths.push(path);
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
    fn check_program_is_exist(&self, full_name: &str) -> bool {
        // 用于判断的名字
        let unique_name = full_name.to_lowercase();
        // 检查程序是否已存在
        if self.program_name_hash.contains(&unique_name) {
            return true;
        }
        // 不存在则插入并返回 false
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

    /// 获取所有的程序
    fn load_program_from_path(&mut self) -> Vec<Arc<Program>> {
        let mut result: Vec<Arc<Program>> = Vec::new();
        for directory in &self.target_paths {
            let mut program_path: Vec<String> = Vec::new();
            let checker = PathChecker::new(
                &directory.pattern,
                &directory.pattern_type,
                &directory.excluded_keywords,
            );
            if checker.is_err() {
                let message = checker.unwrap_err();
                warn!("遇到错误: {}", message);
                notify("ZeroLaunch-rs", &format!("遇到错误: {}", message));
                continue;
            }
            let checker = Arc::new(checker.unwrap());
            program_path.extend(
                self.recursive_visit_dir(
                    Path::new(&directory.root_path),
                    directory.max_depth as usize,
                    checker,
                )
                .unwrap(),
            );
            // 添加通过地址找到的文件
            for path_str in program_path {
                let target_path = path_str;
                let path = Path::new(&target_path);
                let show_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(String::from)
                    .unwrap_or_default();

                if self.check_program_is_exist(&show_name) {
                    continue;
                }

                let guid = self.guid_generator.get_guid();

                let alias: Vec<String> = self.convert_search_keywords(&show_name);
                let unique_name = show_name.to_lowercase();
                let stable_bias = self.get_program_bias(&unique_name);

                let launch_method = if [".url", ".lnk", ".exe"]
                    .contains(&path.extension().unwrap().to_str().unwrap())
                {
                    LaunchMethod::Path(target_path.clone())
                } else {
                    LaunchMethod::File(target_path.clone())
                };
                let program = Arc::new(Program {
                    program_guid: guid,
                    show_name,
                    launch_method: launch_method,
                    search_keywords: alias,
                    stable_bias,
                    icon_path: target_path,
                });
                result.push(program);
            }
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
            items.resize(300, None);

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
                            Err(e) => {
                                warn!("error: {}", e);
                                String::new()
                            }
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
            if str.is_empty() {
                continue;
            }
            let temp = Path::new(&str);
            // 如果当前的路径以禁止路径开头
            if path.starts_with(temp) {
                return false;
            }
        }
        true
    }

    /// 判断一个目标文件是不是想要的
    fn is_target_file(&self, path: &Path, checker: Arc<PathChecker>) -> bool {
        if !path.is_file() && !path.is_symlink() {
            return false;
        }

        let file_name = path.file_name().and_then(|ext| ext.to_str()).unwrap();
        checker.is_match(file_name)
    }

    /// 递归遍历一个文件夹
    /// 会自动跳过不可遍历的文件夹
    /// 返回文件夹中所有的文件
    fn recursive_visit_dir(
        &self,
        dir: &Path,
        depth: usize,
        checker: Arc<PathChecker>,
    ) -> io::Result<Vec<String>> {
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
                                    match self.recursive_visit_dir(
                                        &path,
                                        depth - 1,
                                        checker.clone(),
                                    ) {
                                        Ok(sub_result) => result.extend(sub_result),
                                        Err(e) => warn!(
                                            "Error accessing directory {}: {}",
                                            path.display(),
                                            e
                                        ),
                                    }
                                } else if self.is_target_file(&path, checker.clone()) {
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
    pub fn add_target_path(&self, directory_config: DirectoryConfig) {
        self.inner.write().add_target_path(directory_config);
    }

    /// 添加不扫描的路径
    pub fn add_forbidden_path(&self, path: String) {
        self.inner.write().add_forbidden_path(path);
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

    /// 将 `ProgramLoaderInner` 转换为 `PartialProgramLoaderConfig`
    pub fn to_partial(&self) -> PartialProgramLoaderConfig {
        self.inner.read().to_partial()
    }

    /// 获得一个程序的关键字
    pub fn convert_search_keywords(&self, show_name: &str) -> Vec<String> {
        self.inner.write().convert_search_keywords(show_name)
    }
}
