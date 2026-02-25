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
}
