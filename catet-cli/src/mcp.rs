/// MCP (Model Context Protocol) stdio server.
/// Exposes all CLI commands as tools callable by Claude Desktop.
/// Run via: catet-cli serve-mcp
///
/// Protocol: JSON-RPC 2.0 over stdin/stdout (one JSON object per line).
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::{db, credentials, format, jira};

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
    RpcResponse { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
}

fn err(id: Value, code: i32, message: impl Into<String>) -> RpcResponse {
    RpcResponse { jsonrpc: "2.0".into(), id, result: None, error: Some(RpcError { code, message: message.into() }) }
}

fn write_response(resp: &RpcResponse) {
    let line = serde_json::to_string(resp).unwrap_or_default();
    println!("{}", line);
    let _ = io::stdout().flush();
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
                "description": "Get a daily summary of time tracked this week (Mon–today).",
                "inputSchema": {"type": "object", "properties": {}}
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
                        "minutes": {"type": "integer", "description": "New duration in minutes."}
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
                "name": "catet_tasks",
                "description": "List all cached Jira tasks.",
                "inputSchema": {"type": "object", "properties": {}}
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
                (d - chrono::Duration::days(offset)).format("%Y-%m-%d").to_string()
            };
            let entries = db::get_entries_range(&pool, &monday, &today).await?;
            // Group by date
            let mut by_date: indexmap::IndexMap<String, i64> = indexmap::IndexMap::new();
            for e in &entries {
                let date = e.start_time[..10].to_string();
                *by_date.entry(date).or_insert(0) += e.effective_secs();
            }
            let days: Vec<Value> = by_date
                .iter()
                .map(|(date, secs)| json!({"date": date, "total_secs": secs, "duration": format::fmt_duration(*secs)}))
                .collect();
            let grand_total: i64 = by_date.values().sum();
            Ok(json!({"days": days, "total_secs": grand_total, "total": format::fmt_duration(grand_total)}))
        }

        "catet_entries" => {
            let unlogged_only = args.get("unlogged_only").and_then(|v| v.as_bool()).unwrap_or(false);
            let date = args.get("date").and_then(|v| v.as_str()).map(|s| s.to_string());
            let entries = if let Some(d) = date {
                db::get_entries_range(&pool, &d, &d).await?
            } else {
                db::get_entries_today(&pool).await?
            };
            let filtered: Vec<_> = entries.iter().filter(|e| {
                if unlogged_only { !e.synced_to_jira && e.end_time.is_some() } else { true }
            }).collect();
            Ok(serde_json::to_value(filtered).unwrap())
        }

        "catet_set_comment" => {
            let id = args["entry_id"].as_i64().ok_or("entry_id required")?;
            let comment = args["comment"].as_str().ok_or("comment required")?;
            let pool_rw = db::open_pool_rw(db_path).await?;
            let entry = db::get_entry(&pool_rw, id).await?.ok_or(format!("Entry {} not found", id))?;
            db::update_entry(&pool_rw, id, entry.adjusted_secs, Some(comment)).await?;
            Ok(json!({"ok": true, "entry_id": id, "comment": comment}))
        }

        "catet_set_duration" => {
            let id = args["entry_id"].as_i64().ok_or("entry_id required")?;
            let minutes = args["minutes"].as_i64().ok_or("minutes required")?;
            let pool_rw = db::open_pool_rw(db_path).await?;
            let entry = db::get_entry(&pool_rw, id).await?.ok_or(format!("Entry {} not found", id))?;
            let adjusted_secs = minutes * 60;
            db::update_entry(&pool_rw, id, Some(adjusted_secs), entry.description.as_deref()).await?;
            Ok(json!({"ok": true, "entry_id": id, "adjusted_secs": adjusted_secs}))
        }

        "catet_submit" => {
            let include_running = args.get("include_running").and_then(|v| v.as_bool()).unwrap_or(false);
            let entry_ids: Option<Vec<i64>> = args.get("entry_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let pool_rw = db::open_pool_rw(db_path).await?;
            let (domain, email, token) = credentials::load_credentials(&pool_rw).await?;
            let client = jira::JiraClient::new(&domain, &email, &token);

            let entries = if let Some(ids) = entry_ids {
                let mut result = vec![];
                for id in ids {
                    if let Some(e) = db::get_entry(&pool_rw, id).await? {
                        result.push(e);
                    }
                }
                result
            } else {
                db::get_entries_today(&pool_rw).await?.into_iter()
                    .filter(|e| !e.synced_to_jira && (include_running || e.end_time.is_some()))
                    .collect()
            };

            let mut results = vec![];
            for entry in &entries {
                if entry.is_running() && !include_running {
                    results.push(json!({"entry_id": entry.id, "task_id": entry.task_id, "status": "skipped", "reason": "timer still running"}));
                    continue;
                }
                let secs = entry.effective_secs().max(60) as u64;
                match client.add_worklog(&entry.task_id, secs, &entry.start_time, entry.description.as_deref().unwrap_or("")).await {
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

        "catet_report" => {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let yesterday = (chrono::Local::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
            let today_entries = db::get_entries_today(&pool).await?;
            let yesterday_entries = db::get_entries_range(&pool, &yesterday, &yesterday).await?;
            let tasks = db::get_all_tasks(&pool).await?;
            let task_map: std::collections::HashMap<String, &crate::models::TaskRow> =
                tasks.iter().map(|t| (t.id.clone(), t)).collect();

            fn summarize(entries: &[crate::models::EntryRow], tasks: &std::collections::HashMap<String, &crate::models::TaskRow>) -> Vec<Value> {
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
    let stdout = io::stdout();
    let _ = stdout.lock(); // keep locked for clean output

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        let req: RpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = err(Value::Null, -32700, format!("Parse error: {}", e));
                write_response(&resp);
                continue;
            }
        };

        let id = req.id.clone().unwrap_or(Value::Null);

        let resp = match req.method.as_str() {
            "initialize" => ok(id, json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "catet-task", "version": "0.1.0"}
            })),

            "notifications/initialized" => continue,

            "tools/list" => ok(id, tool_list()),

            "tools/call" => {
                let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = req.params.get("arguments").unwrap_or(&Value::Null);
                match handle_tool(tool_name, args, &db_path).await {
                    Ok(result) => ok(id, json!({
                        "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
                    })),
                    Err(e) => ok(id, json!({
                        "content": [{"type": "text", "text": format!("Error: {}", e)}],
                        "isError": true
                    })),
                }
            }

            other => err(id, -32601, format!("Method not found: {}", other)),
        };

        write_response(&resp);
    }
}
