mod commands;
mod db;
mod jira;
mod timer;

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{SystemTime, UNIX_EPOCH};
use sqlx::SqlitePool;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconEvent,
    Manager, RunEvent, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_positioner::{Position, WindowExt};

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn run() {
    let jira_client: Arc<Mutex<Option<jira::client::JiraClient>>> =
        Arc::new(Mutex::new(None));
    let timer_engine: Arc<Mutex<timer::engine::TimerEngine>> =
        Arc::new(Mutex::new(timer::engine::TimerEngine::new()));

    // Shared timestamp to detect blur-hide → tray-click race condition.
    // When clicking the tray icon while the panel is open, the blur event
    // fires first (hiding the panel), then the tray click fires. Without
    // this guard the tray click would immediately re-show the panel.
    let last_blur_hide = Arc::new(AtomicU64::new(0));
    let blur_ts_for_window = last_blur_hide.clone();
    let blur_ts_for_tray = last_blur_hide;

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
        // Window event handling for the tray-only panel:
        // 1. CloseRequested → hide instead of destroy (keeps the app alive)
        // 2. Focused(false) → auto-dismiss like CleanMyMac's tray popover
        .on_window_event(move |window, event| {
            if window.label() == "main" {
                match event {
                    WindowEvent::CloseRequested { api, .. } => {
                        // Never destroy the window — just hide it.
                        // Without this, macOS closes the window and the app exits.
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    WindowEvent::Focused(false) => {
                        let _ = window.hide();
                        blur_ts_for_window.store(now_millis(), Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        })
        .setup(move |app| {
            eprintln!("[JTT] setup: start");

            // Initialize SQLite database
            let app_dir = app.path().app_config_dir().expect("Failed to get app config dir");
            std::fs::create_dir_all(&app_dir).expect("Failed to create app config dir");
            let db_path = app_dir.join("jtt.db");
            let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
            eprintln!("[JTT] setup: db_url={}", db_url);

            let pool = tauri::async_runtime::block_on(async {
                let pool = SqlitePool::connect(&db_url)
                    .await
                    .expect("Failed to connect to database");
                eprintln!("[JTT] setup: db connected");
                db::queries::init_db(&pool)
                    .await
                    .expect("Failed to initialize database");
                eprintln!("[JTT] setup: db initialized");

                // Recover orphaned entries (crash recovery)
                match db::queries::finalize_orphaned_entries(&pool).await {
                    Ok(count) if count > 0 => {
                        eprintln!("[JTT] setup: recovered {} orphaned entries", count);
                    }
                    Err(e) => {
                        eprintln!("[JTT] setup: orphan recovery failed: {}", e);
                    }
                    _ => {}
                }

                pool
            });

            app.manage(pool.clone());
            eprintln!("[JTT] setup: pool managed");

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
            eprintln!("[JTT] setup: autostart synced");

            // Build tray right-click menu with Quit item
            let quit_item = MenuItemBuilder::with_id("quit", "Quit JTT")
                .build(app)
                .expect("Failed to build quit menu item");
            let menu = MenuBuilder::new(app)
                .item(&quit_item)
                .build()
                .expect("Failed to build tray menu");
            eprintln!("[JTT] setup: tray menu built");

            // Get the tray icon created from tauri.conf.json and attach the menu
            if let Some(tray) = app.tray_by_id("main-tray") {
                tray.set_menu(Some(menu)).expect("Failed to set tray menu");
                eprintln!("[JTT] setup: tray menu attached");

                // Handle left-click: toggle panel below tray icon
                let blur_ts = blur_ts_for_tray.clone();
                tray.on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            // Position directly below the tray icon (like CleanMyMac)
                            let _ = window.as_ref().window().move_window(Position::TrayBottomCenter);

                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                // Skip re-show if the panel was just hidden by a blur event
                                // (the blur fires before the tray click when clicking the
                                // tray icon to dismiss).
                                let since_blur = now_millis().saturating_sub(
                                    blur_ts.load(Ordering::SeqCst),
                                );
                                if since_blur > 300 {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
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
            } else {
                eprintln!("[JTT] setup: WARNING - tray icon 'main-tray' not found!");
            }

            // Show the panel on startup, positioned top-right
            // (avoid TrayBottomCenter here — tray icon may not be fully registered yet)
            eprintln!("[JTT] setup: about to show window");
            if let Some(window) = app.get_webview_window("main") {
                eprintln!("[JTT] setup: got window, positioning top-right...");
                let pos_result = window.as_ref().window().move_window(Position::TopRight);
                eprintln!("[JTT] setup: move_window result: {:?}", pos_result);
                let show_result = window.show();
                eprintln!("[JTT] setup: show result: {:?}", show_result);
                let focus_result = window.set_focus();
                eprintln!("[JTT] setup: set_focus result: {:?}", focus_result);
            } else {
                eprintln!("[JTT] setup: WARNING - window 'main' not found!");
            }

            // Start the timer tick loop
            let app_handle = app.handle().clone();
            let engine = app.state::<Arc<Mutex<timer::engine::TimerEngine>>>().inner().clone();
            timer::engine::start_tick_loop(app_handle, engine);
            eprintln!("[JTT] setup: tick loop started, setup complete");

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // Prevent the app from exiting when the last window is hidden.
            // Essential for tray-only apps on macOS.
            if let RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
