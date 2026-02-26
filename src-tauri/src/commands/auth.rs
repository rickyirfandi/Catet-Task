use crate::jira::client::JiraClient;
use crate::jira::models::AppUser;
use std::sync::{Arc, Mutex};
use tauri::State;

const KEYRING_SERVICE: &str = "catet-task";
const KEYRING_USER: &str = "credentials";

#[tauri::command]
pub async fn jira_login(
    domain: String,
    email: String,
    token: String,
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
) -> Result<AppUser, String> {
    let client = JiraClient::new(&domain, &email, &token);
    let jira_user = client.verify_auth().await?;

    // Store in keychain
    let cred_value = format!("{}|{}|{}", domain, email, token);
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keychain error: {}", e))?;
    entry
        .set_password(&cred_value)
        .map_err(|e| format!("Failed to store credentials: {}", e))?;

    {
        let mut state = client_state.lock().unwrap();
        *state = Some(client);
    }

    Ok(AppUser {
        id: jira_user.account_id,
        email: jira_user.email_address,
        display_name: jira_user.display_name,
        avatar_url: jira_user.avatar_urls.large,
        jira_domain: domain,
        auth_method: "api_token".to_string(),
    })
}

#[tauri::command]
pub async fn jira_logout(
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
) -> Result<(), String> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }
    {
        let mut state = client_state.lock().unwrap();
        *state = None;
    }
    Ok(())
}

#[tauri::command]
pub async fn jira_verify(
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
) -> Result<AppUser, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keychain error: {}", e))?;
    let cred_value = entry
        .get_password()
        .map_err(|_| "No stored credentials found.".to_string())?;

    let parts: Vec<&str> = cred_value.splitn(3, '|').collect();
    if parts.len() != 3 {
        return Err("Invalid stored credentials format.".to_string());
    }

    let (domain, email, token) = (parts[0], parts[1], parts[2]);
    let client = JiraClient::new(domain, email, token);
    let jira_user = client.verify_auth().await?;

    {
        let mut state = client_state.lock().unwrap();
        *state = Some(client);
    }

    Ok(AppUser {
        id: jira_user.account_id,
        email: jira_user.email_address,
        display_name: jira_user.display_name,
        avatar_url: jira_user.avatar_urls.large,
        jira_domain: domain.to_string(),
        auth_method: "api_token".to_string(),
    })
}

#[tauri::command]
pub async fn get_current_user(
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
) -> Result<Option<AppUser>, String> {
    let has_client = {
        let state = client_state.lock().unwrap();
        state.is_some()
    };

    if has_client {
        match jira_verify(client_state).await {
            Ok(user) => Ok(Some(user)),
            Err(_) => Ok(None),
        }
    } else {
        Ok(None)
    }
}
