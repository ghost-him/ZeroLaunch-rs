use super::service_locator::ServiceLocator;
use tauri_plugin_notification::NotificationExt;

pub fn notify(title: &str, message: &str) {
    let state = ServiceLocator::get_state();
    let app_handle = state.get_main_handle();

    if let Ok(handle) = app_handle {
        handle
        .notification()
        .builder()
        .title(title)
        .body(message)
        .show()
        .unwrap();
    }
}
