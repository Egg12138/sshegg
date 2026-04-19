//! Password management using system keyring
//! Cross-platform: libsecret (Linux), Keychain (macOS), Credential Manager (Windows)
//!
//! Also supports unsafe password storage modes for environments where keyring is unavailable:
//! - `bare`: Store password as plaintext in session file
//! - `simple`: Store password with XOR encoding using a configurable key

use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use keyring::{Entry, Error as KeyringError};
use std::env;

use crate::model::PasswdUnsafeMode;

const SERVICE_NAME: &str = "ssher";
const UNSAFE_KEY_ENV_VAR: &str = "SSHER_UNSAFE_KEY";

fn keyring_hint(err: &KeyringError) -> Option<&'static str> {
    #[cfg(target_os = "linux")]
    {
        match err {
            KeyringError::NoStorageAccess(_) | KeyringError::PlatformFailure(_) => Some(
                "hint: ensure a Secret Service provider is running and unlocked (for example gnome-keyring or ksecretservice) and DBus session variables are available",
            ),
            _ => None,
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = err;
        None
    }
}

fn format_keyring_error(context: &str, err: KeyringError) -> anyhow::Error {
    let context = if let Some(hint) = keyring_hint(&err) {
        format!("{context}; {hint}")
    } else {
        context.to_string()
    };

    anyhow::Error::new(err).context(context)
}

fn is_keyring_backend_unavailable(err: &KeyringError) -> bool {
    match err {
        KeyringError::NoStorageAccess(_) => true,
        #[cfg(target_os = "linux")]
        KeyringError::PlatformFailure(source) => {
            let source = source.to_string().to_lowercase();
            source.contains("org.freedesktop.dbus.error.serviceunknown")
                || source.contains("org.freedesktop.dbus.error.namehasnoowner")
                || source.contains("org.freedesktop.secrets")
                || source.contains("cannot autolaunch dbus")
                || source.contains("dbus session")
                || source.contains("zbus error")
        }
        _ => false,
    }
}

/// Returns true when a keyring operation failed because secure storage is unavailable.
pub fn is_backend_unavailable_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<KeyringError>()
            .is_some_and(is_keyring_backend_unavailable)
    })
}

/// Store password for a session in system keyring
pub fn store_password(session_name: &str, password: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, session_name).context("failed to create keyring entry")?;
    entry
        .set_password(password)
        .map_err(|err| format_keyring_error("failed to store password in keyring", err))
}

/// Retrieve password for a session from system keyring
pub fn get_password(session_name: &str) -> Result<Option<String>> {
    let entry = Entry::new(SERVICE_NAME, session_name).context("failed to create keyring entry")?;

    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(err) => Err(format_keyring_error(
            "failed to retrieve password from keyring",
            err,
        )),
    }
}

/// Delete password for a session from system keyring
pub fn delete_password(session_name: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, session_name).context("failed to create keyring entry")?;

    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()), // Already deleted, OK
        Err(err) => Err(format_keyring_error(
            "failed to delete password from keyring",
            err,
        )),
    }
}

/// Check if a session has a stored password
#[allow(dead_code)]
pub fn has_password(session_name: &str) -> Result<bool> {
    get_password(session_name).map(|p| p.is_some())
}

// ============================================================================
// Unsafe Password Storage (for environments without keyring support)
// ============================================================================

/// Get the encryption key for "simple" mode.
/// Resolution order: SSHER_UNSAFE_KEY env var → passwd_unsafe_key from config
pub fn get_encryption_key(config_key: Option<&str>) -> Result<String> {
    // First check environment variable
    if let Ok(key) = env::var(UNSAFE_KEY_ENV_VAR) {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // Fall back to config key
    config_key.map(|s| s.to_string()).ok_or_else(|| {
        anyhow!(
            "No encryption key set. Set {} environment variable or passwd_unsafe_key in config.",
            UNSAFE_KEY_ENV_VAR
        )
    })
}

/// XOR encode a password with a key, then base64 encode for safe JSON storage
pub fn xor_encode(password: &str, key: &str) -> String {
    let password_bytes = password.as_bytes();
    let key_bytes = key.as_bytes();
    let result: Vec<u8> = password_bytes
        .iter()
        .zip(key_bytes.iter().cycle())
        .map(|(p, k)| p ^ k)
        .collect();
    BASE64_STANDARD.encode(&result)
}

/// Decode a XOR-encoded password (base64 decode, then XOR decode)
pub fn xor_decode(encoded: &str, key: &str) -> Result<String> {
    let bytes = BASE64_STANDARD
        .decode(encoded)
        .context("failed to decode base64 password")?;
    let key_bytes = key.as_bytes();
    let result: Vec<u8> = bytes
        .iter()
        .zip(key_bytes.iter().cycle())
        .map(|(p, k)| p ^ k)
        .collect();
    String::from_utf8(result).context("decoded password is not valid UTF-8")
}

/// Store password using the specified unsafe mode
pub fn store_unsafe_password(
    password: &str,
    mode: &PasswdUnsafeMode,
    config_key: Option<&str>,
) -> Result<String> {
    match mode {
        PasswdUnsafeMode::Normal => {
            anyhow::bail!(
                "store_unsafe_password called with Normal mode - use store_password instead"
            )
        }
        PasswdUnsafeMode::Bare => {
            // Store as plaintext
            Ok(password.to_string())
        }
        PasswdUnsafeMode::Simple => {
            // XOR encode with key
            let key = get_encryption_key(config_key)?;
            Ok(xor_encode(password, &key))
        }
    }
}

/// Retrieve password stored in unsafe mode
pub fn get_unsafe_password(
    stored_password: &str,
    mode: &PasswdUnsafeMode,
    config_key: Option<&str>,
) -> Result<String> {
    match mode {
        PasswdUnsafeMode::Normal => {
            anyhow::bail!("get_unsafe_password called with Normal mode - use get_password instead")
        }
        PasswdUnsafeMode::Bare => {
            // Read plaintext directly
            Ok(stored_password.to_string())
        }
        PasswdUnsafeMode::Simple => {
            // XOR decode with key
            let key = get_encryption_key(config_key)?;
            xor_decode(stored_password, &key)
        }
    }
}

/// Re-encode a password from one unsafe mode to another
pub fn reencode_password(
    stored_password: &str,
    from_mode: &PasswdUnsafeMode,
    to_mode: &PasswdUnsafeMode,
    config_key: Option<&str>,
) -> Result<String> {
    // First decode from source mode
    let plain_password = get_unsafe_password(stored_password, from_mode, config_key)?;
    // Then encode to target mode
    store_unsafe_password(&plain_password, to_mode, config_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn keyring_available() -> bool {
        match get_password("ssher_test_keyring_probe") {
            Ok(_) => true,
            Err(err) => {
                eprintln!("skipping keyring test: {err:#}");
                false
            }
        }
    }

    #[test]
    #[ignore = "Requires running keyring service (D-Bus Secret Service)"]
    fn store_and_retrieve_password() {
        let session_name = "test_session_store";
        let password = "test_password_123";

        // Store password
        store_password(session_name, password).unwrap();

        // Retrieve password
        let retrieved = get_password(session_name).unwrap();
        assert_eq!(retrieved, Some(password.to_string()));

        // Cleanup
        delete_password(session_name).unwrap();
    }

    #[test]
    fn get_nonexistent_password_returns_none() {
        if !keyring_available() {
            return;
        }

        let session_name = "test_session_nonexistent";

        // Try to get password for non-existent session
        let result = get_password(session_name).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[ignore = "Requires running keyring service (D-Bus Secret Service)"]
    fn has_password_returns_correct_status() {
        let session_name = "test_session_has_password";

        // Initially should not have password
        assert!(!has_password(session_name).unwrap());

        // Store password
        store_password(session_name, "some_password").unwrap();

        // Now should have password
        assert!(has_password(session_name).unwrap());

        // Cleanup
        delete_password(session_name).unwrap();

        // After deletion should not have password
        assert!(!has_password(session_name).unwrap());
    }

    #[test]
    #[ignore = "Requires running keyring service (D-Bus Secret Service)"]
    fn delete_password_removes_stored_password() {
        let session_name = "test_session_delete";

        // Store password
        store_password(session_name, "delete_me").unwrap();
        assert!(has_password(session_name).unwrap());

        // Delete password
        delete_password(session_name).unwrap();
        assert!(!has_password(session_name).unwrap());
    }

    #[test]
    fn delete_nonexistent_password_succeeds() {
        if !keyring_available() {
            return;
        }

        let session_name = "test_session_delete_nonexistent";

        // Deleting a non-existent password should succeed
        delete_password(session_name).unwrap();
    }

    #[test]
    #[ignore = "Requires running keyring service (D-Bus Secret Service)"]
    fn update_password_overwrites_existing() {
        let session_name = "test_session_update";

        // Store initial password
        store_password(session_name, "initial_password").unwrap();
        assert_eq!(
            get_password(session_name).unwrap(),
            Some("initial_password".to_string())
        );

        // Update password
        store_password(session_name, "updated_password").unwrap();
        assert_eq!(
            get_password(session_name).unwrap(),
            Some("updated_password".to_string())
        );

        // Cleanup
        delete_password(session_name).unwrap();
    }

    #[test]
    #[ignore = "Requires running keyring service (D-Bus Secret Service)"]
    fn multiple_sessions_have_separate_passwords() {
        let session1 = "test_session_1";
        let session2 = "test_session_2";
        let pass1 = "password_1";
        let pass2 = "password_2";

        // Store passwords for different sessions
        store_password(session1, pass1).unwrap();
        store_password(session2, pass2).unwrap();

        // Verify they're separate
        assert_eq!(get_password(session1).unwrap(), Some(pass1.to_string()));
        assert_eq!(get_password(session2).unwrap(), Some(pass2.to_string()));

        // Cleanup
        delete_password(session1).unwrap();
        delete_password(session2).unwrap();
    }

    #[test]
    fn backend_unavailable_detection_matches_no_storage_access() {
        let err = format_keyring_error(
            "failed to store password in keyring",
            KeyringError::NoStorageAccess(Box::new(io::Error::other("storage locked"))),
        );
        assert!(is_backend_unavailable_error(&err));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn backend_unavailable_detection_matches_linux_dbus_backend_errors() {
        let err = format_keyring_error(
            "failed to store password in keyring",
            KeyringError::PlatformFailure(Box::new(io::Error::other(
                "zbus error: org.freedesktop.DBus.Error.ServiceUnknown",
            ))),
        );
        assert!(is_backend_unavailable_error(&err));
    }

    #[test]
    fn backend_unavailable_detection_ignores_unrelated_errors() {
        let err = format_keyring_error(
            "failed to store password in keyring",
            KeyringError::Invalid("target".to_string(), "bad target".to_string()),
        );
        assert!(!is_backend_unavailable_error(&err));
    }

    // ===== Unsafe Password Storage Tests =====

    #[test]
    fn xor_encode_produces_base64_output() {
        let password = "my-secret-password";
        let key = "encryption-key";
        let encoded = xor_encode(password, key);

        // Should be valid base64
        assert!(BASE64_STANDARD.decode(&encoded).is_ok());
        // Should not contain the original password
        assert!(!encoded.contains(password));
    }

    #[test]
    fn xor_decode_reverses_encoding() {
        let password = "my-secret-password";
        let key = "encryption-key";
        let encoded = xor_encode(password, key);
        let decoded = xor_decode(&encoded, key).unwrap();
        assert_eq!(decoded, password);
    }

    #[test]
    fn xor_with_different_keys_gives_different_results() {
        let password = "my-secret-password";
        let key1 = "key1";
        let key2 = "key2";

        let encoded1 = xor_encode(password, key1);
        let encoded2 = xor_encode(password, key2);

        // Different keys should produce different encoded values
        assert_ne!(encoded1, encoded2);
    }

    #[test]
    fn xor_decode_with_wrong_key_gives_wrong_result() {
        let password = "my-secret-password";
        let encode_key = "correct-key";
        let decode_key = "wrong-key";

        let encoded = xor_encode(password, encode_key);
        let decoded = xor_decode(&encoded, decode_key).unwrap();

        // Decoding with wrong key should NOT give original password
        assert_ne!(decoded, password);
    }

    #[test]
    fn store_unsafe_password_bare_returns_plaintext() {
        let password = "my-secret";
        let stored = store_unsafe_password(password, &PasswdUnsafeMode::Bare, None).unwrap();
        assert_eq!(stored, password);
    }

    #[test]
    fn store_unsafe_password_simple_encodes() {
        let password = "my-secret";
        let stored =
            store_unsafe_password(password, &PasswdUnsafeMode::Simple, Some("my-key")).unwrap();

        // Should not be plaintext
        assert_ne!(stored, password);
        // Should be decodable
        let decoded = xor_decode(&stored, "my-key").unwrap();
        assert_eq!(decoded, password);
    }

    #[test]
    fn store_unsafe_password_simple_requires_key() {
        let result = store_unsafe_password("secret", &PasswdUnsafeMode::Simple, None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No encryption key set")
        );
    }

    #[test]
    fn get_unsafe_password_bare_returns_plaintext() {
        let stored = "my-secret";
        let retrieved = get_unsafe_password(stored, &PasswdUnsafeMode::Bare, None).unwrap();
        assert_eq!(retrieved, stored);
    }

    #[test]
    fn get_unsafe_password_simple_decodes() {
        let password = "my-secret";
        let key = "my-key";
        let encoded = xor_encode(password, key);

        let retrieved =
            get_unsafe_password(&encoded, &PasswdUnsafeMode::Simple, Some(key)).unwrap();
        assert_eq!(retrieved, password);
    }

    #[test]
    fn get_unsafe_password_simple_requires_key() {
        let encoded = xor_encode("secret", "some-key");
        let result = get_unsafe_password(&encoded, &PasswdUnsafeMode::Simple, None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No encryption key set")
        );
    }

    #[test]
    fn reencode_password_from_bare_to_simple() {
        let plain_password = "my-secret";
        let key = "my-key";

        let reencoded = reencode_password(
            plain_password,
            &PasswdUnsafeMode::Bare,
            &PasswdUnsafeMode::Simple,
            Some(key),
        )
        .unwrap();

        // Should be different from plaintext
        assert_ne!(reencoded, plain_password);
        // Should decode back to original
        let decoded = xor_decode(&reencoded, key).unwrap();
        assert_eq!(decoded, plain_password);
    }

    #[test]
    fn reencode_password_from_simple_to_bare() {
        let plain_password = "my-secret";
        let key = "my-key";
        let encoded = xor_encode(plain_password, key);

        let reencoded = reencode_password(
            &encoded,
            &PasswdUnsafeMode::Simple,
            &PasswdUnsafeMode::Bare,
            Some(key),
        )
        .unwrap();

        // Should be plaintext
        assert_eq!(reencoded, plain_password);
    }

    #[test]
    fn get_encryption_key_uses_env_var() {
        temp_env::with_var(UNSAFE_KEY_ENV_VAR, Some("env-key"), || {
            let key = get_encryption_key(Some("config-key")).unwrap();
            assert_eq!(key, "env-key");
        });
    }

    #[test]
    fn get_encryption_key_falls_back_to_config() {
        temp_env::with_var(UNSAFE_KEY_ENV_VAR, Option::<&str>::None, || {
            let key = get_encryption_key(Some("config-key")).unwrap();
            assert_eq!(key, "config-key");
        });
    }

    #[test]
    fn get_encryption_key_errors_when_none_set() {
        temp_env::with_var(UNSAFE_KEY_ENV_VAR, Option::<&str>::None, || {
            let result = get_encryption_key(None);
            assert!(result.is_err());
        });
    }
}
