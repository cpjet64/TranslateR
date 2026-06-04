use std::fs;

use tempfile::tempdir;
use translater::{
    history::HistoryDb,
    po::{parser::parse_text, writer::set_translation},
    project::AppConfig,
    vcs::diff::{apply_unified_patch, unified_diff},
};

#[test]
fn sqlite_history_records_versions_and_restores_latest() {
    let dir = tempdir().unwrap();
    let db = HistoryDb::open(dir.path().join("history.sqlite3")).unwrap();
    let po = dir.path().join("de.po");
    fs::write(&po, "msgid \"Hello\"\nmsgstr \"Hallo\"\n").unwrap();

    let v1 = db
        .record_version(&po, &AppConfig::default(), "base")
        .unwrap();
    assert_eq!(v1, 1);

    fs::write(&po, "msgid \"Hello\"\nmsgstr \"Guten Tag\"\n").unwrap();
    let v2 = db
        .record_version(&po, &AppConfig::default(), "edit")
        .unwrap();
    assert_eq!(v2, 2);
    assert_eq!(db.versions(&po).unwrap().len(), 2);

    db.restore_latest(&po).unwrap();
    assert_eq!(
        fs::read_to_string(&po).unwrap(),
        "msgid \"Hello\"\nmsgstr \"Guten Tag\"\n"
    );
}

#[test]
fn maintainer_can_apply_translator_tpatch() {
    let base = "msgid \"Hello\"\nmsgstr \"\"\n";
    let translated = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let patch = unified_diff(base, translated, "base.po", "translator.po").unwrap();
    let merged = apply_unified_patch(base, &patch).unwrap();
    assert_eq!(merged, translated);
}

#[test]
fn translator_tpatch_can_be_generated_without_writing_po() {
    let base = "msgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let mut doc = parse_text("translator.po", base.clone()).unwrap();
    let entry_id = doc.entries[0].id;
    set_translation(&mut doc, entry_id, 0, "Hallo".to_string());

    let edited = translater::po::writer::write_document(&doc).unwrap();
    let patch = unified_diff(&base, &edited, "saved-version", "translator.po").unwrap();
    let merged = apply_unified_patch(&base, &patch).unwrap();

    assert_eq!(merged, "msgid \"Hello\"\nmsgstr \"Hallo\"\n");
}

#[test]
fn applies_real_test_po_tpatch_context_inside_file() {
    let base = fs::read_to_string("test-po/ar-test.po").unwrap();
    let patch = fs::read_to_string("test-po/translation.tpatch").unwrap();
    let merged = apply_unified_patch(&base, &patch).unwrap();

    assert!(merged.contains("msgid \"Privacy Policy\""));
    assert!(merged.contains("msgstr \"privacio policio\""));
    assert!(merged.contains("msgctxt \"web-app/navbar\""));
}
