use crate::db::queries;
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn get_setting(
    key: String,
    pool: State<'_, SqlitePool>,
) -> Result<Option<String>, String> {
    queries::get_setting(&pool, &key)
        .await
        .map_err(|e| format!("Failed to get setting: {}", e))
}

#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    queries::set_setting(&pool, &key, &value)
        .await
        .map_err(|e| format!("Failed to set setting: {}", e))
}
