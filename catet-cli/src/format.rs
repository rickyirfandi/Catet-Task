use chrono::{DateTime, NaiveDateTime, Utc};
use colored::Colorize;
use std::collections::HashMap;

use crate::models::{AggregatedTask, CliTimerState, EntryRow, TaskRow, TimerSession};

// ── Duration formatting ──────────────────────────────────────────────────────

pub fn fmt_duration(total_secs: i64) -> String {
    let total = total_secs.max(0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else if m > 0 {
        format!("{}m {:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

pub fn fmt_duration_hms(total_secs: u64) -> String {
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

fn fmt_time(dt_str: &str) -> String {
    NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.format("%H:%M").to_string())
        .unwrap_or_else(|_| dt_str[..5.min(dt_str.len())].to_string())
}

fn fmt_date_header(date_str: &str) -> String {
    // date_str is "YYYY-MM-DD"
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|d| d.format("%a, %d %b %Y").to_string().to_uppercase())
        .unwrap_or_else(|_| date_str.to_string().to_uppercase())
}

// ── Status ───────────────────────────────────────────────────────────────────

pub fn print_status(state: &CliTimerState, summary: Option<&str>) {
    match state.status.as_str() {
        "running" => {
            let task = state.task_id.as_deref().unwrap_or("?");
            let elapsed = fmt_duration_hms(state.elapsed_secs);
            print!("{}", "●".green());
            print!(" {}", task.cyan().bold());
            print!("  ·  {}", elapsed.yellow());
            print!("  {}", "(running)".dimmed());
            if let Some(s) = summary {
                print!("\n  {}", s.dimmed());
            }
            println!();
        }
        "paused" => {
            let task = state.task_id.as_deref().unwrap_or("?");
            let elapsed = fmt_duration_hms(state.elapsed_secs);
            print!("{}", "⏸".yellow());
            print!(" {}", task.cyan().bold());
            print!("  ·  {}", elapsed.yellow());
            print!("  {}", "(paused)".dimmed());
            if let Some(s) = summary {
                print!("\n  {}", s.dimmed());
            }
            println!();
        }
        _ => {
            println!("{}", "No active timer".dimmed());
        }
    }
}

/// Convert a persisted TimerSession to CliTimerState (computes live elapsed).
pub fn session_to_state(session: &TimerSession) -> CliTimerState {
    let accumulated = session.accumulated_secs;
    let extra = if session.status == "running" {
        if let Some(started_str) = &session.started_at_utc {
            DateTime::parse_from_rfc3339(started_str)
                .or_else(|_| {
                    NaiveDateTime::parse_from_str(started_str, "%Y-%m-%d %H:%M:%S")
                        .map(|dt| dt.and_utc().fixed_offset())
                })
                .map(|started| {
                    let now: DateTime<Utc> = Utc::now();
                    (now - started.with_timezone(&Utc)).num_seconds().max(0) as u64
                })
                .unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };

    CliTimerState {
        status: session.status.clone(),
        task_id: Some(session.task_id.clone()),
        elapsed_secs: accumulated + extra,
    }
}

// ── Today view ───────────────────────────────────────────────────────────────

pub fn aggregate_entries(entries: &[EntryRow], tasks: &HashMap<String, &TaskRow>) -> Vec<AggregatedTask> {
    let mut map: indexmap::IndexMap<String, AggregatedTask> = indexmap::IndexMap::new();

    for entry in entries {
        let agg = map.entry(entry.task_id.clone()).or_insert_with(|| {
            let summary = tasks
                .get(&entry.task_id)
                .map(|t| t.summary.clone())
                .unwrap_or_else(|| entry.task_id.clone());
            AggregatedTask {
                task_id: entry.task_id.clone(),
                summary,
                entry_ids: vec![],
                total_secs: 0,
                is_running: false,
                is_synced: false,
                latest_start: entry.start_time.clone(),
            }
        });

        agg.entry_ids.push(entry.id);
        agg.total_secs += entry.effective_secs();
        if entry.is_running() {
            agg.is_running = true;
        }
        if entry.synced_to_jira {
            agg.is_synced = true;
        }
        if entry.start_time > agg.latest_start {
            agg.latest_start = entry.start_time.clone();
        }
    }

    map.into_values().collect()
}

pub fn print_today(entries: &[EntryRow], tasks: &HashMap<String, &TaskRow>) {
    if entries.is_empty() {
        println!("{}", "No entries today.".dimmed());
        return;
    }

    // Date header from first entry
    let date_str = &entries[0].start_time[..10];
    println!("{}", fmt_date_header(date_str).bold());
    println!();

    let aggregated = aggregate_entries(entries, tasks);
    let mut total_secs: i64 = 0;
    let mut unlogged_secs: i64 = 0;

    // Group entries by task_id for display
    let mut by_task: HashMap<String, Vec<&EntryRow>> = HashMap::new();
    for e in entries {
        by_task.entry(e.task_id.clone()).or_default().push(e);
    }

    for agg in &aggregated {
        let task_entries = by_task.get(&agg.task_id).map(|v| v.as_slice()).unwrap_or(&[]);

        // Task header
        let running_dot = if agg.is_running { "● ".green().to_string() } else { "  ".to_string() };
        let synced_tag = if agg.is_synced { " [logged]".green().to_string() } else { String::new() };
        println!("  {}{}{}", running_dot, agg.task_id.cyan().bold(), synced_tag);
        println!("  {}", agg.summary.dimmed());

        // Individual entries
        let count = task_entries.len();
        for (i, entry) in task_entries.iter().enumerate() {
            let is_last = i == count - 1;
            let branch = if is_last { "└" } else { "├" };
            let start = fmt_time(&entry.start_time);
            let end = entry.end_time.as_deref()
                .map(fmt_time)
                .unwrap_or_else(|| "●".green().to_string());
            let dur = fmt_duration(entry.effective_secs());
            let synced = if entry.synced_to_jira {
                " [logged]".green().to_string()
            } else if entry.is_running() {
                " [running]".yellow().to_string()
            } else {
                String::new()
            };
            let comment = entry.description.as_deref()
                .map(|d| format!("  \"{}\"", d.dimmed()))
                .unwrap_or_default();
            println!("  {} {}  {}–{}  {}  #{}{}{}", branch, " ".repeat(2), start, end, dur.yellow(), entry.id, synced, comment);
        }

        // Task total
        println!("           total: {}", fmt_duration(agg.total_secs).bold());
        println!();

        total_secs += agg.total_secs;
        if !agg.is_synced {
            unlogged_secs += agg.total_secs;
        }
    }

    println!("  {}", "─".repeat(46).dimmed());
    println!(
        "  Total tracked: {}  |  Unlogged: {}",
        fmt_duration(total_secs).bold(),
        if unlogged_secs > 0 {
            fmt_duration(unlogged_secs).yellow().to_string()
        } else {
            "none".green().to_string()
        }
    );
}

// ── Entries table ────────────────────────────────────────────────────────────

pub fn print_entries(entries: &[EntryRow], tasks: &HashMap<String, &TaskRow>) {
    if entries.is_empty() {
        println!("{}", "No entries.".dimmed());
        return;
    }

    println!(
        "  {:>4}  {:>8}  {:>5}  {:>5}  {:>8}  {}",
        "ID".bold(),
        "Task".bold(),
        "Start".bold(),
        "End".bold(),
        "Duration".bold(),
        "Comment".bold()
    );
    println!("  {}", "─".repeat(60).dimmed());

    for entry in entries {
        let start = fmt_time(&entry.start_time);
        let end = entry.end_time.as_deref()
            .map(fmt_time)
            .unwrap_or_else(|| "●".green().to_string());
        let dur = fmt_duration(entry.effective_secs());
        let comment = entry.description.as_deref().unwrap_or("(none)");
        let synced = if entry.synced_to_jira { " ✓".green().to_string() } else { String::new() };
        let running = if entry.is_running() { " ●".yellow().to_string() } else { String::new() };
        println!(
            "  {:>4}  {:>8}  {:>5}  {:>5}  {:>8}  {}{}{}",
            entry.id,
            entry.task_id.cyan(),
            start,
            end,
            dur.yellow(),
            comment.dimmed(),
            synced,
            running
        );
    }
}

// ── Week summary ─────────────────────────────────────────────────────────────

pub fn print_week(entries: &[EntryRow]) {
    // Group by local date
    let mut by_date: indexmap::IndexMap<String, i64> = indexmap::IndexMap::new();
    let mut grand_total: i64 = 0;

    for entry in entries {
        let date = entry.start_time[..10].to_string();
        *by_date.entry(date).or_insert(0) += entry.effective_secs();
        grand_total += entry.effective_secs();
    }

    for (date, secs) in &by_date {
        let header = fmt_date_header(date);
        println!("  {:<22}  {}", header, fmt_duration(*secs).yellow());
    }

    println!("  {}", "─".repeat(34).dimmed());
    println!("  {:<22}  {}", "Week total:".bold(), fmt_duration(grand_total).bold());
}

// ── Tasks list ───────────────────────────────────────────────────────────────

pub fn print_tasks(tasks: &[TaskRow]) {
    if tasks.is_empty() {
        println!("{}", "No cached tasks. Open Catet Task and refresh.".dimmed());
        return;
    }
    for task in tasks {
        let pin = if task.pinned { "★ ".yellow().to_string() } else { "  ".to_string() };
        let status = task.status.as_deref().unwrap_or("?");
        println!(
            "  {}{}   {:<50}  {}",
            pin,
            task.id.cyan().bold(),
            task.summary.chars().take(50).collect::<String>(),
            status.dimmed()
        );
    }
}

// ── Standup report ───────────────────────────────────────────────────────────

pub fn print_standup(yesterday: &[EntryRow], today: &[EntryRow], tasks: &HashMap<String, &TaskRow>) {
    let fmt_day = |entries: &[EntryRow]| {
        let mut by_task: indexmap::IndexMap<String, i64> = indexmap::IndexMap::new();
        for e in entries {
            *by_task.entry(e.task_id.clone()).or_insert(0) += e.effective_secs();
        }
        by_task
            .into_iter()
            .map(|(task_id, secs)| {
                let summary = tasks.get(&task_id).map(|t| t.summary.as_str()).unwrap_or("?");
                format!("• {}: {} — {}", task_id, fmt_duration(secs), summary)
            })
            .collect::<Vec<_>>()
    };

    if !yesterday.is_empty() {
        let date = &yesterday[0].start_time[..10];
        println!("{}", format!("Yesterday ({}):", fmt_date_header(date)).bold());
        for line in fmt_day(yesterday) {
            println!("{}", line);
        }
        println!();
    }

    if !today.is_empty() {
        let date = &today[0].start_time[..10];
        println!("{}", format!("Today ({}):", fmt_date_header(date)).bold());
        for line in fmt_day(today) {
            println!("{}", line);
        }
    }
}
