use chrono::{Local, Timelike};
use sqlx::SqlitePool;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

/// Managed state — true when a reminder just fired and we should open the log tab on next focus
pub struct PendingOpenLog(pub Mutex<bool>);

pub fn start_reminder_loop(app: AppHandle, pool: SqlitePool) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        let mut last_fired: Option<chrono::NaiveDate> = None;

        loop {
            interval.tick().await;

            let enabled = crate::db::queries::get_setting(&pool, "daily_reminder")
                .await
                .ok()
                .flatten();
            if enabled.as_deref() != Some("true") {
                continue;
            }

            let time_str = crate::db::queries::get_setting(&pool, "reminder_time")
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "17:00".to_string());

            let now = Local::now();
            let today = now.date_naive();

            if last_fired == Some(today) {
                continue;
            }

            let (target_h, target_m) = parse_hhmm(&time_str);
            if now.hour() == target_h && now.minute() == target_m {
                match crate::db::queries::count_unlogged_today(&pool).await {
                    Ok(count) if count > 0 => {
                        let body = if count == 1 {
                            "You have 1 unlogged entry from today.".to_string()
                        } else {
                            format!("You have {} unlogged entries from today.", count)
                        };

                        let _ = app
                            .notification()
                            .builder()
                            .title("Catet Task — Daily Reminder")
                            .body(&body)
                            .show();

                        // Flag: next window focus should switch to Today tab
                        if let Some(state) = app.try_state::<PendingOpenLog>() {
                            *state.0.lock().unwrap() = true;
                        }

                        last_fired = Some(today);
                    }
                    _ => {}
                }
            }
        }
    });
}

fn parse_hhmm(s: &str) -> (u32, u32) {
    let mut parts = s.splitn(2, ':');
    let h = parts.next().and_then(|p| p.parse().ok()).unwrap_or(17);
    let m = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (h, m)
}
