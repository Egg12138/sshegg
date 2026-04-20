mod path;

use crate::model::{PasswdUnsafeMode, Session, SessionStoreData};
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::PathBuf;

pub use path::resolve_store_path;

pub trait SessionStore {
    fn add(&self, session: Session) -> Result<()>;
    fn update(&self, session: Session) -> Result<()>;
    fn list(&self) -> Result<Vec<Session>>;
    fn remove(&self, name: &str) -> Result<()>;
    fn touch_last_connected(&self, name: &str, timestamp: i64) -> Result<()>;
    fn get_config(&self) -> Result<StoreConfig>;
    fn set_config(&self, config: &StoreConfig) -> Result<()>;
}

/// Configuration values that can be read/modified at the store level
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreConfig {
    pub passwd_unsafe_mode: PasswdUnsafeMode,
    pub passwd_unsafe_key: Option<String>,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            passwd_unsafe_mode: PasswdUnsafeMode::Normal,
            passwd_unsafe_key: None,
        }
    }
}

pub struct JsonFileStore {
    path: PathBuf,
}

impl JsonFileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn add(&self, session: Session) -> Result<()> {
        let mut data = self.load_full()?;
        if data
            .sessions
            .iter()
            .any(|existing| existing.name == session.name)
        {
            return Err(anyhow!("session '{}' already exists", session.name));
        }
        data.sessions.push(session);
        self.save(&data)
    }

    pub fn update(&self, session: Session) -> Result<()> {
        let mut data = self.load_full()?;
        if let Some(existing) = data.sessions.iter_mut().find(|s| s.name == session.name) {
            *existing = session;
            self.save(&data)
        } else {
            Err(anyhow!("session '{}' not found", session.name))
        }
    }

    pub fn list(&self) -> Result<Vec<Session>> {
        let mut data = self.load_full()?;
        data.sessions.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(data.sessions)
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let mut data = self.load_full()?;
        let before = data.sessions.len();
        data.sessions.retain(|session| session.name != name);
        if data.sessions.len() == before {
            return Err(anyhow!("session '{}' not found", name));
        }
        self.save(&data)
    }

    pub fn touch_last_connected(&self, name: &str, timestamp: i64) -> Result<()> {
        let mut data = self.load_full()?;
        let mut found = false;
        for session in &mut data.sessions {
            if session.name == name {
                session.last_connected_at = Some(timestamp);
                found = true;
                break;
            }
        }
        if !found {
            return Err(anyhow!("session '{}' not found", name));
        }
        self.save(&data)
    }

    /// Load the full store data including config
    fn load_full(&self) -> Result<SessionStoreData> {
        if !self.path.exists() {
            return Ok(SessionStoreData::default());
        }
        let data = fs::read_to_string(&self.path)
            .with_context(|| format!("unable to read store {}", self.path.display()))?;
        if data.trim().is_empty() {
            return Ok(SessionStoreData::default());
        }

        // Try to parse as new format first
        let parsed: serde_json::Value = serde_json::from_str(&data)
            .with_context(|| format!("unable to parse store {}", self.path.display()))?;

        // Check if it's the new format (has "sessions" key at root)
        if let Some(obj) = parsed.as_object()
            && obj.contains_key("sessions")
        {
            let store_data: SessionStoreData = serde_json::from_value(parsed)
                .with_context(|| format!("unable to parse store {}", self.path.display()))?;
            return Ok(store_data);
        }

        // Old format: root is an array of sessions - migrate it
        let sessions: Vec<Session> = serde_json::from_str(&data)
            .with_context(|| format!("unable to parse store {}", self.path.display()))?;
        Ok(SessionStoreData::from_sessions(sessions))
    }

    fn save(&self, data: &SessionStoreData) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("unable to create store directory {}", parent.display())
            })?;
        }
        let json = serde_json::to_string_pretty(data).context("unable to serialize sessions")?;
        fs::write(&self.path, json)
            .with_context(|| format!("unable to write store {}", self.path.display()))?;
        Ok(())
    }

    /// Get the current config
    pub fn get_config(&self) -> Result<StoreConfig> {
        let data = self.load_full()?;
        Ok(StoreConfig {
            passwd_unsafe_mode: data.passwd_unsafe_mode,
            passwd_unsafe_key: data.passwd_unsafe_key,
        })
    }

    /// Update the config
    pub fn set_config(&self, config: &StoreConfig) -> Result<()> {
        let mut data = self.load_full()?;
        data.passwd_unsafe_mode = config.passwd_unsafe_mode.clone();
        data.passwd_unsafe_key = config.passwd_unsafe_key.clone();
        self.save(&data)
    }
}

impl SessionStore for JsonFileStore {
    fn add(&self, session: Session) -> Result<()> {
        JsonFileStore::add(self, session)
    }

    fn update(&self, session: Session) -> Result<()> {
        JsonFileStore::update(self, session)
    }

    fn list(&self) -> Result<Vec<Session>> {
        JsonFileStore::list(self)
    }

    fn remove(&self, name: &str) -> Result<()> {
        JsonFileStore::remove(self, name)
    }

    fn touch_last_connected(&self, name: &str, timestamp: i64) -> Result<()> {
        JsonFileStore::touch_last_connected(self, name, timestamp)
    }

    fn get_config(&self) -> Result<StoreConfig> {
        JsonFileStore::get_config(self)
    }

    fn set_config(&self, config: &StoreConfig) -> Result<()> {
        JsonFileStore::set_config(self, config)
    }
}

#[cfg(test)]
mod tests {
    use super::JsonFileStore;
    use super::StoreConfig;
    use crate::model::{PasswdUnsafeMode, Session};
    use tempfile::tempdir;

    fn sample_session(name: &str) -> Session {
        Session {
            name: name.to_string(),
            host: "example.com".to_string(),
            user: "me".to_string(),
            port: 22,
            identity_file: None,
            tags: Vec::new(),
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        }
    }

    #[test]
    fn add_and_remove_session() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        store.add(sample_session("office")).expect("add");
        let list = store.list().expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "office");

        store.remove("office").expect("remove");
        let list = store.list().expect("list");
        assert!(list.is_empty());
    }

    #[test]
    fn list_sorts_by_name() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        store.add(sample_session("zeta")).expect("add");
        store.add(sample_session("alpha")).expect("add");

        let list = store.list().expect("list");
        assert_eq!(list[0].name, "alpha");
        assert_eq!(list[1].name, "zeta");
    }

    #[test]
    fn touch_last_connected_updates_timestamp() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        store.add(sample_session("office")).expect("add");
        store.touch_last_connected("office", 1234).expect("touch");

        let list = store.list().expect("list");
        assert_eq!(list[0].last_connected_at, Some(1234));
    }

    #[test]
    fn load_returns_empty_for_nonexistent_file() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("nonexistent.json");
        let store = JsonFileStore::new(store_path);

        let list = store.list().expect("list");
        assert!(list.is_empty());
    }

    #[test]
    fn load_returns_empty_for_empty_file() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("empty.json");
        std::fs::write(&store_path, "").expect("write");
        let store = JsonFileStore::new(store_path);

        let list = store.list().expect("list");
        assert!(list.is_empty());
    }

    #[test]
    fn load_returns_empty_for_whitespace_only_file() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("whitespace.json");
        std::fs::write(&store_path, "   \n\t  ").expect("write");
        let store = JsonFileStore::new(store_path);

        let list = store.list().expect("list");
        assert!(list.is_empty());
    }

    #[test]
    fn load_fails_for_invalid_json() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("invalid.json");
        std::fs::write(&store_path, "not valid json {").expect("write");
        let store = JsonFileStore::new(store_path);

        let result = store.list();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unable to parse store") || err.contains("expected"));
    }

    #[test]
    fn add_duplicate_name_fails() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        store.add(sample_session("office")).expect("add");
        let result = store.add(sample_session("office"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn remove_nonexistent_session_fails() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        let result = store.remove("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
    }

    #[test]
    fn touch_last_connected_nonexistent_fails() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        let result = store.touch_last_connected("nonexistent", 1234);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("nested/dir/sessions.json");
        let store = JsonFileStore::new(store_path.clone());

        store.add(sample_session("office")).expect("add");
        assert!(store_path.exists());
        assert!(store_path.parent().unwrap().exists());
    }

    #[test]
    fn update_existing_session() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        store.add(sample_session("office")).expect("add");

        let mut updated = sample_session("office");
        updated.host = "newhost.example.com".to_string();
        updated.port = 2222;
        store.update(updated).expect("update");

        let list = store.list().expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].host, "newhost.example.com");
        assert_eq!(list[0].port, 2222);
    }

    #[test]
    fn update_nonexistent_session_fails() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        let result = store.update(sample_session("nonexistent"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
    }

    #[test]
    fn update_preserves_other_sessions() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        store.add(sample_session("office")).expect("add");
        store.add(sample_session("home")).expect("add");

        let mut updated = sample_session("office");
        updated.host = "newhost.example.com".to_string();
        store.update(updated).expect("update");

        let list = store.list().expect("list");
        assert_eq!(list.len(), 2);
        let office = list.iter().find(|s| s.name == "office").expect("office");
        assert_eq!(office.host, "newhost.example.com");
        let home = list.iter().find(|s| s.name == "home").expect("home");
        assert_eq!(home.host, "example.com");
    }

    #[test]
    fn get_config_returns_default_for_new_store() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        let config = store.get_config().expect("get_config");
        assert_eq!(config.passwd_unsafe_mode, PasswdUnsafeMode::Normal);
        assert!(config.passwd_unsafe_key.is_none());
    }

    #[test]
    fn set_config_updates_store() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        let config = StoreConfig {
            passwd_unsafe_mode: PasswdUnsafeMode::Bare,
            passwd_unsafe_key: Some("my-key".to_string()),
        };
        store.set_config(&config).expect("set_config");

        let loaded = store.get_config().expect("get_config");
        assert_eq!(loaded.passwd_unsafe_mode, PasswdUnsafeMode::Bare);
        assert_eq!(loaded.passwd_unsafe_key, Some("my-key".to_string()));
    }

    #[test]
    fn migrates_old_array_format() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");

        // Write old format (array at root)
        let old_format = r#"[{"name":"office","host":"example.com","user":"me","port":22}]"#;
        std::fs::write(&store_path, old_format).expect("write");

        let store = JsonFileStore::new(store_path);
        let list = store.list().expect("list");

        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "office");

        // Config should be default
        let config = store.get_config().expect("get_config");
        assert_eq!(config.passwd_unsafe_mode, PasswdUnsafeMode::Normal);
    }

    #[test]
    fn saves_new_format_with_config() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path.clone());

        // Set config
        let config = StoreConfig {
            passwd_unsafe_mode: PasswdUnsafeMode::Simple,
            passwd_unsafe_key: Some("secret".to_string()),
        };
        store.set_config(&config).expect("set_config");

        // Add session
        store.add(sample_session("office")).expect("add");

        // Read raw file and verify format
        let content = std::fs::read_to_string(&store_path).expect("read");
        assert!(content.contains(r#""passwd_unsafe_mode": "simple""#));
        assert!(content.contains(r#""passwd_unsafe_key": "secret""#));
        assert!(content.contains(r#""sessions""#));
        assert!(content.contains(r#""name": "office""#));
    }

    #[test]
    fn config_persists_across_operations() {
        let dir = tempdir().expect("tempdir");
        let store_path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(store_path);

        // Set config
        let config = StoreConfig {
            passwd_unsafe_mode: PasswdUnsafeMode::Bare,
            passwd_unsafe_key: None,
        };
        store.set_config(&config).expect("set_config");

        // Add session
        store.add(sample_session("office")).expect("add");

        // Config should still be the same
        let loaded = store.get_config().expect("get_config");
        assert_eq!(loaded.passwd_unsafe_mode, PasswdUnsafeMode::Bare);
    }
}
