use translater::{
    po::{parser::parse_text, writer::set_translation},
    vcs::diff::{apply_unified_patch, unified_diff},
    workflow::{
        ActivePackage, EntryQuestion, add_tpatch_metadata, change_summary,
        materialize_workflow_po_in_dir, next_pack_version, parse_tpatch_metadata, read_trdraft,
        read_trpack, trdraft_from_document, trpack_from_document, version_log_entry, write_trdraft,
        write_trpack,
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
    let current = translater::po::writer::write_document(&doc);
    let package = ActivePackage {
        source_path: "de.trpack".into(),
        project_id: "game-ui".to_string(),
        pack_version: "2026.06.18".to_string(),
        language: Some("de".to_string()),
        base_hash: translater::util::hashing::sha256_bytes(base.as_bytes()),
        po_filename: "de.po".to_string(),
        is_draft: false,
        history: Vec::new(),
        contexts: Vec::new(),
        answers: Vec::new(),
        screenshots: Vec::new(),
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
fn active_package_can_be_built_from_pack_and_draft() {
    let po = "msgid \"Hello\"\nmsgstr \"Hallo\"\n".to_string();
    let doc = parse_text("fallback-name.po", po.clone()).unwrap();
    let pack = trpack_from_document(&doc, po.clone(), None, None);
    let from_pack = ActivePackage::from_pack("fallback-name.trpack".into(), &pack);

    assert_eq!(pack.project_id, "fallback-name");
    assert_eq!(pack.pack_version, "1");
    assert_eq!(pack.po_filename, "fallback-name.po");
    assert!(!from_pack.is_draft);
    assert_eq!(from_pack.history.len(), 1);

    let draft = trdraft_from_document(&doc, po.clone(), po, None);
    let from_draft = ActivePackage::from_draft("fallback-name.trdraft".into(), &draft);

    assert!(from_draft.is_draft);
    assert_eq!(from_draft.project_id, "fallback-name");
    assert_eq!(from_draft.po_filename, "fallback-name.po");
    assert!(!from_draft.pack_version.is_empty());
}

#[test]
fn package_readers_reject_bad_formats_and_draft_hash_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    assert!(read_trpack(&dir.path().join("missing.trpack")).is_err());
    assert!(read_trdraft(&dir.path().join("missing.trdraft")).is_err());

    let invalid_json = dir.path().join("invalid.trpack");
    std::fs::write(&invalid_json, "{not-json").unwrap();
    assert!(read_trpack(&invalid_json).is_err());

    let pack_path = dir.path().join("bad.trpack");
    std::fs::write(&pack_path, r#"{"format":"wrong"}"#).unwrap();
    assert!(
        read_trpack(&pack_path)
            .unwrap_err()
            .to_string()
            .contains("unsupported TRPack")
    );

    let incomplete_pack = dir.path().join("incomplete.trpack");
    std::fs::write(&incomplete_pack, r#"{"format":"TranslateR TRPack v1"}"#).unwrap();
    assert!(read_trpack(&incomplete_pack).is_err());

    let draft_path = dir.path().join("bad.trdraft");
    std::fs::write(&draft_path, r#"{"format":"wrong"}"#).unwrap();
    assert!(
        read_trdraft(&draft_path)
            .unwrap_err()
            .to_string()
            .contains("unsupported TRDraft")
    );

    let incomplete_draft = dir.path().join("incomplete.trdraft");
    std::fs::write(&incomplete_draft, r#"{"format":"TranslateR TRDraft v1"}"#).unwrap();
    assert!(read_trdraft(&incomplete_draft).is_err());

    let po = "msgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let doc = parse_text("de.po", po.clone()).unwrap();
    let mut pack = trpack_from_document(&doc, po.clone(), None, None);
    pack.base_hash = "not-the-real-hash".to_string();
    write_trpack(&pack_path, &pack).unwrap();
    assert!(
        read_trpack(&pack_path)
            .unwrap_err()
            .to_string()
            .contains("base hash")
    );

    let mut draft = trdraft_from_document(&doc, po.clone(), po, None);
    draft.base_hash = "not-the-real-hash".to_string();
    write_trdraft(&draft_path, &draft).unwrap();
    assert!(
        read_trdraft(&draft_path)
            .unwrap_err()
            .to_string()
            .contains("base hash")
    );

    assert!(
        write_trpack(
            &dir.path().join("missing-parent").join("out.trpack"),
            &trpack_from_document(
                &parse_text("de.po", "msgid \"Hello\"\nmsgstr \"\"\n".to_string()).unwrap(),
                "msgid \"Hello\"\nmsgstr \"\"\n".to_string(),
                None,
                None,
            )
        )
        .is_err()
    );
}

#[test]
fn draft_generated_tpatch_applies_to_original_trpack_base() {
    let base = "msgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let mut doc = parse_text("de.po", base.clone()).unwrap();
    let entry_id = doc.entries[0].id;
    set_translation(&mut doc, entry_id, 0, "Hallo".to_string());
    let current = translater::po::writer::write_document(&doc);

    let patch = unified_diff(&base, &current, "package-base", "translator-draft");
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
        contexts: Vec::new(),
        answers: Vec::new(),
        screenshots: Vec::new(),
    };
    let patch = unified_diff(
        "msgid \"A\"\nmsgstr \"\"\n",
        "msgid \"A\"\nmsgstr \"B\"\n",
        "a",
        "b",
    );
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

    let package_only_patch = add_tpatch_metadata(
        "# TranslateR TPatch v1\n--- base\n+++ edited\n line\n".to_string(),
        Some(&package),
        &[],
    );
    let metadata = parse_tpatch_metadata(&package_only_patch);
    assert_eq!(metadata.project_id.as_deref(), Some("game-ui"));
    assert!(metadata.questions.is_empty());
}

#[test]
fn tpatch_metadata_noops_and_invalid_question_json_are_safe() {
    assert_eq!(add_tpatch_metadata("patch".to_string(), None, &[]), "patch");
    assert_eq!(
        add_tpatch_metadata(
            String::new(),
            None,
            &[EntryQuestion {
                entry_id: "entry".to_string(),
                scope: "source".to_string(),
                question: "Question?".to_string(),
                created_at: "now".to_string(),
            }]
        ),
        ""
    );

    let metadata = parse_tpatch_metadata(
        "# TranslateR-Project: game\n# TranslateR-Package-Version: 2\n# TranslateR-Base-Hash: abc\n# TranslateR-Questions-Json: not-json\n",
    );
    assert_eq!(metadata.project_id.as_deref(), Some("game"));
    assert_eq!(metadata.pack_version.as_deref(), Some("2"));
    assert_eq!(metadata.base_hash.as_deref(), Some("abc"));
    assert!(metadata.questions.is_empty());

    let questions_only = add_tpatch_metadata(
        "# TranslateR TPatch v1\n--- base\n+++ edited\n line\n".to_string(),
        None,
        &[EntryQuestion {
            entry_id: "entry".to_string(),
            scope: "translation".to_string(),
            question: "Can this be shorter?".to_string(),
            created_at: "now".to_string(),
        }],
    );
    let metadata = parse_tpatch_metadata(&questions_only);
    assert_eq!(metadata.questions.len(), 1);
    assert_eq!(metadata.questions[0].scope, "translation");
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
fn change_summary_records_added_entries_empty_values_and_long_previews() {
    let long = "A".repeat(90);
    let base = format!("msgid \"{long}\"\nmsgstr \"\"\n");
    let edited =
        format!("msgid \"{long}\"\nmsgstr \"Translated\"\n\nmsgid \"Brand new\"\nmsgstr \"\"\n");

    let summary = change_summary("de.po", &base, &edited).unwrap();

    assert!(summary.line_additions > 0);
    assert!(
        summary
            .changed_translations
            .iter()
            .any(|change| change.contains("<empty> -> Translated"))
    );
    assert!(
        summary
            .changed_translations
            .iter()
            .any(|change| change.contains("Added entry: Brand new"))
    );
    assert!(
        summary
            .changed_translations
            .iter()
            .any(|change| change.contains("..."))
    );
    assert_ne!(next_pack_version("not-a-number"), "not-a-number");
}

#[test]
fn change_summary_uses_plural_indices_and_records_removed_entries() {
    let base = "msgid \"%d file\"\nmsgid_plural \"%d files\"\nmsgstr[1] \"%d files\"\nmsgstr[0] \"%d file\"\n\nmsgid \"Removed\"\nmsgstr \"Gone\"\n";
    let edited = "msgid \"%d file\"\nmsgid_plural \"%d files\"\nmsgstr[1] \"%d Dateien\"\nmsgstr[0] \"%d file\"\n";

    let summary = change_summary("de.po", base, edited).unwrap();

    assert!(
        summary
            .changed_translations
            .iter()
            .any(|change| change.contains("%d file form 1: %d files -> %d Dateien"))
    );
    assert!(
        summary
            .changed_translations
            .iter()
            .any(|change| change.contains("Removed entry: Removed"))
    );
    assert!(
        summary
            .changed_translations
            .iter()
            .all(|change| !change.contains("%d file form 0: %d files"))
    );
}

#[test]
fn materializes_workflow_po_under_short_hash_and_safe_filename() {
    let dir = tempfile::tempdir().unwrap();
    let path = materialize_workflow_po_in_dir(
        dir.path(),
        "1234567890abcdef9999",
        r"C:\unsafe\de.po",
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n",
    )
    .unwrap();
    assert_eq!(path.file_name().unwrap(), "de.po");
    assert!(path.to_string_lossy().contains("1234567890abcdef"));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n"
    );

    let default_name = materialize_workflow_po_in_dir(dir.path(), "abc", "", "").unwrap();
    assert_eq!(default_name.file_name().unwrap(), "translation.po");

    let file_as_config_dir = dir.path().join("not-a-dir");
    std::fs::write(&file_as_config_dir, "not a directory").unwrap();
    assert!(materialize_workflow_po_in_dir(&file_as_config_dir, "abc", "sample.po", "").is_err());
}

#[test]
fn maintainer_can_apply_translator_tpatch() {
    let base = "msgid \"Hello\"\nmsgstr \"\"\n";
    let translated = "msgid \"Hello\"\nmsgstr \"Hallo\"\n";
    let patch = unified_diff(base, translated, "base.po", "translator.po");
    let merged = apply_unified_patch(base, &patch).unwrap();
    assert_eq!(merged, translated);
}

#[test]
fn translator_tpatch_can_be_generated_without_writing_po() {
    let base = "msgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let mut doc = parse_text("translator.po", base.clone()).unwrap();
    let entry_id = doc.entries[0].id;
    set_translation(&mut doc, entry_id, 0, "Hallo".to_string());

    let edited = translater::po::writer::write_document(&doc);
    let patch = unified_diff(&base, &edited, "package-base", "translator.po");
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
