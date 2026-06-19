use std::path::PathBuf;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};

use anyhow::Result;
use directories::ProjectDirs;

#[cfg(test)]
static APP_CONFIG_DIR_OVERRIDE: Mutex<Option<AppConfigDirOverride>> = Mutex::new(None);
#[cfg(test)]
static APP_CONFIG_DIR_OVERRIDE_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
#[derive(Clone)]
enum AppConfigDirOverride {
    Path(PathBuf),
    Error,
}

#[cfg(test)]
pub(crate) struct AppConfigDirOverrideGuard {
    _lock: MutexGuard<'static, ()>,
}

#[cfg(test)]
pub(crate) fn set_app_config_dir_override(path: PathBuf) -> AppConfigDirOverrideGuard {
    set_app_config_dir_override_value(AppConfigDirOverride::Path(path))
}

#[cfg(test)]
pub(crate) fn set_app_config_dir_error_override() -> AppConfigDirOverrideGuard {
    set_app_config_dir_override_value(AppConfigDirOverride::Error)
}

#[cfg(test)]
fn set_app_config_dir_override_value(value: AppConfigDirOverride) -> AppConfigDirOverrideGuard {
    let lock = APP_CONFIG_DIR_OVERRIDE_LOCK
        .lock()
        .expect("app config override lock should not be poisoned");
    {
        let mut override_path = APP_CONFIG_DIR_OVERRIDE
            .lock()
            .expect("app config override value should not be poisoned");
        *override_path = Some(value);
    }
    AppConfigDirOverrideGuard { _lock: lock }
}

#[cfg(test)]
impl Drop for AppConfigDirOverrideGuard {
    fn drop(&mut self) {
        *APP_CONFIG_DIR_OVERRIDE
            .lock()
            .expect("app config override value should not be poisoned") = None;
    }
}

pub fn app_config_dir() -> Result<PathBuf> {
    #[cfg(test)]
    if let Some(override_path) = APP_CONFIG_DIR_OVERRIDE
        .lock()
        .expect("app config override value should not be poisoned")
        .clone()
    {
        return match override_path {
            AppConfigDirOverride::Path(path) => Ok(path),
            AppConfigDirOverride::Error => Err(anyhow::anyhow!(
                crate::i18n::tr("could not resolve app config directory").into_owned()
            )),
        };
    }

    let dirs = ProjectDirs::from("com", "TranslateR", "TranslateR")
        .expect("TranslateR uses a valid application qualifier, organization, and name");
    Ok(dirs.config_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::{app_config_dir, set_app_config_dir_override};

    #[test]
    fn app_config_dir_returns_platform_default_without_override() {
        assert!(app_config_dir().unwrap().is_absolute());
    }

    #[test]
    fn test_override_sets_and_clears_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        {
            let _override = set_app_config_dir_override(dir.path().to_path_buf());
            assert_eq!(app_config_dir().unwrap(), dir.path());
        }
        assert!(app_config_dir().unwrap().is_absolute());
    }
}
