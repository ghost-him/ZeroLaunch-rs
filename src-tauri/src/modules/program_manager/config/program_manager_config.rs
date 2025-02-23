use crate::program_manager::config::program_launcher_config::PartialProgramLauncherConfig;
use crate::program_manager::config::program_launcher_config::ProgramLauncherConfig;
use crate::program_manager::config::program_loader_config::PartialProgramLoaderConfig;
use crate::program_manager::config::program_loader_config::ProgramLoaderConfig;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialProgramManagerConfig {
    pub launcher: Option<PartialProgramLauncherConfig>,
    pub loader: Option<PartialProgramLoaderConfig>,
}

#[derive(Debug)]
pub struct ProgramManagerConfigInner {
    pub launcher_config: Arc<ProgramLauncherConfig>,
    pub loader_config: Arc<ProgramLoaderConfig>,
}

impl Default for ProgramManagerConfigInner {
    fn default() -> Self {
        ProgramManagerConfigInner {
            launcher_config: Arc::new(ProgramLauncherConfig::default()),
            loader_config: Arc::new(ProgramLoaderConfig::default()),
        }
    }
}

impl ProgramManagerConfigInner {
    pub fn to_partial(&self) -> PartialProgramManagerConfig {
        PartialProgramManagerConfig {
            launcher: Some(self.launcher_config.to_partial()),
            loader: Some(self.loader_config.to_partial()),
        }
    }
    pub fn update(&mut self, partial_config: PartialProgramManagerConfig) {
        if let Some(partial_launcher) = partial_config.launcher {
            self.launcher_config.update(partial_launcher);
        }
        if let Some(partial_loader) = partial_config.loader {
            self.loader_config.update(partial_loader);
        }
    }
}
#[derive(Debug)]
pub struct ProgramManagerConfig {
    inner: RwLock<ProgramManagerConfigInner>,
}

impl Default for ProgramManagerConfig {
    fn default() -> Self {
        ProgramManagerConfig {
            inner: RwLock::new(ProgramManagerConfigInner::default()),
        }
    }
}

impl ProgramManagerConfig {
    pub fn to_partial(&self) -> PartialProgramManagerConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_launcher_config(&self) -> Arc<ProgramLauncherConfig> {
        self.inner.read().launcher_config.clone()
    }

    pub fn get_loader_config(&self) -> Arc<ProgramLoaderConfig> {
        self.inner.read().loader_config.clone()
    }
    pub fn update(&self, partial_config: PartialProgramManagerConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_config);
    }
}
