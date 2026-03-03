use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

/// Initialize the database: create tables if they don't exist.
pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // raw_sql supports multiple semicolon-separated statements (sqlx::query does NOT)
    sqlx::raw_sql(include_str!("../../migrations/001_init.sql"))
        .execute(pool)
        .await?;
    Ok(())
}

// ── User queries ──

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRow {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub jira_domain: String,
    pub auth_method: Option<String>,
}

pub async fn insert_user(pool: &SqlitePool, user: &UserRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO users (id, email, display_name, avatar_url, jira_domain, auth_method) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
    .bind(&user.id)
    .bind(&user.email)
    .bind(&user.display_name)
    .bind(&user.avatar_url)
    .bind(&user.jira_domain)
    .bind(&user.auth_method)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_users(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users").execute(pool).await?;
    Ok(())
}

// ── Task queries ──

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskRow {
    pub id: String,
    pub summary: String,
    pub project_key: Option<String>,
    pub project_name: Option<String>,
    pub status: Option<String>,
    pub sprint_name: Option<String>,
    pub pinned: bool,
    pub last_fetched: Option<String>,
}

pub async fn upsert_task(
    pool: &SqlitePool,
    id: &str,
    summary: &str,
    project_key: &str,
    project_name: &str,
    status: &str,
    sprint_name: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tasks (id, summary, project_key, project_name, status, sprint_name, last_fetched)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET summary=?2, project_key=?3, project_name=?4, status=?5, sprint_name=?6, last_fetched=datetime('now')"
    )
    .bind(id)
    .bind(summary)
    .bind(project_key)
    .bind(project_name)
    .bind(status)
    .bind(sprint_name)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_all_tasks(pool: &SqlitePool) -> Result<Vec<TaskRow>, sqlx::Error> {
    sqlx::query_as::<_, TaskRow>(
        "SELECT id, summary, project_key, project_name, status, sprint_name, pinned, last_fetched FROM tasks ORDER BY pinned DESC, last_fetched DESC"
    )
    .fetch_all(pool)
    .await
}

pub async fn search_tasks(pool: &SqlitePool, query: &str) -> Result<Vec<TaskRow>, sqlx::Error> {
    let like = format!("%{}%", query);
    sqlx::query_as::<_, TaskRow>(
        "SELECT id, summary, project_key, project_name, status, sprint_name, pinned, last_fetched FROM tasks WHERE id LIKE ?1 OR summary LIKE ?1 ORDER BY pinned DESC"
    )
    .bind(&like)
    .fetch_all(pool)
    .await
}

pub async fn pin_task(pool: &SqlitePool, task_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tasks SET pinned = 1 WHERE id = ?1")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn unpin_task(pool: &SqlitePool, task_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tasks SET pinned = 0 WHERE id = ?1")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Time Entry queries ──

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EntryRow {
    pub id: i64,
    pub task_id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_secs: Option<i64>,
    pub adjusted_secs: Option<i64>,
    pub description: Option<String>,
    pub synced_to_jira: bool,
    pub jira_worklog_id: Option<String>,
}

pub async fn create_entry(pool: &SqlitePool, task_id: &str) -> Result<i64, sqlx::Error> {
    let result =
        sqlx::query("INSERT INTO time_entries (task_id, start_time) VALUES (?1, datetime('now'))")
            .bind(task_id)
            .execute(pool)
            .await?;
    Ok(result.last_insert_rowid())
}

pub async fn finalize_entry(
    pool: &SqlitePool,
    id: i64,
    duration_secs: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE time_entries SET end_time = datetime('now'), duration_secs = ?2 WHERE id = ?1",
    )
    .bind(id)
    .bind(duration_secs)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_running_entry_for_task(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<Option<EntryRow>, sqlx::Error> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT id, task_id, start_time, end_time, duration_secs, adjusted_secs, description, synced_to_jira, jira_worklog_id FROM time_entries WHERE task_id = ?1 AND end_time IS NULL ORDER BY start_time DESC LIMIT 1"
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_open_entries(pool: &SqlitePool) -> Result<Vec<EntryRow>, sqlx::Error> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT id, task_id, start_time, end_time, duration_secs, adjusted_secs, description, synced_to_jira, jira_worklog_id
         FROM time_entries
         WHERE end_time IS NULL
         ORDER BY start_time DESC"
    )
    .fetch_all(pool)
    .await
}

pub async fn get_entries_today(pool: &SqlitePool) -> Result<Vec<EntryRow>, sqlx::Error> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT id, task_id, start_time, end_time, duration_secs, adjusted_secs, description, synced_to_jira, jira_worklog_id
         FROM time_entries
         WHERE date(start_time, 'localtime') = date('now', 'localtime')
         ORDER BY start_time DESC"
    )
    .fetch_all(pool)
    .await
}

pub async fn get_entries_range(
    pool: &SqlitePool,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<EntryRow>, sqlx::Error> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT id, task_id, start_time, end_time, duration_secs, adjusted_secs, description, synced_to_jira, jira_worklog_id
         FROM time_entries
         WHERE date(start_time, 'localtime') BETWEEN date(?1) AND date(?2)
         ORDER BY start_time DESC",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
}

pub async fn update_entry(
    pool: &SqlitePool,
    id: i64,
    adjusted_secs: Option<i64>,
    description: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE time_entries SET adjusted_secs = ?2, description = ?3 WHERE id = ?1")
        .bind(id)
        .bind(adjusted_secs)
        .bind(description)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_entry_synced(
    pool: &SqlitePool,
    id: i64,
    worklog_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE time_entries SET synced_to_jira = 1, jira_worklog_id = ?2 WHERE id = ?1")
        .bind(id)
        .bind(worklog_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Orphaned entry recovery ──

/// Finalize any entries with NULL end_time (crash recovery).
/// Sets end_time to now and computes duration_secs from start_time.
pub async fn finalize_orphaned_entries(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE time_entries SET end_time = datetime('now'), \
         duration_secs = CAST((julianday('now') - julianday(start_time)) * 86400 AS INTEGER) \
         WHERE end_time IS NULL",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn finalize_open_entries_except(
    pool: &SqlitePool,
    keep_id: Option<i64>,
) -> Result<u64, sqlx::Error> {
    let result = if let Some(id) = keep_id {
        sqlx::query(
            "UPDATE time_entries
             SET end_time = datetime('now'),
                 duration_secs = CAST((julianday('now') - julianday(start_time)) * 86400 AS INTEGER)
             WHERE end_time IS NULL AND id != ?1",
        )
        .bind(id)
        .execute(pool)
        .await?
    } else {
        sqlx::query(
            "UPDATE time_entries
             SET end_time = datetime('now'),
                 duration_secs = CAST((julianday('now') - julianday(start_time)) * 86400 AS INTEGER)
             WHERE end_time IS NULL",
        )
        .execute(pool)
        .await?
    };

    Ok(result.rows_affected())
}

pub async fn count_unlogged_today(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM time_entries
         WHERE date(start_time, 'localtime') = date('now', 'localtime')
         AND synced_to_jira = 0",
    )
    .fetch_one(pool)
    .await
}

pub async fn delete_all_entries(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM time_entries")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// ── Settings queries ──

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_setting(pool: &SqlitePool, key: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM settings WHERE key = ?1")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}
