use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Password storage mode for unsafe environments where keyring is unavailable
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PasswdUnsafeMode {
    /// Use system keyring (default, existing behavior)
    #[default]
    Normal,
    /// Store password as plaintext in session file
    Bare,
    /// Store password with XOR encoding using a configurable key
    Simple,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthStatus {
    pub has_identity_file: bool,
    pub identity_file_exists: bool,
    pub has_stored_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connected_at: Option<i64>,
    #[serde(default, skip_serializing_if = "should_skip_auth_indicator")]
    pub has_stored_password: bool,
    /// Per-session override for password storage mode. None means inherit from global.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passwd_unsafe_mode: Option<PasswdUnsafeMode>,
    /// Password stored in unsafe format (plaintext for bare, base64-encoded XOR for simple)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stored_password: Option<String>,
}

fn should_skip_auth_indicator(b: &bool) -> bool {
    !b
}

impl Session {
    pub fn target(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }

    #[allow(dead_code)]
    pub fn auth_status(&self) -> AuthStatus {
        let has_key = self
            .identity_file
            .as_ref()
            .map(|p| p.exists())
            .unwrap_or(false);
        AuthStatus {
            has_identity_file: self.identity_file.is_some(),
            identity_file_exists: has_key,
            has_stored_password: self.has_stored_password,
        }
    }
}

/// Root-level wrapper for the sessions.json file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStoreData {
    /// Global password storage mode setting
    #[serde(default)]
    pub passwd_unsafe_mode: PasswdUnsafeMode,
    /// Fallback XOR key if SSHER_UNSAFE_KEY env var not set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passwd_unsafe_key: Option<String>,
    /// List of sessions
    pub sessions: Vec<Session>,
}

impl Default for SessionStoreData {
    fn default() -> Self {
        Self {
            passwd_unsafe_mode: PasswdUnsafeMode::Normal,
            passwd_unsafe_key: None,
            sessions: Vec::new(),
        }
    }
}

impl SessionStoreData {
    /// Create from a list of sessions (for backward compatibility with old array format)
    pub fn from_sessions(sessions: Vec<Session>) -> Self {
        Self {
            passwd_unsafe_mode: PasswdUnsafeMode::Normal,
            passwd_unsafe_key: None,
            sessions,
        }
    }

    /// Get the effective password mode for a session
    pub fn effective_passwd_mode(&self, session: &Session) -> PasswdUnsafeMode {
        session
            .passwd_unsafe_mode
            .clone()
            .unwrap_or_else(|| self.passwd_unsafe_mode.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn target_formats_user_at_host() {
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "alice".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        };
        assert_eq!(session.target(), "alice@example.com");
    }

    #[test]
    fn serialize_full_session() {
        let session = Session {
            name: "office".to_string(),
            host: "office.example.com".to_string(),
            user: "bob".to_string(),
            port: 2222,
            identity_file: Some(PathBuf::from("/home/bob/.ssh/id_rsa")),
            tags: vec!["work".to_string(), "prod".to_string()],
            last_connected_at: Some(1234567890),
            has_stored_password: true,
            passwd_unsafe_mode: None,
            stored_password: None,
        };
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains(r#""name":"office""#));
        assert!(json.contains(r#""host":"office.example.com""#));
        assert!(json.contains(r#""user":"bob""#));
        assert!(json.contains(r#""port":2222"#));
        assert!(json.contains(r#""identity_file":"/home/bob/.ssh/id_rsa"#));
        assert!(json.contains(r#""tags":["work","prod"]"#));
        assert!(json.contains(r#""last_connected_at":1234567890"#));
        assert!(json.contains(r#""has_stored_password":true"#));
    }

    #[test]
    fn deserialize_full_session() {
        let json = r#"{
            "name": "office",
            "host": "office.example.com",
            "user": "bob",
            "port": 2222,
            "identity_file": "/home/bob/.ssh/id_rsa",
            "tags": ["work", "prod"],
            "last_connected_at": 1234567890
        }"#;
        let session: Session = serde_json::from_str(json).unwrap();
        assert_eq!(session.name, "office");
        assert_eq!(session.host, "office.example.com");
        assert_eq!(session.user, "bob");
        assert_eq!(session.port, 2222);
        assert_eq!(
            session.identity_file,
            Some(PathBuf::from("/home/bob/.ssh/id_rsa"))
        );
        assert_eq!(session.tags, vec!["work", "prod"]);
        assert_eq!(session.last_connected_at, Some(1234567890));
    }

    #[test]
    fn deserialize_session_with_defaults() {
        let json = r#"{
            "name": "minimal",
            "host": "example.com",
            "user": "user",
            "port": 22
        }"#;
        let session: Session = serde_json::from_str(json).unwrap();
        assert_eq!(session.name, "minimal");
        assert_eq!(session.identity_file, None);
        assert!(session.tags.is_empty());
        assert_eq!(session.last_connected_at, None);
    }

    #[test]
    fn serialize_roundtrip_preserves_data() {
        let original = Session {
            name: "test".to_string(),
            host: "test.example.com".to_string(),
            user: "tester".to_string(),
            port: 2222,
            identity_file: Some(PathBuf::from("/key")),
            tags: vec!["a".to_string(), "b".to_string()],
            last_connected_at: Some(999),
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn auth_status_with_identity_file() {
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: Some(PathBuf::from("/nonexistent/key")),
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        };

        let status = session.auth_status();
        assert!(status.has_identity_file);
        assert!(!status.identity_file_exists);
        assert!(!status.has_stored_password);
    }

    #[test]
    fn auth_status_with_password_only() {
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: true,
            passwd_unsafe_mode: None,
            stored_password: None,
        };

        let status = session.auth_status();
        assert!(!status.has_identity_file);
        assert!(!status.identity_file_exists);
        assert!(status.has_stored_password);
    }

    #[test]
    fn auth_status_with_both_auth_methods() {
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: Some(PathBuf::from("/existent/key")),
            tags: vec![],
            last_connected_at: None,
            has_stored_password: true,
            passwd_unsafe_mode: None,
            stored_password: None,
        };

        let status = session.auth_status();
        assert!(status.has_identity_file);
        assert!(!status.identity_file_exists); // File doesn't actually exist
        assert!(status.has_stored_password);
    }

    #[test]
    fn has_stored_password_field_is_skipped_when_false() {
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(!json.contains("has_stored_password"));
    }

    #[test]
    fn has_stored_password_field_is_included_when_true() {
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: true,
            passwd_unsafe_mode: None,
            stored_password: None,
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("has_stored_password"));
        assert!(json.contains(r#""has_stored_password":true"#));
    }

    #[test]
    fn passwd_unsafe_mode_default_is_normal() {
        assert_eq!(PasswdUnsafeMode::default(), PasswdUnsafeMode::Normal);
    }

    #[test]
    fn passwd_unsafe_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&PasswdUnsafeMode::Normal).unwrap(),
            r#""normal""#
        );
        assert_eq!(
            serde_json::to_string(&PasswdUnsafeMode::Bare).unwrap(),
            r#""bare""#
        );
        assert_eq!(
            serde_json::to_string(&PasswdUnsafeMode::Simple).unwrap(),
            r#""simple""#
        );
    }

    #[test]
    fn passwd_unsafe_mode_deserialization() {
        assert_eq!(
            serde_json::from_str::<PasswdUnsafeMode>(r#""normal""#).unwrap(),
            PasswdUnsafeMode::Normal
        );
        assert_eq!(
            serde_json::from_str::<PasswdUnsafeMode>(r#""bare""#).unwrap(),
            PasswdUnsafeMode::Bare
        );
        assert_eq!(
            serde_json::from_str::<PasswdUnsafeMode>(r#""simple""#).unwrap(),
            PasswdUnsafeMode::Simple
        );
    }

    #[test]
    fn session_with_unsafe_mode_serialization() {
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: true,
            passwd_unsafe_mode: Some(PasswdUnsafeMode::Bare),
            stored_password: Some("secret".to_string()),
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains(r#""passwd_unsafe_mode":"bare""#));
        assert!(json.contains(r#""stored_password":"secret""#));
    }

    #[test]
    fn session_unsafe_mode_skipped_when_none() {
        let session = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(!json.contains("passwd_unsafe_mode"));
        assert!(!json.contains("stored_password"));
    }

    #[test]
    fn session_store_data_default() {
        let data = SessionStoreData::default();
        assert_eq!(data.passwd_unsafe_mode, PasswdUnsafeMode::Normal);
        assert!(data.passwd_unsafe_key.is_none());
        assert!(data.sessions.is_empty());
    }

    #[test]
    fn session_store_data_from_sessions() {
        let sessions = vec![Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        }];

        let data = SessionStoreData::from_sessions(sessions);
        assert_eq!(data.passwd_unsafe_mode, PasswdUnsafeMode::Normal);
        assert!(data.passwd_unsafe_key.is_none());
        assert_eq!(data.sessions.len(), 1);
    }

    #[test]
    fn effective_passwd_mode_uses_session_override() {
        let data = SessionStoreData {
            passwd_unsafe_mode: PasswdUnsafeMode::Bare,
            ..SessionStoreData::default()
        };

        let session_with_override = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: Some(PasswdUnsafeMode::Simple),
            stored_password: None,
        };

        // Session override takes precedence
        assert_eq!(
            data.effective_passwd_mode(&session_with_override),
            PasswdUnsafeMode::Simple
        );

        let session_without_override = Session {
            name: "test".to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        };

        // Falls back to global
        assert_eq!(
            data.effective_passwd_mode(&session_without_override),
            PasswdUnsafeMode::Bare
        );
    }

    #[test]
    fn session_store_data_serialization() {
        let data = SessionStoreData {
            passwd_unsafe_mode: PasswdUnsafeMode::Simple,
            passwd_unsafe_key: Some("my-key".to_string()),
            sessions: vec![],
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""passwd_unsafe_mode":"simple""#));
        assert!(json.contains(r#""passwd_unsafe_key":"my-key""#));
    }
}
