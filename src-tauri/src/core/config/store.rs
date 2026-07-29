use crate::core::config::models::{ComponentPersistentState, PersistentConfig};
use std::path::PathBuf;
use tracing::{debug, warn};
use zerolaunch_plugin_api::config::ConfigError;

/// 配置持久化层。
/// 负责将配置序列化为 JSON 并读写文件。
pub struct ConfigStore {
    /// 配置文件所在目录
    config_dir: PathBuf,
}

impl ConfigStore {
    /// 创建 ConfigStore，指定配置目录
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// 获取配置文件路径
    fn config_file_path(&self) -> PathBuf {
        self.config_dir.join("zerolaunch_config.json")
    }

    /// 从文件加载持久化配置。
    /// 文件不存在时返回默认空配置。
    pub fn load(&self) -> Result<PersistentConfig, ConfigError> {
        let path = self.config_file_path();
        if !path.exists() {
            debug!("配置文件不存在，返回默认配置: {:?}", path);
            return Ok(PersistentConfig::default());
        }

        let content = std::fs::read_to_string(&path)?;
        if content.trim().is_empty() {
            debug!("配置文件为空，返回默认配置: {:?}", path);
            return Ok(PersistentConfig::default());
        }

        match serde_json::from_str(&content) {
            Ok(config) => {
                debug!("成功加载配置文件: {:?}", path);
                Ok(config)
            }
            Err(e) => {
                warn!("配置文件解析失败: {:?}, 错误: {}", path, e);
                Err(ConfigError::SerializationError(e))
            }
        }
    }

    /// 将配置保存到文件。
    ///
    /// 使用原子写入策略：先写入临时文件，再 rename 替换目标文件。
    /// 避免写入过程中崩溃导致文件截断或损坏。
    pub fn save(&self, config: &PersistentConfig) -> Result<(), ConfigError> {
        let path: PathBuf = self.config_file_path();

        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(config)?;

        // 原子写入：先写 .tmp 文件，再 rename 替换目标
        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, &content)?;
        // 同步文件数据到磁盘
        if let Ok(file) = std::fs::File::open(&tmp_path) {
            file.sync_all().ok();
        }
        // 在 Windows 上，rename 在同一卷内是原子操作
        std::fs::rename(&tmp_path, &path)?;

        debug!("配置已保存到: {:?}", path);
        Ok(())
    }

    /// 备份损坏的配置文件。
    /// 将当前配置文件重命名为 .json.bak 后缀，保留现场便于排查。
    pub fn backup_corrupted(&self) -> Result<(), ConfigError> {
        let path = self.config_file_path();
        if !path.exists() {
            return Ok(());
        }

        let backup_path = path.with_extension("json.bak");
        // 如果已存在备份，先移除旧备份
        if backup_path.exists() {
            std::fs::remove_file(&backup_path).ok();
        }
        std::fs::rename(&path, &backup_path)?;
        warn!("已备份损坏配置文件: {:?} → {:?}", path, backup_path);
        Ok(())
    }

    /// 保存单个组件的状态到持久化配置。
    /// 读取现有配置、更新指定组件、再写回文件。
    pub fn save_component(
        &self,
        component_id: &str,
        state: &ComponentPersistentState,
    ) -> Result<(), ConfigError> {
        let mut config = self.load().unwrap_or_default();
        config
            .components
            .insert(component_id.to_string(), state.clone());
        self.save(&config)
    }
}
