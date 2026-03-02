use crate::db::queries;
use crate::jira::models::{AppTimeEntry, AppTimerState};
use crate::timer::engine::{self, PersistedTimerState, StoppedEntry, TimerEngine};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::State;

pub const TIMER_SESSION_KEY: &str = "timer_session_v1";

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

async fn persist_timer_session(
    pool: &SqlitePool,
    session: Option<PersistedTimerState>,
) -> Result<(), String> {
    match session {
        Some(session) => {
            let raw = serde_json::to_string(&session)
                .map_err(|e| format!("Failed to serialize timer session: {}", e))?;
            queries::set_setting(pool, TIMER_SESSION_KEY, &raw)
                .await
                .map_err(|e| format!("Failed to persist timer session: {}", e))
        }
        None => queries::delete_setting(pool, TIMER_SESSION_KEY)
            .await
            .map_err(|e| format!("Failed to clear timer session: {}", e)),
    }
}

async fn load_persisted_timer_session(
    pool: &SqlitePool,
) -> Result<Option<PersistedTimerState>, String> {
    let raw = queries::get_setting(pool, TIMER_SESSION_KEY)
        .await
        .map_err(|e| format!("Failed to load persisted timer session: {}", e))?;

    let Some(raw) = raw else {
        return Ok(None);
    };

    match serde_json::from_str::<PersistedTimerState>(&raw) {
        Ok(session) => Ok(Some(session)),
        Err(e) => {
            eprintln!(
                "[CT] timer recovery: invalid persisted session, dropping: {}",
                e
            );
            Ok(None)
        }
    }
}

async fn finalize_stopped_entry(pool: &SqlitePool, stopped: &StoppedEntry) -> Result<(), String> {
    if let Some(entry) = queries::get_running_entry_for_task(pool, &stopped.task_id)
        .await
        .map_err(|e| format!("Failed to query running entry: {}", e))?
    {
        queries::finalize_entry(pool, entry.id, stopped.duration_secs as i64)
            .await
            .map_err(|e| format!("Failed to finalize running entry: {}", e))?;
    }
    Ok(())
}

pub async fn recover_timer_on_startup(
    engine_state: &Arc<Mutex<TimerEngine>>,
    pool: &SqlitePool,
) -> Result<(), String> {
    let open_entries = queries::get_open_entries(pool)
        .await
        .map_err(|e| format!("Failed to query open entries: {}", e))?;
    let primary_open_entry = open_entries.first().cloned();

    if open_entries.len() > 1 {
        let recovered_id = primary_open_entry.as_ref().map(|entry| entry.id);
        let finalized = queries::finalize_open_entries_except(pool, recovered_id)
            .await
            .map_err(|e| format!("Failed to clean extra open entries: {}", e))?;
        if finalized > 0 {
            eprintln!(
                "[CT] timer recovery: finalized {} extra open entries",
                finalized
            );
        }
    }

    let persisted = load_persisted_timer_session(pool).await?;
    let fallback_started_at_utc = primary_open_entry
        .as_ref()
        .and_then(|entry| engine::parse_utc_timestamp(&entry.start_time));

    let (session_to_persist, should_finalize_unrecoverable_open) = {
        let mut eng = engine_state.lock().unwrap();
        let mut recovered = false;
        let mut should_finalize_unrecoverable_open = false;

        if let (Some(session), Some(open_entry)) = (persisted.as_ref(), primary_open_entry.as_ref())
        {
            if session.task_id == open_entry.task_id {
                if let Err(e) = eng.restore_from_persisted(session, fallback_started_at_utc) {
                    eprintln!(
                        "[CT] timer recovery: persisted session restore failed: {}",
                        e
                    );
                } else {
                    recovered = true;
                }
            } else {
                eprintln!(
                    "[CT] timer recovery: task mismatch between persisted session ({}) and open entry ({})",
                    session.task_id, open_entry.task_id
                );
            }
        } else if persisted.is_some() {
            eprintln!(
                "[CT] timer recovery: persisted session found without open DB entry, dropping session"
            );
        }

        if !recovered {
            if let Some(open_entry) = primary_open_entry.as_ref() {
                if let Some(started_at_utc) = engine::parse_utc_timestamp(&open_entry.start_time) {
                    eng.restore_running_from_start(open_entry.task_id.clone(), started_at_utc);
                    recovered = true;
                } else {
                    eprintln!(
                        "[CT] timer recovery: unparsable open entry start_time, finalizing open entries"
                    );
                    should_finalize_unrecoverable_open = true;
                }
            }
        }

        if !recovered {
            let _ = eng.stop();
        }

        (eng.persisted_state(), should_finalize_unrecoverable_open)
    };

    if should_finalize_unrecoverable_open {
        let finalized = queries::finalize_open_entries_except(pool, None)
            .await
            .map_err(|e| format!("Failed to finalize unrecoverable open entries: {}", e))?;
        if finalized > 0 {
            eprintln!(
                "[CT] timer recovery: finalized {} unrecoverable open entries",
                finalized
            );
        }
    }

    persist_timer_session(pool, session_to_persist).await?;
    Ok(())
}

#[tauri::command]
pub async fn start_timer(
    task_id: String,
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let (stopped, session_after_start) = {
        let mut eng = engine_state.lock().unwrap();
        let stopped = eng.start(&task_id);
        let session_after_start = eng.persisted_state();
        engine::update_tray_now(&app, &eng);
        (stopped, session_after_start)
    };

    // Finalize the previously running entry if any
    if let Some(stopped_entry) = stopped {
        finalize_stopped_entry(&pool, &stopped_entry).await?;
    }

    // Create a new entry. If creation fails, roll back the in-memory state to idle.
    if let Err(e) = queries::create_entry(&pool, &task_id).await {
        {
            let mut eng = engine_state.lock().unwrap();
            let _ = eng.stop();
            engine::update_tray_now(&app, &eng);
        }
        let _ = persist_timer_session(&pool, None).await;
        return Err(format!("Failed to create time entry: {}", e));
    }

    persist_timer_session(&pool, session_after_start).await?;

    Ok(())
}

#[tauri::command]
pub async fn stop_timer(
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let (stopped, session_after_stop) = {
        let mut eng = engine_state.lock().unwrap();
        let stopped = eng.stop();
        let session_after_stop = eng.persisted_state();
        engine::update_tray_now(&app, &eng);
        (stopped, session_after_stop)
    };

    if let Some(stopped_entry) = stopped {
        finalize_stopped_entry(&pool, &stopped_entry).await?;
    }

    persist_timer_session(&pool, session_after_stop).await?;

    Ok(())
}

#[tauri::command]
pub async fn pause_timer(
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let session_after_pause = {
        let mut eng = engine_state.lock().unwrap();
        eng.pause()?;
        engine::update_tray_now(&app, &eng);
        eng.persisted_state()
    };

    persist_timer_session(&pool, session_after_pause).await?;
    Ok(())
}

#[tauri::command]
pub async fn resume_timer(
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let session_after_resume = {
        let mut eng = engine_state.lock().unwrap();
        eng.resume()?;
        engine::update_tray_now(&app, &eng);
        eng.persisted_state()
    };

    persist_timer_session(&pool, session_after_resume).await?;
    Ok(())
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
pub async fn get_entries_today(pool: State<'_, SqlitePool>) -> Result<Vec<AppTimeEntry>, String> {
    let rows = queries::get_entries_today(&pool)
        .await
        .map_err(|e| format!("Failed to query entries: {}", e))?;

    Ok(rows.iter().map(row_to_app_entry).collect())
}

#[tauri::command]
pub async fn get_entries_range(
    start_date: String,
    end_date: String,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AppTimeEntry>, String> {
    if start_date > end_date {
        return Err("Invalid date range: startDate is after endDate.".to_string());
    }

    let rows = queries::get_entries_range(&pool, &start_date, &end_date)
        .await
        .map_err(|e| format!("Failed to query entries by range: {}", e))?;

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

#[tauri::command]
pub async fn quit_app(
    engine_state: State<'_, Arc<Mutex<TimerEngine>>>,
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Stop running timer gracefully
    let (stopped, session_after_stop) = {
        let mut engine = engine_state.lock().unwrap();
        let stopped = engine.stop();
        let session_after_stop = engine.persisted_state();
        (stopped, session_after_stop)
    };

    // Finalize DB entry if timer was active
    if let Some(entry) = stopped {
        finalize_stopped_entry(&pool, &entry).await?;
    }

    persist_timer_session(&pool, session_after_stop).await?;

    app.exit(0);
    Ok(())
}
