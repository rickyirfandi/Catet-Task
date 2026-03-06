use crate::db::queries;
use crate::jira::client::JiraClient;
use crate::jira::models::{AppTask, AppTaskDetail};
use sqlx::SqlitePool;
use serde_json::Value;
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
        in_current_sprint: false,
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
        eprintln!("[CT] fetch_my_tasks JQL error: {}", e);
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
    let q = query.trim().to_string();
    if q.is_empty() {
        return Ok(vec![]);
    }

    // 1. Local DB search (instant)
    let local_rows = queries::search_tasks(&pool, &q)
        .await
        .unwrap_or_default();

    // 2. Remote Jira search (if logged in)
    let client = {
        let state = client_state.lock().unwrap();
        state.clone()
    };

    let Some(client) = client else {
        return Ok(local_rows.iter().map(row_to_app_task).collect());
    };

    let sanitized = q.replace('"', "");
    let is_key_like = sanitized.contains('-') && sanitized.chars().any(|c| c.is_ascii_digit());

    let text_clause = if is_key_like {
        format!(r#"key = "{}""#, sanitized)
    } else {
        format!(r#"text ~ "{}""#, sanitized)
    };

    // Two concurrent queries: sprint-scoped (prioritized) and broad
    let jql_sprint = format!(
        r#"{} AND sprint in openSprints() ORDER BY updated DESC"#,
        text_clause
    );
    let jql_broad = format!(
        r#"{} ORDER BY updated DESC"#,
        text_clause
    );

    let (sprint_result, broad_result) = tokio::join!(
        client.search_issues_limited(&jql_sprint, 20),
        client.search_issues_limited(&jql_broad, 15),
    );

    let mut seen = std::collections::HashSet::new();
    let mut tasks = Vec::new();

    // Sprint results first (tagged as in_current_sprint)
    if let Ok(result) = sprint_result {
        for issue in &result.issues {
            if seen.insert(issue.key.clone()) {
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
                    in_current_sprint: true,
                });
            }
        }
    }

    // Broad results (non-sprint or fallback)
    if let Ok(result) = broad_result {
        for issue in &result.issues {
            if seen.insert(issue.key.clone()) {
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
                    in_current_sprint: false,
                });
            }
        }
    }

    // Merge local DB matches not already in remote results
    for row in &local_rows {
        if seen.insert(row.id.clone()) {
            tasks.push(row_to_app_task(row));
        }
    }

    Ok(tasks)
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

#[tauri::command]
pub async fn get_task_detail(
    task_id: String,
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
) -> Result<AppTaskDetail, String> {
    let client = {
        let state = client_state.lock().unwrap();
        state.clone().ok_or("Not logged in")?
    };

    let issue = client.get_issue(&task_id).await?;
    let fields = issue.fields;

    let project_key = fields
        .project
        .as_ref()
        .map(|p| p.key.clone())
        .unwrap_or_default();
    let project_name = fields
        .project
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_default();
    let status = fields
        .status
        .as_ref()
        .map(|s| s.name.clone())
        .unwrap_or_default();
    let issue_type = fields.issue_type.as_ref().map(|v| v.name.clone());
    let priority = fields.priority.as_ref().map(|v| v.name.clone());
    let assignee = fields.assignee.as_ref().map(|v| v.display_name.clone());
    let description = fields
        .description
        .as_ref()
        .map(adf_to_plain_text)
        .filter(|text| !text.is_empty());

    Ok(AppTaskDetail {
        task_id: issue.key,
        summary: fields.summary,
        description,
        status,
        project_key,
        project_name,
        issue_type,
        priority,
        assignee,
        updated_at: fields.updated,
        created_at: fields.created,
    })
}

fn adf_to_plain_text(value: &Value) -> String {
    fn walk(node: &Value, out: &mut String) {
        if let Some(text) = node.as_str() {
            out.push_str(text);
            return;
        }

        if let Some(items) = node.as_array() {
            for item in items {
                walk(item, out);
            }
            return;
        }

        if let Some(node_type) = node.get("type").and_then(Value::as_str) {
            if node_type == "hardBreak" {
                out.push('\n');
            }
        }

        if let Some(text) = node.get("text").and_then(Value::as_str) {
            out.push_str(text);
        }

        if let Some(children) = node.get("content").and_then(Value::as_array) {
            let is_paragraph = node.get("type").and_then(Value::as_str) == Some("paragraph");
            for child in children {
                walk(child, out);
            }
            if is_paragraph && !out.ends_with('\n') {
                out.push('\n');
            }
        }
    }

    let mut output = String::new();
    walk(value, &mut output);
    output.trim().to_string()
}
