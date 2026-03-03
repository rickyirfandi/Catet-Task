use chrono::{Local, NaiveDate, Timelike};
use sqlx::SqlitePool;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

/// Managed state: true when a reminder fired and we should open Today tab on next focus.
pub struct PendingOpenLog(pub Mutex<bool>);

const REMINDER_LAST_FIRED_KEY: &str = "daily_reminder_last_fired_date_v1";

pub fn start_reminder_loop(app: AppHandle, pool: SqlitePool) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut last_fired = load_last_fired_date(&pool).await;

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
            let now_mins = now.hour() * 60 + now.minute();
            let target_mins = target_h * 60 + target_m;

            // Robust condition: do not rely on exact minute equality.
            if now_mins < target_mins {
                continue;
            }

            match crate::db::queries::count_unlogged_today(&pool).await {
                Ok(count) if count > 0 => {
                    let body = if count == 1 {
                        "You have 1 unlogged entry from today.".to_string()
                    } else {
                        format!("You have {} unlogged entries from today.", count)
                    };

                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title("Catet Task - Daily Reminder")
                        .body(&body)
                        .show()
                    {
                        eprintln!("[CT] reminder: failed to show notification: {}", e);
                    }

                    if let Some(state) = app.try_state::<PendingOpenLog>() {
                        *state.0.lock().unwrap() = true;
                    }

                    last_fired = Some(today);
                    persist_last_fired_date(&pool, today).await;
                }
                Ok(_) => {}
                Err(e) => eprintln!("[CT] reminder: count_unlogged_today failed: {}", e),
            }
        }
    });
}

fn parse_hhmm(s: &str) -> (u32, u32) {
    let mut parts = s.splitn(2, ':');
    let h = parts
        .next()
        .and_then(|p| p.parse::<u32>().ok())
        .filter(|h| *h <= 23)
        .unwrap_or(17);
    let m = parts
        .next()
        .and_then(|p| p.parse::<u32>().ok())
        .filter(|m| *m <= 59)
        .unwrap_or(0);
    (h, m)
}

async fn load_last_fired_date(pool: &SqlitePool) -> Option<NaiveDate> {
    let raw = crate::db::queries::get_setting(pool, REMINDER_LAST_FIRED_KEY)
        .await
        .ok()
        .flatten()?;
    NaiveDate::parse_from_str(&raw, "%Y-%m-%d").ok()
}

async fn persist_last_fired_date(pool: &SqlitePool, date: NaiveDate) {
    let value = date.format("%Y-%m-%d").to_string();
    if let Err(e) = crate::db::queries::set_setting(pool, REMINDER_LAST_FIRED_KEY, &value).await {
        eprintln!("[CT] reminder: failed to persist last fired date: {}", e);
    }
}
