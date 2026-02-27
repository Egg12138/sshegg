use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UiConfig {
    pub logo: LogoConfig,
    pub layout: LayoutConfig,
    pub theme: ThemeConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            logo: LogoConfig::default(),
            layout: LayoutConfig::default(),
            theme: ThemeConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LogoConfig {
    pub enabled: bool,
    pub lines: Vec<String>,
}

impl Default for LogoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            lines: vec![
                "  ____  ____  _   _ ".to_string(),
                " / ___||  _ \\| | | |".to_string(),
                "| |    | |_) | |_| |".to_string(),
                "| |___ |  __/|  _  |".to_string(),
                " \\____||_|   |_| |_|".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LayoutConfig {
    pub show_logo: bool,
    pub show_search: bool,
    pub show_monitor: bool,
    pub show_help: bool,
    pub show_status: bool,
    pub logo_height: u16,
    pub search_height: u16,
    pub monitor_height: u16,
    pub help_height: u16,
    pub status_height: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            show_logo: true,
            show_search: true,
            show_monitor: false,
            show_help: true,
            show_status: true,
            logo_height: 5,
            search_height: 3,
            monitor_height: 5,
            help_height: 2,
            status_height: 1,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ThemeConfig {
    pub logo: String,
    pub header: String,
    pub highlight: String,
    pub border: String,
    pub help: String,
    pub status: String,
    pub text: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            logo: "Cyan".to_string(),
            header: "Yellow".to_string(),
            highlight: "Blue".to_string(),
            border: "DarkGray".to_string(),
            help: "Green".to_string(),
            status: "Magenta".to_string(),
            text: "White".to_string(),
        }
    }
}

pub fn load_ui_config(override_path: Option<PathBuf>) -> Result<UiConfig> {
    let path = resolve_ui_config_path(override_path)?;
    if let Some(path) = path {
        let data = fs::read_to_string(&path)
            .with_context(|| format!("unable to read {}", path.display()))?;
        let config = serde_json::from_str(&data)
            .with_context(|| format!("unable to parse {}", path.display()))?;
        return Ok(config);
    }
    Ok(UiConfig::default())
}

fn resolve_ui_config_path(override_path: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if let Some(path) = override_path {
        return Ok(Some(path));
    }

    let project_dirs = ProjectDirs::from("", "", "ssher")
        .ok_or_else(|| anyhow!("unable to resolve config directory"))?;
    let candidate = project_dirs.config_dir().join("ui.json");
    if candidate.exists() {
        Ok(Some(candidate))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_config_default_values() {
        let config = UiConfig::default();
        // Logo config
        assert!(config.logo.enabled);
        assert_eq!(config.logo.lines.len(), 5);
        assert!(config.logo.lines[0].contains("____"));

        // Layout config
        assert!(config.layout.show_logo);
        assert!(config.layout.show_search);
        assert!(!config.layout.show_monitor);
        assert!(config.layout.show_help);
        assert!(config.layout.show_status);
        assert_eq!(config.layout.logo_height, 5);
        assert_eq!(config.layout.search_height, 3);

        // Theme config
        assert_eq!(config.theme.logo, "Cyan");
        assert_eq!(config.theme.header, "Yellow");
        assert_eq!(config.theme.highlight, "Blue");
        assert_eq!(config.theme.border, "DarkGray");
        assert_eq!(config.theme.help, "Green");
        assert_eq!(config.theme.status, "Magenta");
        assert_eq!(config.theme.text, "White");
    }

    #[test]
    fn logo_config_default_values() {
        let config = LogoConfig::default();
        assert!(config.enabled);
        assert_eq!(config.lines.len(), 5);
    }

    #[test]
    fn layout_config_default_values() {
        let config = LayoutConfig::default();
        assert!(config.show_logo);
        assert!(config.show_search);
        assert!(!config.show_monitor);
        assert!(config.show_help);
        assert!(config.show_status);
        assert_eq!(config.logo_height, 5);
        assert_eq!(config.search_height, 3);
        assert_eq!(config.monitor_height, 5);
        assert_eq!(config.help_height, 2);
        assert_eq!(config.status_height, 1);
    }

    #[test]
    fn theme_config_default_values() {
        let config = ThemeConfig::default();
        assert_eq!(config.logo, "Cyan");
        assert_eq!(config.header, "Yellow");
        assert_eq!(config.highlight, "Blue");
        assert_eq!(config.border, "DarkGray");
        assert_eq!(config.help, "Green");
        assert_eq!(config.status, "Magenta");
        assert_eq!(config.text, "White");
    }

    #[test]
    fn deserialize_ui_config() {
        let json = r#"{
            "logo": {"enabled": false, "lines": ["test"]},
            "layout": {
                "show_logo": false,
                "show_search": false,
                "show_monitor": true,
                "show_help": false,
                "show_status": false,
                "logo_height": 10,
                "search_height": 5,
                "monitor_height": 10,
                "help_height": 3,
                "status_height": 3
            },
            "theme": {
                "logo": "Red",
                "header": "Blue",
                "highlight": "Green",
                "border": "White",
                "help": "Yellow",
                "status": "Cyan",
                "text": "Magenta"
            }
        }"#;
        let config: UiConfig = serde_json::from_str(json).unwrap();
        assert!(!config.logo.enabled);
        assert_eq!(config.logo.lines, vec!["test"]);
        assert!(!config.layout.show_logo);
        assert!(config.layout.show_monitor);
        assert_eq!(config.layout.logo_height, 10);
        assert_eq!(config.theme.logo, "Red");
        assert_eq!(config.theme.header, "Blue");
    }

    #[test]
    fn deserialize_ui_config_with_defaults() {
        let json = r#"{"logo": {}, "layout": {}, "theme": {}}"#;
        let config: UiConfig = serde_json::from_str(json).unwrap();
        assert!(config.logo.enabled);
        assert!(config.layout.show_logo);
        assert_eq!(config.theme.logo, "Cyan");
    }

    #[test]
    fn deserialize_ui_config_empty() {
        let json = r#"{}"#;
        let config: UiConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config, UiConfig::default());
    }
}
