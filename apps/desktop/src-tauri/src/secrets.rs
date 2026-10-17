//! API keys live in the OS keychain (macOS Keychain, Windows Credential Manager,
//! Secret Service on Linux), never in the database or settings files.

use anyhow::Result;
use waypoint_ai::Provider;

const SERVICE: &str = "app.waypoint.desktop";

fn account(provider: Provider) -> String {
    format!("api-key:{}", serde_json::to_value(provider).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default())
}

fn entry(provider: Provider) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(SERVICE, &account(provider))?)
}

pub fn get(provider: Provider) -> Result<Option<String>> {
    match entry(provider)?.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn set(provider: Provider, key: &str) -> Result<()> {
    Ok(entry(provider)?.set_password(key.trim())?)
}

pub fn delete(provider: Provider) -> Result<()> {
    match entry(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
