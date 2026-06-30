#![cfg(feature = "mock")]

use zerolaunch_plugin_api::host::OpenTarget;
use zerolaunch_plugin_api::mock::helpers::mock_plugin_handle;

#[tokio::test]
async fn mock_plugin_handle_constructible() {
    let handle = mock_plugin_handle();
    let _ = handle.shell_open(OpenTarget::File("test.txt".into())).await;
    let apps = handle.enumerate_apps().await;
    assert!(apps.is_empty());
}

#[tokio::test]
async fn mock_plugin_handle_icon_returns_empty() {
    let handle = mock_plugin_handle();
    let icon = handle
        .get_icon(zerolaunch_plugin_api::services::IconRequest::Path(
            "test.exe".into(),
        ))
        .await;
    assert!(icon.is_ok());
}
