use sqlx::SqlitePool;

use crate::models::{EntryRow, TaskRow, TimerSession};

const TIMER_SESSION_KEY: &str = "timer_session_v1";

pub async fn open_pool(path: &std::path::Path) -> Result<SqlitePool, String> {
    let url = format!("sqlite:{}?mode=ro", path.display());
    SqlitePool::connect(&url)
        .await
        .map_err(|e| format!("Cannot open database at {}: {}\nIs Catet Task installed?", path.display(), e))
}

pub async fn open_pool_rw(path: &std::path::Path) -> Result<SqlitePool, String> {
    let url = format!("sqlite:{}?mode=rw", path.display());
    SqlitePool::connect(&url)
        .await
        .map_err(|e| format!("Cannot open database at {}: {}", path.display(), e))
}

pub async fn get_entries_today(pool: &SqlitePool) -> Result<Vec<EntryRow>, String> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT id, task_id, start_time, end_time, duration_secs, adjusted_secs, description, synced_to_jira, jira_worklog_id
         FROM time_entries
         WHERE date(start_time, 'localtime') = date('now', 'localtime')
         ORDER BY start_time ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))
}

pub async fn get_entries_range(
    pool: &SqlitePool,
    from: &str,
    to: &str,
) -> Result<Vec<EntryRow>, String> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT id, task_id, start_time, end_time, duration_secs, adjusted_secs, description, synced_to_jira, jira_worklog_id
         FROM time_entries
         WHERE date(start_time, 'localtime') BETWEEN date(?1) AND date(?2)
         ORDER BY start_time ASC",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))
}

pub async fn get_all_tasks(pool: &SqlitePool) -> Result<Vec<TaskRow>, String> {
    sqlx::query_as::<_, TaskRow>(
        "SELECT id, summary, project_key, project_name, status, sprint_name, pinned, last_fetched
         FROM tasks ORDER BY pinned DESC, last_fetched DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))
}

pub async fn search_tasks(
    pool: &SqlitePool,
    query: &str,
    project_key: Option<&str>,
    limit: i64,
) -> Result<Vec<TaskRow>, String> {
    let like = format!("%{}%", query);
    match project_key {
        Some(project) => {
            sqlx::query_as::<_, TaskRow>(
                "SELECT id, summary, project_key, project_name, status, sprint_name, pinned, last_fetched
                 FROM tasks
                 WHERE (id LIKE ?1 OR summary LIKE ?1) AND project_key = ?2
                 ORDER BY pinned DESC, last_fetched DESC
                 LIMIT ?3",
            )
            .bind(&like)
            .bind(project)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))
        }
        None => {
            sqlx::query_as::<_, TaskRow>(
                "SELECT id, summary, project_key, project_name, status, sprint_name, pinned, last_fetched
                 FROM tasks
                 WHERE id LIKE ?1 OR summary LIKE ?1
                 ORDER BY pinned DESC, last_fetched DESC
                 LIMIT ?2",
            )
            .bind(&like)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))
        }
    }
}

pub async fn upsert_task(
    pool: &SqlitePool,
    id: &str,
    summary: &str,
    project_key: &str,
    project_name: &str,
    status: &str,
    sprint_name: Option<&str>,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO tasks (id, summary, project_key, project_name, status, sprint_name, last_fetched)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
           summary = ?2,
           project_key = ?3,
           project_name = ?4,
           status = ?5,
           sprint_name = ?6,
           last_fetched = datetime('now')",
    )
    .bind(id)
    .bind(summary)
    .bind(project_key)
    .bind(project_name)
    .bind(status)
    .bind(sprint_name)
    .execute(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;
    Ok(())
}

pub async fn get_task(pool: &SqlitePool, task_id: &str) -> Result<Option<TaskRow>, String> {
    sqlx::query_as::<_, TaskRow>(
        "SELECT id, summary, project_key, project_name, status, sprint_name, pinned, last_fetched
         FROM tasks WHERE id = ?1",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))
}

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, String> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    Ok(row.map(|r| r.0))
}

pub async fn load_timer_session(pool: &SqlitePool) -> Result<Option<TimerSession>, String> {
    let raw = get_setting(pool, TIMER_SESSION_KEY).await?;
    let Some(raw) = raw else { return Ok(None) };
    if raw.is_empty() { return Ok(None); }
    serde_json::from_str::<TimerSession>(&raw)
        .map(Some)
        .map_err(|e| format!("Failed to parse timer session: {}", e))
}

pub async fn update_entry(
    pool: &SqlitePool,
    id: i64,
    adjusted_secs: Option<i64>,
    description: Option<&str>,
) -> Result<(), String> {
    sqlx::query("UPDATE time_entries SET adjusted_secs = ?2, description = ?3 WHERE id = ?1")
        .bind(id)
        .bind(adjusted_secs)
        .bind(description)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    Ok(())
}

pub async fn mark_entry_synced(
    pool: &SqlitePool,
    id: i64,
    worklog_id: &str,
) -> Result<(), String> {
    sqlx::query("UPDATE time_entries SET synced_to_jira = 1, jira_worklog_id = ?2 WHERE id = ?1")
        .bind(id)
        .bind(worklog_id)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    Ok(())
}

pub async fn get_entry(pool: &SqlitePool, id: i64) -> Result<Option<EntryRow>, String> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT id, task_id, start_time, end_time, duration_secs, adjusted_secs, description, synced_to_jira, jira_worklog_id
         FROM time_entries WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))
}

pub async fn get_overlapping_entries(
    pool: &SqlitePool,
    start_time: &str,
    end_time: &str,
) -> Result<Vec<EntryRow>, String> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT id, task_id, start_time, end_time, duration_secs, adjusted_secs, description, synced_to_jira, jira_worklog_id
         FROM time_entries
         WHERE COALESCE(end_time, datetime('now')) > ?1
           AND start_time < ?2
         ORDER BY start_time ASC",
    )
    .bind(start_time)
    .bind(end_time)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))
}

pub async fn create_manual_entry(
    pool: &SqlitePool,
    task_id: &str,
    start_time: &str,
    end_time: &str,
    duration_secs: i64,
    description: Option<&str>,
) -> Result<i64, String> {
    let result = sqlx::query(
        "INSERT INTO time_entries (task_id, start_time, end_time, duration_secs, description, synced_to_jira)
         VALUES (?1, ?2, ?3, ?4, ?5, 0)",
    )
    .bind(task_id)
    .bind(start_time)
    .bind(end_time)
    .bind(duration_secs)
    .bind(description)
    .execute(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;
    Ok(result.last_insert_rowid())
}
