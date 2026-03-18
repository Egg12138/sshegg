use ssher::model::Session;
use ssher::ui::{
    OrderingConfig, SessionHighlight, SessionHighlightConfig, SessionOrderMode, sort_sessions,
};

#[test]
fn test_sort_sessions_latest_first() {
    let mut sessions = vec![
        Session {
            name: "old".to_string(),
            host: "old.example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: Some(100),
            has_stored_password: false,
        },
        Session {
            name: "new".to_string(),
            host: "new.example.com".to_string(),
            user: "user".to_string(),
            port: 22,
            identity_file: None,
            tags: vec![],
            last_connected_at: Some(200),
            has_stored_password: false,
        },
    ];

    sort_sessions(&mut sessions, SessionOrderMode::LatestFirst);
    assert_eq!(sessions[0].name, "new");
    assert_eq!(sessions[1].name, "old");
}

#[test]
fn test_session_highlight_classify() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Hot session (connected 1 hour ago)
    let hot_session = Session {
        name: "hot".to_string(),
        host: "example.com".to_string(),
        user: "user".to_string(),
        port: 22,
        identity_file: None,
        tags: vec![],
        last_connected_at: Some(now - 3600),
        has_stored_password: false,
    };
    assert_eq!(
        SessionHighlight::classify(&hot_session, 7),
        SessionHighlight::Hot
    );

    // Normal session (connected 3 days ago)
    let normal_session = Session {
        name: "normal".to_string(),
        host: "example.com".to_string(),
        user: "user".to_string(),
        port: 22,
        identity_file: None,
        tags: vec![],
        last_connected_at: Some(now - (3 * 24 * 3600)),
        has_stored_password: false,
    };
    assert_eq!(
        SessionHighlight::classify(&normal_session, 7),
        SessionHighlight::Normal
    );

    // Dying session (connected 10 days ago)
    let dying_session = Session {
        name: "dying".to_string(),
        host: "example.com".to_string(),
        user: "user".to_string(),
        port: 22,
        identity_file: None,
        tags: vec![],
        last_connected_at: Some(now - (10 * 24 * 3600)),
        has_stored_password: false,
    };
    assert_eq!(
        SessionHighlight::classify(&dying_session, 7),
        SessionHighlight::Dying
    );
}

#[test]
fn test_session_highlight_config_defaults() {
    let config = SessionHighlightConfig::default();
    assert_eq!(config.hot, "Yellow");
    assert_eq!(config.normal, "Blue");
    assert_eq!(config.dying, "DarkGray");
}

#[test]
fn test_ordering_config_defaults() {
    let config = OrderingConfig::default();
    assert_eq!(config.mode, SessionOrderMode::LatestFirst);
    assert_eq!(config.lifetime.dying_threshold_days, 7);
}
