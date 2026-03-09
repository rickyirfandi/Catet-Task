use crate::commands::timer::TIMER_SESSION_KEY;
use crate::db::queries;
use crate::timer;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
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

// CLI + Claude Desktop integration

const MCP_SERVER_NAME: &str = "catet-task";
const MCP_SERVER_COMMAND_ARG: &str = "serve-mcp";

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

fn is_usable_binary(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    meta.is_file() && meta.len() > 0
}

fn target_triple() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return "aarch64-apple-darwin";
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return "x86_64-apple-darwin";
    }
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        return "x86_64-pc-windows-msvc";
    }
    #[cfg(all(windows, target_arch = "aarch64"))]
    {
        return "aarch64-pc-windows-msvc";
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return "x86_64-unknown-linux-gnu";
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return "aarch64-unknown-linux-gnu";
    }

    #[allow(unreachable_code)]
    "unknown-target"
}

fn candidate_binary_names(base: &str) -> Vec<String> {
    let mut names = vec![base.to_string()];

    #[cfg(windows)]
    names.push(format!("{}.exe", base));

    let sidecar_base = format!("{}-{}", base, target_triple());
    names.push(sidecar_base.clone());

    #[cfg(windows)]
    names.push(format!("{}.exe", sidecar_base));

    names
}

fn find_in_dir(dir: &Path, names: &[String]) -> Option<PathBuf> {
    for name in names {
        let candidate = dir.join(name);
        if is_usable_binary(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Finds bundled `catet-cli` binaries (plain or Tauri sidecar naming).
fn find_bundled_cli_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let names = candidate_binary_names("catet-cli");

    if let Some(found) = find_in_dir(dir, &names) {
        return Some(found);
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(contents_dir) = dir.parent() {
            let resources_dir = contents_dir.join("Resources");
            if let Some(found) = find_in_dir(&resources_dir, &names) {
                return Some(found);
            }
        }
    }

    None
}

fn resolve_cli_binary() -> Option<PathBuf> {
    find_bundled_cli_binary().or_else(|| {
        let installed = cli_install_path();
        if is_usable_binary(&installed) {
            Some(installed)
        } else {
            None
        }
    })
}

/// Path where the symlink / copy of `catet-cli` is installed for PATH access.
fn cli_install_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/tmp".to_string());
    let home_path = PathBuf::from(home);

    #[cfg(windows)]
    {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            home_path
                .join("AppData")
                .join("Local")
                .to_string_lossy()
                .to_string()
        });
        PathBuf::from(local_app_data)
            .join("Programs")
            .join("catet-cli")
            .join("catet-cli.exe")
    }
    #[cfg(not(windows))]
    {
        home_path.join(".local").join("bin").join("catet-cli")
    }
}

fn claude_desktop_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from)?;
        return Some(
            home.join("Library")
                .join("Application Support")
                .join("Claude")
                .join("claude_desktop_config.json"),
        );
    }
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("USERPROFILE")
                    .map(PathBuf::from)
                    .map(|h| h.join("AppData").join("Roaming"))
            })
            .ok()?;
        return Some(appdata.join("Claude").join("claude_desktop_config.json"));
    }
    #[cfg(all(not(target_os = "macos"), not(windows)))]
    {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME")
                    .map(PathBuf::from)
                    .map(|h| h.join(".config"))
            })
            .ok()?;
        return Some(config_dir.join("Claude").join("claude_desktop_config.json"));
    }

    #[allow(unreachable_code)]
    None
}

fn parse_config_object(raw: &str, path: &Path) -> Result<Map<String, Value>, String> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("Claude config at {} must be a JSON object", path.display()))
}

fn load_claude_config(path: &Path) -> Result<Map<String, Value>, String> {
    if !path.exists() {
        return Ok(Map::new());
    }

    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }

    parse_config_object(&raw, path)
}

fn write_claude_config(path: &Path, config: &Map<String, Value>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create config dir {}: {}", parent.display(), e))?;
    }
    let output = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Cannot serialize config: {}", e))?;
    std::fs::write(path, output).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

fn entry_is_connected(server_entry: &Value) -> bool {
    let Some(server) = server_entry.as_object() else {
        return false;
    };

    let command = server
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if command.is_empty() {
        return false;
    }

    let args_ok = server
        .get("args")
        .and_then(|v| v.as_array())
        .map(|args| {
            args.iter()
                .filter_map(|a| a.as_str())
                .any(|arg| arg == MCP_SERVER_COMMAND_ARG)
        })
        .unwrap_or(false);
    if !args_ok {
        return false;
    }

    let path_like = command.contains('/') || command.contains('\\');
    if path_like {
        return is_usable_binary(&PathBuf::from(command));
    }

    true
}

#[tauri::command]
pub async fn get_cli_status() -> Result<CliStatus, String> {
    let install_path = cli_install_path();
    let installed = is_usable_binary(&install_path);
    let cli_binary = resolve_cli_binary();

    Ok(CliStatus {
        installed,
        install_path: if installed {
            Some(install_path.to_string_lossy().to_string())
        } else {
            None
        },
        cli_binary_found: cli_binary.is_some(),
        cli_binary_path: cli_binary.map(|p| p.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub async fn install_cli() -> Result<String, String> {
    let cli_binary = find_bundled_cli_binary().ok_or_else(|| {
        "Bundled catet-cli binary not found. Ensure sidecar packaging is configured and built."
            .to_string()
    })?;

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
        .and_then(|p| load_claude_config(p).ok())
        .and_then(|v| v.get("mcpServers").cloned())
        .and_then(|s| s.get(MCP_SERVER_NAME).cloned())
        .filter(entry_is_connected)
        .is_some();

    Ok(ClaudeDesktopStatus {
        claude_installed,
        connected,
        config_path: config_path.map(|p| p.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub async fn connect_claude_desktop() -> Result<String, String> {
    let cli_binary = resolve_cli_binary()
        .ok_or_else(|| "catet-cli binary not found. Install CLI tools first.".to_string())?;

    let config_path = claude_desktop_config_path()
        .ok_or("Cannot determine Claude Desktop config path. Is Claude Desktop installed?")?;

    // Check if Claude Desktop directory exists
    let config_dir = config_path.parent().ok_or("Invalid config path")?;
    if !config_dir.exists() {
        return Err("Claude Desktop not found. Download it at claude.ai/download".to_string());
    }

    let mut config = load_claude_config(&config_path)?;

    // Upsert catet-task MCP server entry
    let mcp_servers = config
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or("mcpServers is not an object in Claude Desktop config")?;

    mcp_servers.insert(
        MCP_SERVER_NAME.to_string(),
        json!({
            "command": cli_binary.to_string_lossy(),
            "args": [MCP_SERVER_COMMAND_ARG]
        }),
    );

    write_claude_config(&config_path, &config)?;

    Ok(config_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn disconnect_claude_desktop() -> Result<(), String> {
    let config_path =
        claude_desktop_config_path().ok_or("Cannot determine Claude Desktop config path")?;

    if !config_path.exists() {
        return Ok(());
    }

    let mut config = load_claude_config(&config_path)?;

    if let Some(existing) = config.get_mut("mcpServers") {
        let mcp = existing
            .as_object_mut()
            .ok_or("mcpServers is not an object in Claude Desktop config")?;
        mcp.remove(MCP_SERVER_NAME);
    }

    write_claude_config(&config_path, &config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_config_requires_object_root() {
        let path = Path::new("claude_desktop_config.json");
        let err = parse_config_object("[]", path).unwrap_err();
        assert!(err.contains("must be a JSON object"));
    }

    #[test]
    fn parse_config_rejects_invalid_json() {
        let path = Path::new("claude_desktop_config.json");
        let err = parse_config_object("{", path).unwrap_err();
        assert!(err.contains("Invalid JSON"));
    }

    #[test]
    fn connected_entry_requires_serve_mcp_arg() {
        let valid_command_form = json!({
            "command": "catet-cli",
            "args": ["serve-mcp"]
        });
        let invalid_command_form = json!({
            "command": "catet-cli",
            "args": ["status"]
        });

        assert!(entry_is_connected(&valid_command_form));
        assert!(!entry_is_connected(&invalid_command_form));
    }

    #[test]
    fn zero_byte_path_is_not_considered_connected() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("catet-zero-{}.exe", unique));
        std::fs::write(&tmp, b"").unwrap();

        let value = json!({
            "command": tmp.to_string_lossy(),
            "args": ["serve-mcp"]
        });
        assert!(!entry_is_connected(&value));

        let _ = std::fs::remove_file(tmp);
    }
}
