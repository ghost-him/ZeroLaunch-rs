use super::service_locator::ServiceLocator;
use tauri_plugin_notification::NotificationExt;

pub fn notify(title: &str, message: &str) {
    let state = ServiceLocator::get_state();
    let app_handle = state.get_main_handle();

    if app_handle.is_ok() {
        app_handle
            .unwrap()
            .notification()
            .builder()
            .title(title)
            .body(message)
            .show()
            .unwrap();
    }
}
