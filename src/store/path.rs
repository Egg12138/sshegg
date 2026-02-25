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
