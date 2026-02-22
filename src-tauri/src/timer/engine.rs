use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub enum TimerState {
    Idle,
    Running {
        task_id: String,
        start_instant: Instant,
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
                start_instant,
                accumulated_secs,
                ..
            } => accumulated_secs + start_instant.elapsed().as_secs(),
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
            start_instant: Instant::now(),
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
                start_instant,
                accumulated_secs,
            } => Some(StoppedEntry {
                task_id: task_id.clone(),
                duration_secs: accumulated_secs + start_instant.elapsed().as_secs(),
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
                start_instant,
                accumulated_secs,
            } => {
                let elapsed = accumulated_secs + start_instant.elapsed().as_secs();
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
                    start_instant: Instant::now(),
                    accumulated_secs: *elapsed_secs,
                };
                Ok(())
            }
            _ => Err("No timer is paused.".to_string()),
        }
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

            // Update tray title
            if let Some(tray) = app.tray_by_id("main-tray") {
                let title = match payload.status.as_str() {
                    "running" => {
                        let task_id = payload.task_id.as_deref().unwrap_or("?");
                        let elapsed = format_elapsed(payload.elapsed_secs);
                        format!("\u{25CF} {} \u{00B7} {}", task_id, elapsed)
                    }
                    "paused" => {
                        let task_id = payload.task_id.as_deref().unwrap_or("?");
                        let elapsed = format_elapsed(payload.elapsed_secs);
                        format!("{} \u{00B7} {} \u{23F8}", task_id, elapsed)
                    }
                    _ => "\u{23F1} JTT".to_string(),
                };
                let _ = tray.set_title(Some(&title));
            }

            // Emit tick event to frontend
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
