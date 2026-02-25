use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

impl Session {
    pub fn target(&self) -> String {
        format!("{}@{}", self.user, self.host)
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
        };
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains(r#""name":"office""#));
        assert!(json.contains(r#""host":"office.example.com""#));
        assert!(json.contains(r#""user":"bob""#));
        assert!(json.contains(r#""port":2222"#));
        assert!(json.contains(r#""identity_file":"/home/bob/.ssh/id_rsa"#));
        assert!(json.contains(r#""tags":["work","prod"]"#));
        assert!(json.contains(r#""last_connected_at":1234567890"#));
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
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }
}
