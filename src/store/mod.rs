mod path;

use crate::model::Session;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::PathBuf;

pub use path::resolve_store_path;

pub trait SessionStore {
    fn add(&self, session: Session) -> Result<()>;
    fn list(&self) -> Result<Vec<Session>>;
    fn remove(&self, name: &str) -> Result<()>;
    fn touch_last_connected(&self, name: &str, timestamp: i64) -> Result<()>;
}

pub struct JsonFileStore {
    path: PathBuf,
}

impl JsonFileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn add(&self, session: Session) -> Result<()> {
        let mut sessions = self.load()?;
        if sessions
            .iter()
            .any(|existing| existing.name == session.name)
        {
            return Err(anyhow!("session '{}' already exists", session.name));
        }
        sessions.push(session);
        self.save(&sessions)
    }

    pub fn list(&self) -> Result<Vec<Session>> {
        let mut sessions = self.load()?;
        sessions.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(sessions)
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let mut sessions = self.load()?;
        let before = sessions.len();
        sessions.retain(|session| session.name != name);
        if sessions.len() == before {
            return Err(anyhow!("session '{}' not found", name));
        }
        self.save(&sessions)
    }

    pub fn touch_last_connected(&self, name: &str, timestamp: i64) -> Result<()> {
        let mut sessions = self.load()?;
        let mut found = false;
        for session in &mut sessions {
            if session.name == name {
                session.last_connected_at = Some(timestamp);
                found = true;
                break;
            }
        }
        if !found {
            return Err(anyhow!("session '{}' not found", name));
        }
        self.save(&sessions)
    }

    fn load(&self) -> Result<Vec<Session>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read_to_string(&self.path)
            .with_context(|| format!("unable to read store {}", self.path.display()))?;
        if data.trim().is_empty() {
            return Ok(Vec::new());
        }
        let sessions = serde_json::from_str(&data)
            .with_context(|| format!("unable to parse store {}", self.path.display()))?;
        Ok(sessions)
    }

    fn save(&self, sessions: &[Session]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("unable to create store directory {}", parent.display())
            })?;
        }
        let data =
            serde_json::to_string_pretty(sessions).context("unable to serialize sessions")?;
        fs::write(&self.path, data)
            .with_context(|| format!("unable to write store {}", self.path.display()))?;
        Ok(())
    }
}

impl SessionStore for JsonFileStore {
    fn add(&self, session: Session) -> Result<()> {
        JsonFileStore::add(self, session)
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
}

#[cfg(test)]
mod tests {
    use super::JsonFileStore;
    use crate::model::Session;
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
}
