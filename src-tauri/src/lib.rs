mod commands;
mod db;
mod jira;
mod timer;

use std::sync::{Arc, Mutex};
use sqlx::SqlitePool;
use tauri::{
    tray::TrayIconEvent, Manager,
};
use tauri_plugin_positioner::{Position, WindowExt};

pub fn run() {
    let jira_client: Arc<Mutex<Option<jira::client::JiraClient>>> =
        Arc::new(Mutex::new(None));
    let timer_engine: Arc<Mutex<timer::engine::TimerEngine>> =
        Arc::new(Mutex::new(timer::engine::TimerEngine::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(jira_client)
        .manage(timer_engine)
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::jira_login,
            commands::auth::jira_logout,
            commands::auth::jira_verify,
            commands::auth::get_current_user,
            // Tasks
            commands::tasks::fetch_my_tasks,
            commands::tasks::search_task,
            commands::tasks::pin_task,
            commands::tasks::unpin_task,
            // Timer
            commands::timer::start_timer,
            commands::timer::stop_timer,
            commands::timer::pause_timer,
            commands::timer::resume_timer,
            commands::timer::get_active_timer,
            // Entries
            commands::timer::get_entries_today,
            commands::timer::update_entry,
            // Worklog
            commands::worklog::submit_batch_worklog,
            // Settings
            commands::settings::get_setting,
            commands::settings::set_setting,
        ])
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.as_ref().window().move_window(Position::BottomCenter);
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .setup(|app| {
            // Initialize SQLite database
            let app_dir = app.path().app_config_dir().expect("Failed to get app config dir");
            std::fs::create_dir_all(&app_dir).expect("Failed to create app config dir");
            let db_path = app_dir.join("jtt.db");
            let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

            let pool = tauri::async_runtime::block_on(async {
                let pool = SqlitePool::connect(&db_url)
                    .await
                    .expect("Failed to connect to database");
                db::queries::init_db(&pool)
                    .await
                    .expect("Failed to initialize database");
                pool
            });

            app.manage(pool);

            // Hide the main window on startup — it's toggled by tray click
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            // Start the timer tick loop
            let app_handle = app.handle().clone();
            let engine = app.state::<Arc<Mutex<timer::engine::TimerEngine>>>().inner().clone();
            timer::engine::start_tick_loop(app_handle, engine);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
