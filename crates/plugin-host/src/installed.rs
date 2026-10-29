//! 已安装插件的解析与投影。
//!
//! 安装事实的唯一权威是 plugins 目录下的清单：目录定位、清单与元数据在扫描时
//! 各解析一次，供列表、详情、重载、卸载与覆盖安装共用。未加载（启动/重载失败）
//! 的插件同样可解析，从其投影出 error 态条目，界面不会因加载失败而丢条目。

use std::path::{Path, PathBuf};

use tracing::warn;
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

impl PluginHostManager {
    /// 枚举包含 manifest.toml 的插件目录（不解析清单）。
    /// 返回：目录列表；安装根目录不存在时为空。
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

    /// 全量枚举已安装插件（唯一权威：目录下的清单）。
    /// 返回：解析成功的记录；清单损坏的目录告警后跳过。
    pub fn scan_installed(&self) -> Vec<InstalledPlugin> {
        self.scan_dirs()
            .iter()
            .filter_map(|dir| InstalledPlugin::from_dir(dir))
            .collect()
    }

    /// 按清单 id 定位安装记录：已加载取注册包快照（免磁盘 IO），否则扫描磁盘。
    /// 参数：plugin_id - 插件 id。
    /// 返回：安装记录；未安装返回 None。
    pub fn installed(&self, plugin_id: &str) -> Option<InstalledPlugin> {
        if let Some(registration) = self.plugins.get(plugin_id) {
            return Some(InstalledPlugin {
                dir: registration.plugin_dir.clone(),
                manifest: registration.manifest.clone(),
                metadata: (*registration.metadata).clone(),
            });
        }
        self.scan_installed()
            .into_iter()
            .find(|plugin| plugin.metadata.id == plugin_id)
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
