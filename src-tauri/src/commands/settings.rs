use crate::commands::timer::TIMER_SESSION_KEY;
use crate::db::queries;
use crate::timer;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

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

#[tauri::command]
pub async fn reset_timer_data(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<u64, String> {
    // 1. Stop running timer (in-memory)
    let engine = app.state::<Arc<Mutex<timer::engine::TimerEngine>>>();
    {
        engine.lock().unwrap().stop();
    }

    // 2. Delete all time entries from DB
    let deleted = queries::delete_all_entries(&pool)
        .await
        .map_err(|e| format!("Failed to reset data: {}", e))?;

    // 3. Clear persisted timer session snapshot
    queries::delete_setting(&pool, TIMER_SESSION_KEY)
        .await
        .map_err(|e| format!("Failed to clear timer session: {}", e))?;

    // 4. Emit event so frontend stores refresh
    let _ = app.emit("timer-stopped", ());

    Ok(deleted)
}

#[tauri::command]
pub async fn set_launch_at_login(
    enabled: bool,
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    let autostart = app.autolaunch();
    if enabled {
        autostart
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
    } else {
        autostart
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {}", e))?;
    }
    queries::set_setting(&pool, "launch_at_login", &enabled.to_string())
        .await
        .map_err(|e| format!("Failed to save setting: {}", e))
}
