use crate::db::queries;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

pub const TIMER_HEARTBEAT_KEY: &str = "timer_heartbeat_utc_v1";

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
    last_observed_utc: DateTime<Utc>,
    sleep_started_at_utc: Option<DateTime<Utc>>,
}

impl TimerEngine {
    pub fn new() -> Self {
        Self {
            state: TimerState::Idle,
            last_observed_utc: Utc::now(),
            sleep_started_at_utc: None,
        }
    }

    /// Detect long inactivity gaps (typically laptop sleep/close-lid) and
    /// exclude that downtime from a running timer.
    ///
    /// Returns the excluded seconds when compensation was applied.
    pub fn compensate_inactive_gap(&mut self) -> Option<u64> {
        const INACTIVE_GAP_THRESHOLD_SECS: i64 = 20;

        let now_utc = Utc::now();
        let gap_secs = now_utc
            .signed_duration_since(self.last_observed_utc)
            .num_seconds();
        self.last_observed_utc = now_utc;

        if gap_secs <= INACTIVE_GAP_THRESHOLD_SECS {
            return None;
        }

        // Subtract one expected tick second; exclude only the true inactivity window.
        let inactive_gap_secs = (gap_secs - 1) as u64;

        if let TimerState::Running { started_at_utc, .. } = &mut self.state {
            *started_at_utc =
                started_at_utc.to_owned() + Duration::seconds(inactive_gap_secs as i64);
            return Some(inactive_gap_secs);
        }

        None
    }

    /// Compensate downtime across process restarts using persisted heartbeat.
    /// This handles force-close scenarios where in-memory gap tracking is unavailable.
    pub fn compensate_external_inactive_gap(&mut self, last_active_utc: DateTime<Utc>) -> Option<u64> {
        const INACTIVE_GAP_THRESHOLD_SECS: i64 = 20;

        let now_utc = Utc::now();
        let gap_secs = now_utc
            .signed_duration_since(last_active_utc)
            .num_seconds();

        self.last_observed_utc = now_utc;

        if gap_secs <= INACTIVE_GAP_THRESHOLD_SECS {
            return None;
        }

        let inactive_gap_secs = (gap_secs - 1) as u64;

        if let TimerState::Running { started_at_utc, .. } = &mut self.state {
            *started_at_utc =
                started_at_utc.to_owned() + Duration::seconds(inactive_gap_secs as i64);
            return Some(inactive_gap_secs);
        }

        None
    }

    /// macOS sleep callback hook. Marks sleep start for precise wake compensation.
    #[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
    pub fn on_system_will_sleep(&mut self) {
        let now_utc = Utc::now();
        if matches!(self.state, TimerState::Running { .. }) {
            self.sleep_started_at_utc = Some(now_utc);
        } else {
            self.sleep_started_at_utc = None;
        }
        self.last_observed_utc = now_utc;
    }

    /// macOS wake callback hook. Excludes exact sleep interval from running timer.
    #[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
    pub fn on_system_did_wake(&mut self) -> Option<u64> {
        let now_utc = Utc::now();
        let excluded = match (&mut self.state, self.sleep_started_at_utc.take()) {
            (TimerState::Running { started_at_utc, .. }, Some(slept_at_utc)) => {
                let sleep_secs = now_utc.signed_duration_since(slept_at_utc).num_seconds();
                if sleep_secs > 0 {
                    *started_at_utc =
                        started_at_utc.to_owned() + Duration::seconds(sleep_secs);
                    Some(sleep_secs as u64)
                } else {
                    None
                }
            }
            _ => None,
        };
        self.last_observed_utc = now_utc;
        excluded
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

    pub fn get_tick_payload(&mut self) -> TimerTickPayload {
        let _ = self.compensate_inactive_gap();
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
        self.last_observed_utc = Utc::now();
        self.sleep_started_at_utc = None;

        stopped
    }

    /// Stop the current timer. Returns the stopped entry info.
    pub fn stop(&mut self) -> Option<StoppedEntry> {
        let stopped = self.stop_internal();
        self.state = TimerState::Idle;
        self.sleep_started_at_utc = None;
        stopped
    }

    fn stop_internal(&mut self) -> Option<StoppedEntry> {
        let _ = self.compensate_inactive_gap();
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
        let _ = self.compensate_inactive_gap();
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
                self.sleep_started_at_utc = None;
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
                self.last_observed_utc = Utc::now();
                self.sleep_started_at_utc = None;
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
                self.last_observed_utc = Utc::now();
                self.sleep_started_at_utc = None;
                Ok(())
            }
            "paused" => {
                self.state = TimerState::Paused {
                    task_id: persisted.task_id.clone(),
                    elapsed_secs: persisted.accumulated_secs,
                };
                self.last_observed_utc = Utc::now();
                self.sleep_started_at_utc = None;
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
        self.last_observed_utc = Utc::now();
        self.sleep_started_at_utc = None;
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
    let pool = app.state::<SqlitePool>().inner().clone();

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        let mut last_heartbeat_write: Option<DateTime<Utc>> = None;

        loop {
            interval.tick().await;

            let payload = {
                let mut engine = engine.lock().unwrap();
                engine.get_tick_payload()
            };

            if payload.status == "running" {
                let now_utc = Utc::now();
                let should_write = match last_heartbeat_write {
                    Some(prev) => now_utc.signed_duration_since(prev).num_seconds() >= 5,
                    None => true,
                };

                if should_write {
                    if let Err(e) =
                        queries::set_setting(&pool, TIMER_HEARTBEAT_KEY, &now_utc.to_rfc3339())
                            .await
                    {
                        eprintln!("[CT] heartbeat persist failed: {e}");
                    } else {
                        last_heartbeat_write = Some(now_utc);
                    }
                }
            } else if last_heartbeat_write.is_some() {
                if let Err(e) = queries::delete_setting(&pool, TIMER_HEARTBEAT_KEY).await {
                    eprintln!("[CT] heartbeat clear failed: {e}");
                } else {
                    last_heartbeat_write = None;
                }
            }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn running_engine_with_offsets(total_secs: i64, gap_secs: i64) -> TimerEngine {
        let now = Utc::now();
        let mut engine = TimerEngine::new();
        engine.state = TimerState::Running {
            task_id: "TEST-1".to_string(),
            started_at_utc: now - Duration::seconds(total_secs),
            accumulated_secs: 0,
        };
        engine.last_observed_utc = now - Duration::seconds(gap_secs);
        engine
    }

    #[test]
    fn compensate_inactive_gap_excludes_large_gap_when_running() {
        let mut engine = running_engine_with_offsets(120, 60);
        let excluded = engine.compensate_inactive_gap();
        assert!(excluded.is_some());
        let excluded_secs = excluded.unwrap();
        assert!((58..=62).contains(&excluded_secs));

        let elapsed = engine.get_elapsed();
        assert!((57..=66).contains(&elapsed));
    }

    #[test]
    fn compensate_inactive_gap_ignores_small_gap() {
        let mut engine = running_engine_with_offsets(10, 5);
        let excluded = engine.compensate_inactive_gap();
        assert!(excluded.is_none());
    }

    #[test]
    fn compensate_external_gap_only_applies_to_running_state() {
        let mut engine = TimerEngine::new();
        engine.state = TimerState::Paused {
            task_id: "TEST-2".to_string(),
            elapsed_secs: 123,
        };

        let excluded = engine.compensate_external_inactive_gap(Utc::now() - Duration::seconds(90));
        assert!(excluded.is_none());
    }

    #[test]
    fn system_wake_excludes_recorded_sleep_for_running_timer() {
        let now = Utc::now();
        let mut engine = TimerEngine::new();
        engine.state = TimerState::Running {
            task_id: "TEST-3".to_string(),
            started_at_utc: now - Duration::seconds(120),
            accumulated_secs: 0,
        };
        engine.sleep_started_at_utc = Some(now - Duration::seconds(30));

        let excluded = engine.on_system_did_wake();
        assert!(excluded.is_some());
        let excluded_secs = excluded.unwrap();
        assert!((29..=33).contains(&excluded_secs));

        let elapsed = engine.get_elapsed();
        assert!((86..=94).contains(&elapsed));
    }
}
