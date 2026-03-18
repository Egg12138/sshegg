use crate::cli::theme::CliThemeConfig;
use crate::ui::{ThemeConfig, UiConfig};
use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ThemePreset {
    pub name: String,
    pub description: String,
    pub cli: CliThemeConfig,
    pub ui: ThemeConfig,
}

/// Get the themes directory
/// Checks assets/themes first, then system config directory
pub fn get_themes_dir() -> Result<PathBuf> {
    // First check relative to the binary (assets/themes in the repo)
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        // Check for assets/themes next to the binary
        let repo_themes = parent
            .parent()
            .map(|p| p.join("assets/themes"))
            .filter(|p| p.exists());
        if let Some(path) = repo_themes {
            return Ok(path);
        }
    }

    // Fall back to system config directory
    let project_dirs = ProjectDirs::from("", "", "ssher")
        .ok_or_else(|| anyhow!("unable to resolve config directory"))?;
    let system_themes = project_dirs.config_dir().join("themes");

    if system_themes.exists() {
        return Ok(system_themes);
    }

    // Final fallback: check if we're in the repo
    let current_dir = std::env::current_dir()?;
    let repo_themes = current_dir.join("assets/themes");
    if repo_themes.exists() {
        return Ok(repo_themes);
    }

    Err(anyhow!("themes directory not found"))
}

/// Load all available theme JSON files
pub fn load_available_themes() -> Result<Vec<ThemePreset>> {
    let themes_dir = get_themes_dir()?;
    let mut themes = Vec::new();

    let entries = fs::read_dir(&themes_dir)
        .with_context(|| format!("failed to read themes directory: {}", themes_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read theme file: {}", path.display()))?;

        let theme: ThemePreset = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse theme file: {}", path.display()))?;

        themes.push(theme);
    }

    // Sort themes by name for consistent display
    themes.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(themes)
}

/// Find a theme by name (case-insensitive, supports partial matching)
pub fn find_theme(name: &str) -> Result<ThemePreset> {
    let themes = load_available_themes()?;
    let name_lower = name.to_lowercase();

    // First try exact match (case-insensitive)
    for theme in &themes {
        if theme.name.to_lowercase() == name_lower {
            return Ok(theme.clone());
        }
    }

    // Then try partial match
    for theme in &themes {
        if theme.name.to_lowercase().contains(&name_lower) {
            return Ok(theme.clone());
        }
    }

    Err(anyhow!(
        "theme '{}' not found. Run 'ssher theme list' to see available themes",
        name
    ))
}

/// Apply a CLI theme by writing cli.json to config directory
pub fn apply_cli_theme(theme: &CliThemeConfig) -> Result<()> {
    let project_dirs = ProjectDirs::from("", "", "ssher")
        .ok_or_else(|| anyhow!("unable to resolve config directory"))?;
    let config_dir = project_dirs.config_dir();

    // Create config directory if it doesn't exist
    fs::create_dir_all(config_dir).with_context(|| {
        format!(
            "failed to create config directory: {}",
            config_dir.display()
        )
    })?;

    let cli_config_path = config_dir.join("cli.json");

    // Backup existing config if it exists
    if cli_config_path.exists() {
        let backup_path = cli_config_path.with_extension("json.bak");
        fs::copy(&cli_config_path, &backup_path).with_context(|| {
            format!(
                "failed to backup existing config to {}",
                backup_path.display()
            )
        })?;
    }

    // Write new config
    let json = serde_json::to_string_pretty(theme).context("failed to serialize theme config")?;

    fs::write(&cli_config_path, json)
        .with_context(|| format!("failed to write config to {}", cli_config_path.display()))?;

    Ok(())
}

/// Apply a UI theme by writing ui.json to config directory
pub fn apply_ui_theme(theme: &ThemeConfig) -> Result<()> {
    let project_dirs = ProjectDirs::from("", "", "ssher")
        .ok_or_else(|| anyhow!("unable to resolve config directory"))?;
    let config_dir = project_dirs.config_dir();

    // Create config directory if it doesn't exist
    fs::create_dir_all(config_dir).with_context(|| {
        format!(
            "failed to create config directory: {}",
            config_dir.display()
        )
    })?;

    let ui_config_path = config_dir.join("ui.json");

    // Backup existing config if it exists
    if ui_config_path.exists() {
        let backup_path = ui_config_path.with_extension("json.bak");
        fs::copy(&ui_config_path, &backup_path).with_context(|| {
            format!(
                "failed to backup existing config to {}",
                backup_path.display()
            )
        })?;
    }

    // Read existing config to preserve logo and layout, or create new default
    let final_config = if ui_config_path.exists() {
        let content = fs::read_to_string(&ui_config_path)?;
        let mut config: UiConfig =
            serde_json::from_str(&content).unwrap_or_else(|_| UiConfig::default());
        config.theme = theme.clone();
        config
    } else {
        UiConfig {
            logo: Default::default(),
            layout: Default::default(),
            theme: theme.clone(),
            input: Default::default(),
            ordering: Default::default(),
            highlights: Default::default(),
        }
    };

    // Write new config
    let json =
        serde_json::to_string_pretty(&final_config).context("failed to serialize theme config")?;

    fs::write(&ui_config_path, json)
        .with_context(|| format!("failed to write config to {}", ui_config_path.display()))?;

    Ok(())
}

/// Detect the currently active theme by reading config files
pub fn detect_current_theme() -> Result<(Option<String>, Option<String>)> {
    let project_dirs = ProjectDirs::from("", "", "ssher")
        .ok_or_else(|| anyhow!("unable to resolve config directory"))?;
    let config_dir = project_dirs.config_dir();

    let cli_config_path = config_dir.join("cli.json");
    let ui_config_path = config_dir.join("ui.json");

    let cli_theme_name = if cli_config_path.exists() {
        let content = fs::read_to_string(&cli_config_path)?;
        let config: CliThemeConfig = serde_json::from_str(&content)?;

        // Try to match against available themes
        let themes = load_available_themes()?;
        let mut matched = None;
        for theme in &themes {
            if theme.cli.header == config.header
                && theme.cli.name == config.name
                && theme.cli.target == config.target
                && theme.cli.port == config.port
                && theme.cli.identity == config.identity
                && theme.cli.tags == config.tags
            {
                matched = Some(theme.name.clone());
                break;
            }
        }
        matched
    } else {
        None
    };

    let ui_theme_name = if ui_config_path.exists() {
        let content = fs::read_to_string(&ui_config_path)?;
        // Try to parse as UiConfig first (full structure with logo, layout, theme)
        let config: UiConfig = serde_json::from_str(&content)?;
        let theme_config = config.theme;

        // Try to match against available themes
        let themes = load_available_themes()?;
        let mut matched = None;
        for theme in &themes {
            if theme.ui.logo == theme_config.logo
                && theme.ui.header == theme_config.header
                && theme.ui.highlight == theme_config.highlight
                && theme.ui.border == theme_config.border
                && theme.ui.help == theme_config.help
                && theme.ui.status == theme_config.status
                && theme.ui.text == theme_config.text
            {
                matched = Some(theme.name.clone());
                break;
            }
        }
        matched
    } else {
        None
    };

    Ok((cli_theme_name, ui_theme_name))
}

// Command handlers
pub fn list_themes_cmd() -> Result<()> {
    let themes = load_available_themes()?;

    // Detect current theme to mark as active
    let (current_cli, current_ui) = detect_current_theme()?;

    if themes.is_empty() {
        println!("No themes found.");
        return Ok(());
    }

    println!("\nAvailable themes:\n");

    // Find the longest name for formatting
    let max_name_len = themes.iter().map(|t| t.name.len()).max().unwrap_or(0);

    for theme in &themes {
        let mut markers = Vec::new();
        if current_cli.as_ref() == Some(&theme.name) {
            markers.push("CLI");
        }
        if current_ui.as_ref() == Some(&theme.name) {
            markers.push("UI");
        }

        let marker = if markers.is_empty() {
            String::new()
        } else {
            format!(" [{}]", markers.join(", "))
        };

        println!(
            "  {: <width$}{} - {}",
            theme.name,
            marker,
            theme.description,
            width = max_name_len
        );
    }

    println!();
    println!("Use 'ssher theme apply <name>' to apply a theme to both CLI and UI");
    println!("Use 'ssher theme apply-cli <name>' to apply only to CLI");
    println!("Use 'ssher theme apply-ui <name>' to apply only to UI");
    println!();

    Ok(())
}

pub fn apply_cli_theme_cmd(name: &str) -> Result<()> {
    let theme = find_theme(name)?;
    apply_cli_theme(&theme.cli)?;
    println!("Applied CLI theme: {}", theme.name);
    Ok(())
}

pub fn apply_ui_theme_cmd(name: &str) -> Result<()> {
    let theme = find_theme(name)?;
    apply_ui_theme(&theme.ui)?;
    println!("Applied UI theme: {}", theme.name);
    Ok(())
}

pub fn apply_theme_cmd(name: &str) -> Result<()> {
    let theme = find_theme(name)?;
    apply_cli_theme(&theme.cli)?;
    apply_ui_theme(&theme.ui)?;
    println!("Applied theme: {} (CLI and UI)", theme.name);
    Ok(())
}

pub fn current_theme_cmd() -> Result<()> {
    let (cli_name, ui_name) = detect_current_theme()?;

    println!("\nCurrent theme:\n");

    if let Some(name) = cli_name {
        println!("  CLI:  {}", name);
    } else if cli_name.is_none() {
        println!("  CLI:  default");
    }

    if let Some(name) = ui_name {
        println!("  UI:   {}", name);
    } else if ui_name.is_none() {
        println!("  UI:   default");
    }

    println!();
    println!("Use 'ssher theme list' to see available themes");
    println!();

    Ok(())
}
