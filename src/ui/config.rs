use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct UiConfig {
    pub logo: LogoConfig,
    pub layout: LayoutConfig,
    pub theme: ThemeConfig,
    pub input: InputConfig,
    pub ordering: OrderingConfig,
    pub highlights: SessionHighlightConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct InputConfig {
    pub form_default_mode: FormStartMode,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            form_default_mode: FormStartMode::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum FormStartMode {
    #[default]
    Normal,
    Insert,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionOrderMode {
    #[default]
    LatestFirst,
    FrequencyBased,
    Alphabetical,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct SessionLifetimeConfig {
    /// Days after which a session is considered "dying"
    pub dying_threshold_days: u32,
}

impl Default for SessionLifetimeConfig {
    fn default() -> Self {
        Self {
            dying_threshold_days: 7,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct SessionHighlightConfig {
    pub hot: String,
    pub normal: String,
    pub dying: String,
}

impl Default for SessionHighlightConfig {
    fn default() -> Self {
        Self {
            hot: "Yellow".to_string(),
            normal: "Blue".to_string(),
            dying: "DarkGray".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct OrderingConfig {
    pub mode: SessionOrderMode,
    pub lifetime: SessionLifetimeConfig,
}

impl Default for OrderingConfig {
    fn default() -> Self {
        Self {
            mode: SessionOrderMode::LatestFirst,
            lifetime: SessionLifetimeConfig::default(),
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

        // Input config
        assert_eq!(config.input.form_default_mode, FormStartMode::Normal);
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
            },
            "input": {
                "form_default_mode": "insert"
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
        assert_eq!(config.input.form_default_mode, FormStartMode::Insert);
    }

    #[test]
    fn deserialize_ui_config_with_defaults() {
        let json = r#"{"logo": {}, "layout": {}, "theme": {}, "input": {}}"#;
        let config: UiConfig = serde_json::from_str(json).unwrap();
        assert!(config.logo.enabled);
        assert!(config.layout.show_logo);
        assert_eq!(config.theme.logo, "Cyan");
        assert_eq!(config.input.form_default_mode, FormStartMode::Normal);
    }

    #[test]
    fn deserialize_ui_config_empty() {
        let json = r#"{}"#;
        let config: UiConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config, UiConfig::default());
    }

    #[test]
    fn session_order_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&SessionOrderMode::LatestFirst).unwrap(),
            r#""latest_first""#
        );
        assert_eq!(
            serde_json::to_string(&SessionOrderMode::FrequencyBased).unwrap(),
            r#""frequency_based""#
        );
        assert_eq!(
            serde_json::to_string(&SessionOrderMode::Alphabetical).unwrap(),
            r#""alphabetical""#
        );
    }

    #[test]
    fn session_lifetime_config_default_values() {
        let config = SessionLifetimeConfig::default();
        assert_eq!(config.dying_threshold_days, 7);
    }

    #[test]
    fn session_highlight_config_default_values() {
        let config = SessionHighlightConfig::default();
        assert_eq!(config.hot, "Yellow");
        assert_eq!(config.normal, "Blue");
        assert_eq!(config.dying, "DarkGray");
    }

    #[test]
    fn ordering_config_default_values() {
        let config = OrderingConfig::default();
        assert_eq!(config.mode, SessionOrderMode::LatestFirst);
        assert_eq!(config.lifetime.dying_threshold_days, 7);
    }

    #[test]
    fn deserialize_ui_config_with_ordering_and_highlights() {
        let json = r#"{
            "ordering": {
                "mode": "frequency_based",
                "lifetime": {
                    "dying_threshold_days": 14
                }
            },
            "highlights": {
                "hot": "Red",
                "normal": "Green",
                "dying": "White"
            }
        }"#;
        let config: UiConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.ordering.mode, SessionOrderMode::FrequencyBased);
        assert_eq!(config.ordering.lifetime.dying_threshold_days, 14);
        assert_eq!(config.highlights.hot, "Red");
        assert_eq!(config.highlights.normal, "Green");
        assert_eq!(config.highlights.dying, "White");
    }

    #[test]
    fn deserialize_ui_config_ordering_and_highlights_defaults() {
        let json = r#"{"ordering": {}, "highlights": {}}"#;
        let config: UiConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.ordering.mode, SessionOrderMode::LatestFirst);
        assert_eq!(config.ordering.lifetime.dying_threshold_days, 7);
        assert_eq!(config.highlights.hot, "Yellow");
        assert_eq!(config.highlights.normal, "Blue");
        assert_eq!(config.highlights.dying, "DarkGray");
    }
}
