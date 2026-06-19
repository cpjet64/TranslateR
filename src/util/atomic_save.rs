use std::{io::Write, path::Path};

#[cfg(windows)]
use std::{fs, path::PathBuf};

use anyhow::{Error, Result};
use tempfile::NamedTempFile;

pub fn save_atomic(path: &Path, content: &str) -> Result<()> {
    save_atomic_bytes(path, content.as_bytes())
}

pub fn save_atomic_bytes(path: &Path, content: &[u8]) -> Result<()> {
    let dir = save_directory(path);
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(content)?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    persist_replace(tmp, path)?;
    Ok(())
}

fn save_directory(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}

fn persist_replace(tmp: NamedTempFile, path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let backup = backup_path(path);
        if path.exists() {
            fs::rename(path, &backup)?;
        }
        match tmp.persist(path) {
            Ok(_) => {
                let _ = fs::remove_file(backup);
                Ok(())
            }
            Err(err) => restore_backup_after_persist_error(&backup, path, err.error.into()),
        }
    }
    #[cfg(not(windows))]
    {
        tmp.persist(path)
            .map(|_| ())
            .map_err(|err| err.error.into())
    }
}

#[cfg(windows)]
fn backup_path(path: &Path) -> PathBuf {
    let mut candidate = path.to_path_buf();
    let ext = path
        .extension()
        .map(|e| format!("{}.translater-bak", e.to_string_lossy()))
        .unwrap_or_else(|| "translater-bak".to_string());
    candidate.set_extension(ext);
    candidate
}

#[cfg(windows)]
fn restore_backup_after_persist_error(backup: &Path, path: &Path, error: Error) -> Result<()> {
    let _ = fs::rename(backup, path);
    Err(error)
}

#[cfg(all(test, windows))]
mod tests {
    use super::{
        backup_path, persist_replace, restore_backup_after_persist_error, save_atomic,
        save_directory,
    };
    use anyhow::Error;
    use std::{fs, io::Write, path::Path};
    use tempfile::NamedTempFile;

    #[test]
    fn windows_backup_restore_helper_restores_original_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("translation.po");
        let backup = dir.path().join("translation.po.translater-bak");
        fs::write(&backup, "original").unwrap();

        let err = restore_backup_after_persist_error(
            &backup,
            &path,
            Error::msg("forced atomic save failure"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("forced atomic save failure"));
        assert_eq!(fs::read_to_string(path).unwrap(), "original");
        assert!(!backup.exists());

        save_atomic(dir.path().join("new.po").as_path(), "replacement").unwrap();
    }

    #[test]
    fn windows_path_helpers_cover_no_parent_and_no_extension() {
        assert_eq!(save_directory(Path::new("")), Path::new("."));
        assert_eq!(
            backup_path(Path::new("translation")).as_path(),
            Path::new("translation.translater-bak")
        );
    }

    #[test]
    fn windows_persist_replace_reports_invalid_destination() {
        let dir = tempfile::tempdir().unwrap();
        let mut tmp = NamedTempFile::new_in(dir.path()).unwrap();
        tmp.write_all(b"replacement").unwrap();

        let err = persist_replace(tmp, &dir.path().join("bad:name.po")).unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}
