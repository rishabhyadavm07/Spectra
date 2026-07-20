//! Secret storage backed by the macOS Keychain (PRD Section 21: "Store
//! secrets using macOS Keychain rather than plaintext files"). Only
//! environment variables explicitly marked `secret: true` go through here —
//! everything else stays in the plain JSON files under ~/.spectra.
//!
//! The JSON on disk never holds plaintext for a secret variable, only its
//! Keychain account name (see `keychain_account` below); the actual value is
//! fetched just-in-time when a request is sent.

use crate::error::{ApiError, ApiResult};
use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

const SERVICE_NAME: &str = "com.fanaticalnerd.spectra-app.secrets";

/// macOS `errSecItemNotFound` OSStatus. Not re-exported by the `security-framework`
/// crate (only its `-sys` counterpart has it), so it's inlined here as a constant.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// Builds the Keychain "account" key for a given environment variable.
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
        set_generic_password(SERVICE_NAME, account, value.as_bytes())
            .map_err(|e| ApiError::IoError(format!("keychain write failed: {e}")))
    }

    fn get(&self, account: &str) -> ApiResult<Option<String>> {
        match get_generic_password(SERVICE_NAME, account) {
            Ok(bytes) => Ok(Some(
                String::from_utf8(bytes).map_err(|e| ApiError::IoError(format!("keychain value not utf8: {e}")))?,
            )),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(e) => Err(ApiError::IoError(format!("keychain read failed: {e}"))),
        }
    }

    fn delete(&self, account: &str) -> ApiResult<()> {
        match delete_generic_password(SERVICE_NAME, account) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(ApiError::IoError(format!("keychain delete failed: {e}"))),
        }
    }
}
