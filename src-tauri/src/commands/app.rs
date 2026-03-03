use crate::jira::client::JiraClient;
use std::sync::{Arc, Mutex};
use tauri::State;

fn is_valid_issue_key(issue_key: &str) -> bool {
    !issue_key.is_empty()
        && issue_key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[tauri::command]
pub async fn open_jira(
    issue_key: Option<String>,
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
) -> Result<(), String> {
    let base_url = {
        let state = client_state.lock().unwrap();
        state
            .as_ref()
            .map(|c| c.base_url.trim_end_matches('/').to_string())
            .ok_or_else(|| "Not logged in.".to_string())?
    };

    let url = if let Some(raw_key) = issue_key {
        let key = raw_key.trim();
        if key.is_empty() {
            base_url
        } else {
            if !is_valid_issue_key(key) {
                return Err("Invalid issue key.".to_string());
            }
            format!("{}/browse/{}", base_url, key)
        }
    } else {
        base_url
    };

    open_external_url(&url)
}

fn open_external_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to launch browser: {}", e))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| format!("Failed to launch browser: {}", e))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to launch browser: {}", e))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Unsupported platform for opening browser.".to_string())
}
