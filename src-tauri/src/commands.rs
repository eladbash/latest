#[tauri::command]
pub async fn get_apps() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[tauri::command]
pub async fn check_updates_now() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[tauri::command]
pub async fn trigger_update(_app_path: String, _source: String) -> Result<String, String> {
    Ok("Not implemented yet".to_string())
}
