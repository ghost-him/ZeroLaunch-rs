// src-tauri/src/infrastructure/service_locator.rs
use super::super::error::{OptionExt, ResultExt};
use super::super::state::app_state::AppState;
use std::sync::Arc;
use std::sync::OnceLock;

static APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

pub struct ServiceLocator;

impl ServiceLocator {
    pub fn init(state: Arc<AppState>) {
        APP_STATE
            .set(state)
            .expect_programming("Failed to initialize app state");
    }

    pub fn get_state() -> Arc<AppState> {
        APP_STATE
            .get()
            .cloned()
            .expect_programming("State not initialized")
    }
}
