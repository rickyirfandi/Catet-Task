mod commands;
mod db;
mod jira;
mod reminder;
mod timer;

use reminder::PendingOpenLog;

use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconEvent,
    Emitter, Manager, RunEvent, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;

pub fn run() {
    let jira_client: Arc<Mutex<Option<jira::client::JiraClient>>> = Arc::new(Mutex::new(None));
    let timer_engine: Arc<Mutex<timer::engine::TimerEngine>> =
        Arc::new(Mutex::new(timer::engine::TimerEngine::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
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
            commands::tasks::get_task_detail,
            // Timer
            commands::timer::start_timer,
            commands::timer::stop_timer,
            commands::timer::pause_timer,
            commands::timer::resume_timer,
            commands::timer::get_active_timer,
            commands::timer::quit_app,
            // Entries
            commands::timer::get_entries_today,
            commands::timer::get_entries_range,
            commands::timer::update_entry,
            // Worklog
            commands::worklog::submit_batch_worklog,
            // Settings
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::set_launch_at_login,
            commands::settings::reset_timer_data,
        ])
        // CloseRequested → hide instead of destroy so the app stays alive in the tray.
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            eprintln!("[CT] setup: start");

            // Initialize SQLite database
            let app_dir = app
                .path()
                .app_config_dir()
                .expect("Failed to get app config dir");
            std::fs::create_dir_all(&app_dir).expect("Failed to create app config dir");
            let db_path = app_dir.join("catet-task.db");
            let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
            eprintln!("[CT] setup: db_url={}", db_url);

            let pool = tauri::async_runtime::block_on(async {
                let pool = SqlitePool::connect(&db_url)
                    .await
                    .expect("Failed to connect to database");
                eprintln!("[CT] setup: db connected");
                db::queries::init_db(&pool)
                    .await
                    .expect("Failed to initialize database");
                eprintln!("[CT] setup: db initialized");

                pool
            });

            app.manage(pool.clone());
            app.manage(PendingOpenLog(std::sync::Mutex::new(false)));
            eprintln!("[CT] setup: pool managed");

            // Recover timer state from DB/settings so force-close and restart keep timer continuity.
            let engine_for_recovery = app
                .state::<Arc<Mutex<timer::engine::TimerEngine>>>()
                .inner()
                .clone();
            match tauri::async_runtime::block_on(commands::timer::recover_timer_on_startup(
                &engine_for_recovery,
                &pool,
            )) {
                Ok(_) => eprintln!("[CT] setup: timer recovery complete"),
                Err(e) => eprintln!("[CT] setup: timer recovery failed: {}", e),
            }

            // Sync autostart state with saved preference
            let autostart = app.handle().autolaunch();
            let launch_pref =
                tauri::async_runtime::block_on(db::queries::get_setting(&pool, "launch_at_login"));
            match launch_pref {
                Ok(Some(val)) if val == "true" => {
                    let _ = autostart.enable();
                }
                _ => {
                    let _ = autostart.disable();
                }
            }
            eprintln!("[CT] setup: autostart synced");

            // Build tray right-click menu
            let app_label = MenuItemBuilder::with_id("app-label", "Catet Task")
                .enabled(false)
                .build(app)
                .expect("Failed to build app label");
            let show_window_item = MenuItemBuilder::with_id("show-window", "Open Catet Task")
                .build(app)
                .expect("Failed to build show window item");
            let quit_item = MenuItemBuilder::with_id("quit", "Quit Catet Task")
                .build(app)
                .expect("Failed to build quit menu item");
            let menu = MenuBuilder::new(app)
                .item(&app_label)
                .separator()
                .item(&show_window_item)
                .separator()
                .item(&quit_item)
                .build()
                .expect("Failed to build tray menu");
            eprintln!("[CT] setup: tray menu built");

            // Get the tray icon created from tauri.conf.json and attach the menu
            if let Some(tray) = app.tray_by_id("main-tray") {
                tray.set_menu(Some(menu)).expect("Failed to set tray menu");
                // Only show the menu on right-click, not left-click
                #[cfg(any(target_os = "macos", windows))]
                let _ = tray.set_show_menu_on_left_click(false);
                eprintln!("[CT] setup: tray menu attached");

                // Handle left-click: show/focus the window
                tray.on_tray_icon_event(move |tray, event| {
                    let is_left_click = matches!(
                        event,
                        TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            ..
                        } | TrayIconEvent::DoubleClick {
                            button: tauri::tray::MouseButton::Left,
                            ..
                        }
                    );
                    if is_left_click {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });

                // Handle menu events (right-click menu)
                tray.on_menu_event(move |app, event| {
                    if event.id().as_ref() == "show-window" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if event.id().as_ref() == "quit" {
                        // Gracefully stop the timer before exiting
                        let engine = app.state::<Arc<Mutex<timer::engine::TimerEngine>>>();
                        let pool = app.state::<SqlitePool>();

                        let stopped = {
                            let mut eng = engine.lock().unwrap();
                            eng.stop()
                        };

                        let pool = pool.inner().clone();
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(entry) = stopped {
                                if let Ok(Some(running)) =
                                    db::queries::get_running_entry_for_task(&pool, &entry.task_id)
                                        .await
                                {
                                    let _ = db::queries::finalize_entry(
                                        &pool,
                                        running.id,
                                        entry.duration_secs as i64,
                                    )
                                    .await;
                                }
                            }

                            let _ = db::queries::delete_setting(
                                &pool,
                                commands::timer::TIMER_SESSION_KEY,
                            )
                            .await;

                            app_handle.exit(0);
                        });
                    }
                });
            } else {
                eprintln!("[CT] setup: WARNING - tray icon 'main-tray' not found!");
            }

            // Show the window centered on startup
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.center();
                let _ = window.show();
                let _ = window.set_focus();
                eprintln!("[CT] setup: window shown");
            } else {
                eprintln!("[CT] setup: WARNING - window 'main' not found!");
            }

            // Start the timer tick loop
            let app_handle = app.handle().clone();
            let engine = app
                .state::<Arc<Mutex<timer::engine::TimerEngine>>>()
                .inner()
                .clone();
            timer::engine::start_tick_loop(app_handle.clone(), engine);
            eprintln!("[CT] setup: tick loop started");

            // Apply tray title immediately after recovery.
            {
                let engine = app.state::<Arc<Mutex<timer::engine::TimerEngine>>>();
                let eng = engine.lock().unwrap();
                timer::engine::update_tray_now(&app_handle, &eng);
            }

            // Start the daily reminder loop
            reminder::start_reminder_loop(app_handle.clone(), pool.clone());
            eprintln!("[CT] setup: reminder loop started");

            // Wire up window focus handler to open Today tab when reminder is clicked
            if let Some(window) = app.get_webview_window("main") {
                let app_for_focus = app_handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(true) = event {
                        let pending = app_for_focus.state::<PendingOpenLog>();
                        let mut flag = pending.0.lock().unwrap();
                        if *flag {
                            *flag = false;
                            let _ = app_for_focus.emit("open-today-tab", ());
                        }
                    }
                });
            }

            eprintln!("[CT] setup: setup complete");

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
