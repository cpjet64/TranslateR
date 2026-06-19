use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::util::paths::app_config_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub translator_name: String,
    pub translator_email: String,
    pub ui_language: String,
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UpdateConfig {
    pub check_on_startup: bool,
    pub check_hourly: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            translator_name: "Translator".to_string(),
            translator_email: "translator@local".to_string(),
            ui_language: "en".to_string(),
            update: UpdateConfig::default(),
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check_on_startup: true,
            check_hourly: true,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        match config_path() {
            Ok(path) => Self::load_from_path(&path),
            Err(_) => Self::default(),
        }
    }

    pub fn load_from_path(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        match config_path() {
            Ok(path) => self.save_to_path(&path),
            Err(err) => Err(err),
        }
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                return Err(err.into());
            }
        }
        let text = serde_json::to_string_pretty(self)
            .expect("AppConfig contains only serializable fields");
        match fs::write(path, text) {
            Ok(()) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

fn config_path() -> Result<PathBuf> {
    match app_config_dir() {
        Ok(dir) => Ok(dir.join("config.json")),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, UpdateConfig};
    use crate::util::paths::{set_app_config_dir_error_override, set_app_config_dir_override};

    #[test]
    fn load_falls_back_to_default_when_config_dir_cannot_be_resolved() {
        let _override = set_app_config_dir_error_override();
        assert_eq!(AppConfig::load().translator_name, "Translator");
        assert!(AppConfig::default().save().is_err());
    }

    #[test]
    fn load_from_path_defaults_when_file_or_json_is_bad_and_save_reports_io_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            AppConfig::load_from_path(&dir.path().join("missing.json")).translator_email,
            "translator@local"
        );

        let bad_json = dir.path().join("bad.json");
        std::fs::write(&bad_json, "{not-json").unwrap();
        assert_eq!(AppConfig::load_from_path(&bad_json).ui_language, "en");

        let old_json = dir.path().join("old.json");
        std::fs::write(
            &old_json,
            r#"{"translator_name":"Old","translator_email":"old@example.test","ui_language":"en"}"#,
        )
        .unwrap();
        let old = AppConfig::load_from_path(&old_json);
        assert!(old.update.check_on_startup);
        assert!(old.update.check_hourly);

        let unwritable_path = dir.path().join("as-directory");
        std::fs::create_dir(&unwritable_path).unwrap();
        assert!(AppConfig::default().save_to_path(&unwritable_path).is_err());

        let saved_path = dir.path().join("nested").join("config.json");
        AppConfig {
            translator_name: "Ada".to_string(),
            translator_email: "ada@example.test".to_string(),
            ui_language: "de".to_string(),
            update: UpdateConfig::default(),
        }
        .save_to_path(&saved_path)
        .unwrap();
        let saved = AppConfig::load_from_path(&saved_path);
        assert_eq!(saved.translator_name, "Ada");
        assert_eq!(saved.ui_language, "de");
    }

    #[test]
    fn save_uses_config_directory_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let _override = set_app_config_dir_override(dir.path().to_path_buf());
        let config = AppConfig {
            translator_name: "Lin".to_string(),
            translator_email: "lin@example.test".to_string(),
            ui_language: "zh-Hans".to_string(),
            update: UpdateConfig::default(),
        };

        config.save().unwrap();

        let saved = AppConfig::load_from_path(&dir.path().join("config.json"));
        assert_eq!(saved.translator_name, "Lin");
        assert_eq!(saved.translator_email, "lin@example.test");
        assert_eq!(saved.ui_language, "zh-Hans");
    }
}
