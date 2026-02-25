use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CliThemeConfig {
    pub enabled: bool,
    pub header: String,
    pub name: String,
    pub target: String,
    pub port: String,
    pub identity: String,
    pub tags: String,
}

impl Default for CliThemeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            header: "Yellow".to_string(),
            name: "Cyan".to_string(),
            target: "Green".to_string(),
            port: "Magenta".to_string(),
            identity: "Blue".to_string(),
            tags: "DarkGray".to_string(),
        }
    }
}

pub struct CliTheme {
    pub enabled: bool,
    pub header: crossterm::style::Color,
    pub name: crossterm::style::Color,
    pub target: crossterm::style::Color,
    pub port: crossterm::style::Color,
    pub identity: crossterm::style::Color,
    pub tags: crossterm::style::Color,
}

impl CliTheme {
    fn from_config(config: CliThemeConfig) -> Self {
        Self {
            enabled: config.enabled,
            header: parse_color(&config.header),
            name: parse_color(&config.name),
            target: parse_color(&config.target),
            port: parse_color(&config.port),
            identity: parse_color(&config.identity),
            tags: parse_color(&config.tags),
        }
    }
}

pub fn load_cli_theme(override_path: Option<PathBuf>) -> Result<CliTheme> {
    let path = resolve_cli_theme_path(override_path)?;
    if let Some(path) = path {
        let data = fs::read_to_string(&path)
            .with_context(|| format!("unable to read {}", path.display()))?;
        let config = serde_json::from_str(&data)
            .with_context(|| format!("unable to parse {}", path.display()))?;
        return Ok(CliTheme::from_config(config));
    }
    Ok(CliTheme::from_config(CliThemeConfig::default()))
}

fn resolve_cli_theme_path(override_path: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if let Some(path) = override_path {
        return Ok(Some(path));
    }

    let project_dirs = ProjectDirs::from("", "", "ssher")
        .ok_or_else(|| anyhow!("unable to resolve config directory"))?;
    let candidate = project_dirs.config_dir().join("cli.json");
    if candidate.exists() {
        Ok(Some(candidate))
    } else {
        Ok(None)
    }
}

fn parse_color(name: &str) -> crossterm::style::Color {
    match name.to_lowercase().as_str() {
        "black" => crossterm::style::Color::Black,
        "red" => crossterm::style::Color::DarkRed,
        "green" => crossterm::style::Color::DarkGreen,
        "yellow" => crossterm::style::Color::DarkYellow,
        "blue" => crossterm::style::Color::DarkBlue,
        "magenta" => crossterm::style::Color::DarkMagenta,
        "cyan" => crossterm::style::Color::DarkCyan,
        "gray" => crossterm::style::Color::Grey,
        "darkgray" | "dark_gray" => crossterm::style::Color::DarkGrey,
        "lightred" | "light_red" => crossterm::style::Color::Red,
        "lightgreen" | "light_green" => crossterm::style::Color::Green,
        "lightyellow" | "light_yellow" => crossterm::style::Color::Yellow,
        "lightblue" | "light_blue" => crossterm::style::Color::Blue,
        "lightmagenta" | "light_magenta" => crossterm::style::Color::Magenta,
        "lightcyan" | "light_cyan" => crossterm::style::Color::Cyan,
        "white" => crossterm::style::Color::White,
        _ => crossterm::style::Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_theme_config_default_values() {
        let config = CliThemeConfig::default();
        assert!(config.enabled);
        assert_eq!(config.header, "Yellow");
        assert_eq!(config.name, "Cyan");
        assert_eq!(config.target, "Green");
        assert_eq!(config.port, "Magenta");
        assert_eq!(config.identity, "Blue");
        assert_eq!(config.tags, "DarkGray");
    }

    #[test]
    fn cli_theme_from_config() {
        let config = CliThemeConfig {
            enabled: false,
            header: "Red".to_string(),
            name: "Blue".to_string(),
            target: "Green".to_string(),
            port: "Yellow".to_string(),
            identity: "Cyan".to_string(),
            tags: "White".to_string(),
        };
        let theme = CliTheme::from_config(config);
        assert!(!theme.enabled);
        assert_eq!(theme.header, crossterm::style::Color::DarkRed);
        assert_eq!(theme.name, crossterm::style::Color::DarkBlue);
        assert_eq!(theme.target, crossterm::style::Color::DarkGreen);
        assert_eq!(theme.port, crossterm::style::Color::DarkYellow);
        assert_eq!(theme.identity, crossterm::style::Color::DarkCyan);
        assert_eq!(theme.tags, crossterm::style::Color::White);
    }

    #[test]
    fn parse_color_basic_colors() {
        assert_eq!(parse_color("black"), crossterm::style::Color::Black);
        assert_eq!(parse_color("red"), crossterm::style::Color::DarkRed);
        assert_eq!(parse_color("green"), crossterm::style::Color::DarkGreen);
        assert_eq!(parse_color("yellow"), crossterm::style::Color::DarkYellow);
        assert_eq!(parse_color("blue"), crossterm::style::Color::DarkBlue);
        assert_eq!(parse_color("magenta"), crossterm::style::Color::DarkMagenta);
        assert_eq!(parse_color("cyan"), crossterm::style::Color::DarkCyan);
        assert_eq!(parse_color("white"), crossterm::style::Color::White);
    }

    #[test]
    fn parse_color_case_insensitive() {
        assert_eq!(parse_color("RED"), crossterm::style::Color::DarkRed);
        assert_eq!(parse_color("Red"), crossterm::style::Color::DarkRed);
        assert_eq!(parse_color("rEd"), crossterm::style::Color::DarkRed);
    }

    #[test]
    fn parse_color_underscore_variants() {
        assert_eq!(parse_color("light_red"), crossterm::style::Color::Red);
        assert_eq!(parse_color("lightred"), crossterm::style::Color::Red);
        assert_eq!(parse_color("dark_gray"), crossterm::style::Color::DarkGrey);
        assert_eq!(parse_color("darkgray"), crossterm::style::Color::DarkGrey);
    }

    #[test]
    fn parse_color_invalid_defaults_to_white() {
        assert_eq!(parse_color("invalidcolor"), crossterm::style::Color::White);
        assert_eq!(parse_color(""), crossterm::style::Color::White);
        assert_eq!(parse_color("notarecolor"), crossterm::style::Color::White);
    }

    #[test]
    fn parse_color_light_variants() {
        assert_eq!(parse_color("lightred"), crossterm::style::Color::Red);
        assert_eq!(parse_color("lightgreen"), crossterm::style::Color::Green);
        assert_eq!(parse_color("lightyellow"), crossterm::style::Color::Yellow);
        assert_eq!(parse_color("lightblue"), crossterm::style::Color::Blue);
        assert_eq!(
            parse_color("lightmagenta"),
            crossterm::style::Color::Magenta
        );
        assert_eq!(parse_color("lightcyan"), crossterm::style::Color::Cyan);
    }
}
