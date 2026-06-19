use translater::{
    po::{parser::parse_text, writer::set_translation},
    vcs::diff::{apply_unified_patch, unified_diff},
    workflow::{
        ActivePackage, EntryQuestion, add_tpatch_metadata, change_summary, next_pack_version,
        parse_tpatch_metadata, read_trdraft, read_trpack, trdraft_from_document,
        trpack_from_document, version_log_entry, write_trdraft, write_trpack,
    },
};

#[test]
fn trpack_preserves_po_text_and_version_identity() {
    let po =
        "msgid \"\"\nmsgstr \"Language: de\\n\"\n\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n".to_string();
    let doc = parse_text("de.po", po.clone()).unwrap();
    let pack = trpack_from_document(
        &doc,
        po.clone(),
        Some("game-ui".to_string()),
        Some("2026.06.18".to_string()),
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("de.trpack");
    write_trpack(&path, &pack).unwrap();
    let loaded = read_trpack(&path).unwrap();

    assert_eq!(loaded.project_id, "game-ui");
    assert_eq!(loaded.pack_version, "2026.06.18");
    assert_eq!(loaded.language.as_deref(), Some("de"));
    assert_eq!(loaded.po_text, po);
    assert_eq!(loaded.base_hash, pack.base_hash);
    assert_eq!(loaded.history.len(), 1);
    assert_eq!(loaded.history[0].version, "2026.06.18");
}

#[test]
fn trdraft_stores_base_and_current_translation_text() {
    let base = "msgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let mut doc = parse_text("de.po", base.clone()).unwrap();
    let entry_id = doc.entries[0].id;
    set_translation(&mut doc, entry_id, 0, "Hallo".to_string());
    let current = translater::po::writer::write_document(&doc).unwrap();
    let package = ActivePackage {
        source_path: "de.trpack".into(),
        project_id: "game-ui".to_string(),
        pack_version: "2026.06.18".to_string(),
        language: Some("de".to_string()),
        base_hash: translater::util::hashing::sha256_bytes(base.as_bytes()),
        po_filename: "de.po".to_string(),
        is_draft: false,
        history: Vec::new(),
    };

    let draft = trdraft_from_document(&doc, current.clone(), base.clone(), Some(&package));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("de.trdraft");
    write_trdraft(&path, &draft).unwrap();
    let loaded = read_trdraft(&path).unwrap();

    assert_eq!(loaded.project_id, "game-ui");
    assert_eq!(loaded.pack_version, "2026.06.18");
    assert_eq!(loaded.base_po_text, base);
    assert_eq!(loaded.po_text, current);
}

#[test]
fn draft_generated_tpatch_applies_to_original_trpack_base() {
    let base = "msgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let mut doc = parse_text("de.po", base.clone()).unwrap();
    let entry_id = doc.entries[0].id;
    set_translation(&mut doc, entry_id, 0, "Hallo".to_string());
    let current = translater::po::writer::write_document(&doc).unwrap();

    let patch = unified_diff(&base, &current, "package-base", "translator-draft").unwrap();
    let merged = apply_unified_patch(&base, &patch).unwrap();

    assert_eq!(merged, current);
}

#[test]
fn tpatch_metadata_records_package_version() {
    let package = ActivePackage {
        source_path: "de.trpack".into(),
        project_id: "game-ui".to_string(),
        pack_version: "2026.06.18".to_string(),
        language: Some("de".to_string()),
        base_hash: "abcdef0123456789".to_string(),
        po_filename: "de.po".to_string(),
        is_draft: false,
        history: Vec::new(),
    };
    let patch = unified_diff(
        "msgid \"A\"\nmsgstr \"\"\n",
        "msgid \"A\"\nmsgstr \"B\"\n",
        "a",
        "b",
    )
    .unwrap();
    let questions = vec![EntryQuestion {
        entry_id: "entry-1".to_string(),
        scope: "source".to_string(),
        question: "Where is this shown?".to_string(),
        created_at: "2026-06-18T00:00:00Z".to_string(),
    }];
    let patch = add_tpatch_metadata(patch, Some(&package), &questions);
    let metadata = parse_tpatch_metadata(&patch);

    assert_eq!(metadata.project_id.as_deref(), Some("game-ui"));
    assert_eq!(metadata.pack_version.as_deref(), Some("2026.06.18"));
    assert_eq!(metadata.base_hash.as_deref(), Some("abcdef0123456789"));
    assert_eq!(metadata.questions.len(), 1);
    assert_eq!(metadata.questions[0].question, "Where is this shown?");
    assert!(apply_unified_patch("msgid \"A\"\nmsgstr \"\"\n", &patch).is_ok());
}

#[test]
fn trpack_history_records_translation_changes() {
    let base = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let edited = "msgid \"Hello\"\nmsgstr \"Guten Tag\"\n";
    let doc = parse_text("de.po", edited.to_string()).unwrap();
    let mut pack = trpack_from_document(
        &doc,
        edited.to_string(),
        Some("game-ui".to_string()),
        Some("1".to_string()),
    );
    let version = next_pack_version("1");
    let summary = change_summary("de.po", base, edited).unwrap();
    pack.history.push(version_log_entry(
        version.clone(),
        "Translator".to_string(),
        "Save PO".to_string(),
        base,
        edited,
        summary,
    ));
    pack.pack_version = version;

    assert_eq!(pack.pack_version, "2");
    assert!(
        pack.history
            .last()
            .unwrap()
            .change_summary
            .changed_translations
            .iter()
            .any(|change| change.contains("Hello form 0: Hallo -> Guten Tag"))
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
    let patch = unified_diff(&base, &edited, "package-base", "translator.po").unwrap();
    let merged = apply_unified_patch(&base, &patch).unwrap();

    assert_eq!(merged, "msgid \"Hello\"\nmsgstr \"Hallo\"\n");
}

#[test]
fn applies_real_test_po_tpatch_context_inside_file() {
    let base = std::fs::read_to_string("test-po/ar-test.po").unwrap();
    let patch = std::fs::read_to_string("test-po/translation.tpatch").unwrap();
    let merged = apply_unified_patch(&base, &patch).unwrap();

    assert!(merged.contains("msgid \"Privacy Policy\""));
    assert!(merged.contains("msgstr \"privacio policio\""));
    assert!(merged.contains("msgctxt \"web-app/navbar\""));
}
