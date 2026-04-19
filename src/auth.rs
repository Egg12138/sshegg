use crate::model::{PasswdUnsafeMode, Session};
use crate::password;
use crate::store::SessionStore;
use anyhow::Result;

pub fn resolve_session_password(
    store: &dyn SessionStore,
    session: &Session,
) -> Result<Option<String>> {
    if !session.has_stored_password {
        return Ok(None);
    }

    let config = store.get_config()?;
    let effective_mode = session
        .passwd_unsafe_mode
        .as_ref()
        .unwrap_or(&config.passwd_unsafe_mode);

    match effective_mode {
        PasswdUnsafeMode::Normal => password::get_password(&session.name),
        PasswdUnsafeMode::Bare => Ok(session.stored_password.clone()),
        PasswdUnsafeMode::Simple => match &session.stored_password {
            Some(encoded) => password::get_unsafe_password(
                encoded,
                &PasswdUnsafeMode::Simple,
                config.passwd_unsafe_key.as_deref(),
            )
            .map(Some),
            None => Ok(None),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SessionStoreData;
    use crate::store::JsonFileStore;
    use tempfile::tempdir;

    fn sample_session() -> Session {
        Session {
            name: "office".to_string(),
            host: "example.com".to_string(),
            user: "alice".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
            has_stored_password: true,
            passwd_unsafe_mode: Some(PasswdUnsafeMode::Bare),
            stored_password: Some("secret".to_string()),
        }
    }

    #[test]
    fn resolve_session_password_reads_bare_passwords_from_store() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("sessions.json");
        let store = JsonFileStore::new(path);
        let session = sample_session();
        let data = SessionStoreData::from_sessions(vec![session.clone()]);
        std::fs::write(
            dir.path().join("sessions.json"),
            serde_json::to_string_pretty(&data).expect("serialize"),
        )
        .expect("write store");

        let password = resolve_session_password(&store, &session).expect("resolve");
        assert_eq!(password.as_deref(), Some("secret"));
    }
}
