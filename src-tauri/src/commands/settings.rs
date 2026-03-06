use crate::commands::timer::TIMER_SESSION_KEY;
use crate::db::queries;
use crate::timer;
use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub async fn get_setting(
    key: String,
    pool: State<'_, SqlitePool>,
) -> Result<Option<String>, String> {
    queries::get_setting(&pool, &key)
        .await
        .map_err(|e| format!("Failed to get setting: {}", e))
}

#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    queries::set_setting(&pool, &key, &value)
        .await
        .map_err(|e| format!("Failed to set setting: {}", e))
}

#[tauri::command]
pub async fn reset_timer_data(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<u64, String> {
    // 1. Stop running timer (in-memory)
    let engine = app.state::<Arc<Mutex<timer::engine::TimerEngine>>>();
    {
        engine.lock().unwrap().stop();
    }

    // 2. Delete all time entries from DB
    let deleted = queries::delete_all_entries(&pool)
        .await
        .map_err(|e| format!("Failed to reset data: {}", e))?;

    // 3. Clear persisted timer session snapshot
    queries::delete_setting(&pool, TIMER_SESSION_KEY)
        .await
        .map_err(|e| format!("Failed to clear timer session: {}", e))?;

    // 4. Emit event so frontend stores refresh
    let _ = app.emit("timer-stopped", ());

    Ok(deleted)
}

#[tauri::command]
pub async fn set_launch_at_login(
    enabled: bool,
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    let autostart = app.autolaunch();
    if enabled {
        autostart
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
    } else {
        autostart
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {}", e))?;
    }
    queries::set_setting(&pool, "launch_at_login", &enabled.to_string())
        .await
        .map_err(|e| format!("Failed to save setting: {}", e))
}

// ── CLI + Claude Desktop integration ─────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub installed: bool,
    pub install_path: Option<String>,
    pub cli_binary_found: bool,
    pub cli_binary_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopStatus {
    pub claude_installed: bool,
    pub connected: bool,
    pub config_path: Option<String>,
}

/// Finds the bundled `catet-cli` binary.
/// In dev builds it lives in the same `target/` directory as the main binary.
/// In production it is placed next to the main app binary (same dir or inside the bundle).
fn find_cli_binary() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;

    // Same directory as the running app binary
    let candidate = dir.join("catet-cli");
    if candidate.exists() {
        return Some(candidate);
    }

    // macOS app bundle: the main binary is at Contents/MacOS/<AppName>,
    // and catet-cli is bundled at Contents/MacOS/catet-cli (same dir, already checked above).
    // Also check Contents/Resources/ for non-executable resources layout.
    #[cfg(target_os = "macos")]
    {
        if let Some(contents_dir) = dir.parent() {
            let resources_bin = contents_dir.join("Resources").join("catet-cli");
            if resources_bin.exists() {
                return Some(resources_bin);
            }
        }
    }

    None
}

/// Path where the symlink / copy of `catet-cli` is installed for PATH access.
fn cli_install_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/tmp".to_string());
    let home_path = std::path::PathBuf::from(home);

    #[cfg(windows)]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| home_path.join("AppData").join("Local").to_string_lossy().to_string());
        std::path::PathBuf::from(local_app_data)
            .join("Programs")
            .join("catet-cli")
            .join("catet-cli.exe")
    }
    #[cfg(not(windows))]
    {
        home_path.join(".local").join("bin").join("catet-cli")
    }
}

fn claude_desktop_config_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(std::path::PathBuf::from)?;

    #[cfg(target_os = "macos")]
    {
        Some(home.join("Library").join("Application Support").join("Claude").join("claude_desktop_config.json"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"));
        Some(config_dir.join("Claude").join("claude_desktop_config.json"))
    }
}

#[tauri::command]
pub async fn get_cli_status() -> Result<CliStatus, String> {
    let install_path = cli_install_path();
    let installed = install_path.exists();
    let cli_binary = find_cli_binary();

    Ok(CliStatus {
        installed,
        install_path: if installed { Some(install_path.to_string_lossy().to_string()) } else { None },
        cli_binary_found: cli_binary.is_some(),
        cli_binary_path: cli_binary.map(|p| p.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub async fn install_cli() -> Result<String, String> {
    let cli_binary = find_cli_binary()
        .ok_or_else(|| "catet-cli binary not found next to the app. Build it first with: cd catet-cli && cargo build --release".to_string())?;

    let install_path = cli_install_path();

    // Create parent dir
    if let Some(parent) = install_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory {}: {}", parent.display(), e))?;
    }

    // Remove existing
    if install_path.exists() || install_path.symlink_metadata().is_ok() {
        std::fs::remove_file(&install_path)
            .map_err(|e| format!("Cannot remove existing install: {}", e))?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&cli_binary, &install_path)
            .map_err(|e| format!("Cannot create symlink: {}", e))?;
    }
    #[cfg(windows)]
    {
        std::fs::copy(&cli_binary, &install_path)
            .map_err(|e| format!("Cannot copy binary: {}", e))?;
    }

    Ok(install_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn uninstall_cli() -> Result<(), String> {
    let install_path = cli_install_path();
    if install_path.exists() || install_path.symlink_metadata().is_ok() {
        std::fs::remove_file(&install_path)
            .map_err(|e| format!("Cannot remove {}: {}", install_path.display(), e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_claude_desktop_status() -> Result<ClaudeDesktopStatus, String> {
    let config_path = claude_desktop_config_path();

    let claude_installed = config_path
        .as_ref()
        .and_then(|p| p.parent())
        .map(|dir| dir.exists())
        .unwrap_or(false);

    let connected = config_path
        .as_ref()
        .filter(|p| p.exists())
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("mcpServers").cloned())
        .and_then(|s| s.get("catet-task").cloned())
        .is_some();

    Ok(ClaudeDesktopStatus {
        claude_installed,
        connected,
        config_path: config_path.map(|p| p.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub async fn connect_claude_desktop() -> Result<String, String> {
    let cli_binary = find_cli_binary()
        .ok_or_else(|| "catet-cli binary not found. Install CLI tools first.".to_string())?;

    let config_path = claude_desktop_config_path()
        .ok_or("Cannot determine Claude Desktop config path. Is Claude Desktop installed?")?;

    // Check if Claude Desktop directory exists
    let config_dir = config_path.parent()
        .ok_or("Invalid config path")?;
    if !config_dir.exists() {
        return Err("Claude Desktop not found. Download it at claude.ai/download".to_string());
    }

    // Read or create config
    let mut config: serde_json::Map<String, serde_json::Value> = if config_path.exists() {
        let raw = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Cannot read config: {}", e))?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };

    // Upsert catet-task MCP server entry
    let mcp_servers = config
        .entry("mcpServers")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or("mcpServers is not an object in Claude Desktop config")?;

    mcp_servers.insert(
        "catet-task".to_string(),
        serde_json::json!({
            "command": cli_binary.to_string_lossy(),
            "args": ["serve-mcp"]
        }),
    );

    let output = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Cannot serialize config: {}", e))?;
    std::fs::write(&config_path, output)
        .map_err(|e| format!("Cannot write config: {}", e))?;

    Ok(config_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn disconnect_claude_desktop() -> Result<(), String> {
    let config_path = claude_desktop_config_path()
        .ok_or("Cannot determine Claude Desktop config path")?;

    if !config_path.exists() {
        return Ok(());
    }

    let raw = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Cannot read config: {}", e))?;
    let mut config: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();

    if let Some(mcp) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        mcp.remove("catet-task");
    }

    let output = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Cannot serialize config: {}", e))?;
    std::fs::write(&config_path, output)
        .map_err(|e| format!("Cannot write config: {}", e))?;

    Ok(())
}
