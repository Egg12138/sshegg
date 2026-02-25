use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<PathBuf>,
}

impl Session {
    pub fn target(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }
}
