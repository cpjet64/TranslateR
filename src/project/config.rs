use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::util::paths::app_config_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub translator_name: String,
    pub translator_email: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            translator_name: "Translator".to_string(),
            translator_email: "translator@local".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let Ok(path) = config_path() else {
            return Self::default();
        };
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

fn config_path() -> Result<PathBuf> {
    Ok(app_config_dir()?.join("config.json"))
}
