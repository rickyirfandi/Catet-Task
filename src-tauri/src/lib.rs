mod commands;
mod db;
mod jira;
mod timer;

use std::sync::{Arc, Mutex};
use sqlx::SqlitePool;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconEvent,
    Manager,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_positioner::{Position, WindowExt};

pub fn run() {
    let jira_client: Arc<Mutex<Option<jira::client::JiraClient>>> =
        Arc::new(Mutex::new(None));
    let timer_engine: Arc<Mutex<timer::engine::TimerEngine>> =
        Arc::new(Mutex::new(timer::engine::TimerEngine::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
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
            commands::timer::quit_app,
            // Entries
            commands::timer::get_entries_today,
            commands::timer::update_entry,
            // Worklog
            commands::worklog::submit_batch_worklog,
            // Settings
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::set_launch_at_login,
            commands::settings::reset_timer_data,
        ])
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

                // Recover orphaned entries (crash recovery)
                match db::queries::finalize_orphaned_entries(&pool).await {
                    Ok(count) if count > 0 => {
                        log::info!("Recovered {} orphaned time entries from previous session", count);
                    }
                    Err(e) => {
                        log::warn!("Failed to recover orphaned entries: {}", e);
                    }
                    _ => {}
                }

                pool
            });

            app.manage(pool.clone());

            // Sync autostart state with saved preference
            let autostart = app.handle().autolaunch();
            let launch_pref = tauri::async_runtime::block_on(
                db::queries::get_setting(&pool, "launch_at_login")
            );
            match launch_pref {
                Ok(Some(val)) if val == "true" => {
                    let _ = autostart.enable();
                }
                _ => {
                    let _ = autostart.disable();
                }
            }

            // Build tray right-click menu with Quit item
            let quit_item = MenuItemBuilder::with_id("quit", "Quit JTT")
                .build(app)
                .expect("Failed to build quit menu item");
            let menu = MenuBuilder::new(app)
                .item(&quit_item)
                .build()
                .expect("Failed to build tray menu");

            // Get the tray icon created from tauri.conf.json and attach the menu
            if let Some(tray) = app.tray_by_id("main-tray") {
                tray.set_menu(Some(menu)).expect("Failed to set tray menu");

                // Handle left-click: toggle panel
                tray.on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.as_ref().window().move_window(Position::TrayBottomCenter);
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });

                // Handle menu events (right-click menu)
                tray.on_menu_event(move |app, event| {
                    if event.id().as_ref() == "quit" {
                        // Gracefully stop the timer before exiting
                        let engine = app.state::<Arc<Mutex<timer::engine::TimerEngine>>>();
                        let pool = app.state::<SqlitePool>();

                        let stopped = {
                            let mut eng = engine.lock().unwrap();
                            eng.stop()
                        };

                        if let Some(entry) = stopped {
                            let pool = pool.inner().clone();
                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Ok(Some(running)) = db::queries::get_running_entry_for_task(&pool, &entry.task_id).await {
                                    let _ = db::queries::finalize_entry(&pool, running.id, entry.duration_secs as i64).await;
                                }
                                app_handle.exit(0);
                            });
                        } else {
                            app.exit(0);
                        }
                    }
                });
            }

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
