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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_store() -> (ConfigStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        (ConfigStore::new(dir.path().to_path_buf()), dir)
    }

    #[test]
    fn load_missing_file_returns_default() {
        let (store, _dir) = temp_store();
        let config = store.load().expect("加载缺失文件应返回默认配置");
        assert_eq!(config.version, "3");
        assert!(config.components.is_empty());
    }

    #[test]
    fn load_empty_file_returns_default() {
        let (store, dir) = temp_store();
        std::fs::write(dir.path().join("zerolaunch_config.json"), "").unwrap();
        let config = store.load().expect("空文件应返回默认配置");
        assert!(config.components.is_empty());
    }

    #[test]
    fn save_writes_atomic_file_without_tmp_leftover() {
        let (store, dir) = temp_store();
        let config = PersistentConfig {
            version: "3".to_string(),
            components: Default::default(),
        };
        store.save(&config).expect("保存失败");
        let path = dir.path().join("zerolaunch_config.json");
        assert!(path.exists(), "保存后配置文件应存在于配置目录");
        assert!(
            !dir.path().join("zerolaunch_config.tmp").exists(),
            "原子写入不应残留 .tmp 文件"
        );
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("配置文件应为合法 JSON");
        assert_eq!(parsed["version"], "3");
    }

    #[test]
    fn save_then_load_roundtrip_preserves_settings() {
        let (store, _dir) = temp_store();
        let state = ComponentPersistentState {
            enabled: true,
            settings: json!({ "theme": "dark", "log_level": "warn" }),
        };
        store
            .save_component("appearance-config", &state)
            .expect("保存失败");
        let loaded = store.load().expect("加载失败");
        let loaded_state = loaded
            .components
            .get("appearance-config")
            .expect("组件应存在");
        assert!(loaded_state.enabled);
        assert_eq!(loaded_state.settings["theme"], "dark");
        assert_eq!(loaded_state.settings["log_level"], "warn");
    }

    #[test]
    fn load_corrupted_file_returns_serialization_error() {
        let (store, dir) = temp_store();
        std::fs::write(dir.path().join("zerolaunch_config.json"), "{not json").unwrap();
        let err = store.load().expect_err("损坏文件应报错");
        assert!(matches!(err, ConfigError::SerializationError(_)));
    }

    #[test]
    fn backup_corrupted_renames_to_bak() {
        let (store, dir) = temp_store();
        let path = dir.path().join("zerolaunch_config.json");
        std::fs::write(&path, "corrupted").unwrap();
        store.backup_corrupted().expect("备份失败");
        assert!(!path.exists(), "损坏文件应被移走");
        assert!(
            dir.path().join("zerolaunch_config.json.bak").exists(),
            "应生成 .json.bak 备份"
        );
    }

    #[test]
    fn backup_corrupted_noop_when_missing() {
        let (store, _dir) = temp_store();
        store.backup_corrupted().expect("缺失文件备份应幂等");
    }
}
