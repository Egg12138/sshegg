use crate::model::Session;

pub fn filter_sessions(sessions: &[Session], filter: &str) -> Vec<usize> {
    if filter.trim().is_empty() {
        return (0..sessions.len()).collect();
    }

    let needle = filter.to_lowercase();
    sessions
        .iter()
        .enumerate()
        .filter(|(_, session)| session_matches(session, &needle))
        .map(|(index, _)| index)
        .collect()
}

fn session_matches(session: &Session, needle: &str) -> bool {
    let name = session.name.to_lowercase();
    let host = session.host.to_lowercase();
    let user = session.user.to_lowercase();

    if name.contains(needle) || host.contains(needle) || user.contains(needle) {
        return true;
    }

    if let Some(identity) = &session.identity_file {
        let identity_str = identity.to_string_lossy().to_lowercase();
        if identity_str.contains(needle) {
            return true;
        }
    }

    if session
        .tags
        .iter()
        .any(|tag| tag.to_lowercase().contains(needle))
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::filter_sessions;
    use crate::model::Session;
    use std::path::PathBuf;

    fn session(name: &str, host: &str, user: &str, identity: Option<&str>) -> Session {
        Session {
            name: name.to_string(),
            host: host.to_string(),
            user: user.to_string(),
            port: 22,
            identity_file: identity.map(PathBuf::from),
            tags: Vec::new(),
            last_connected_at: None,
        }
    }

    #[test]
    fn filter_matches_name_host_user() {
        let sessions = vec![
            session("office", "office.example.com", "me", None),
            session("prod", "prod.example.com", "deploy", None),
        ];
        assert_eq!(filter_sessions(&sessions, "office"), vec![0]);
        assert_eq!(filter_sessions(&sessions, "deploy"), vec![1]);
        assert_eq!(filter_sessions(&sessions, "example"), vec![0, 1]);
    }

    #[test]
    fn filter_matches_identity_path() {
        let sessions = vec![
            session(
                "office",
                "office.example.com",
                "me",
                Some("/home/me/.ssh/id_rsa"),
            ),
            session("lab", "lab.example.com", "me", None),
        ];
        assert_eq!(filter_sessions(&sessions, "id_rsa"), vec![0]);
    }

    #[test]
    fn filter_matches_tags() {
        let mut tagged = session("office", "office.example.com", "me", None);
        tagged.tags = vec!["prod".to_string(), "critical".to_string()];
        let sessions = vec![tagged, session("lab", "lab.example.com", "me", None)];
        assert_eq!(filter_sessions(&sessions, "critical"), vec![0]);
    }

    #[test]
    fn empty_filter_returns_all_indices() {
        let sessions = vec![
            session("office", "office.example.com", "me", None),
            session("prod", "prod.example.com", "deploy", None),
        ];
        assert_eq!(filter_sessions(&sessions, ""), vec![0, 1]);
    }

    #[test]
    fn whitespace_filter_returns_all_indices() {
        let sessions = vec![
            session("office", "office.example.com", "me", None),
            session("prod", "prod.example.com", "deploy", None),
        ];
        assert_eq!(filter_sessions(&sessions, "   "), vec![0, 1]);
        assert_eq!(filter_sessions(&sessions, "\t"), vec![0, 1]);
    }

    #[test]
    fn filter_no_matches_returns_empty() {
        let sessions = vec![
            session("office", "office.example.com", "me", None),
            session("prod", "prod.example.com", "deploy", None),
        ];
        assert!(filter_sessions(&sessions, "nonexistent").is_empty());
    }

    #[test]
    fn filter_case_insensitive() {
        let sessions = vec![session("Office", "office.example.com", "Me", None)];
        assert_eq!(filter_sessions(&sessions, "OFFICE"), vec![0]);
        assert_eq!(filter_sessions(&sessions, "office"), vec![0]);
        assert_eq!(filter_sessions(&sessions, "Me"), vec![0]);
        assert_eq!(filter_sessions(&sessions, "ME"), vec![0]);
    }

    #[test]
    fn filter_unicode_characters() {
        let sessions = vec![session("сервер", "пример.ком", "пользователь", None)];
        assert_eq!(filter_sessions(&sessions, "сервер"), vec![0]);
        assert_eq!(filter_sessions(&sessions, "пример"), vec![0]);
    }

    #[test]
    fn filter_empty_sessions_list() {
        let sessions: Vec<Session> = vec![];
        assert!(filter_sessions(&sessions, "anything").is_empty());
        assert!(filter_sessions(&sessions, "").is_empty());
    }

    #[test]
    fn filter_special_characters_in_identity() {
        let mut s = session(
            "office",
            "office.example.com",
            "me",
            Some("/home/user/.ssh/id-ed25519"),
        );
        s.tags = vec!["key-2024".to_string()];
        let sessions = vec![s];
        assert_eq!(filter_sessions(&sessions, "ed25519"), vec![0]);
        assert_eq!(filter_sessions(&sessions, "2024"), vec![0]);
    }
}
