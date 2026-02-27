use crate::db::queries;
use crate::jira::client::JiraClient;
use crate::jira::models::AppUser;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::State;

const KEYRING_SERVICE: &str = "catet-task";
const KEYRING_USER: &str = "credentials";
const SETTINGS_CRED_KEY: &str = "cred_fallback";

// ── Encryption helpers ──

fn derive_key() -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"id.rickyirfandi.catettask");
    // Machine-specific entropy (best-effort)
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default();
    h.update(hostname.as_bytes());
    h.finalize().into()
}

fn encrypt_cred(plaintext: &str) -> Result<String, String> {
    let key_bytes = derive_key();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption error: {e}"))?;
    // Store as "base64(nonce):base64(ciphertext)"
    Ok(format!("{}:{}", B64.encode(nonce), B64.encode(ciphertext)))
}

fn decrypt_cred(stored: &str) -> Result<String, String> {
    let (nonce_b64, ct_b64) = stored
        .split_once(':')
        .ok_or("Invalid credential format")?;
    let nonce_bytes = B64.decode(nonce_b64).map_err(|_| "Bad nonce")?;
    let ciphertext = B64.decode(ct_b64).map_err(|_| "Bad ciphertext")?;
    let key_bytes = derive_key();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Decryption failed — credential may belong to a different machine")?;
    String::from_utf8(plaintext).map_err(|_| "Invalid UTF-8 in credential".to_string())
}

// ── Commands ──

#[tauri::command]
pub async fn jira_login(
    domain: String,
    email: String,
    token: String,
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
    pool: State<'_, SqlitePool>,
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

    // Write encrypted fallback to SQLite (for unsigned dev builds)
    if let Ok(encrypted) = encrypt_cred(&cred_value) {
        let _ = queries::set_setting(&pool, SETTINGS_CRED_KEY, &encrypted).await;
    }

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
    pool: State<'_, SqlitePool>,
) -> Result<(), String> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }
    // Clear the encrypted SQLite fallback
    let _ = queries::set_setting(&pool, SETTINGS_CRED_KEY, "").await;
    {
        let mut state = client_state.lock().unwrap();
        *state = None;
    }
    Ok(())
}

#[tauri::command]
pub async fn jira_verify(
    client_state: State<'_, Arc<Mutex<Option<JiraClient>>>>,
    pool: State<'_, SqlitePool>,
) -> Result<AppUser, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keychain error: {}", e))?;

    let cred_value = match entry.get_password() {
        Ok(v) => v,
        Err(_) => {
            // Keychain failed (unsigned dev build) — try encrypted SQLite fallback
            let stored = queries::get_setting(&pool, SETTINGS_CRED_KEY)
                .await
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "No stored credentials found.".to_string())?;
            decrypt_cred(&stored)?
        }
    };

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
    pool: State<'_, SqlitePool>,
) -> Result<Option<AppUser>, String> {
    let has_client = {
        let state = client_state.lock().unwrap();
        state.is_some()
    };

    if has_client {
        match jira_verify(client_state, pool).await {
            Ok(user) => Ok(Some(user)),
            Err(_) => Ok(None),
        }
    } else {
        Ok(None)
    }
}
