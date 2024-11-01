// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;

#[tauri::command]
async fn search(query: String) -> Result<Vec<String>, String> {
    // 实现搜索逻辑
    Ok(vec![
        format!("Result 1 for '{}'", query),
        format!("Result 2 for '{}'", query),
        format!("Result 3 for '{}'", query),
        format!("Result 4 for '{}'", query),
    ])
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![search])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
