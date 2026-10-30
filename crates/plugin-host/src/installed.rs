//! 已安装插件的解析、内存快照与投影。
//!
//! 安装事实的唯一权威是 plugins 目录下的清单：目录定位、清单与元数据在扫描时
//! 各解析一次，供列表、详情、重载、卸载与覆盖安装共用。未加载（启动/重载失败）
//! 的插件同样可解析，从其投影出 error 态条目，界面不会因加载失败而丢条目。
//!
//! 解析结果落在 [`InstalledCache`]（内存读模型），刷新点只有三处：启动扫描、
//! 插件目录监听线程、安装/卸载/重载动作完成后的显式刷新。读接口一律只读内存，
//! 不再在每次查询时扫盘。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{RecursiveMode, Watcher};
use parking_lot::RwLock;
use tracing::{info, warn};
use zerolaunch_plugin_api::plugin::{PluginKind, PluginMetadata, PluginMode};
use zerolaunch_plugin_protocol::Manifest;

use crate::icon::{mime_from_extension, to_data_url, MAX_ICON_BYTES};
use crate::info::PluginInfo;
use crate::manager::{PluginHostManager, PluginRuntimeState};

/// 已安装插件 —— 安装事实的唯一解析结果。
///
/// 目录名可异于清单 id，所有目录操作以 `dir` 为准；`metadata` 在解析时一次算好
/// （读取图标文件等代价不重复支付）。
pub struct InstalledPlugin {
    /// 真实安装目录。
    pub dir: PathBuf,
    /// 已解析的清单。
    pub manifest: Manifest,
    /// 由清单派生的插件级元数据。
    pub metadata: PluginMetadata,
}

impl InstalledPlugin {
    /// 从安装目录解析插件：读取 manifest.toml 并派生元数据。
    /// 参数：dir - 安装目录。
    /// 返回：解析成功返回记录；清单缺失或非法返回 None（调用方告警跳过）。
    pub fn from_dir(dir: &Path) -> Option<Self> {
        let manifest_path = dir.join("manifest.toml");
        let text = std::fs::read_to_string(&manifest_path).ok()?;
        let manifest: Manifest = match toml::from_str(&text) {
            Ok(manifest) => manifest,
            Err(e) => {
                warn!("插件清单解析失败，跳过 {}: {}", manifest_path.display(), e);
                return None;
            }
        };
        let metadata = metadata_from_manifest(dir, &manifest);
        Some(Self {
            dir: dir.to_path_buf(),
            manifest,
            metadata,
        })
    }
}

/// 已安装插件快照 —— plugins 目录清单（唯一权威）的内存读模型。
///
/// 仅由本文件与 `manager.rs` 使用；外部（src-tauri）经 `PluginHostManager` 的
/// `installed` / `installed_snapshot` 读接口访问，不得自行扫描磁盘或另存一份。
/// 内部可变性用 `RwLock`：读（每次列表/详情）远多于写（启动一次 + 目录变更）。
pub struct InstalledCache {
    /// 插件安装根目录（扫描对象）。
    plugins_dir: PathBuf,
    /// 已解析的安装记录；读方只做 Arc 克隆，不触发磁盘 IO。
    snapshot: RwLock<Vec<Arc<InstalledPlugin>>>,
}

impl InstalledCache {
    /// 构造并执行首次全量扫描。
    /// 参数：plugins_dir - 插件安装根目录（不存在时快照为空）。
    pub(crate) fn load(plugins_dir: PathBuf) -> Self {
        let cache = Self {
            plugins_dir,
            snapshot: RwLock::new(Vec::new()),
        };
        cache.refresh();
        cache
    }

    /// 重扫安装目录并原子替换快照。
    fn refresh(&self) {
        let snapshot = scan_installed(&self.plugins_dir);
        *self.snapshot.write() = snapshot;
    }

    /// 读取快照（Arc 克隆，无磁盘 IO）。
    fn list(&self) -> Vec<Arc<InstalledPlugin>> {
        self.snapshot.read().clone()
    }

    /// 按清单 id 在快照中定位安装记录。
    /// 参数：plugin_id - 清单 id。
    /// 返回：命中返回记录，未安装返回 None。
    fn get(&self, plugin_id: &str) -> Option<Arc<InstalledPlugin>> {
        self.snapshot
            .read()
            .iter()
            .find(|plugin| plugin.metadata.id == plugin_id)
            .cloned()
    }
}

/// 全量扫描安装目录：解析每个含 manifest.toml 的子目录。
/// 仅限本文件使用（`InstalledCache::refresh` 的唯一扫描实现）。
fn scan_installed(root: &Path) -> Vec<Arc<InstalledPlugin>> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join("manifest.toml").is_file())
        .filter_map(|dir| InstalledPlugin::from_dir(&dir).map(Arc::new))
        .collect()
}

/// 事件是否需要触发快照重扫：清单/插件目录变更、`need_rescan`（丢事件）标记，
/// 或监听后端报错（此时无法确定漏掉了哪些变更，只能重扫补齐）。
/// 仅限本文件使用（监听线程的事件过滤条件）。
fn event_triggers_refresh(event: notify::Result<notify::Event>, root: &Path) -> bool {
    match event {
        Ok(event) => {
            event.need_rescan() || event.paths.iter().any(|path| affects_installed(path, root))
        }
        Err(_) => true,
    }
}

/// 目录事件是否影响安装事实：插件目录本身的增删改名，或任意层级的清单写入。
/// 仅限本文件使用（监听线程的过滤条件）。
fn affects_installed(path: &Path, root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    relative.components().count() <= 1
        || path.file_name().is_some_and(|name| name == "manifest.toml")
}

impl PluginHostManager {
    /// 枚举包含 manifest.toml 的插件目录（不解析清单）。
    /// 返回：目录列表；安装根目录不存在时为空。
    ///
    /// 仅用于启动期加载（`load_all_third_party`）；列表/详情一律读内存快照。
    pub fn scan_dirs(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        if let Ok(entries) = std::fs::read_dir(self.plugins_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("manifest.toml").exists() {
                    found.push(path);
                }
            }
        }
        found
    }

    /// 已安装插件的内存快照（Arc 克隆，无磁盘 IO）。
    pub fn installed_snapshot(&self) -> Vec<Arc<InstalledPlugin>> {
        self.installed_cache.list()
    }

    /// 立即重扫安装目录并替换快照。
    ///
    /// 安装/卸载/重载链路在动作完成后调用，保证紧随其后的 IPC 读到新事实；
    /// 监听线程同样只调用本方法，刷新逻辑唯一。
    pub fn refresh_installed(&self) {
        self.installed_cache.refresh();
    }

    /// 按清单 id 定位安装记录：已加载取注册包快照，否则读内存安装快照。
    /// 参数：plugin_id - 插件 id。
    /// 返回：安装记录；未安装（或快照尚未刷新到）返回 None。
    pub fn installed(&self, plugin_id: &str) -> Option<Arc<InstalledPlugin>> {
        if let Some(registration) = self.plugins.get(plugin_id) {
            return Some(Arc::new(InstalledPlugin {
                dir: registration.plugin_dir.clone(),
                manifest: registration.manifest.clone(),
                metadata: (*registration.metadata).clone(),
            }));
        }
        self.installed_cache.get(plugin_id)
    }

    /// 启动插件目录监听：目录变更后异步重扫安装快照。
    ///
    /// 幂等：已启动直接返回 Ok。最终一致（事件合并窗口约 300ms），覆盖用户手动
    /// 拷贝/删除插件目录的场景；监听失败只降级为「只在动作后刷新」，不阻断启动。
    /// 返回：启动成功 Ok(())，创建目录或监听失败返回错误。
    pub fn start_installed_watcher(&self) -> Result<()> {
        let mut slot = self.installed_watcher.lock();
        if slot.is_some() {
            return Ok(());
        }

        let root = self.plugins_dir.clone();
        // 监听要求目录存在；插件根目录的创建是幂等的。
        std::fs::create_dir_all(&root)
            .with_context(|| format!("无法创建插件目录: {}", root.display()))?;

        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let filter_root = root.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                // 报错或 need_rescan（递归监听缓冲溢出等）意味着无法确定漏掉了哪些变更：
                // 只能重扫补齐，否则快照会永久停在旧事实、直到下一次有事件进来。
                if event_triggers_refresh(event, &filter_root) {
                    let _ = tx.send(());
                }
            })
            .with_context(|| format!("无法监听插件目录: {}", root.display()))?;
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .with_context(|| format!("无法监听插件目录: {}", root.display()))?;

        // 事件合并：连续事件静默一个窗口后只重扫一次，避免整包安装时逐文件重扫。
        // 线程只持有快照缓存（不持有管理器），监听器 drop 后发送端关闭，线程退出。
        let cache = Arc::clone(&self.installed_cache);
        std::thread::Builder::new()
            .name("plugin-install-watch".to_string())
            .spawn(move || {
                while rx.recv().is_ok() {
                    while rx.recv_timeout(Duration::from_millis(300)).is_ok() {}
                    cache.refresh();
                }
            })
            .with_context(|| "无法启动插件目录监听线程")?;

        *slot = Some(watcher);
        info!("插件目录监听已启动: {}", root.display());
        Ok(())
    }

    /// 安装记录 → 展示条目：已加载取运行时注册，未加载呈现 error 态。
    /// 参数：installed - 安装记录；is_component_enabled - 组件启用态谓词（由调用方提供）。
    /// 返回：IPC 展示条目（未加载时组件为空、enabled 为 false）。
    pub fn to_info(
        &self,
        installed: &InstalledPlugin,
        is_component_enabled: impl Fn(&str) -> bool,
    ) -> PluginInfo {
        self.get_plugin_info(&installed.metadata.id, is_component_enabled)
            .unwrap_or_else(|| {
                PluginInfo::from_metadata(
                    &installed.metadata,
                    PluginRuntimeState::Error,
                    Vec::new(),
                    false,
                )
            })
    }
}

/// 由清单构造插件级元数据。
///
/// 加载期（构建注册包）与磁盘条目（未加载插件）共用本函数，保证图标、热键、
/// 形态与优先级只在同一处从 manifest 派生。
/// 参数：plugin_dir - 安装目录（读取 [icon] 文件用）；manifest - 已解析清单。
/// 返回：插件级元数据，kind 恒为 ThirdParty。
pub(crate) fn metadata_from_manifest(plugin_dir: &Path, manifest: &Manifest) -> PluginMetadata {
    let metadata = PluginMetadata {
        id: manifest.plugin.id.clone(),
        name: manifest.plugin.name.clone(),
        version: manifest.plugin.version.clone(),
        description: manifest.plugin.description.clone(),
        author: manifest.plugin.author.clone(),
        trigger_keywords: manifest.plugin.trigger_keywords.clone(),
        trigger_description: manifest.plugin.trigger_description.clone(),
        supported_os: manifest.plugin.supported_os.clone(),
        priority: manifest.plugin.priority,
        // 第三方插件由宿主强制标注
        kind: PluginKind::ThirdParty,
        hotkey: manifest.plugin.hotkey.clone(),
        icon: manifest
            .icon
            .as_ref()
            .and_then(|icon| read_plugin_icon(plugin_dir, &icon.path)),
        mode: manifest.plugin.mode,
    };
    // 热键仅对 panel 形态生效（前端热键表与 wake_plugin 均按 mode 过滤）
    if metadata.hotkey.is_some() && metadata.mode != PluginMode::Panel {
        warn!(
            plugin_id = %metadata.id,
            declared = ?metadata.mode,
            "插件清单声明热键但形态非 panel：热键仅对 panel 形态生效，该热键将不注册"
        );
    }
    metadata
}

/// 读取清单 [icon] 段声明的图标文件（相对插件目录）并转为 data URL。
/// 路径逃逸插件目录、文件缺失或超过大小上限返回 None（图标缺失不阻断加载）。
fn read_plugin_icon(plugin_dir: &Path, icon_path: &str) -> Option<String> {
    let icon_abs = plugin_dir.join(icon_path);
    let canonical_icon = icon_abs.canonicalize().ok()?;
    let canonical_plugin_dir = plugin_dir.canonicalize().ok()?;
    // 防路径遍历：图标必须位于插件目录内
    if !canonical_icon.starts_with(&canonical_plugin_dir) {
        warn!("插件图标路径逃逸插件目录，忽略: {}", icon_path);
        return None;
    }
    // 上限预检：避免把超大文件整个读进内存（字节级上限由 icon::to_data_url 兜底）
    let meta = std::fs::metadata(&canonical_icon).ok()?;
    if meta.len() > MAX_ICON_BYTES {
        warn!("插件图标超过 1MB 上限，忽略: {}", icon_path);
        return None;
    }
    let bytes = std::fs::read(&canonical_icon).ok()?;
    to_data_url(mime_from_extension(&canonical_icon), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 事件过滤：清单/插件目录变更与 rescan 标记必须重扫（否则快照可能永久陈旧）；
    /// 插件目录深处的普通文件写入不重扫；监听报错也重扫。
    #[test]
    fn filters_install_relevant_events() {
        let root = Path::new(r"C:\data\plugins");
        let modify = || notify::EventKind::Modify(notify::event::ModifyKind::Any);

        let dir_event =
            notify::Event::new(modify()).add_path(PathBuf::from(r"C:\data\plugins\com.x"));
        let manifest_event = notify::Event::new(modify())
            .add_path(PathBuf::from(r"C:\data\plugins\com.x\manifest.toml"));
        let deep_event = notify::Event::new(modify())
            .add_path(PathBuf::from(r"C:\data\plugins\com.x\bin\app.exe"));
        let outside_event = notify::Event::new(modify()).add_path(PathBuf::from(r"C:\data\other"));
        let rescan_event =
            notify::Event::new(notify::EventKind::Other).set_flag(notify::event::Flag::Rescan);

        assert!(event_triggers_refresh(Ok(dir_event), root));
        assert!(event_triggers_refresh(Ok(manifest_event), root));
        assert!(!event_triggers_refresh(Ok(deep_event), root));
        assert!(!event_triggers_refresh(Ok(outside_event), root));
        // 丢事件（缓冲溢出）后必须无条件重扫
        assert!(event_triggers_refresh(Ok(rescan_event), root));
        assert!(event_triggers_refresh(
            Err(notify::Error::generic("buffer overflow")),
            root
        ));
    }

    /// 启动时扫描一次即入内存；未刷新时新增目录不出现在快照里，刷新后立即出现。
    #[test]
    fn snapshot_is_scanned_at_start_and_refreshed_explicitly() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("plugins").join("manually-placed-dir");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(plugin_dir.join("manifest.toml"), SAMPLE_MANIFEST).unwrap();

        let cache = InstalledCache::load(dir.path().join("plugins"));
        assert_eq!(cache.list().len(), 1);
        assert_eq!(cache.get("com.example.sample").unwrap().dir, plugin_dir);
        assert!(cache.get("com.example.other").is_none());

        let added = dir.path().join("plugins").join("later");
        std::fs::create_dir_all(&added).unwrap();
        std::fs::write(
            added.join("manifest.toml"),
            SAMPLE_MANIFEST.replace("com.example.sample", "com.example.later"),
        )
        .unwrap();
        // 读接口不扫盘：未刷新前看不到新目录
        assert_eq!(cache.list().len(), 1);
        cache.refresh();
        assert_eq!(cache.list().len(), 2);
        assert!(cache.get("com.example.later").is_some());
    }

    const SAMPLE_MANIFEST: &str = r#"
[plugin]
id = "com.example.sample"
name = "Sample Plugin"
version = "0.1.0"
description = "test"
author = "test"
mode = "inline"
triggerKeywords = []
supportedOs = ["windows"]
priority = 100

[components]
provides = ["data_source"]
"#;
}
