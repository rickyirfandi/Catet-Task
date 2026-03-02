use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub enum TimerState {
    Idle,
    Running {
        task_id: String,
        started_at_utc: DateTime<Utc>,
        accumulated_secs: u64,
    },
    Paused {
        task_id: String,
        elapsed_secs: u64,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct TimerTickPayload {
    pub status: String,
    pub task_id: Option<String>,
    pub elapsed_secs: u64,
}

/// Returned when a timer is stopped, containing info about the stopped entry
#[derive(Debug, Clone)]
pub struct StoppedEntry {
    pub task_id: String,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedTimerState {
    pub status: String,
    pub task_id: String,
    pub accumulated_secs: u64,
    pub started_at_utc: Option<String>,
}

pub struct TimerEngine {
    pub state: TimerState,
}

impl TimerEngine {
    pub fn new() -> Self {
        Self {
            state: TimerState::Idle,
        }
    }

    pub fn get_elapsed(&self) -> u64 {
        match &self.state {
            TimerState::Idle => 0,
            TimerState::Running {
                started_at_utc,
                accumulated_secs,
                ..
            } => accumulated_secs + elapsed_since(started_at_utc),
            TimerState::Paused { elapsed_secs, .. } => *elapsed_secs,
        }
    }

    pub fn get_task_id(&self) -> Option<String> {
        match &self.state {
            TimerState::Idle => None,
            TimerState::Running { task_id, .. } => Some(task_id.clone()),
            TimerState::Paused { task_id, .. } => Some(task_id.clone()),
        }
    }

    pub fn get_status_str(&self) -> &str {
        match &self.state {
            TimerState::Idle => "idle",
            TimerState::Running { .. } => "running",
            TimerState::Paused { .. } => "paused",
        }
    }

    pub fn get_tick_payload(&self) -> TimerTickPayload {
        TimerTickPayload {
            status: self.get_status_str().to_string(),
            task_id: self.get_task_id(),
            elapsed_secs: self.get_elapsed(),
        }
    }

    /// Start a timer for a task. If another timer is running, stops it first.
    /// Returns the stopped entry if one was stopped.
    pub fn start(&mut self, task_id: &str) -> Option<StoppedEntry> {
        let stopped = self.stop_internal();

        self.state = TimerState::Running {
            task_id: task_id.to_string(),
            started_at_utc: Utc::now(),
            accumulated_secs: 0,
        };

        stopped
    }

    /// Stop the current timer. Returns the stopped entry info.
    pub fn stop(&mut self) -> Option<StoppedEntry> {
        let stopped = self.stop_internal();
        self.state = TimerState::Idle;
        stopped
    }

    fn stop_internal(&mut self) -> Option<StoppedEntry> {
        match &self.state {
            TimerState::Idle => None,
            TimerState::Running {
                task_id,
                started_at_utc,
                accumulated_secs,
            } => Some(StoppedEntry {
                task_id: task_id.clone(),
                duration_secs: accumulated_secs + elapsed_since(started_at_utc),
            }),
            TimerState::Paused {
                task_id,
                elapsed_secs,
            } => Some(StoppedEntry {
                task_id: task_id.clone(),
                duration_secs: *elapsed_secs,
            }),
        }
    }

    /// Pause the current timer.
    pub fn pause(&mut self) -> Result<(), String> {
        match &self.state {
            TimerState::Running {
                task_id,
                started_at_utc,
                accumulated_secs,
            } => {
                let elapsed = accumulated_secs + elapsed_since(started_at_utc);
                self.state = TimerState::Paused {
                    task_id: task_id.clone(),
                    elapsed_secs: elapsed,
                };
                Ok(())
            }
            _ => Err("No timer is running.".to_string()),
        }
    }

    /// Resume a paused timer.
    pub fn resume(&mut self) -> Result<(), String> {
        match &self.state {
            TimerState::Paused {
                task_id,
                elapsed_secs,
            } => {
                self.state = TimerState::Running {
                    task_id: task_id.clone(),
                    started_at_utc: Utc::now(),
                    accumulated_secs: *elapsed_secs,
                };
                Ok(())
            }
            _ => Err("No timer is paused.".to_string()),
        }
    }

    pub fn persisted_state(&self) -> Option<PersistedTimerState> {
        match &self.state {
            TimerState::Idle => None,
            TimerState::Running {
                task_id,
                started_at_utc,
                accumulated_secs,
            } => Some(PersistedTimerState {
                status: "running".to_string(),
                task_id: task_id.clone(),
                accumulated_secs: *accumulated_secs,
                started_at_utc: Some(started_at_utc.to_rfc3339()),
            }),
            TimerState::Paused {
                task_id,
                elapsed_secs,
            } => Some(PersistedTimerState {
                status: "paused".to_string(),
                task_id: task_id.clone(),
                accumulated_secs: *elapsed_secs,
                started_at_utc: None,
            }),
        }
    }

    pub fn restore_from_persisted(
        &mut self,
        persisted: &PersistedTimerState,
        fallback_started_at_utc: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        match persisted.status.as_str() {
            "running" => {
                let started_at_utc = persisted
                    .started_at_utc
                    .as_deref()
                    .and_then(parse_utc_timestamp)
                    .or(fallback_started_at_utc)
                    .ok_or_else(|| "Missing started_at_utc for running timer.".to_string())?;

                self.state = TimerState::Running {
                    task_id: persisted.task_id.clone(),
                    started_at_utc,
                    accumulated_secs: persisted.accumulated_secs,
                };
                Ok(())
            }
            "paused" => {
                self.state = TimerState::Paused {
                    task_id: persisted.task_id.clone(),
                    elapsed_secs: persisted.accumulated_secs,
                };
                Ok(())
            }
            other => Err(format!("Invalid persisted timer status: {}", other)),
        }
    }

    pub fn restore_running_from_start(&mut self, task_id: String, started_at_utc: DateTime<Utc>) {
        self.state = TimerState::Running {
            task_id,
            started_at_utc,
            accumulated_secs: 0,
        };
    }
}

/// Immediately updates the tray title based on the current engine state.
/// Call this after state changes so the tray reflects the new state without waiting for the next tick.
pub fn update_tray_now(app: &AppHandle, engine: &TimerEngine) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let title = match &engine.state {
            TimerState::Running {
                started_at_utc,
                accumulated_secs,
                ..
            } => {
                let elapsed = accumulated_secs + elapsed_since(started_at_utc);
                format!("\u{25CF} {}", format_elapsed(elapsed))
            }
            TimerState::Paused { elapsed_secs, .. } => {
                format!("{} \u{23F8}", format_elapsed(*elapsed_secs))
            }
            TimerState::Idle => "CT".to_string(),
        };
        if let Err(e) = tray.set_title(Some(&title)) {
            eprintln!("[CT] update_tray_now set_title error: {e}");
        }
    } else {
        eprintln!("[CT] update_tray_now: tray_by_id returned None");
    }
}

/// Spawns an async task that ticks every second, updating the tray title and emitting events.
pub fn start_tick_loop(app: AppHandle, engine: Arc<Mutex<TimerEngine>>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;

            let payload = {
                let engine = engine.lock().unwrap();
                engine.get_tick_payload()
            };

            if let Some(tray) = app.tray_by_id("main-tray") {
                let title = match payload.status.as_str() {
                    "running" => {
                        format!("\u{25CF} {}", format_elapsed(payload.elapsed_secs))
                    }
                    "paused" => {
                        format!("{} \u{23F8}", format_elapsed(payload.elapsed_secs))
                    }
                    _ => "CT".to_string(),
                };
                if let Err(e) = tray.set_title(Some(&title)) {
                    eprintln!("[CT] tick set_title error: {e}");
                }
            } else {
                eprintln!("[CT] tick: tray_by_id returned None");
            }

            let _ = app.emit("timer-tick", &payload);
        }
    });
}

fn format_elapsed(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

fn elapsed_since(started_at_utc: &DateTime<Utc>) -> u64 {
    let secs = Utc::now()
        .signed_duration_since(started_at_utc.to_owned())
        .num_seconds();
    if secs > 0 {
        secs as u64
    } else {
        0
    }
}

pub fn parse_utc_timestamp(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Some(parsed.with_timezone(&Utc));
    }

    let db_formats = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M:%S%.f"];
    for fmt in db_formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, fmt) {
            return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
        }
    }

    None
}
