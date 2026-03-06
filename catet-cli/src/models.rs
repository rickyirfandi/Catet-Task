use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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

impl EntryRow {
    /// Returns adjusted_secs if set, otherwise duration_secs.
    pub fn effective_secs(&self) -> i64 {
        self.adjusted_secs
            .or(self.duration_secs)
            .unwrap_or(0)
    }

    pub fn is_running(&self) -> bool {
        self.end_time.is_none()
    }
}

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

/// Aggregated entries per task (for grouped views).
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatedTask {
    pub task_id: String,
    pub summary: String,
    pub entry_ids: Vec<i64>,
    pub total_secs: i64,
    pub is_running: bool,
    pub is_synced: bool,
    pub latest_start: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CliTimerState {
    pub status: String,        // "idle" | "running" | "paused"
    pub task_id: Option<String>,
    pub elapsed_secs: u64,
}

/// Stored in settings as JSON under "timer_session_v1".
/// Mirrors PersistedTimerState in src-tauri/src/timer/engine.rs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSession {
    pub status: String,            // "running" | "paused" | "idle"
    pub task_id: String,
    pub accumulated_secs: u64,
    pub started_at_utc: Option<String>,
}
