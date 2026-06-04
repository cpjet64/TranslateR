use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use rusqlite::{Connection, OptionalExtension, params};
use time::OffsetDateTime;

use crate::{
    po::{parse_document, validate::validate_document},
    project::AppConfig,
    util::{hashing::sha256_bytes, paths::app_config_dir},
};

#[derive(Debug, Clone)]
pub struct HistoryState {
    pub db_path: PathBuf,
    pub active_file_id: Option<i64>,
    pub latest_version: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version_number: i64,
    pub created_at: String,
    pub translator_name: String,
    pub content_hash: String,
}

pub struct HistoryDb {
    conn: Connection,
}

impl HistoryDb {
    pub fn open_default() -> Result<Self> {
        let dir = app_config_dir()?.join("history");
        fs::create_dir_all(&dir)?;
        Self::open(dir.join("translater-history.sqlite3"))
    }

    pub fn open(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS files (
              id INTEGER PRIMARY KEY,
              path TEXT NOT NULL,
              path_hash TEXT NOT NULL UNIQUE,
              created_at TEXT NOT NULL,
              last_seen_hash TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS versions (
              id INTEGER PRIMARY KEY,
              file_id INTEGER NOT NULL,
              version_number INTEGER NOT NULL,
              created_at TEXT NOT NULL,
              translator_name TEXT NOT NULL,
              source_hash TEXT NOT NULL,
              content_hash TEXT NOT NULL,
              content_bytes BLOB NOT NULL,
              note TEXT,
              validation_summary_json TEXT,
              FOREIGN KEY(file_id) REFERENCES files(id),
              UNIQUE(file_id, version_number)
            );
            ",
        )?;
        Ok(Self { conn })
    }

    pub fn state_for_file(&self, path: &Path) -> Result<HistoryState> {
        let file_id = self.file_id(path)?;
        Ok(HistoryState {
            db_path: PathBuf::new(),
            active_file_id: Some(file_id),
            latest_version: self.latest_version_number(file_id)?,
        })
    }

    pub fn record_version(&self, path: &Path, config: &AppConfig, note: &str) -> Result<i64> {
        let bytes = fs::read(path)?;
        let file_id = self.ensure_file(path, &bytes)?;
        let version = self.latest_version_number(file_id)?.unwrap_or(0) + 1;
        let created_at = OffsetDateTime::now_utc().to_string();
        let content_hash = sha256_bytes(&bytes);
        let source_hash = content_hash.clone();
        let validation_summary = validation_summary(path).unwrap_or_else(|_| "{}".to_string());
        self.conn.execute(
            "INSERT INTO versions
             (file_id, version_number, created_at, translator_name, source_hash, content_hash, content_bytes, note, validation_summary_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                file_id,
                version,
                created_at,
                config.translator_name,
                source_hash,
                content_hash,
                bytes,
                note,
                validation_summary
            ],
        )?;
        Ok(version)
    }

    pub fn versions(&self, path: &Path) -> Result<Vec<VersionInfo>> {
        let file_id = self.file_id(path)?;
        let mut stmt = self.conn.prepare(
            "SELECT version_number, created_at, translator_name, content_hash
             FROM versions WHERE file_id = ?1 ORDER BY version_number DESC",
        )?;
        let rows = stmt.query_map(params![file_id], |row| {
            Ok(VersionInfo {
                version_number: row.get(0)?,
                created_at: row.get(1)?,
                translator_name: row.get(2)?,
                content_hash: row.get(3)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn latest_bytes(&self, path: &Path) -> Result<Option<Vec<u8>>> {
        let file_id = self.file_id(path)?;
        self.bytes_for_version(file_id, None)
    }

    pub fn restore_latest(&self, path: &Path) -> Result<()> {
        let file_id = self.file_id(path)?;
        let bytes = self
            .bytes_for_version(file_id, None)?
            .ok_or_else(|| anyhow!("no saved versions for this file"))?;
        fs::write(path, bytes)?;
        Ok(())
    }

    fn ensure_file(&self, path: &Path, bytes: &[u8]) -> Result<i64> {
        let path_text = path.display().to_string();
        let path_hash = sha256_bytes(path_text.as_bytes());
        let last_seen_hash = sha256_bytes(bytes);
        let created_at = OffsetDateTime::now_utc().to_string();
        self.conn.execute(
            "INSERT INTO files (path, path_hash, created_at, last_seen_hash)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path_hash) DO UPDATE SET path = excluded.path, last_seen_hash = excluded.last_seen_hash",
            params![path_text, path_hash, created_at, last_seen_hash],
        )?;
        self.file_id(path)
    }

    fn file_id(&self, path: &Path) -> Result<i64> {
        let path_hash = sha256_bytes(path.display().to_string().as_bytes());
        self.conn
            .query_row(
                "SELECT id FROM files WHERE path_hash = ?1",
                params![path_hash],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("file has no history yet"))
    }

    fn latest_version_number(&self, file_id: i64) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT MAX(version_number) FROM versions WHERE file_id = ?1",
                params![file_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(Into::into)
    }

    fn bytes_for_version(&self, file_id: i64, version: Option<i64>) -> Result<Option<Vec<u8>>> {
        let sql = if version.is_some() {
            "SELECT content_bytes FROM versions WHERE file_id = ?1 AND version_number = ?2"
        } else {
            "SELECT content_bytes FROM versions WHERE file_id = ?1 ORDER BY version_number DESC LIMIT 1"
        };
        if let Some(version) = version {
            self.conn
                .query_row(sql, params![file_id, version], |row| row.get(0))
                .optional()
                .map_err(Into::into)
        } else {
            self.conn
                .query_row(sql, params![file_id], |row| row.get(0))
                .optional()
                .map_err(Into::into)
        }
    }
}

fn validation_summary(path: &Path) -> Result<String> {
    let mut doc = parse_document(path)?;
    validate_document(&mut doc);
    Ok(format!(
        "{{\"entries\":{},\"diagnostics\":{}}}",
        doc.entries.len(),
        doc.diagnostics.len()
    ))
}
