use crate::db::queries;
use crate::jira::client::JiraClient;
use crate::jira::models::AppTask;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::State;

const DEFAULT_JQL: &str = r#"assignee = currentUser() AND statusCategory != "Done" ORDER BY updated DESC"#;

fn row_to_app_task(row: &queries::TaskRow) -> AppTask {
    AppTask {
        id: row.id.clone(),
        summary: row.summary.clone(),
        project_key: row.project_key.clone().unwrap_or_default(),
        project_name: row.project_name.clone().unwrap_or_default(),
        status: row.status.clone().unwrap_or_default(),
        sprint_name: row.sprint_name.clone(),
        pinned: row.pinned,
        last_fetched: row.last_fetched.clone(),
    }
}

#[tauri::command]
pub async fn fetch_my_tasks(
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AppTask>, String> {
    let client = {
        let state = client_state.lock().unwrap();
        state.clone().ok_or("Not logged in")?
    };

    let result = client.search_issues(DEFAULT_JQL).await.map_err(|e| {
        eprintln!("[JTT] fetch_my_tasks JQL error: {}", e);
        e
    })?;

    for issue in &result.issues {
        let project_key = issue.fields.project.as_ref().map(|p| p.key.as_str()).unwrap_or("");
        let project_name = issue.fields.project.as_ref().map(|p| p.name.as_str()).unwrap_or("");
        let status = issue.fields.status.as_ref().map(|s| s.name.as_str()).unwrap_or("");

        let _ = queries::upsert_task(
            &pool, &issue.key, &issue.fields.summary,
            project_key, project_name, status, None,
        ).await;
    }

    let rows = queries::get_all_tasks(&pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(rows.iter().map(row_to_app_task).collect())
}

#[tauri::command]
pub async fn search_task(
    query: String,
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AppTask>, String> {
    // First try local DB
    let rows = queries::search_tasks(&pool, &query)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    if !rows.is_empty() {
        return Ok(rows.iter().map(row_to_app_task).collect());
    }

    // If local is empty and query looks like a Jira key (e.g. "ABC-123") or text, search via API
    let client = {
        let state = client_state.lock().unwrap();
        match state.clone() {
            Some(c) => c,
            None => return Ok(vec![]),
        }
    };

    let jql = if query.contains('-') && query.chars().any(|c| c.is_ascii_digit()) {
        // Looks like a Jira key
        format!(r#"key = "{}" ORDER BY updated DESC"#, query.replace('"', ""))
    } else {
        format!(r#"text ~ "{}" ORDER BY updated DESC"#, query.replace('"', ""))
    };

    match client.search_issues(&jql).await {
        Ok(result) => {
            let mut tasks = Vec::new();
            for issue in &result.issues {
                let project_key = issue.fields.project.as_ref().map(|p| p.key.as_str()).unwrap_or("");
                let project_name = issue.fields.project.as_ref().map(|p| p.name.as_str()).unwrap_or("");
                let status = issue.fields.status.as_ref().map(|s| s.name.as_str()).unwrap_or("");

                let _ = queries::upsert_task(
                    &pool, &issue.key, &issue.fields.summary,
                    project_key, project_name, status, None,
                ).await;

                tasks.push(AppTask {
                    id: issue.key.clone(),
                    summary: issue.fields.summary.clone(),
                    project_key: project_key.to_string(),
                    project_name: project_name.to_string(),
                    status: status.to_string(),
                    sprint_name: None,
                    pinned: false,
                    last_fetched: None,
                });
            }
            Ok(tasks)
        }
        Err(_) => Ok(vec![]),
    }
}

#[tauri::command]
pub async fn pin_task(
    task_id: String,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    queries::pin_task(&pool, &task_id)
        .await
        .map_err(|e| format!("Failed to pin task: {}", e))
}

#[tauri::command]
pub async fn unpin_task(
    task_id: String,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    queries::unpin_task(&pool, &task_id)
        .await
        .map_err(|e| format!("Failed to unpin task: {}", e))
}
