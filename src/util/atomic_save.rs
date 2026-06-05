use std::{io::Write, path::Path};

#[cfg(windows)]
use std::{fs, path::PathBuf};

use anyhow::Result;
use tempfile::NamedTempFile;

pub fn save_atomic(path: &Path, content: &str) -> Result<()> {
    save_atomic_bytes(path, content.as_bytes())
}

pub fn save_atomic_bytes(path: &Path, content: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(content)?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    persist_replace(tmp, path)?;
    Ok(())
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
            Err(err) => {
                let _ = fs::rename(&backup, path);
                Err(err.error.into())
            }
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
