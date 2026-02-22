use crate::db::queries;
use crate::jira::models::{AppTimeEntry, AppTimerState};
use crate::timer::engine::TimerEngine;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::State;

fn row_to_app_entry(row: &queries::EntryRow) -> AppTimeEntry {
    AppTimeEntry {
        id: row.id,
        task_id: row.task_id.clone(),
        start_time: row.start_time.clone(),
        end_time: row.end_time.clone(),
        duration_secs: row.duration_secs,
        adjusted_secs: row.adjusted_secs,
        description: row.description.clone(),
        synced_to_jira: row.synced_to_jira,
        jira_worklog_id: row.jira_worklog_id.clone(),
    }
}

#[tauri::command]
pub async fn start_timer(
    task_id: String,
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    let stopped = {
        let mut engine = engine_state.lock().unwrap();
        engine.start(&task_id)
    };

    // Finalize the previously running entry if any
    if let Some(stopped_entry) = stopped {
        if let Ok(Some(entry)) = queries::get_running_entry_for_task(&pool, &stopped_entry.task_id).await {
            let _ = queries::finalize_entry(&pool, entry.id, stopped_entry.duration_secs as i64).await;
        }
    }

    // Create a new entry
    queries::create_entry(&pool, &task_id)
        .await
        .map_err(|e| format!("Failed to create time entry: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn stop_timer(
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    let stopped = {
        let mut engine = engine_state.lock().unwrap();
        engine.stop()
    };

    if let Some(stopped_entry) = stopped {
        if let Ok(Some(entry)) = queries::get_running_entry_for_task(&pool, &stopped_entry.task_id).await {
            let _ = queries::finalize_entry(&pool, entry.id, stopped_entry.duration_secs as i64).await;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn pause_timer(
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
) -> Result<(), String> {
    let mut engine = engine_state.lock().unwrap();
    engine.pause()
}

#[tauri::command]
pub async fn resume_timer(
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
) -> Result<(), String> {
    let mut engine = engine_state.lock().unwrap();
    engine.resume()
}

#[tauri::command]
pub async fn get_active_timer(
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
) -> Result<AppTimerState, String> {
    let engine = engine_state.lock().unwrap();
    Ok(AppTimerState {
        status: engine.get_status_str().to_string(),
        task_id: engine.get_task_id(),
        elapsed_secs: engine.get_elapsed(),
    })
}

#[tauri::command]
pub async fn get_entries_today(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AppTimeEntry>, String> {
    let rows = queries::get_entries_today(&pool)
        .await
        .map_err(|e| format!("Failed to query entries: {}", e))?;

    Ok(rows.iter().map(row_to_app_entry).collect())
}

#[tauri::command]
pub async fn update_entry(
    entry_id: i64,
    adjusted_secs: Option<i64>,
    description: Option<String>,
    _date: Option<String>,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    queries::update_entry(&pool, entry_id, adjusted_secs, description.as_deref())
        .await
        .map_err(|e| format!("Failed to update entry: {}", e))
}
