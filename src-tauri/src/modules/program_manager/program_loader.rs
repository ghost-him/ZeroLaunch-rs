use super::config::program_loader_config::DirectoryConfig;
use super::localization_translation::parse_localized_names_from_dir;
use super::pinyin_mapper::PinyinMapper;
use super::LaunchMethod;
use crate::core::image_processor::ImageIdentity;
use crate::error::OptionExt;
use crate::modules::config::default::APP_PIC_PATH;
use crate::program_manager::config::program_loader_config::PartialProgramLoaderConfig;
use crate::program_manager::config::program_loader_config::ProgramLoaderConfig;
use crate::program_manager::search_model::*;
#[cfg(feature = "ai")]
use crate::program_manager::semantic_manager::GenerateEmbeddingForLoader;
use crate::program_manager::semantic_manager::SemanticManager;
/// 这个类用于加载电脑上程序，通过扫描路径或使用系统调用接口
use crate::program_manager::Program;
use crate::utils::defer::defer;
use crate::utils::notify::notify;
use crate::utils::windows::get_u16_vec;
use crate::utils::{dashmap_to_hashmap, hashmap_to_dashmap};
use core::time::Duration;
use dashmap::DashMap;
use dashmap::DashSet;
use globset::GlobSetBuilder;
use globset::{Glob, GlobSet};
use image::ImageReader;
use parking_lot::RwLock;
use regex::RegexSet;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, warn};
use walkdir::WalkDir;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PROPVARIANT};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PSGetPropertyKeyFromName};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, SHCreateItemFromParsingName, SIGDN_NORMALDISPLAY,
};
use windows_core::PCWSTR;
#[derive(Debug)]
struct GuidGenerator {
    next_id: AtomicU64,
}

impl GuidGenerator {
    pub fn new() -> Self {
        GuidGenerator {
            next_id: AtomicU64::new(0),
        }
    }
    pub fn get_guid(&self) -> u64 {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
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
        excluded_keys: &[String],
    ) -> Result<PathChecker, String> {
        let excluded_keys = excluded_keys
            .iter()
            .map(|item| item.to_lowercase())
            .collect();

        match pattern_type.as_str() {
            "Wildcard" => {
                let mut builder = GlobSetBuilder::new();
                for pattern in patterns {
                    match Glob::new(pattern) {
                        Ok(glob) => {
                            builder.add(glob);
                        }
                        Err(e) => {
                            warn!("添加通配符失败: {}", e);
                            return Err(format!("添加通配符失败：{:?}", e.to_string()));
                        }
                    }
                }

                match builder.build() {
                    Ok(globset) => Ok(PathChecker {
                        glob: Some(globset),
                        regex: None,
                        excluded_keys,
                        is_glob: true,
                    }),
                    Err(e) => {
                        warn!("编译通配符检查器失败: {}", e);
                        Err(format!("编译通配符检查器失败：{:?}", e.to_string()))
                    }
                }
            }
            "Regex" => match RegexSet::new(patterns) {
                Ok(regex) => Ok(PathChecker {
                    glob: None,
                    regex: Some(regex),
                    excluded_keys,
                    is_glob: false,
                }),
                Err(e) => {
                    warn!("编译正则表达式失败: {}", e);
                    Err(format!("编译正则表达式失败：{:?}", e.to_string()))
                }
            },
            _ => Err(format!("无当前该匹配项：{}", pattern_type)),
        }
    }

    pub fn is_match(&self, path: &str) -> bool {
        let path = path.to_lowercase();
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
                let ret = regex_set.is_match(&path);
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
    /// 自定义程序别名
    program_alias: DashMap<String, Vec<String>>,
    /// 语义描述信息
    semantic_descriptions: HashMap<String, String>,
    /// 语义管理器
    #[allow(dead_code)]
    semantic_manager: Arc<SemanticManager>,
    /// 是否在加载时生成/读取程序的embedding（仅 ai 构建有效）
    #[cfg(feature = "ai")]
    compute_embeddings: bool,
}

impl Default for ProgramLoaderInner {
    fn default() -> Self {
        panic!("ProgramLoaderInner::default() should not be used; provide SemanticManager")
    }
}

impl ProgramLoaderInner {
    /// 创建
    pub fn new(semantic_manager: Arc<SemanticManager>) -> Self {
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
            program_alias: DashMap::new(),
            semantic_descriptions: HashMap::new(),
            semantic_manager,
            #[cfg(feature = "ai")]
            compute_embeddings: true,
        }
    }

    pub fn to_partial(&self) -> PartialProgramLoaderConfig {
        let program_alias_hash_map = dashmap_to_hashmap(&self.program_alias);

        PartialProgramLoaderConfig {
            target_paths: Some(self.target_paths.clone()),
            forbidden_paths: Some(self.forbidden_paths.clone()),
            program_bias: Some(self.program_bias.clone()),
            is_scan_uwp_programs: Some(self.is_scan_uwp_programs),
            index_web_pages: Some(self.index_web_pages.clone()),
            custom_command: Some(self.custom_command.clone()),
            program_alias: Some(program_alias_hash_map),
            semantic_descriptions: Some(self.semantic_descriptions.clone()),
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
        self.program_alias = hashmap_to_dashmap(&config.get_program_alias());
        self.semantic_descriptions = config.get_semantic_descriptions();
    }
    /// 设置是否生成程序embedding
    #[cfg(feature = "ai")]
    pub fn set_compute_embeddings(&mut self, enabled: bool) {
        self.compute_embeddings = enabled;
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
        use tracing::{debug, info};

        info!("🔄 开始加载程序列表");
        // 开始计时
        let start = Instant::now();
        let mut result = Vec::new();

        if self.is_scan_uwp_programs {
            info!("📱 开始扫描UWP程序");
            let uwp_infos = self.load_uwp_program();
            info!("📱 UWP程序扫描完成，找到 {} 个程序", uwp_infos.len());
            result.extend(uwp_infos);
        } else {
            debug!("⏭️ 跳过UWP程序扫描（已禁用）");
        }

        // 添加普通的程序
        info!("💻 开始扫描路径中的程序");
        let program_infos = self.load_program_from_path();
        info!("💻 路径程序扫描完成，找到 {} 个程序", program_infos.len());
        result.extend(program_infos);

        info!("🌐 开始加载网页程序");
        let web_infos = self.load_web();
        info!("🌐 网页程序加载完成，找到 {} 个程序", web_infos.len());
        result.extend(web_infos);

        info!("⚡ 开始加载自定义命令");
        let command_infos = self.load_custom_command();
        info!("⚡ 自定义命令加载完成，找到 {} 个命令", command_infos.len());
        result.extend(command_infos);

        // 结束计时
        self.loading_time = Some(start.elapsed());
        let total_time = self
            .loading_time
            .expect_programming("加载时间应该已被设置")
            .as_millis();

        info!(
            "✅ 程序加载完成！总计 {} 个程序，耗时 {} ms",
            result.len(),
            total_time
        );
        result
    }

    /// 检查用户有没有添加别名
    fn check_program_alias(&self, key: &LaunchMethod) -> Vec<String> {
        let key = key.get_text();
        let mut keywords_to_append = vec![];
        if let Some(alias) = self.program_alias.get(&key) {
            // 如果有，则将其添加到program的搜索关键字中
            for item in alias.iter() {
                let mut converted = self.convert_search_keywords(item);
                keywords_to_append.append(&mut converted);
            }
        }
        keywords_to_append
    }

    /// 获取程序的语义描述信息
    fn get_program_semantic_description(&self, key: &LaunchMethod) -> Option<String> {
        let key = key.get_text();
        self.semantic_descriptions.get(&key).cloned()
    }

    /// 创建Program的辅助函数，消除重复代码
    /// 这个函数统一处理Program的创建逻辑，包括生成GUID、计算stable_bias等
    fn create_program(
        &self,
        show_name: String,
        unique_name: String,
        launch_method: LaunchMethod,
        mut search_keywords: Vec<String>,
        icon_path: ImageIdentity,
    ) -> Arc<Program> {
        let guid = self.guid_generator.get_guid();
        let stable_bias = self.get_program_bias(&unique_name);

        // 如果用户自己添加了别名，则添加上去
        let alias_name_to_append = self.check_program_alias(&launch_method);
        search_keywords.extend(alias_name_to_append);

        #[allow(unused_variables)]
        let description = self
            .get_program_semantic_description(&launch_method)
            .unwrap_or_default();

        // 生成或读取 embedding（仅当启用语义搜索时）
        #[cfg(feature = "ai")]
        let embedding = if self.compute_embeddings {
            let key = launch_method.clone();
            if let Some(cached) = self.semantic_manager.get_cached_embedding(&key) {
                println!("已命中语义缓存！");
                cached
            } else {
                println!(
                    "未命中语义缓存，开始计算新的embedding, show_name: {}, launch_method: {:?}",
                    &show_name, &launch_method
                );
                let computed = self
                    .semantic_manager
                    .generate_embedding_for_loader(
                        &show_name,
                        &search_keywords.join("，"),
                        &launch_method,
                        &description,
                    )
                    .unwrap_or_default();
                self.semantic_manager.put_cached_embedding(&key, &computed);
                computed
            }
        } else {
            // 未启用则返回空 embedding
            Default::default()
        };
        #[cfg(not(feature = "ai"))]
        let embedding = Default::default();
        Arc::new(Program {
            program_guid: guid,
            show_name,
            launch_method,
            search_keywords,
            stable_bias,
            icon_path,
            embedding,
        })
    }

    /// 获得加载程序的耗时
    pub fn get_loading_time(&self) -> f64 {
        if let Some(ref loading_time) = self.loading_time {
            return loading_time.as_secs_f64() * 1000.0;
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
            let check_name = "[网页]".to_string() + show_name;
            if self.check_program_is_exist(&check_name) {
                continue;
            }
            let unique_name = check_name.to_lowercase();
            let alias_names: Vec<String> = self.convert_search_keywords(show_name);
            let launch_method = LaunchMethod::File(url.clone());

            let program = self.create_program(
                show_name.clone(),
                unique_name,
                launch_method,
                alias_names,
                ImageIdentity::Web(url.to_string()),
            );
            result.push(program);
        }
        result
    }

    /// 获取所有的程序
    fn load_program_from_path(&mut self) -> Vec<Arc<Program>> {
        let mut result: Vec<Arc<Program>> = Vec::new();
        for directory in &self.target_paths {
            let mut program_paths_str: Vec<String> = Vec::new();
            let checker = PathChecker::new(
                &directory.pattern,
                &directory.pattern_type,
                &directory.excluded_keywords,
            );
            let checker = match checker {
                Ok(checker) => Arc::new(checker),
                Err(message) => {
                    warn!("遇到错误: {}", message);
                    notify("ZeroLaunch-rs", &format!("遇到错误: {}", message));
                    continue;
                }
            };

            let paths = self.recursive_visit_dir(
                Path::new(&directory.root_path),
                directory.max_depth as usize,
                checker,
            );

            let paths_count = paths.len();
            program_paths_str.extend(paths);
            debug!(
                "成功扫描目录: {}, 找到 {} 个程序",
                directory.root_path, paths_count
            );

            let mut grouped_paths: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
            for path_str in program_paths_str {
                let path = PathBuf::from(path_str);
                if let Some(parent) = path.parent() {
                    grouped_paths
                        .entry(parent.to_path_buf())
                        .or_default()
                        .push(path);
                }
            }
            for (dir_path, files_in_dir) in grouped_paths {
                let localized_names = parse_localized_names_from_dir(&dir_path);
                for target_path_buf in files_in_dir {
                    let target_path = target_path_buf.as_path();
                    let target_path_str = target_path.to_string_lossy().to_string();

                    // 这个是本地的文件名，这个用于匹配会不会有翻译过的本地化名字
                    let file_name = target_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(String::from)
                        .unwrap_or_default()
                        .to_lowercase();
                    // 这个是用于显示的名字（就是去除了后缀的）
                    let show_name = target_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(String::from)
                        .unwrap_or_default();

                    if self.check_program_is_exist(&show_name) {
                        continue;
                    }

                    // 基础别名：来自文件名本身
                    let mut alias_names: Vec<String> = self.convert_search_keywords(&show_name);
                    let unique_name = show_name.to_lowercase();
                    let launch_method = if let Some(ext) = target_path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            if ["url", "lnk", "exe"].contains(&ext_str) {
                                LaunchMethod::Path(target_path_str.clone())
                            } else {
                                LaunchMethod::File(target_path_str.clone())
                            }
                        } else {
                            LaunchMethod::File(target_path_str.clone())
                        }
                    } else {
                        LaunchMethod::File(target_path_str.clone())
                    };

                    // 再最后检查一下有没有本地化的名字
                    let localized_name = localized_names.get(&file_name).cloned();
                    if let Some(ref localized_name_str) = localized_name {
                        let mut localized_alias = self.convert_search_keywords(localized_name_str);
                        alias_names.append(&mut localized_alias);
                    }
                    // 如果有本地化的名字，则使用本地化的名字
                    let show_name = localized_name.unwrap_or(show_name);

                    let program = self.create_program(
                        show_name,
                        unique_name,
                        launch_method,
                        alias_names,
                        ImageIdentity::File(target_path_str),
                    );

                    result.push(program);
                }
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
            let check_name = "[命令]".to_string() + show_name;
            if self.check_program_is_exist(&check_name) {
                continue;
            }

            let unique_name = show_name.to_lowercase();
            let alias_names = self.convert_search_keywords(show_name);
            let launch_method = LaunchMethod::Command(command.clone());
            let icon_path = match APP_PIC_PATH.get("terminal") {
                Some(path) => path.value().clone(),
                None => {
                    warn!("未找到终端图标路径");
                    String::new()
                }
            };

            let program = self.create_program(
                show_name.clone(),
                unique_name,
                launch_method,
                alias_names,
                ImageIdentity::File(icon_path),
            );
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
            let com_init = windows::Win32::System::Com::CoInitialize(None);
            if com_init.is_err() {
                warn!("初始化com库失败：{:?}", com_init);
            }

            defer(move || {
                if com_init.is_ok() {
                    windows::Win32::System::Com::CoUninitialize();
                }
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
                        let shell_item = match shell_item.clone() {
                            Some(item) => item,
                            None => continue,
                        };

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

                        if let Err(e) = PropVariantClear(&mut pv_launcher) {
                            warn!("清理PropVariant失败: {}", e);
                        }

                        // Get Display Name
                        let short_name = match shell_item.GetDisplayName(SIGDN_NORMALDISPLAY) {
                            Ok(name) => match name.to_string() {
                                Ok(s) => s,
                                Err(e) => {
                                    warn!("转换显示名称失败: {}", e);
                                    String::new()
                                }
                            },
                            Err(e) => {
                                warn!("获取显示名称失败: {}", e);
                                String::new()
                            }
                        };

                        // Get AppUserModel.ID
                        let mut pv_app_id = PROPVARIANT::default();
                        if let Ok(value) = property_store.GetValue(&pk_app_user_model_id) {
                            pv_app_id = value.clone();
                        };

                        let app_id = self.prop_variant_to_string(&pv_app_id);
                        if let Err(e) = PropVariantClear(&mut pv_app_id) {
                            warn!("清理PropVariant失败: {}", e);
                        }

                        // Get PackageInstallPath
                        let mut pv_install = PROPVARIANT::default();
                        if let Ok(value) = property_store.GetValue(&pk_install_path) {
                            pv_install = value.clone();
                        };
                        let install_path = self.prop_variant_to_string(&pv_install);
                        if install_path.is_empty() {
                            continue;
                        }
                        if let Err(e) = PropVariantClear(&mut pv_install) {
                            warn!("清理PropVariant失败: {}", e);
                        }

                        // Get SmallLogoPath

                        let mut pv_icon = PROPVARIANT::default();
                        if let Ok(value) = property_store.GetValue(&pk_small_logo_path) {
                            pv_icon = value.clone();
                        };
                        let path = self.prop_variant_to_string(&pv_icon);
                        if let Err(e) = PropVariantClear(&mut pv_icon) {
                            warn!("清理PropVariant失败: {}", e);
                        }

                        let mut full_icon_path = PathBuf::from(&install_path);
                        full_icon_path.push(&path);
                        let icon_path =
                            self.validate_icon_path(full_icon_path.to_string_lossy().into_owned());

                        if self.check_program_is_exist(&short_name) {
                            continue;
                        }

                        let unique_name = short_name.to_lowercase();
                        let alias_name = self.convert_search_keywords(&short_name);
                        let launch_method = LaunchMethod::PackageFamilyName(app_id);

                        let program = self.create_program(
                            short_name,
                            unique_name,
                            launch_method,
                            alias_name,
                            ImageIdentity::File(icon_path),
                        );
                        ret.push(program);
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
            Err(e) => {
                warn!("Failed to read directory for icon validation: {}", e);
                return String::new();
            }
        };

        // 存储所有匹配的图标及其分辨率信息
        let mut matching_icons: Vec<(PathBuf, u64)> = Vec::new();

        for entry in entries.flatten() {
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
                            Err(e) => {
                                warn!("Failed to get image dimensions: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to guess image format: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to open image file: {}", e);
                None
            }
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

        match path.file_name().and_then(|ext| ext.to_str()) {
            Some(file_name) => checker.is_match(file_name),
            None => {
                warn!("无法获取文件名: {:?}", path);
                false
            }
        }
    }

    /// 递归遍历一个文件夹
    /// 会自动跳过不可遍历的文件夹
    /// 返回文件夹中所有的文件
    fn recursive_visit_dir(
        &self,
        dir: &Path,
        depth: usize,
        checker: Arc<PathChecker>,
    ) -> Vec<String> {
        // 注意：返回类型可以简化为 Vec<String>，因为 walkdir 的迭代器在内部处理错误
        if !self.is_valid_path(dir) {
            return Vec::new();
        }

        WalkDir::new(dir)
            .max_depth(depth)
            .into_iter()
            // 使用 filter_entry 提前剪枝。如果目录无效，则不再深入
            .filter_entry(|e| self.is_valid_path(e.path()))
            // filter_map 用于处理 Result<DirEntry, Error>
            .filter_map(|entry_result| {
                match entry_result {
                    Ok(entry) => Some(entry),
                    Err(e) => {
                        // 记录遍历过程中的错误，与原实现行为一致
                        debug!("Error reading directory entry: {}", e);
                        None
                    }
                }
            })
            // 筛选出我们想要的目标文件
            .filter(|entry| self.is_target_file(entry.path(), checker.clone()))
            // 将路径转换为字符串
            .map(|entry| entry.path().to_string_lossy().into_owned())
            // 收集所有结果
            .collect()
    }
}
#[derive(Debug)]
pub struct ProgramLoader {
    inner: RwLock<ProgramLoaderInner>,
}

impl Default for ProgramLoader {
    fn default() -> Self {
        panic!("ProgramLoader::default() should not be used; provide SemanticManager")
    }
}

impl ProgramLoader {
    /// 创建一个新的 `ProgramLoader` 实例
    pub fn new(semantic_manager: Arc<SemanticManager>) -> Self {
        ProgramLoader {
            inner: RwLock::new(ProgramLoaderInner::new(semantic_manager)),
        }
    }

    /// 从配置文件中加载配置
    pub fn load_from_config(&self, config: &ProgramLoaderConfig) {
        self.inner.write().load_from_config(config);
    }

    /// 设置是否在加载时生成/读取程序的embedding
    #[cfg(feature = "ai")]
    pub fn set_compute_embeddings(&self, enabled: bool) {
        self.inner.write().set_compute_embeddings(enabled);
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
