//! Password management using system keyring
//! Cross-platform: libsecret (Linux), Keychain (macOS), Credential Manager (Windows)

use anyhow::{Context, Result};
use keyring::{Entry, Error as KeyringError};

const SERVICE_NAME: &str = "ssher";

/// Store password for a session in system keyring
pub fn store_password(session_name: &str, password: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, session_name).context("failed to create keyring entry")?;
    entry
        .set_password(password)
        .context("failed to store password in keyring")
}

/// Retrieve password for a session from system keyring
pub fn get_password(session_name: &str) -> Result<Option<String>> {
    let entry = Entry::new(SERVICE_NAME, session_name).context("failed to create keyring entry")?;

    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(e).context("failed to retrieve password from keyring"),
    }
}

/// Delete password for a session from system keyring
pub fn delete_password(session_name: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, session_name).context("failed to create keyring entry")?;

    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()), // Already deleted, OK
        Err(e) => Err(e).context("failed to delete password from keyring"),
    }
}

/// Check if a session has a stored password
pub fn has_password(session_name: &str) -> Result<bool> {
    get_password(session_name).map(|p| p.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_retrieve_password() {
        let session_name = "test_session_store_retrieve";
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
        let session_name = "test_session_nonexistent";

        // Try to get password for non-existent session
        let result = get_password(session_name).unwrap();
        assert_eq!(result, None);
    }

    #[test]
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
        let session_name = "test_session_delete_nonexistent";

        // Deleting a non-existent password should succeed
        delete_password(session_name).unwrap();
    }

    #[test]
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
