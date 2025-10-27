use super::i18n::{t, t_with};
use super::service_locator::ServiceLocator;
use tauri_plugin_notification::NotificationExt;

pub fn notify(title: &str, message: &str) {
    let state = ServiceLocator::get_state();
    let app_handle = state.get_main_handle();

    if let Err(e) = app_handle
        .notification()
        .builder()
        .title(title)
        .body(message)
        .show()
    {
        tracing::error!("Failed to show notification: {}", e);
    }
}

/// 国际化通知函数
///
/// 使用翻译键显示通知
///
/// # 参数
/// * `title` - 通知标题（不翻译）
/// * `message_key` - 通知消息的翻译键
///
pub fn notify_i18n(title: &str, message_key: &str) {
    let message = t(message_key);
    notify(title, &message);
}

/// 国际化通知函数（带占位符替换）
///
/// 使用翻译键显示通知，并替换占位符
///
/// # 参数
/// * `title` - 通知标题（不翻译）
/// * `message_key` - 通知消息的翻译键
/// * `replacements` - 占位符替换数组
///
pub fn notify_i18n_with(title: &str, message_key: &str, replacements: &[(&str, &str)]) {
    let message = t_with(message_key, replacements);
    notify(title, &message);
}
