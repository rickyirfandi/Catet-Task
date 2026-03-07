mod credentials;
mod db;
mod format;
mod jira;
mod mcp;
mod models;

use clap::{Parser, Subcommand};
use colored::Colorize;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

// ── DB path resolution ────────────────────────────────────────────────────────

fn db_path() -> PathBuf {
    let config_dir = dirs::config_dir().expect("Cannot determine config directory");

    // Tauri v2 uses app_config_dir() → same as dirs::config_dir() + identifier
    config_dir
        .join("id.rickyirfandi.catettask")
        .join("catet-task.db")
}

// ── Parse duration strings ("45m", "1h30m", "3600") ─────────────────────────

fn parse_duration_to_minutes(s: &str) -> Result<i64, String> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n); // bare number = minutes
    }
    // e.g. "1h30m", "45m", "2h", "3600s"
    let mut total = 0i64;
    let mut num = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else if ch == 'h' || ch == 'H' {
            total += num.parse::<i64>().map_err(|_| "Invalid duration")? * 60;
            num.clear();
        } else if ch == 'm' || ch == 'M' {
            total += num.parse::<i64>().map_err(|_| "Invalid duration")?;
            num.clear();
        } else if ch == 's' || ch == 'S' {
            // Convert seconds to minutes, rounding up so we don't silently discard time
            let secs = num.parse::<i64>().map_err(|_| "Invalid duration")?;
            total += (secs + 59) / 60;
            num.clear();
        } else {
            return Err(format!("Unexpected character '{}' in duration", ch));
        }
    }
    if !num.is_empty() {
        // trailing number = minutes
        total += num.parse::<i64>().map_err(|_| "Invalid duration")?;
    }
    Ok(total)
}

fn parse_json_object(
    raw: &str,
    path: &std::path::Path,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("Claude config at {} must be a JSON object", path.display()))
}

fn load_claude_config(
    path: &std::path::Path,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    if !path.exists() {
        return Ok(serde_json::Map::new());
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read Claude Desktop config: {}", e))?;
    if raw.trim().is_empty() {
        return Ok(serde_json::Map::new());
    }
    parse_json_object(&raw, path)
}

fn write_claude_config(
    path: &std::path::Path,
    config: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create config dir: {}", e))?;
    }
    let output = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Cannot serialize config: {}", e))?;
    std::fs::write(path, output).map_err(|e| format!("Cannot write Claude Desktop config: {}", e))
}

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "catet-cli",
    about = "Catet Task CLI — time tracking for Jira",
    version
)]
struct Cli {
    /// Output raw JSON instead of human-readable text
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current timer status
    Status,

    /// Show today's time entries
    Today,

    /// Show this week's daily summary
    Week,

    /// Show entries for a date range
    Range {
        #[arg(long, help = "Start date (YYYY-MM-DD)")]
        from: String,
        #[arg(long, help = "End date (YYYY-MM-DD)")]
        to: String,
    },

    /// List cached tasks
    Tasks,

    /// List individual time entries
    Entries {
        #[arg(long, help = "Only unlogged stopped entries")]
        unlogged: bool,
        #[arg(long, help = "Only the currently running entry")]
        running: bool,
        #[arg(long, help = "Date to show (YYYY-MM-DD, defaults to today)")]
        date: Option<String>,
    },

    /// Set worklog comment on an entry
    SetComment {
        #[arg(help = "Entry ID")]
        entry_id: i64,
        #[arg(help = "Comment text")]
        comment: String,
    },

    /// Override duration of an entry
    SetDuration {
        #[arg(help = "Entry ID")]
        entry_id: i64,
        #[arg(help = "Duration: 45m, 1h30m, 90 (minutes)")]
        duration: String,
    },

    /// Submit entries as worklogs to Jira
    Submit {
        #[arg(long, help = "Specific entry ID to submit")]
        entry: Option<Vec<i64>>,
        #[arg(long, help = "Submit all unlogged stopped entries")]
        all_unlogged: bool,
        #[arg(long, help = "Also submit the currently running entry")]
        include_running: bool,
    },

    /// Generate a standup report
    Report {
        #[arg(
            long,
            default_value = "standup",
            help = "Output format: standup, json, csv"
        )]
        format: String,
    },

    /// Install this binary to PATH (default: ~/.local/bin or %LOCALAPPDATA%\\Programs\\catet-cli)
    Install {
        #[arg(long, help = "Target directory (default: ~/.local/bin)")]
        target: Option<PathBuf>,
    },

    /// Configure Claude Desktop to use this CLI as an MCP server
    ConnectClaude,

    /// Configure Claude Code to use this CLI as an MCP server (user scope)
    ConnectClaudeCode,

    /// Run as an MCP server (for Claude Desktop integration)
    ServeMcp,
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let db_path = db_path();

    if let Err(e) = run(cli, &db_path).await {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli, db_path: &PathBuf) -> Result<(), String> {
    match cli.command {
        Commands::Status => {
            let pool = db::open_pool(db_path).await?;
            let session = db::load_timer_session(&pool).await?;

            if cli.json {
                match &session {
                    None => println!(
                        "{}",
                        json!({"status": "idle", "task_id": null, "elapsed_secs": 0})
                    ),
                    Some(s) => {
                        let state = format::session_to_state(s);
                        // If running, look up summary
                        let summary = if let Some(ref task_id) = state.task_id {
                            db::get_task(&pool, task_id)
                                .await
                                .ok()
                                .flatten()
                                .map(|t| t.summary)
                        } else {
                            None
                        };
                        println!(
                            "{}",
                            json!({
                                "status": state.status,
                                "task_id": state.task_id,
                                "elapsed_secs": state.elapsed_secs,
                                "summary": summary
                            })
                        );
                    }
                }
            } else {
                match &session {
                    None => println!("{}", "No active timer.".dimmed()),
                    Some(s) => {
                        let state = format::session_to_state(s);
                        let summary = if let Some(ref task_id) = state.task_id {
                            db::get_task(&pool, task_id)
                                .await
                                .ok()
                                .flatten()
                                .map(|t| t.summary)
                        } else {
                            None
                        };
                        format::print_status(&state, summary.as_deref());
                    }
                }
            }
        }

        Commands::Today => {
            let pool = db::open_pool(db_path).await?;
            let entries = db::get_entries_today(&pool).await?;
            let tasks = db::get_all_tasks(&pool).await?;
            let task_map: HashMap<String, &models::TaskRow> =
                tasks.iter().map(|t| (t.id.clone(), t)).collect();

            if cli.json {
                let aggregated = format::aggregate_entries(&entries, &task_map);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "entries": entries,
                        "aggregated": aggregated,
                    }))
                    .unwrap()
                );
            } else {
                format::print_today(&entries, &task_map);
            }
        }

        Commands::Week => {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let monday = {
                use chrono::{Datelike, NaiveDate};
                let d = NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap();
                let offset = d.weekday().num_days_from_monday() as i64;
                (d - chrono::Duration::days(offset))
                    .format("%Y-%m-%d")
                    .to_string()
            };
            let pool = db::open_pool(db_path).await?;
            let entries = db::get_entries_range(&pool, &monday, &today).await?;

            if cli.json {
                let mut by_date: indexmap::IndexMap<String, i64> = indexmap::IndexMap::new();
                for e in &entries {
                    *by_date
                        .entry(e.start_time.get(..10).unwrap_or(&e.start_time).to_string())
                        .or_insert(0) += e.effective_secs();
                }
                let days: Vec<_> = by_date.iter().map(|(d, s)| json!({"date": d, "total_secs": s, "duration": format::fmt_duration(*s)})).collect();
                let total: i64 = by_date.values().sum();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"days": days, "total_secs": total}))
                        .unwrap()
                );
            } else {
                format::print_week(&entries);
            }
        }

        Commands::Range { from, to } => {
            let pool = db::open_pool(db_path).await?;
            let entries = db::get_entries_range(&pool, &from, &to).await?;
            let tasks = db::get_all_tasks(&pool).await?;
            let task_map: HashMap<String, &models::TaskRow> =
                tasks.iter().map(|t| (t.id.clone(), t)).collect();

            if cli.json {
                let aggregated = format::aggregate_entries(&entries, &task_map);
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &json!({"entries": entries, "aggregated": aggregated})
                    )
                    .unwrap()
                );
            } else {
                format::print_week(&entries);
            }
        }

        Commands::Tasks => {
            let pool = db::open_pool(db_path).await?;
            let tasks = db::get_all_tasks(&pool).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&tasks).unwrap());
            } else {
                format::print_tasks(&tasks);
            }
        }

        Commands::Entries {
            unlogged,
            running,
            date,
        } => {
            let pool = db::open_pool(db_path).await?;
            let entries = if let Some(d) = date {
                db::get_entries_range(&pool, &d, &d).await?
            } else {
                db::get_entries_today(&pool).await?
            };
            let filtered: Vec<_> = entries
                .into_iter()
                .filter(|e| {
                    if running {
                        return e.end_time.is_none();
                    }
                    if unlogged {
                        return !e.synced_to_jira && e.end_time.is_some();
                    }
                    true
                })
                .collect();
            let tasks = db::get_all_tasks(&pool).await?;
            let task_map: HashMap<String, &models::TaskRow> =
                tasks.iter().map(|t| (t.id.clone(), t)).collect();

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&filtered).unwrap());
            } else {
                format::print_entries(&filtered, &task_map);
            }
        }

        Commands::SetComment { entry_id, comment } => {
            let pool = db::open_pool_rw(db_path).await?;
            let entry = db::get_entry(&pool, entry_id)
                .await?
                .ok_or(format!("Entry {} not found", entry_id))?;
            db::update_entry(&pool, entry_id, entry.adjusted_secs, Some(&comment)).await?;
            if cli.json {
                println!("{}", json!({"ok": true, "entry_id": entry_id}));
            } else {
                println!(
                    "{} Comment set on entry {} ({})",
                    "✓".green(),
                    entry_id,
                    entry.task_id.cyan()
                );
            }
        }

        Commands::SetDuration { entry_id, duration } => {
            let minutes = parse_duration_to_minutes(&duration)?;
            let adjusted_secs = minutes * 60;
            let pool = db::open_pool_rw(db_path).await?;
            let entry = db::get_entry(&pool, entry_id)
                .await?
                .ok_or(format!("Entry {} not found", entry_id))?;
            db::update_entry(
                &pool,
                entry_id,
                Some(adjusted_secs),
                entry.description.as_deref(),
            )
            .await?;
            if cli.json {
                println!(
                    "{}",
                    json!({"ok": true, "entry_id": entry_id, "adjusted_secs": adjusted_secs})
                );
            } else {
                println!(
                    "{} Duration set to {} on entry {} ({})",
                    "✓".green(),
                    format::fmt_duration(adjusted_secs),
                    entry_id,
                    entry.task_id.cyan()
                );
            }
        }

        Commands::Submit {
            entry,
            all_unlogged,
            include_running,
        } => {
            let pool = db::open_pool_rw(db_path).await?;
            let (domain, email, token) = credentials::load_credentials(&pool).await?;
            let client = jira::JiraClient::new(&domain, &email, &token);

            let entries: Vec<models::EntryRow> = if let Some(ids) = entry {
                let mut result = vec![];
                for id in ids {
                    if let Some(e) = db::get_entry(&pool, id).await? {
                        result.push(e);
                    } else {
                        eprintln!("{} Entry {} not found", "!".yellow(), id);
                    }
                }
                result
            } else if all_unlogged {
                db::get_entries_today(&pool)
                    .await?
                    .into_iter()
                    .filter(|e| !e.synced_to_jira && (include_running || e.end_time.is_some()))
                    .collect()
            } else {
                return Err("Specify --entry <id> or --all-unlogged. Use `catet-cli entries --unlogged` to see what's available.".to_string());
            };

            if entries.is_empty() {
                if cli.json {
                    println!(
                        "{}",
                        json!({"results": [], "message": "No entries to submit"})
                    );
                } else {
                    println!("{}", "No entries to submit.".dimmed());
                }
                return Ok(());
            }

            let mut results = vec![];
            for entry in &entries {
                if entry.is_running() && !include_running {
                    if !cli.json {
                        println!(
                            "{} {} {} skipped (timer still running)",
                            "○".yellow(),
                            entry.task_id.cyan(),
                            format::fmt_duration(entry.effective_secs())
                        );
                    }
                    results.push(json!({"entry_id": entry.id, "task_id": entry.task_id, "status": "skipped", "reason": "timer running"}));
                    continue;
                }

                let secs = entry.effective_secs().max(60) as u64;
                if !cli.json {
                    print!(
                        "  Submitting {} {} ... ",
                        entry.task_id.cyan(),
                        format::fmt_duration(entry.effective_secs()).yellow()
                    );
                }

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
                        let _ = db::mark_entry_synced(&pool, entry.id, &wl.id).await;
                        if !cli.json {
                            println!("{}", "✓ logged".green());
                        }
                        results.push(json!({"entry_id": entry.id, "task_id": entry.task_id, "status": "logged", "worklog_id": wl.id}));
                    }
                    Err(e) => {
                        if !cli.json {
                            println!("{} {}", "✗".red(), e);
                        }
                        results.push(json!({"entry_id": entry.id, "task_id": entry.task_id, "status": "error", "error": e}));
                    }
                }
            }

            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"results": results})).unwrap()
                );
            } else {
                let logged = results.iter().filter(|r| r["status"] == "logged").count();
                let skipped = results.iter().filter(|r| r["status"] == "skipped").count();
                let errors = results.iter().filter(|r| r["status"] == "error").count();
                println!();
                println!(
                    "Summary: {} logged, {} skipped, {} errors",
                    logged.to_string().green(),
                    skipped.to_string().yellow(),
                    errors.to_string().red()
                );
            }
        }

        Commands::Report { format } => {
            let pool = db::open_pool(db_path).await?;
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let yesterday = (chrono::Local::now() - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string();
            let today_entries = db::get_entries_today(&pool).await?;
            let yesterday_entries = db::get_entries_range(&pool, &yesterday, &yesterday).await?;
            let tasks = db::get_all_tasks(&pool).await?;
            let task_map: HashMap<String, &models::TaskRow> =
                tasks.iter().map(|t| (t.id.clone(), t)).collect();

            match format.as_str() {
                "standup" => format::print_standup(&yesterday_entries, &today_entries, &task_map),
                "json" | _ => {
                    // JSON report (also fallback)
                    fn summarize<'a>(
                        entries: &[models::EntryRow],
                        tasks: &HashMap<String, &'a models::TaskRow>,
                    ) -> Vec<serde_json::Value> {
                        let mut by_task: indexmap::IndexMap<String, i64> =
                            indexmap::IndexMap::new();
                        for e in entries {
                            *by_task.entry(e.task_id.clone()).or_insert(0) += e.effective_secs();
                        }
                        by_task.iter().map(|(task_id, secs)| {
                            let summary = tasks.get(task_id.as_str()).map(|t| t.summary.as_str()).unwrap_or("?");
                            json!({"task_id": task_id, "duration": format::fmt_duration(*secs), "total_secs": secs, "summary": summary})
                        }).collect()
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "today": summarize(&today_entries, &task_map),
                            "yesterday": summarize(&yesterday_entries, &task_map),
                            "today_date": today,
                            "yesterday_date": yesterday,
                        }))
                        .unwrap()
                    );
                }
            }
        }

        Commands::Install { target } => {
            let self_path =
                std::env::current_exe().map_err(|e| format!("Cannot determine own path: {}", e))?;

            let target_dir = target.unwrap_or_else(|| {
                #[cfg(windows)]
                {
                    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
                        home.join("AppData")
                            .join("Local")
                            .to_string_lossy()
                            .to_string()
                    });
                    return PathBuf::from(local_app_data)
                        .join("Programs")
                        .join("catet-cli");
                }
                #[cfg(not(windows))]
                {
                    dirs::home_dir()
                        .map(|h| h.join(".local").join("bin"))
                        .unwrap_or_else(|| PathBuf::from("/usr/local/bin"))
                }
            });

            std::fs::create_dir_all(&target_dir)
                .map_err(|e| format!("Cannot create {}: {}", target_dir.display(), e))?;

            #[cfg(windows)]
            let binary_name = "catet-cli.exe";
            #[cfg(not(windows))]
            let binary_name = "catet-cli";
            let link_path = target_dir.join(binary_name);

            // Remove existing symlink/file if present
            if link_path.exists() || link_path.symlink_metadata().is_ok() {
                std::fs::remove_file(&link_path).map_err(|e| {
                    format!("Cannot remove existing {}: {}", link_path.display(), e)
                })?;
            }

            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&self_path, &link_path)
                    .map_err(|e| format!("Cannot create symlink: {}", e))?;
            }
            #[cfg(windows)]
            {
                std::fs::copy(&self_path, &link_path)
                    .map_err(|e| format!("Cannot copy binary: {}", e))?;
            }

            println!(
                "{} Installed to {}",
                "✓".green(),
                link_path.display().to_string().cyan()
            );
            println!(
                "  Make sure {} is in your PATH.",
                target_dir.display().to_string().yellow()
            );
        }

        Commands::ConnectClaude => {
            let self_path =
                std::env::current_exe().map_err(|e| format!("Cannot determine own path: {}", e))?;

            let config_path = claude_desktop_config_path()
                .ok_or("Claude Desktop config directory not found. Is Claude Desktop installed?")?;

            // Read existing config or start fresh
            let mut config = load_claude_config(&config_path)?;

            // Ensure mcpServers key exists
            let mcp_servers = config
                .entry("mcpServers")
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
                .as_object_mut()
                .ok_or("mcpServers is not an object")?;

            mcp_servers.insert(
                "catet-task".to_string(),
                json!({
                    "command": self_path.to_string_lossy(),
                    "args": ["serve-mcp"]
                }),
            );

            write_claude_config(&config_path, &config)?;

            println!("{} Claude Desktop configured!", "✓".green());
            println!("  Config: {}", config_path.display().to_string().cyan());
            println!("  {}", "Restart Claude Desktop to activate.".yellow());
        }

        Commands::ConnectClaudeCode => {
            let self_path =
                std::env::current_exe().map_err(|e| format!("Cannot determine own path: {}", e))?;
            let self_path_str = self_path.to_string_lossy().to_string();

            let output = ProcessCommand::new("claude")
                .args([
                    "mcp",
                    "add",
                    "catet-task",
                    &self_path_str,
                    "serve-mcp",
                    "--scope",
                    "user",
                ])
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    println!("{} Claude Code configured!", "âœ“".green());
                    println!("  Verify with: {}", "claude mcp list".cyan());
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(format!(
                        "Failed to configure Claude Code via `claude mcp add`: {}",
                        stderr.trim()
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "Cannot run `claude` command ({}). Install Claude Code first, then run:\n  claude mcp add catet-task \"{}\" serve-mcp --scope user",
                        e, self_path_str
                    ));
                }
            }
        }

        Commands::ServeMcp => {
            mcp::serve(db_path.clone()).await;
        }
    }

    Ok(())
}

fn claude_desktop_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|h| h.join("Library/Application Support/Claude/claude_desktop_config.json"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::config_dir().map(|c| c.join("Claude/claude_desktop_config.json"))
    }
    #[cfg(windows)]
    {
        dirs::config_dir().map(|c| c.join("Claude/claude_desktop_config.json"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        None
    }
}
