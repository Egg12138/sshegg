use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn resolve_store_path(override_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path);
    }

    let project_dirs = ProjectDirs::from("", "", "ssher")
        .ok_or_else(|| anyhow!("unable to resolve config directory"))?;
    Ok(project_dirs.config_dir().join("sessions.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_path_takes_precedence() {
        let custom_path = PathBuf::from("/custom/path/sessions.json");
        let result = resolve_store_path(Some(custom_path.clone()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), custom_path);
    }

    #[test]
    fn none_override_uses_project_dirs() {
        let result = resolve_store_path(None);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with("sessions.json"));
    }
}
