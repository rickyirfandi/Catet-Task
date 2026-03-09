/// Loads Jira credentials from OS Keychain (primary) or AES-256-GCM encrypted
/// SQLite fallback. Mirrors the logic in src-tauri/src/commands/auth.rs.
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::db;

const KEYRING_SERVICE: &str = "catet-task";
const KEYRING_USER: &str = "credentials";
const SETTINGS_CRED_KEY: &str = "cred_fallback";

fn derive_key() -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"id.rickyirfandi.catettask");
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default();
    h.update(hostname.as_bytes());
    h.finalize().into()
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
        .map_err(|_| "Decryption failed — credentials may belong to a different machine")?;
    String::from_utf8(plaintext).map_err(|_| "Invalid UTF-8 in credential".to_string())
}

/// Returns (domain, email, token) or an error message.
pub async fn load_credentials(pool: &SqlitePool) -> Result<(String, String, String), String> {
    // 1. Try OS Keychain
    let cred_value = match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => match entry.get_password() {
            Ok(v) => Some(v),
            Err(_) => None,
        },
        Err(_) => None,
    };

    // 2. Fall back to AES-GCM encrypted value in SQLite
    let cred_value = match cred_value {
        Some(v) => v,
        None => {
            let stored = db::get_setting(pool, SETTINGS_CRED_KEY)
                .await?
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    "Not logged in. Open Catet Task and log in first.".to_string()
                })?;
            decrypt_cred(&stored)?
        }
    };

    let parts: Vec<&str> = cred_value.splitn(3, '|').collect();
    if parts.len() != 3 {
        return Err("Invalid stored credentials format.".to_string());
    }

    Ok((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
    ))
}
