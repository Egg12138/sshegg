use crate::cli::theme::CliTheme;
use crate::model::Session;
use crossterm::style::Stylize;
use std::io::IsTerminal;

pub fn print_sessions(sessions: &[Session], theme: &CliTheme) {
    if sessions.is_empty() {
        println!("No sessions found.");
        return;
    }

    let use_color = theme.enabled && std::io::stdout().is_terminal();
    println!(
        "{}\t{}\t{}\t{}\t{}",
        colorize("NAME", theme.header, use_color),
        colorize("TARGET", theme.header, use_color),
        colorize("PORT", theme.header, use_color),
        colorize("IDENTITY", theme.header, use_color),
        colorize("TAGS", theme.header, use_color)
    );
    for session in sessions {
        let identity = session
            .identity_file
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string());
        let tags = if session.tags.is_empty() {
            "-".to_string()
        } else {
            session.tags.join(",")
        };
        println!(
            "{}\t{}\t{}\t{}\t{}",
            colorize(&session.name, theme.name, use_color),
            colorize(&session.target(), theme.target, use_color),
            colorize(&session.port.to_string(), theme.port, use_color),
            colorize(&identity, theme.identity, use_color),
            colorize(&tags, theme.tags, use_color)
        );
    }
}

fn colorize(text: &str, color: crossterm::style::Color, enabled: bool) -> String {
    if enabled {
        format!("{}", text.with(color))
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color;

    #[allow(dead_code)]
    fn default_theme() -> CliTheme {
        CliTheme {
            enabled: false,
            header: Color::White,
            name: Color::White,
            target: Color::White,
            port: Color::White,
            identity: Color::White,
            tags: Color::White,
        }
    }

    #[allow(dead_code)]
    fn session(name: &str, host: &str, user: &str, port: u16) -> Session {
        Session {
            name: name.to_string(),
            host: host.to_string(),
            user: user.to_string(),
            port,
            identity_file: None,
            tags: vec![],
            last_connected_at: None,
        }
    }

    #[test]
    fn colorize_disabled_returns_plain_text() {
        assert_eq!(colorize("test", Color::Cyan, false), "test");
    }

    #[test]
    fn colorize_enabled_returns_ansi_colored() {
        let result = colorize("test", Color::Cyan, true);
        // ANSI escape sequences should be present
        assert!(result.contains("\x1b[")); // CSI sequence
        assert!(result.contains("test"));
    }

    // Note: Testing print_sessions is difficult as it prints to stdout
    // The function is simple enough that manual testing covers the main cases
}
