use std::path::PathBuf;

use anyhow::{Result, anyhow};
use directories::ProjectDirs;

pub fn app_config_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "TranslateR", "TranslateR")
        .ok_or_else(|| anyhow!("could not resolve app config directory"))?;
    Ok(dirs.config_dir().to_path_buf())
}
