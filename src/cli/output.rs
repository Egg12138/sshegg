use crate::model::Session;

pub fn print_sessions(sessions: &[Session]) {
    if sessions.is_empty() {
        println!("No sessions found.");
        return;
    }

    println!("NAME\tTARGET\tPORT\tIDENTITY");
    for session in sessions {
        let identity = session
            .identity_file
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{}\t{}\t{}\t{}",
            session.name,
            session.target(),
            session.port,
            identity
        );
    }
}
