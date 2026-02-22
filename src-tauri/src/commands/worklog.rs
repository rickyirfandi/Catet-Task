use crate::db::queries;
use crate::jira::client::JiraClient;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorklogEntry {
    pub entry_ids: Vec<i64>,
    pub task_id: String,
    pub time_spent_seconds: u64,
    pub started: String,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorklogProgressEvent {
    pub task_id: String,
    pub status: String,
    pub error: Option<String>,
    pub worklog_id: Option<String>,
}

#[tauri::command]
pub async fn submit_batch_worklog(
    entries: Vec<WorklogEntry>,
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let client = {
        let state = client_state.lock().unwrap();
        state.clone().ok_or("Not logged in")?
    };

    for entry in &entries {
        let _ = app.emit(
            "worklog-progress",
            WorklogProgressEvent {
                task_id: entry.task_id.clone(),
                status: "submitting".to_string(),
                error: None,
                worklog_id: None,
            },
        );

        match client
            .add_worklog(
                &entry.task_id,
                entry.time_spent_seconds,
                &entry.started,
                &entry.comment,
            )
            .await
        {
            Ok(worklog) => {
                // Mark ALL entries for this task as synced
                for eid in &entry.entry_ids {
                    let _ = queries::mark_entry_synced(&pool, *eid, &worklog.id).await;
                }

                let _ = app.emit(
                    "worklog-progress",
                    WorklogProgressEvent {
                        task_id: entry.task_id.clone(),
                        status: "done".to_string(),
                        error: None,
                        worklog_id: Some(worklog.id),
                    },
                );
            }
            Err(e) => {
                let _ = app.emit(
                    "worklog-progress",
                    WorklogProgressEvent {
                        task_id: entry.task_id.clone(),
                        status: "error".to_string(),
                        error: Some(e.clone()),
                        worklog_id: None,
                    },
                );

                if e.contains("Authentication failed") {
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}
