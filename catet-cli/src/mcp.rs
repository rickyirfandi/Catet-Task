/// MCP (Model Context Protocol) stdio server.
/// Exposes all CLI commands as tools callable by Claude Desktop.
/// Run via: catet-cli serve-mcp
///
/// Protocol: JSON-RPC 2.0 over stdin/stdout (one JSON object per line).
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Write};

use crate::{credentials, db, format, jira};
use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone, Utc};

const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-11-05", "2025-06-18", "2024-11-05"];
const DEFAULT_PROTOCOL_VERSION: &str = "2025-11-25";
const MAX_MANUAL_DURATION_SECS: i64 = 16 * 3600;
const FUTURE_END_TOLERANCE_SECS: i64 = 5 * 60;

// ── JSON-RPC types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

fn ok(id: Value, result: Value) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

fn err(id: Value, code: i32, message: impl Into<String>) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(RpcError {
            code,
            message: message.into(),
        }),
    }
}

fn write_response(resp: &RpcResponse) {
    let line = serde_json::to_string(resp).unwrap_or_default();
    println!("{}", line);
    let _ = io::stdout().flush();
}

fn negotiate_protocol(requested: Option<&str>) -> Result<&'static str, String> {
    match requested {
        None => Ok(DEFAULT_PROTOCOL_VERSION),
        Some(version) => SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .copied()
            .find(|v| *v == version)
            .ok_or_else(|| {
                format!(
                    "Unsupported protocol version '{}'. Supported versions: {}",
                    version,
                    SUPPORTED_PROTOCOL_VERSIONS.join(", ")
                )
            }),
    }
}

fn parse_request(value: Value) -> Result<RpcRequest, RpcResponse> {
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    serde_json::from_value::<RpcRequest>(value)
        .map_err(|e| err(id, -32600, format!("Invalid Request: {}", e)))
}

fn parse_ymd(label: &str, raw: &str) -> Result<chrono::NaiveDate, String> {
    chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|_| format!("{} must be in YYYY-MM-DD format", label))
}

fn parse_local_datetime(raw: &str) -> Result<DateTime<Local>, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("datetime cannot be empty".to_string());
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Local));
    }

    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            if let Some(local_dt) = Local.from_local_datetime(&naive).single() {
                return Ok(local_dt);
            }
            return Err(format!(
                "datetime '{}' is invalid in local timezone (DST transition)",
                s
            ));
        }
    }

    Err(format!(
        "datetime '{}' must be 'YYYY-MM-DD HH:MM[:SS]' or RFC3339",
        s
    ))
}

fn to_utc_storage_string(dt: DateTime<Local>) -> String {
    dt.with_timezone(&Utc)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn utc_storage_to_local_display(raw: &str) -> String {
    NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| raw.to_string())
}

fn resolve_manual_window(args: &Value) -> Result<(DateTime<Local>, DateTime<Local>, i64), String> {
    let started_at = args
        .get("started_at")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let ended_at = args
        .get("ended_at")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let duration_minutes = args.get("duration_minutes").and_then(|v| v.as_i64());

    let (start_local, end_local) = if let Some(minutes) = duration_minutes {
        if minutes <= 0 {
            return Err("duration_minutes must be > 0".to_string());
        }
        let dur = Duration::minutes(minutes);
        match (started_at, ended_at) {
            (Some(_), Some(_)) => {
                return Err(
                    "When duration_minutes is provided, use only started_at or ended_at (not both)"
                        .to_string(),
                )
            }
            (Some(start_raw), None) => {
                let start = parse_local_datetime(start_raw)?;
                (start, start + dur)
            }
            (None, Some(end_raw)) => {
                let end = parse_local_datetime(end_raw)?;
                (end - dur, end)
            }
            (None, None) => {
                let end = Local::now();
                (end - dur, end)
            }
        }
    } else {
        let start_raw = started_at.ok_or("started_at required when duration_minutes is not provided")?;
        let end_raw = ended_at.ok_or("ended_at required when duration_minutes is not provided")?;
        (parse_local_datetime(start_raw)?, parse_local_datetime(end_raw)?)
    };

    if end_local <= start_local {
        return Err("ended_at must be after started_at".to_string());
    }

    let now = Local::now();
    if end_local > now + Duration::seconds(FUTURE_END_TOLERANCE_SECS) {
        return Err("ended_at is too far in the future".to_string());
    }

    let duration_secs = (end_local - start_local).num_seconds();
    if duration_secs <= 0 {
        return Err("duration must be > 0 seconds".to_string());
    }

    Ok((start_local, end_local, duration_secs))
}

fn evaluate_submission_entry(
    entry: &crate::models::EntryRow,
    include_running: bool,
) -> (&'static str, Option<String>, Option<i64>) {
    if entry.synced_to_jira {
        return (
            "skipped",
            Some("already synced to Jira".to_string()),
            None,
        );
    }
    if entry.is_running() && !include_running {
        return (
            "skipped",
            Some("timer still running".to_string()),
            None,
        );
    }

    let effective = entry.effective_secs();
    if effective <= 0 {
        return (
            "blocked",
            Some("entry has zero duration".to_string()),
            None,
        );
    }

    ("ready", None, Some(effective.max(60)))
}

async fn collect_entries_for_submission(
    pool_rw: &SqlitePool,
    entry_ids: Option<Vec<i64>>,
    include_running: bool,
) -> Result<(Vec<crate::models::EntryRow>, Vec<i64>), String> {
    if let Some(ids) = entry_ids {
        let mut entries = Vec::new();
        let mut missing = Vec::new();
        for id in ids {
            if let Some(entry) = db::get_entry(pool_rw, id).await? {
                entries.push(entry);
            } else {
                missing.push(id);
            }
        }
        return Ok((entries, missing));
    }

    let entries = db::get_entries_today(pool_rw)
        .await?
        .into_iter()
        .filter(|e| !e.synced_to_jira && (include_running || e.end_time.is_some()))
        .collect();
    Ok((entries, Vec::new()))
}

// ── Tool definitions ──────────────────────────────────────────────────────────

fn tool_list() -> Value {
    json!({
        "tools": [
            {
                "name": "catet_status",
                "description": "Get the current timer status (running task, elapsed time).",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "catet_today",
                "description": "Get all time entries tracked today, grouped by task. Returns totals and sync status.",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "catet_week",
                "description": "Get a daily summary of time tracked this week (Mon-today).",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "catet_range",
                "description": "Get entries and totals for a date range (inclusive).",
                "inputSchema": {
                    "type": "object",
                    "required": ["from", "to"],
                    "properties": {
                        "from": {"type": "string", "description": "Start date (YYYY-MM-DD)."},
                        "to": {"type": "string", "description": "End date (YYYY-MM-DD)."},
                        "unlogged_only": {"type": "boolean", "description": "Only include entries not yet submitted to Jira."}
                    }
                }
            },
            {
                "name": "catet_entries",
                "description": "List individual time entries with optional filters.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "unlogged_only": {"type": "boolean", "description": "Only show entries not yet submitted to Jira."},
                        "date": {"type": "string", "description": "Filter by date (YYYY-MM-DD). Defaults to today."}
                    }
                }
            },
            {
                "name": "catet_set_comment",
                "description": "Set the worklog comment on a time entry (by entry ID).",
                "inputSchema": {
                    "type": "object",
                    "required": ["entry_id", "comment"],
                    "properties": {
                        "entry_id": {"type": "integer", "description": "Entry ID from catet_entries."},
                        "comment": {"type": "string", "description": "Worklog comment to set."}
                    }
                }
            },
            {
                "name": "catet_set_duration",
                "description": "Override the duration of a time entry.",
                "inputSchema": {
                    "type": "object",
                    "required": ["entry_id", "minutes"],
                    "properties": {
                        "entry_id": {"type": "integer"},
                        "minutes": {"type": "integer", "minimum": 1, "description": "New duration in minutes (must be > 0)."}
                    }
                }
            },
            {
                "name": "catet_submit",
                "description": "Submit time entries as worklogs to Jira. Skips running entries by default.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entry_ids": {"type": "array", "items": {"type": "integer"}, "description": "Specific entry IDs to submit. Omit to submit all unlogged stopped entries."},
                        "include_running": {"type": "boolean", "description": "Also submit the currently running entry (default false)."}
                    }
                }
            },
            {
                "name": "catet_submit_preview",
                "description": "Preview which entries would be submitted to Jira (no writes).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entry_ids": {"type": "array", "items": {"type": "integer"}, "description": "Specific entry IDs to preview. Omit to preview all unlogged stopped entries for today."},
                        "include_running": {"type": "boolean", "description": "Include currently running entry in preview (default false)."}
                    }
                }
            },
            {
                "name": "catet_tasks",
                "description": "List all cached Jira tasks.",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "catet_search_tasks",
                "description": "Search tasks by key/summary from local cache and Jira (if logged in).",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": {"type": "string", "description": "Task key or text query."},
                        "project_key": {"type": "string", "description": "Optional Jira project key filter (e.g. PROJ)."},
                        "limit": {"type": "integer", "minimum": 1, "maximum": 50, "description": "Max tasks to return (default 20)."}
                    }
                }
            },
            {
                "name": "catet_plan_manual_log",
                "description": "Plan a manual time log (forgotten timer) without writing to DB.",
                "inputSchema": {
                    "type": "object",
                    "required": ["task_id"],
                    "properties": {
                        "task_id": {"type": "string", "description": "Task key (e.g. MTI-30)."},
                        "duration_minutes": {"type": "integer", "minimum": 1, "description": "Duration in minutes."},
                        "started_at": {"type": "string", "description": "Local start time: YYYY-MM-DD HH:MM[:SS] or RFC3339."},
                        "ended_at": {"type": "string", "description": "Local end time: YYYY-MM-DD HH:MM[:SS] or RFC3339. Defaults to now when duration_minutes is used."},
                        "description": {"type": "string", "description": "Optional worklog comment."}
                    }
                }
            },
            {
                "name": "catet_add_manual_log",
                "description": "Create a manual time entry in local DB (requires confirm=true).",
                "inputSchema": {
                    "type": "object",
                    "required": ["task_id", "confirm"],
                    "properties": {
                        "task_id": {"type": "string", "description": "Task key (e.g. MTI-30)."},
                        "duration_minutes": {"type": "integer", "minimum": 1, "description": "Duration in minutes."},
                        "started_at": {"type": "string", "description": "Local start time: YYYY-MM-DD HH:MM[:SS] or RFC3339."},
                        "ended_at": {"type": "string", "description": "Local end time: YYYY-MM-DD HH:MM[:SS] or RFC3339."},
                        "description": {"type": "string", "description": "Optional worklog comment."},
                        "allow_overlap": {"type": "boolean", "description": "Allow overlap with existing entries (default false)."},
                        "confirm": {"type": "boolean", "description": "Must be true to create the entry."}
                    }
                }
            },
            {
                "name": "catet_report",
                "description": "Generate a standup report for yesterday and today.",
                "inputSchema": {"type": "object", "properties": {}}
            }
        ]
    })
}

// ── Tool dispatch ─────────────────────────────────────────────────────────────

async fn handle_tool(name: &str, args: &Value, db_path: &std::path::Path) -> Result<Value, String> {
    let pool = db::open_pool(db_path).await?;

    match name {
        "catet_status" => {
            let session = db::load_timer_session(&pool).await?;
            match session {
                None => Ok(json!({"status": "idle", "task_id": null, "elapsed_secs": 0})),
                Some(s) => {
                    let state = format::session_to_state(&s);
                    Ok(serde_json::to_value(&state).unwrap())
                }
            }
        }

        "catet_today" => {
            let entries = db::get_entries_today(&pool).await?;
            let tasks = db::get_all_tasks(&pool).await?;
            let task_map: std::collections::HashMap<String, &crate::models::TaskRow> =
                tasks.iter().map(|t| (t.id.clone(), t)).collect();
            let aggregated = format::aggregate_entries(&entries, &task_map);
            Ok(json!({
                "entries": entries,
                "aggregated": aggregated,
            }))
        }

        "catet_week" => {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let monday = {
                use chrono::{Datelike, NaiveDate};
                let d = NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap();
                let offset = d.weekday().num_days_from_monday() as i64;
                (d - chrono::Duration::days(offset))
                    .format("%Y-%m-%d")
                    .to_string()
            };
            let entries = db::get_entries_range(&pool, &monday, &today).await?;
            // Group by date
            let mut by_date: indexmap::IndexMap<String, i64> = indexmap::IndexMap::new();
            for e in &entries {
                let date = e.start_time.get(..10).unwrap_or(&e.start_time).to_string();
                *by_date.entry(date).or_insert(0) += e.effective_secs();
            }
            let days: Vec<Value> = by_date
                .iter()
                .map(|(date, secs)| json!({"date": date, "total_secs": secs, "duration": format::fmt_duration(*secs)}))
                .collect();
            let grand_total: i64 = by_date.values().sum();
            Ok(
                json!({"days": days, "total_secs": grand_total, "total": format::fmt_duration(grand_total)}),
            )
        }

        "catet_range" => {
            let from = args["from"].as_str().ok_or("from required (YYYY-MM-DD)")?;
            let to = args["to"].as_str().ok_or("to required (YYYY-MM-DD)")?;
            let from_date = parse_ymd("from", from)?;
            let to_date = parse_ymd("to", to)?;
            if from_date > to_date {
                return Err("from must be <= to".to_string());
            }

            let unlogged_only = args
                .get("unlogged_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut entries = db::get_entries_range(&pool, from, to).await?;
            if unlogged_only {
                entries.retain(|e| !e.synced_to_jira && e.end_time.is_some());
            }

            let tasks = db::get_all_tasks(&pool).await?;
            let task_map: HashMap<String, &crate::models::TaskRow> =
                tasks.iter().map(|t| (t.id.clone(), t)).collect();
            let aggregated = format::aggregate_entries(&entries, &task_map);
            let total_secs: i64 = entries.iter().map(|e| e.effective_secs()).sum();

            Ok(json!({
                "from": from,
                "to": to,
                "entries": entries,
                "aggregated": aggregated,
                "total_secs": total_secs,
                "total": format::fmt_duration(total_secs)
            }))
        }

        "catet_entries" => {
            let unlogged_only = args
                .get("unlogged_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let date = args
                .get("date")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let entries = if let Some(d) = date {
                db::get_entries_range(&pool, &d, &d).await?
            } else {
                db::get_entries_today(&pool).await?
            };
            let filtered: Vec<_> = entries
                .iter()
                .filter(|e| {
                    if unlogged_only {
                        !e.synced_to_jira && e.end_time.is_some()
                    } else {
                        true
                    }
                })
                .collect();
            Ok(serde_json::to_value(filtered).unwrap())
        }

        "catet_set_comment" => {
            let id = args["entry_id"].as_i64().ok_or("entry_id required")?;
            let comment = args["comment"].as_str().ok_or("comment required")?;
            let pool_rw = db::open_pool_rw(db_path).await?;
            let entry = db::get_entry(&pool_rw, id)
                .await?
                .ok_or(format!("Entry {} not found", id))?;
            db::update_entry(&pool_rw, id, entry.adjusted_secs, Some(comment)).await?;
            Ok(json!({"ok": true, "entry_id": id, "comment": comment}))
        }

        "catet_set_duration" => {
            let id = args["entry_id"].as_i64().ok_or("entry_id required")?;
            let minutes = args["minutes"].as_i64().ok_or("minutes required")?;
            if minutes <= 0 {
                return Err("minutes must be > 0".to_string());
            }
            let pool_rw = db::open_pool_rw(db_path).await?;
            let entry = db::get_entry(&pool_rw, id)
                .await?
                .ok_or(format!("Entry {} not found", id))?;
            let adjusted_secs = minutes * 60;
            db::update_entry(
                &pool_rw,
                id,
                Some(adjusted_secs),
                entry.description.as_deref(),
            )
            .await?;
            Ok(json!({"ok": true, "entry_id": id, "adjusted_secs": adjusted_secs}))
        }

        "catet_submit_preview" => {
            let include_running = args
                .get("include_running")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let entry_ids: Option<Vec<i64>> = args
                .get("entry_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let pool_rw = db::open_pool_rw(db_path).await?;
            let (entries, missing_ids) =
                collect_entries_for_submission(&pool_rw, entry_ids, include_running).await?;

            let mut candidates = Vec::new();
            let mut ready = 0usize;
            let mut skipped = 0usize;
            let mut blocked = 0usize;
            let mut total_ready_secs = 0i64;

            for entry in entries {
                let (status, reason, seconds_to_submit) =
                    evaluate_submission_entry(&entry, include_running);
                match status {
                    "ready" => {
                        ready += 1;
                        total_ready_secs += seconds_to_submit.unwrap_or(0);
                    }
                    "skipped" => skipped += 1,
                    _ => blocked += 1,
                }
                candidates.push(json!({
                    "entry_id": entry.id,
                    "task_id": entry.task_id,
                    "status": status,
                    "reason": reason,
                    "is_running": entry.is_running(),
                    "synced_to_jira": entry.synced_to_jira,
                    "start_time": entry.start_time,
                    "end_time": entry.end_time,
                    "effective_secs": entry.effective_secs(),
                    "seconds_to_submit": seconds_to_submit,
                    "comment": entry.description,
                }));
            }

            for missing_id in missing_ids {
                blocked += 1;
                candidates.push(json!({
                    "entry_id": missing_id,
                    "task_id": null,
                    "status": "blocked",
                    "reason": "entry not found",
                    "is_running": false,
                    "synced_to_jira": false,
                    "seconds_to_submit": null
                }));
            }

            Ok(json!({
                "summary": {
                    "total": candidates.len(),
                    "ready": ready,
                    "skipped": skipped,
                    "blocked": blocked,
                    "total_ready_secs": total_ready_secs,
                    "total_ready": format::fmt_duration(total_ready_secs),
                },
                "candidates": candidates
            }))
        }

        "catet_submit" => {
            let include_running = args
                .get("include_running")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let entry_ids: Option<Vec<i64>> = args
                .get("entry_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let pool_rw = db::open_pool_rw(db_path).await?;
            let (domain, email, token) = credentials::load_credentials(&pool_rw).await?;
            let client = jira::JiraClient::new(&domain, &email, &token);

            let (entries, missing_ids) =
                collect_entries_for_submission(&pool_rw, entry_ids, include_running).await?;

            let mut results = vec![];
            for missing_id in missing_ids {
                results.push(json!({
                    "entry_id": missing_id,
                    "task_id": null,
                    "status": "error",
                    "error": "entry not found"
                }));
            }

            for entry in &entries {
                let (status, reason, seconds_to_submit) =
                    evaluate_submission_entry(entry, include_running);
                match status {
                    "ready" => {}
                    "skipped" => {
                        results.push(json!({
                            "entry_id": entry.id,
                            "task_id": entry.task_id,
                            "status": "skipped",
                            "reason": reason.unwrap_or_else(|| "skipped".to_string())
                        }));
                        continue;
                    }
                    _ => {
                        results.push(json!({
                            "entry_id": entry.id,
                            "task_id": entry.task_id,
                            "status": "error",
                            "error": reason.unwrap_or_else(|| "entry is not eligible for submission".to_string())
                        }));
                        continue;
                    }
                }

                let secs = seconds_to_submit.unwrap_or(60) as u64;
                match client
                    .add_worklog(
                        &entry.task_id,
                        secs,
                        &entry.start_time,
                        entry.description.as_deref().unwrap_or(""),
                    )
                    .await
                {
                    Ok(wl) => {
                        let _ = db::mark_entry_synced(&pool_rw, entry.id, &wl.id).await;
                        results.push(json!({"entry_id": entry.id, "task_id": entry.task_id, "status": "logged", "worklog_id": wl.id}));
                    }
                    Err(e) => {
                        results.push(json!({"entry_id": entry.id, "task_id": entry.task_id, "status": "error", "error": e}));
                    }
                }
            }
            Ok(json!({"results": results}))
        }

        "catet_tasks" => {
            let tasks = db::get_all_tasks(&pool).await?;
            Ok(serde_json::to_value(tasks).unwrap())
        }

        "catet_search_tasks" => {
            let query = args["query"]
                .as_str()
                .ok_or("query required")?
                .trim()
                .to_string();
            if query.is_empty() {
                return Ok(json!({"tasks": []}));
            }

            let project_key = args
                .get("project_key")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let limit = args
                .get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(20)
                .clamp(1, 50);
            let limit_u32 = limit as u32;

            let local_rows = db::search_tasks(&pool, &query, project_key.as_deref(), limit).await?;
            let pinned_lookup: HashMap<String, bool> =
                local_rows.iter().map(|r| (r.id.clone(), r.pinned)).collect();

            let mut tasks = Vec::new();
            let mut seen = HashSet::new();
            let mut warnings = Vec::new();
            let pool_rw = db::open_pool_rw(db_path).await.ok();

            if let Ok((domain, email, token)) = credentials::load_credentials(&pool).await {
                let client = jira::JiraClient::new(&domain, &email, &token);
                let sanitized = query.replace('"', "");
                let is_key_like = sanitized.contains('-') && sanitized.chars().any(|c| c.is_ascii_digit());
                let text_clause = if is_key_like {
                    format!(r#"key = "{}""#, sanitized)
                } else {
                    format!(r#"text ~ "{}""#, sanitized)
                };
                let scoped_clause = if let Some(project) = &project_key {
                    format!(r#"project = "{}" AND {}"#, project.replace('"', ""), text_clause)
                } else {
                    text_clause
                };

                let jql_sprint =
                    format!(r#"{} AND sprint in openSprints() ORDER BY updated DESC"#, scoped_clause);
                let jql_broad = format!(r#"{} ORDER BY updated DESC"#, scoped_clause);
                let (sprint_result, broad_result) = tokio::join!(
                    client.search_issues_limited(&jql_sprint, limit_u32),
                    client.search_issues_limited(&jql_broad, limit_u32),
                );

                match sprint_result {
                    Ok(result) => {
                        for issue in result.issues {
                            if !seen.insert(issue.key.clone()) {
                                continue;
                            }
                            let project_key = issue
                                .fields
                                .project
                                .as_ref()
                                .map(|p| p.key.clone())
                                .unwrap_or_default();
                            let project_name = issue
                                .fields
                                .project
                                .as_ref()
                                .map(|p| p.name.clone())
                                .unwrap_or_default();
                            let status = issue
                                .fields
                                .status
                                .as_ref()
                                .map(|s| s.name.clone())
                                .unwrap_or_default();
                            let parent_key = issue.fields.parent.as_ref().map(|p| p.key.clone());
                            let parent_summary = issue
                                .fields
                                .parent
                                .as_ref()
                                .map(|p| p.fields.summary.clone());
                            let pinned = pinned_lookup.get(&issue.key).copied().unwrap_or(false);

                            if let Some(pool_rw) = &pool_rw {
                                let _ = db::upsert_task(
                                    pool_rw,
                                    &issue.key,
                                    &issue.fields.summary,
                                    &project_key,
                                    &project_name,
                                    &status,
                                    None,
                                )
                                .await;
                            }

                            tasks.push(json!({
                                "id": issue.key,
                                "summary": issue.fields.summary,
                                "project_key": project_key,
                                "project_name": project_name,
                                "status": status,
                                "sprint_name": null,
                                "pinned": pinned,
                                "in_current_sprint": true,
                                "parent_key": parent_key,
                                "parent_summary": parent_summary,
                                "source": "jira"
                            }));
                        }
                    }
                    Err(e) => warnings.push(format!("Sprint search failed: {}", e)),
                }

                match broad_result {
                    Ok(result) => {
                        for issue in result.issues {
                            if !seen.insert(issue.key.clone()) {
                                continue;
                            }
                            let project_key = issue
                                .fields
                                .project
                                .as_ref()
                                .map(|p| p.key.clone())
                                .unwrap_or_default();
                            let project_name = issue
                                .fields
                                .project
                                .as_ref()
                                .map(|p| p.name.clone())
                                .unwrap_or_default();
                            let status = issue
                                .fields
                                .status
                                .as_ref()
                                .map(|s| s.name.clone())
                                .unwrap_or_default();
                            let parent_key = issue.fields.parent.as_ref().map(|p| p.key.clone());
                            let parent_summary = issue
                                .fields
                                .parent
                                .as_ref()
                                .map(|p| p.fields.summary.clone());
                            let pinned = pinned_lookup.get(&issue.key).copied().unwrap_or(false);

                            if let Some(pool_rw) = &pool_rw {
                                let _ = db::upsert_task(
                                    pool_rw,
                                    &issue.key,
                                    &issue.fields.summary,
                                    &project_key,
                                    &project_name,
                                    &status,
                                    None,
                                )
                                .await;
                            }

                            tasks.push(json!({
                                "id": issue.key,
                                "summary": issue.fields.summary,
                                "project_key": project_key,
                                "project_name": project_name,
                                "status": status,
                                "sprint_name": null,
                                "pinned": pinned,
                                "in_current_sprint": false,
                                "parent_key": parent_key,
                                "parent_summary": parent_summary,
                                "source": "jira"
                            }));
                        }
                    }
                    Err(e) => warnings.push(format!("Broad search failed: {}", e)),
                }
            } else {
                warnings.push("Jira search unavailable: not logged in".to_string());
            }

            for row in local_rows {
                if !seen.insert(row.id.clone()) {
                    continue;
                }
                tasks.push(json!({
                    "id": row.id,
                    "summary": row.summary,
                    "project_key": row.project_key.unwrap_or_default(),
                    "project_name": row.project_name.unwrap_or_default(),
                    "status": row.status.unwrap_or_default(),
                    "sprint_name": row.sprint_name,
                    "pinned": row.pinned,
                    "in_current_sprint": false,
                    "parent_key": Value::Null,
                    "parent_summary": Value::Null,
                    "source": "local"
                }));
            }

            tasks.truncate(limit as usize);

            if warnings.is_empty() {
                Ok(json!({ "tasks": tasks }))
            } else {
                Ok(json!({ "tasks": tasks, "warnings": warnings }))
            }
        }

        "catet_plan_manual_log" => {
            let task_id = args["task_id"]
                .as_str()
                .ok_or("task_id required")?
                .trim()
                .to_string();
            if task_id.is_empty() {
                return Err("task_id cannot be empty".to_string());
            }

            let task = db::get_task(&pool, &task_id).await?;
            let task = task.ok_or_else(|| {
                format!(
                    "Task '{}' not found in local cache. Use catet_search_tasks first.",
                    task_id
                )
            })?;

            let description = args
                .get("description")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string);

            let (start_local, end_local, duration_secs) = resolve_manual_window(args)?;
            let start_utc = to_utc_storage_string(start_local);
            let end_utc = to_utc_storage_string(end_local);

            let overlaps = db::get_overlapping_entries(&pool, &start_utc, &end_utc).await?;
            let overlap_items: Vec<Value> = overlaps
                .iter()
                .map(|entry| {
                    json!({
                        "entry_id": entry.id,
                        "task_id": entry.task_id,
                        "start_time_local": utc_storage_to_local_display(&entry.start_time),
                        "end_time_local": entry.end_time.as_deref().map(utc_storage_to_local_display),
                        "is_running": entry.is_running(),
                        "effective_secs": entry.effective_secs(),
                        "synced_to_jira": entry.synced_to_jira
                    })
                })
                .collect();
            let has_overlaps = !overlap_items.is_empty();

            let mut warnings = Vec::new();
            if duration_secs > MAX_MANUAL_DURATION_SECS {
                warnings.push(format!(
                    "Long duration detected ({}). Please confirm this is intentional.",
                    format::fmt_duration(duration_secs)
                ));
            }
            if start_local.date_naive() != end_local.date_naive() {
                warnings.push("Entry spans multiple dates.".to_string());
            }
            if has_overlaps {
                warnings.push(format!(
                    "Proposed range overlaps {} existing entr{}.",
                    overlap_items.len(),
                    if overlap_items.len() == 1 { "y" } else { "ies" }
                ));
            }

            Ok(json!({
                "task": {
                    "id": task.id,
                    "summary": task.summary,
                    "project_key": task.project_key,
                    "project_name": task.project_name,
                    "status": task.status
                },
                "proposal": {
                    "task_id": task_id,
                    "start_time_local": start_local.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "end_time_local": end_local.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "start_time_utc": start_utc,
                    "end_time_utc": end_utc,
                    "duration_secs": duration_secs,
                    "duration": format::fmt_duration(duration_secs),
                    "description": description,
                },
                "overlaps": overlap_items,
                "requires_allow_overlap": has_overlaps,
                "can_create": !has_overlaps,
                "warnings": warnings
            }))
        }

        "catet_add_manual_log" => {
            let confirm = args.get("confirm").and_then(|v| v.as_bool()).unwrap_or(false);
            if !confirm {
                return Err("confirm=true is required to create manual log".to_string());
            }

            let task_id = args["task_id"]
                .as_str()
                .ok_or("task_id required")?
                .trim()
                .to_string();
            if task_id.is_empty() {
                return Err("task_id cannot be empty".to_string());
            }

            let allow_overlap = args
                .get("allow_overlap")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let description = args
                .get("description")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string);

            let (start_local, end_local, duration_secs) = resolve_manual_window(args)?;
            let start_utc = to_utc_storage_string(start_local);
            let end_utc = to_utc_storage_string(end_local);

            let pool_rw = db::open_pool_rw(db_path).await?;
            let task = db::get_task(&pool_rw, &task_id).await?;
            let task = task.ok_or_else(|| {
                format!(
                    "Task '{}' not found in local cache. Use catet_search_tasks first.",
                    task_id
                )
            })?;

            let overlaps = db::get_overlapping_entries(&pool_rw, &start_utc, &end_utc).await?;
            if !allow_overlap && !overlaps.is_empty() {
                return Err(format!(
                    "Proposed range overlaps {} existing entr{}. Re-run with allow_overlap=true if intentional.",
                    overlaps.len(),
                    if overlaps.len() == 1 { "y" } else { "ies" }
                ));
            }

            let entry_id = db::create_manual_entry(
                &pool_rw,
                &task_id,
                &start_utc,
                &end_utc,
                duration_secs,
                description.as_deref(),
            )
            .await?;

            let mut warnings = Vec::new();
            if duration_secs > MAX_MANUAL_DURATION_SECS {
                warnings.push(format!(
                    "Long duration detected ({}).",
                    format::fmt_duration(duration_secs)
                ));
            }
            if !overlaps.is_empty() {
                warnings.push(format!(
                    "Entry created with {} overlap(s).",
                    overlaps.len()
                ));
            }

            Ok(json!({
                "ok": true,
                "entry_id": entry_id,
                "task": {
                    "id": task.id,
                    "summary": task.summary
                },
                "entry": {
                    "task_id": task_id,
                    "start_time_local": start_local.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "end_time_local": end_local.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "start_time_utc": start_utc,
                    "end_time_utc": end_utc,
                    "duration_secs": duration_secs,
                    "duration": format::fmt_duration(duration_secs),
                    "description": description,
                },
                "overlap_count": overlaps.len(),
                "warnings": warnings
            }))
        }

        "catet_report" => {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let yesterday = (chrono::Local::now() - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string();
            let today_entries = db::get_entries_today(&pool).await?;
            let yesterday_entries = db::get_entries_range(&pool, &yesterday, &yesterday).await?;
            let tasks = db::get_all_tasks(&pool).await?;
            let task_map: std::collections::HashMap<String, &crate::models::TaskRow> =
                tasks.iter().map(|t| (t.id.clone(), t)).collect();

            fn summarize(
                entries: &[crate::models::EntryRow],
                tasks: &std::collections::HashMap<String, &crate::models::TaskRow>,
            ) -> Vec<Value> {
                let mut by_task: indexmap::IndexMap<String, i64> = indexmap::IndexMap::new();
                for e in entries {
                    *by_task.entry(e.task_id.clone()).or_insert(0) += e.effective_secs();
                }
                by_task.iter().map(|(task_id, secs)| {
                    let summary = tasks.get(task_id.as_str()).map(|t| t.summary.as_str()).unwrap_or("?");
                    json!({"task_id": task_id, "duration": format::fmt_duration(*secs), "total_secs": secs, "summary": summary})
                }).collect()
            }

            Ok(json!({
                "today": summarize(&today_entries, &task_map),
                "yesterday": summarize(&yesterday_entries, &task_map),
                "today_date": today,
                "yesterday_date": yesterday,
            }))
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}

// ── Main serve loop ───────────────────────────────────────────────────────────

pub async fn serve(db_path: std::path::PathBuf) {
    let stdin = io::stdin();
    let mut initialized = false;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        let parsed: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let resp = err(Value::Null, -32700, format!("Parse error: {}", e));
                write_response(&resp);
                continue;
            }
        };

        let mut requests = Vec::new();
        if let Some(arr) = parsed.as_array() {
            if arr.is_empty() {
                write_response(&err(
                    Value::Null,
                    -32600,
                    "Invalid Request: empty batch is not allowed",
                ));
                continue;
            }
            requests.extend(arr.iter().cloned());
        } else {
            requests.push(parsed);
        }

        for raw_request in requests {
            let req = match parse_request(raw_request) {
                Ok(r) => r,
                Err(resp) => {
                    write_response(&resp);
                    continue;
                }
            };

            // Notifications have no id and must not receive responses.
            let Some(id) = req.id.clone() else {
                continue;
            };

            if req.jsonrpc != "2.0" {
                write_response(&err(id, -32600, "Invalid Request: jsonrpc must be '2.0'"));
                continue;
            }

            let resp = match req.method.as_str() {
                "initialize" => {
                    let requested = req.params.get("protocolVersion").and_then(|v| v.as_str());
                    match negotiate_protocol(requested) {
                        Ok(negotiated) => {
                            initialized = true;
                            ok(
                                id,
                                json!({
                                    "protocolVersion": negotiated,
                                    "capabilities": {"tools": {}},
                                    "serverInfo": {"name": "catet-task", "version": "0.1.0"}
                                }),
                            )
                        }
                        Err(e) => err(id, -32602, e),
                    }
                }
                "ping" => ok(id, json!({})),
                _ if !initialized => err(id, -32002, "Server not initialized"),
                "tools/list" => ok(id, tool_list()),
                "tools/call" => {
                    let tool_name = req
                        .params
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let args = req.params.get("arguments").unwrap_or(&Value::Null);
                    match handle_tool(tool_name, args, &db_path).await {
                        Ok(result) => ok(
                            id,
                            json!({
                                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
                            }),
                        ),
                        Err(e) => ok(
                            id,
                            json!({
                                "content": [{"type": "text", "text": format!("Error: {}", e)}],
                                "isError": true
                            }),
                        ),
                    }
                }
                "notifications/initialized" => continue,
                other => err(id, -32601, format!("Method not found: {}", other)),
            };

            write_response(&resp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EntryRow;

    #[test]
    fn protocol_negotiation_accepts_supported_version() {
        let negotiated = negotiate_protocol(Some("2024-11-05")).unwrap();
        assert_eq!(negotiated, "2024-11-05");
    }

    #[test]
    fn protocol_negotiation_accepts_latest_claude_version() {
        let negotiated = negotiate_protocol(Some("2025-11-25")).unwrap();
        assert_eq!(negotiated, "2025-11-25");
    }

    #[test]
    fn protocol_negotiation_rejects_unknown_version() {
        let err = negotiate_protocol(Some("1999-01-01")).unwrap_err();
        assert!(err.contains("Unsupported protocol version"));
    }

    #[test]
    fn parse_request_allows_notification_without_id() {
        let value = json!({"jsonrpc":"2.0","method":"tools/list","params":{}});
        let req = parse_request(value).unwrap();
        assert!(req.id.is_none());
    }

    #[test]
    fn parse_ymd_rejects_invalid_date() {
        let err = parse_ymd("from", "2026/01/01").unwrap_err();
        assert!(err.contains("YYYY-MM-DD"));
    }

    #[test]
    fn evaluate_submission_blocks_zero_duration() {
        let entry = EntryRow {
            id: 1,
            task_id: "PROJ-1".to_string(),
            start_time: "2026-03-09 08:00:00".to_string(),
            end_time: Some("2026-03-09 08:00:00".to_string()),
            duration_secs: Some(0),
            adjusted_secs: None,
            description: None,
            synced_to_jira: false,
            jira_worklog_id: None,
        };
        let (status, _, seconds) = evaluate_submission_entry(&entry, false);
        assert_eq!(status, "blocked");
        assert!(seconds.is_none());
    }

    #[test]
    fn resolve_manual_window_from_duration_and_end_time() {
        let args = json!({
            "duration_minutes": 120,
            "ended_at": "2026-03-09 12:00:00"
        });
        let (start, end, secs) = resolve_manual_window(&args).unwrap();
        assert_eq!(secs, 7200);
        assert_eq!(start.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-03-09 10:00:00");
        assert_eq!(end.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-03-09 12:00:00");
    }

    #[test]
    fn resolve_manual_window_rejects_non_positive_duration() {
        let args = json!({
            "duration_minutes": 0,
            "ended_at": "2026-03-09 12:00:00"
        });
        let err = resolve_manual_window(&args).unwrap_err();
        assert!(err.contains("duration_minutes must be > 0"));
    }

    #[test]
    fn resolve_manual_window_rejects_ambiguous_duration_with_both_bounds() {
        let args = json!({
            "duration_minutes": 60,
            "started_at": "2026-03-09 10:00:00",
            "ended_at": "2026-03-09 11:00:00"
        });
        let err = resolve_manual_window(&args).unwrap_err();
        assert!(err.contains("not both"));
    }
}
