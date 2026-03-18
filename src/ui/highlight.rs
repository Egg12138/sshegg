use crate::model::Session;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionHighlight {
    Hot,
    Normal,
    Dying,
}

impl SessionHighlight {
    pub fn classify(session: &Session, dying_threshold_days: u32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let Some(last_connected) = session.last_connected_at else {
            // Sessions never connected are "dying"
            return SessionHighlight::Dying;
        };

        let age_seconds = now - last_connected;
        let age_days = age_seconds / (24 * 3600);

        if age_days > dying_threshold_days as i64 {
            SessionHighlight::Dying
        } else if age_days <= 1 {
            // Connected within last day = hot
            SessionHighlight::Hot
        } else {
            SessionHighlight::Normal
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
        }
    }

    #[test]
    fn classify_hot_session() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let one_hour_ago = now - 3600;

        let s = session("hot", Some(one_hour_ago));
        assert_eq!(SessionHighlight::classify(&s, 7), SessionHighlight::Hot);
    }

    #[test]
    fn classify_normal_session() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let three_days_ago = now - (3 * 24 * 3600);

        let s = session("normal", Some(three_days_ago));
        assert_eq!(SessionHighlight::classify(&s, 7), SessionHighlight::Normal);
    }

    #[test]
    fn classify_dying_session() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let ten_days_ago = now - (10 * 24 * 3600);

        let s = session("dying", Some(ten_days_ago));
        assert_eq!(SessionHighlight::classify(&s, 7), SessionHighlight::Dying);
    }

    #[test]
    fn classify_never_connected() {
        let s = session("never", None);
        assert_eq!(SessionHighlight::classify(&s, 7), SessionHighlight::Dying);
    }

    #[test]
    fn classify_custom_threshold() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let five_days_ago = now - (5 * 24 * 3600);

        let s = session("custom", Some(five_days_ago));
        // With 3-day threshold, 5 days is dying
        assert_eq!(SessionHighlight::classify(&s, 3), SessionHighlight::Dying);
        // With 7-day threshold, 5 days is normal
        assert_eq!(SessionHighlight::classify(&s, 7), SessionHighlight::Normal);
    }
}
