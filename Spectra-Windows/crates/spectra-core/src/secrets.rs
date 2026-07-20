//! Secret storage backed by the System Credential Manager (PRD Section 21: "Store
//! secrets using macOS Keychain rather than plaintext files"). Only
//! environment variables explicitly marked `secret: true` go through here —
//! everything else stays in the plain JSON files under ~/.spectra.
//!
//! The JSON on disk never holds plaintext for a secret variable, only its
//! Credential Manager account name (see `keychain_account` below); the actual value is
//! fetched just-in-time when a request is sent.

use crate::error::{ApiError, ApiResult};
use keyring::Entry;

const SERVICE_NAME: &str = "com.fanaticalnerd.spectra-app.secrets";

/// Builds the Credential Manager "account" key for a given environment variable.
/// Scoped by workspace + environment so the same variable name in two
/// different environments never collides.
pub fn keychain_account(workspace_id: &str, environment_id: &str, var_name: &str) -> String {
    format!("{workspace_id}:{environment_id}:{var_name}")
}

pub trait SecretStore: Send + Sync {
    fn set(&self, account: &str, value: &str) -> ApiResult<()>;
    fn get(&self, account: &str) -> ApiResult<Option<String>>;
    fn delete(&self, account: &str) -> ApiResult<()>;
}

pub struct KeychainSecretStore;

impl SecretStore for KeychainSecretStore {
    fn set(&self, account: &str, value: &str) -> ApiResult<()> {
        let entry = Entry::new(SERVICE_NAME, account)
            .map_err(|e| ApiError::IoError(format!("keyring init failed: {e}")))?;
        entry.set_password(value)
            .map_err(|e| ApiError::IoError(format!("keyring write failed: {e}")))
    }

    fn get(&self, account: &str) -> ApiResult<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, account)
            .map_err(|e| ApiError::IoError(format!("keyring init failed: {e}")))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(ApiError::IoError(format!("keyring read failed: {e}"))),
        }
    }

    fn delete(&self, account: &str) -> ApiResult<()> {
        let entry = Entry::new(SERVICE_NAME, account)
            .map_err(|e| ApiError::IoError(format!("keyring init failed: {e}")))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(ApiError::IoError(format!("keyring delete failed: {e}"))),
        }
    }
}
