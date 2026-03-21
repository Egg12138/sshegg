use crate::model::Session;
use crate::ui::config::SessionOrderMode;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn sort_sessions(sessions: &mut [Session], mode: SessionOrderMode) {
    match mode {
        SessionOrderMode::LatestFirst => {
            sessions.sort_by(|a, b| {
                let a_time = a.last_connected_at.unwrap_or(0);
                let b_time = b.last_connected_at.unwrap_or(0);
                b_time.cmp(&a_time) // Descending: most recent first
            });
        }
        SessionOrderMode::FrequencyBased => {
            // Count connections per session (using timestamps as proxy)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            // Group sessions by connection patterns
            let mut frequency_map: HashMap<String, usize> = HashMap::new();
            for session in sessions.iter() {
                if let Some(ts) = session.last_connected_at {
                    // Sessions connected in last 24 hours count as "hot"
                    let age_hours = (now - ts) / 3600;
                    let count = frequency_map.entry(session.name.clone()).or_insert(0);
                    if age_hours < 24 {
                        *count += 10; // Weight recent connections higher
                    } else if age_hours < 168 {
                        // 7 days
                        *count += 5;
                    } else {
                        *count += 1;
                    }
                }
            }

            sessions.sort_by(|a, b| {
                let a_freq = frequency_map.get(&a.name).copied().unwrap_or(0);
                let b_freq = frequency_map.get(&b.name).copied().unwrap_or(0);
                b_freq.cmp(&a_freq) // Descending: most frequent first
            });
        }
        SessionOrderMode::Alphabetical => {
            sessions.sort_by(|a, b| a.name.cmp(&b.name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Session;

    fn session(name: &str, ts: Option<i64>) -> Session {
        Session {
            name: name.to_string(),
            host: "example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: ts,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        }
    }

    #[test]
    fn sort_latest_first() {
        let mut sessions = vec![
            session("old", Some(100)),
            session("new", Some(200)),
            session("medium", Some(150)),
        ];
        sort_sessions(&mut sessions, SessionOrderMode::LatestFirst);
        assert_eq!(sessions[0].name, "new");
        assert_eq!(sessions[1].name, "medium");
        assert_eq!(sessions[2].name, "old");
    }

    #[test]
    fn sort_latest_first_with_none() {
        let mut sessions = vec![session("with_time", Some(100)), session("no_time", None)];
        sort_sessions(&mut sessions, SessionOrderMode::LatestFirst);
        assert_eq!(sessions[0].name, "with_time");
        assert_eq!(sessions[1].name, "no_time");
    }

    #[test]
    fn sort_alphabetical() {
        let mut sessions = vec![
            session("zebra", Some(100)),
            session("alpha", Some(200)),
            session("beta", Some(150)),
        ];
        sort_sessions(&mut sessions, SessionOrderMode::Alphabetical);
        assert_eq!(sessions[0].name, "alpha");
        assert_eq!(sessions[1].name, "beta");
        assert_eq!(sessions[2].name, "zebra");
    }

    #[test]
    fn sort_alphabetical_with_empty_list() {
        let mut sessions: Vec<Session> = vec![];
        sort_sessions(&mut sessions, SessionOrderMode::Alphabetical);
        assert!(sessions.is_empty());
    }

    #[test]
    fn sort_latest_first_empty_list() {
        let mut sessions: Vec<Session> = vec![];
        sort_sessions(&mut sessions, SessionOrderMode::LatestFirst);
        assert!(sessions.is_empty());
    }

    #[test]
    fn sort_latest_first_all_none() {
        let mut sessions = vec![
            session("first", None),
            session("second", None),
            session("third", None),
        ];
        sort_sessions(&mut sessions, SessionOrderMode::LatestFirst);
        // All have same value (0), order should be stable
        assert_eq!(sessions.len(), 3);
    }

    #[test]
    fn sort_frequency_based_recent_connections() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let recent = now - 1000; // Very recent
        let older = now - 100000; // Older (but within 7 days)
        let ancient = now - 1000000; // Ancient

        let mut sessions = vec![
            session("ancient", Some(ancient)),
            session("recent", Some(recent)),
            session("older", Some(older)),
        ];
        sort_sessions(&mut sessions, SessionOrderMode::FrequencyBased);
        // recent should be first (score 10), older second (score 5), ancient last (score 1)
        assert_eq!(sessions[0].name, "recent");
        assert_eq!(sessions[1].name, "older");
        assert_eq!(sessions[2].name, "ancient");
    }

    #[test]
    fn sort_frequency_based_with_none() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let recent = now - 1000;

        let mut sessions = vec![session("with_time", Some(recent)), session("no_time", None)];
        sort_sessions(&mut sessions, SessionOrderMode::FrequencyBased);
        // Session with recent connection should come first
        assert_eq!(sessions[0].name, "with_time");
        assert_eq!(sessions[1].name, "no_time");
    }
}
