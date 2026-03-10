//! Password management using system keyring
//! Cross-platform: libsecret (Linux), Keychain (macOS), Credential Manager (Windows)

use anyhow::{Context, Result, anyhow};
use keyring::{Entry, Error as KeyringError};

const SERVICE_NAME: &str = "ssher";

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
    if let Some(hint) = keyring_hint(&err) {
        anyhow!("{context}: {err}; {hint}")
    } else {
        anyhow!("{context}: {err}")
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
