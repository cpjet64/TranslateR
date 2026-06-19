use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    i18n::tr,
    po::{PoDocument, PoEntry, header::parse_header, parser::parse_text},
    util::{atomic_save::save_atomic_bytes, hashing::sha256_bytes, paths::app_config_dir},
};

pub const TRPACK_FORMAT: &str = "TranslateR TRPack v1";
pub const TRDRAFT_FORMAT: &str = "TranslateR TRDraft v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrPack {
    pub format: String,
    pub project_id: String,
    pub pack_version: String,
    pub language: Option<String>,
    pub created_at: String,
    pub base_hash: String,
    pub po_filename: String,
    pub po_text: String,
    #[serde(default)]
    pub history: Vec<VersionLogEntry>,
    pub contexts: Vec<EntryContext>,
    pub answers: Vec<EntryAnswer>,
    pub screenshots: Vec<ScreenshotRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrDraft {
    pub format: String,
    pub project_id: String,
    pub pack_version: String,
    pub language: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub base_hash: String,
    pub po_filename: String,
    pub base_po_text: String,
    pub po_text: String,
    pub questions: Vec<EntryQuestion>,
    #[serde(default)]
    pub history: Vec<VersionLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeSummary {
    pub line_additions: usize,
    pub line_deletions: usize,
    pub changed_translations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionLogEntry {
    pub version: String,
    pub created_at: String,
    pub author: String,
    pub note: String,
    pub base_hash: String,
    pub content_hash: String,
    pub change_summary: ChangeSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryContext {
    pub entry_id: String,
    pub note: String,
    pub screenshot_id: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryQuestion {
    pub entry_id: String,
    pub scope: String,
    pub question: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryAnswer {
    pub entry_id: String,
    pub question: String,
    pub answer: String,
    pub answered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotRef {
    pub id: String,
    pub file_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct TPatchMetadata {
    pub project_id: Option<String>,
    pub pack_version: Option<String>,
    pub base_hash: Option<String>,
    pub questions: Vec<EntryQuestion>,
}

#[derive(Debug, Clone)]
pub struct ActivePackage {
    pub source_path: PathBuf,
    pub project_id: String,
    pub pack_version: String,
    pub language: Option<String>,
    pub base_hash: String,
    pub po_filename: String,
    pub is_draft: bool,
    pub history: Vec<VersionLogEntry>,
}

impl ActivePackage {
    pub fn from_pack(path: PathBuf, pack: &TrPack) -> Self {
        Self {
            source_path: path,
            project_id: pack.project_id.clone(),
            pack_version: pack.pack_version.clone(),
            language: pack.language.clone(),
            base_hash: pack.base_hash.clone(),
            po_filename: pack.po_filename.clone(),
            is_draft: false,
            history: pack.history.clone(),
        }
    }

    pub fn from_draft(path: PathBuf, draft: &TrDraft) -> Self {
        Self {
            source_path: path,
            project_id: draft.project_id.clone(),
            pack_version: draft.pack_version.clone(),
            language: draft.language.clone(),
            base_hash: draft.base_hash.clone(),
            po_filename: draft.po_filename.clone(),
            is_draft: true,
            history: draft.history.clone(),
        }
    }
}

pub fn trpack_from_document(
    doc: &PoDocument,
    po_text: String,
    project_id: Option<String>,
    pack_version: Option<String>,
) -> TrPack {
    let header = parse_header(doc);
    let project_id = project_id.unwrap_or_else(|| {
        doc.path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });
    let base_hash = sha256_bytes(po_text.as_bytes());
    let pack_version = pack_version.unwrap_or_else(|| "1".to_string());
    let created_at = now_rfc3339();
    TrPack {
        format: TRPACK_FORMAT.to_string(),
        project_id,
        pack_version: pack_version.clone(),
        language: header.language,
        created_at: created_at.clone(),
        base_hash: base_hash.clone(),
        po_filename: doc
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        po_text,
        history: vec![VersionLogEntry {
            version: pack_version,
            created_at,
            author: "Maintainer".to_string(),
            note: "Initial TRPack export".to_string(),
            base_hash: String::new(),
            content_hash: base_hash,
            change_summary: ChangeSummary {
                line_additions: doc.original_text.lines().count(),
                line_deletions: 0,
                changed_translations: vec!["Initial package version".to_string()],
            },
        }],
        contexts: Vec::new(),
        answers: Vec::new(),
        screenshots: Vec::new(),
    }
}

pub fn trdraft_from_document(
    doc: &PoDocument,
    current_po_text: String,
    base_po_text: String,
    package: Option<&ActivePackage>,
) -> TrDraft {
    let header = parse_header(doc);
    let now = now_rfc3339();
    TrDraft {
        format: TRDRAFT_FORMAT.to_string(),
        project_id: package
            .map(|p| p.project_id.clone())
            .unwrap_or_else(|| file_stem(&doc.path)),
        pack_version: package
            .map(|p| p.pack_version.clone())
            .unwrap_or_else(now_version),
        language: header.language,
        created_at: now.clone(),
        updated_at: now,
        base_hash: sha256_bytes(base_po_text.as_bytes()),
        po_filename: package
            .map(|p| p.po_filename.clone())
            .unwrap_or_else(|| file_name(&doc.path)),
        base_po_text,
        po_text: current_po_text,
        questions: Vec::new(),
        history: package.map(|p| p.history.clone()).unwrap_or_default(),
    }
}

pub fn read_trpack(path: &Path) -> Result<TrPack> {
    let bytes = fs::read(path)?;
    ensure_format(&bytes, TRPACK_FORMAT, tr("unsupported TRPack format"))?;
    let pack: TrPack = serde_json::from_slice(&bytes)?;
    Ok(pack)
}

pub fn write_trpack(path: &Path, pack: &TrPack) -> Result<()> {
    let bytes =
        serde_json::to_vec_pretty(pack).expect("TRPack contains only JSON-serializable fields");
    save_atomic_bytes(path, &bytes)
}

pub fn read_trdraft(path: &Path) -> Result<TrDraft> {
    let bytes = fs::read(path)?;
    ensure_format(&bytes, TRDRAFT_FORMAT, tr("unsupported TRDraft format"))?;
    let draft: TrDraft = serde_json::from_slice(&bytes)?;
    if sha256_bytes(draft.base_po_text.as_bytes()) != draft.base_hash {
        return Err(anyhow!(
            tr("TRDraft base hash does not match its base PO text").into_owned()
        ));
    }
    Ok(draft)
}

pub fn write_trdraft(path: &Path, draft: &TrDraft) -> Result<()> {
    let bytes =
        serde_json::to_vec_pretty(draft).expect("TRDraft contains only JSON-serializable fields");
    save_atomic_bytes(path, &bytes)
}

fn ensure_format(
    bytes: &[u8],
    expected: &str,
    message: std::borrow::Cow<'static, str>,
) -> Result<()> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    if value
        .get("format")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|format| format == expected)
    {
        Ok(())
    } else {
        Err(anyhow!(message.into_owned()))
    }
}

pub fn add_tpatch_metadata(
    patch: String,
    package: Option<&ActivePackage>,
    questions: &[EntryQuestion],
) -> String {
    if package.is_none() && questions.is_empty() {
        return patch;
    }
    let mut lines = patch.lines();
    let Some(first) = lines.next() else {
        return patch;
    };
    let mut out = String::new();
    out.push_str(first);
    out.push('\n');
    if let Some(package) = package {
        out.push_str("# TranslateR-Project: ");
        out.push_str(&package.project_id);
        out.push('\n');
        out.push_str("# TranslateR-Package-Version: ");
        out.push_str(&package.pack_version);
        out.push('\n');
        out.push_str("# TranslateR-Base-Hash: ");
        out.push_str(&package.base_hash);
        out.push('\n');
    }
    if !questions.is_empty() {
        let json =
            serde_json::to_string(questions).expect("EntryQuestion values serialize to JSON");
        out.push_str("# TranslateR-Questions-Json: ");
        out.push_str(&json);
        out.push('\n');
    }
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn parse_tpatch_metadata(patch: &str) -> TPatchMetadata {
    let mut metadata = TPatchMetadata::default();
    for line in patch.lines() {
        if let Some(value) = line.strip_prefix("# TranslateR-Project: ") {
            metadata.project_id = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("# TranslateR-Package-Version: ") {
            metadata.pack_version = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("# TranslateR-Base-Hash: ") {
            metadata.base_hash = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("# TranslateR-Questions-Json: ") {
            metadata.questions = serde_json::from_str(value.trim()).unwrap_or_default();
        }
    }
    metadata
}

pub fn materialize_workflow_po(
    base_hash: &str,
    po_filename: &str,
    po_text: &str,
) -> Result<PathBuf> {
    materialize_workflow_po_in_dir(&app_config_dir()?, base_hash, po_filename, po_text)
}

pub fn materialize_workflow_po_in_dir(
    config_dir: &Path,
    base_hash: &str,
    po_filename: &str,
    po_text: &str,
) -> Result<PathBuf> {
    let mut dir = config_dir.join("workflow").join(short_hash(base_hash));
    fs::create_dir_all(&dir)?;
    dir.push(safe_po_filename(po_filename));
    save_atomic_bytes(&dir, po_text.as_bytes())?;
    Ok(dir)
}

pub fn entry_key(entry: &PoEntry) -> String {
    let mut key = String::new();
    key.push_str(
        entry
            .msgctxt
            .as_ref()
            .map(|f| f.value())
            .unwrap_or_default(),
    );
    key.push('\0');
    key.push_str(entry.msgid.value());
    key.push('\0');
    key.push_str(
        entry
            .msgid_plural
            .as_ref()
            .map(|f| f.value())
            .unwrap_or_default(),
    );
    sha256_bytes(key.as_bytes())
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("UTC timestamps always format as RFC3339")
}

fn now_version() -> String {
    now_rfc3339().replace(':', "-")
}

pub fn next_pack_version(current: &str) -> String {
    current
        .parse::<u64>()
        .map(|version| (version + 1).to_string())
        .unwrap_or_else(|_| now_version())
}

pub fn change_summary(
    path: impl AsRef<Path>,
    previous_text: &str,
    current_text: &str,
) -> Result<ChangeSummary> {
    let diff = TextDiff::from_lines(previous_text, current_text);
    let mut summary = ChangeSummary::default();
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => summary.line_deletions += 1,
            ChangeTag::Insert => summary.line_additions += 1,
            ChangeTag::Equal => {}
        }
    }

    let previous_doc = parse_text(path.as_ref(), previous_text.to_string())
        .expect("parse_text records PO parse issues as diagnostics");
    let current_doc = parse_text(path.as_ref(), current_text.to_string())
        .expect("parse_text records PO parse issues as diagnostics");
    for current in current_doc
        .entries
        .iter()
        .filter(|entry| !entry.is_header())
    {
        let key = entry_key(current);
        let Some(previous) = previous_doc
            .entries
            .iter()
            .find(|entry| !entry.is_header() && entry_key(entry) == key)
        else {
            summary
                .changed_translations
                .push(format!("Added entry: {}", preview(current.msgid.value())));
            continue;
        };
        for (index, current_field) in current.msgstr.iter().enumerate() {
            let previous_value = previous
                .msgstr
                .get(index)
                .map(|field| field.value())
                .unwrap_or_default();
            if previous_value != current_field.value() {
                summary.changed_translations.push(format!(
                    "{} form {}: {} -> {}",
                    preview(current.msgid.value()),
                    index,
                    preview_value(previous_value),
                    preview_value(current_field.value())
                ));
            }
        }
    }
    Ok(summary)
}

pub fn version_log_entry(
    version: String,
    author: String,
    note: String,
    previous_text: &str,
    current_text: &str,
    summary: ChangeSummary,
) -> VersionLogEntry {
    VersionLogEntry {
        version,
        created_at: now_rfc3339(),
        author,
        note,
        base_hash: sha256_bytes(previous_text.as_bytes()),
        content_hash: sha256_bytes(current_text.as_bytes()),
        change_summary: summary,
    }
}

fn preview(value: &str) -> String {
    let value = value.replace('\n', "\\n");
    let mut chars = value.chars();
    let mut out = chars.by_ref().take(80).collect::<String>();
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

fn preview_value(value: &str) -> String {
    if value.is_empty() {
        "<empty>".to_string()
    } else {
        preview(value)
    }
}

fn short_hash(hash: &str) -> String {
    hash.chars().take(16).collect()
}

fn safe_po_filename(filename: &str) -> String {
    let name = Path::new(filename)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    if name.is_empty() {
        "translation.po".to_string()
    } else {
        name.to_string()
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::paths::{set_app_config_dir_error_override, set_app_config_dir_override};

    #[test]
    fn materializes_workflow_po_using_app_config_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let _override = set_app_config_dir_override(temp_dir.path().to_path_buf());

        let path = materialize_workflow_po("1234567890abcdef9876", "../sample.po", "msgid \"\"\n")
            .unwrap();

        assert_eq!(
            path,
            temp_dir
                .path()
                .join("workflow")
                .join("1234567890abcdef")
                .join("sample.po")
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "msgid \"\"\n");
    }

    #[test]
    fn materialize_workflow_po_reports_missing_config_dir() {
        let _override = set_app_config_dir_error_override();

        assert!(
            materialize_workflow_po("abc", "sample.po", "msgid \"\"\n")
                .unwrap_err()
                .to_string()
                .contains("could not resolve app config")
        );
    }

    #[test]
    fn metadata_and_entry_key_cover_questions_context_and_plural() {
        let questions = vec![EntryQuestion {
            entry_id: "entry-1".to_string(),
            scope: "source".to_string(),
            question: "Where does this appear?".to_string(),
            created_at: "2026-06-19T00:00:00Z".to_string(),
        }];
        let patch = add_tpatch_metadata("--- old\n+++ new\n".to_string(), None, &questions);
        assert!(patch.contains("TranslateR-Questions-Json"));
        assert_eq!(parse_tpatch_metadata(&patch).questions.len(), 1);

        let doc = parse_text(
            "plural.po",
            "msgctxt \"achievement\"\nmsgid \"%d file\"\nmsgid_plural \"%d files\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n".to_string(),
        )
        .unwrap();
        let key = entry_key(&doc.entries[0]);
        assert_eq!(key.len(), 64);
    }
}
