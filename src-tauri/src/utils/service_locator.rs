// src-tauri/src/infrastructure/service_locator.rs
use super::super::state::app_state::AppState;
use std::sync::Arc;
use std::sync::OnceLock;

static APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

pub struct ServiceLocator;

impl ServiceLocator {
    pub fn init(state: Arc<AppState>) {
        APP_STATE.set(state).unwrap();
    }

    pub fn get_state() -> Arc<AppState> {
        APP_STATE.get().cloned().expect("State not initialized")
    }
}
