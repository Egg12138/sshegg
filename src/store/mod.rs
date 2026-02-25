mod path;

use crate::model::Session;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::PathBuf;

pub use path::resolve_store_path;

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
}
