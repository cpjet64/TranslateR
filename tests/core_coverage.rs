use std::fs;

use translater::{
    po::{
        DiagnosticSeverity,
        escape::{decode_po_string, encode_po_string, quoted_payload},
        header::{parse_header, parse_plural_forms, set_header_language},
        parser::{parse_document, parse_text, parse_text_with_bytes},
        stats::{is_untranslated, stats},
        writer::{set_translation, write_document, write_document_bytes},
    },
    project::{AppConfig, ProjectState, document_store::PoFileSummary, scanner::scan_po_files},
    util::{
        atomic_save::{save_atomic, save_atomic_bytes},
        hashing::{sha256_bytes, sha256_text},
        paths::app_config_dir,
    },
    vcs::diff::{apply_unified_patch, unified_diff},
};

#[test]
fn stats_and_summary_count_translator_relevant_statuses() {
    let input = "msgid \"\"\nmsgstr \"\"\n\"Language: de\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n#, fuzzy\nmsgid \"Hello\"\nmsgstr \"\"\n\nmsgid \"Done\\n\"\nmsgstr \"Fertig\"\n"
        .to_string();
    let doc = parse_text("de.po", input).unwrap();

    let stats = stats(&doc);
    assert_eq!(stats.entries, 3);
    assert_eq!(stats.untranslated, 1);
    assert_eq!(stats.fuzzy, 1);
    assert!(stats.warnings >= 2);
    assert!(!is_untranslated(&doc.entries[0]));
    assert!(is_untranslated(&doc.entries[1]));

    let summary = PoFileSummary::from_doc(&doc);
    assert_eq!(summary.path, std::path::PathBuf::from("de.po"));
    assert_eq!(summary.language.as_deref(), Some("de"));
    assert_eq!(summary.stats.untranslated, 1);

    let state = ProjectState {
        root_dir: Some("root".into()),
        files: vec![summary],
        active_file: Some(0),
    };
    assert_eq!(state.files.len(), 1);
    assert_eq!(state.active_file, Some(0));
    assert_eq!(
        state.root_dir.as_deref(),
        Some(std::path::Path::new("root"))
    );
}

#[test]
fn scanner_finds_po_files_case_insensitively_and_sorts() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("nested")).unwrap();
    fs::write(dir.path().join("b.PO"), "msgid \"B\"\nmsgstr \"\"\n").unwrap();
    fs::write(
        dir.path().join("nested").join("a.po"),
        "msgid \"A\"\nmsgstr \"\"\n",
    )
    .unwrap();
    fs::write(dir.path().join("ignore.txt"), "").unwrap();

    let files = scan_po_files(dir.path());
    let names = files
        .iter()
        .map(|path| {
            path.strip_prefix(dir.path())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["b.PO", "nested/a.po"]);
}

#[test]
fn hashing_and_atomic_save_helpers_are_deterministic() {
    assert_eq!(
        sha256_text("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(sha256_text("abc"), sha256_bytes(b"abc"));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("translation.po");
    save_atomic(&path, "msgid \"A\"\nmsgstr \"B\"\n").unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "msgid \"A\"\nmsgstr \"B\"\n"
    );

    save_atomic_bytes(&path, b"replacement").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"replacement");

    let config_dir = app_config_dir().unwrap();
    assert!(
        config_dir
            .to_string_lossy()
            .to_ascii_lowercase()
            .contains("translater")
    );
}

#[test]
fn escape_helpers_cover_po_escape_edges() {
    let decoded = decode_po_string(r#"\n\t\r\b\f\a\v\\\" \? \' \z"#).unwrap();
    assert_eq!(decoded, "\n\t\r\u{8}\u{c}\u{7}\u{b}\\\" ? ' \\z");
    assert!(decode_po_string("\\").is_err());

    assert_eq!(
        encode_po_string("\n\t\r\u{8}\u{c}\u{7}\u{b}\\\""),
        r#"\n\t\r\b\f\a\v\\\""#
    );
    assert_eq!(quoted_payload("prefix \"hello\" suffix"), Some("hello"));
    assert_eq!(quoted_payload("no quote"), None);
    assert_eq!(quoted_payload("\"unterminated"), None);
}

#[test]
fn header_parsing_handles_absent_and_invalid_plural_headers() {
    let no_header = parse_text(
        "plain.po",
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n".to_string(),
    )
    .unwrap();
    assert!(parse_header(&no_header).language.is_none());

    let header_without_msgstr = parse_text("plain.po", "msgid \"\"\n".to_string()).unwrap();
    assert!(parse_header(&header_without_msgstr).language.is_none());

    assert_eq!(
        parse_plural_forms("plural=n; nplurals= 4 ;")
            .unwrap()
            .nplurals,
        4
    );
    assert!(parse_plural_forms("plural=n; nplural=2;").is_none());
    assert!(parse_plural_forms("nplurals=two; plural=n;").is_none());

    let header_with_unknown_line = parse_text(
        "plain.po",
        "msgid \"\"\nmsgstr \"\"\n\"Language: en\\n\"\n\"No separator\\n\"\n".to_string(),
    )
    .unwrap();
    assert_eq!(
        parse_header(&header_with_unknown_line).language.as_deref(),
        Some("en")
    );
}

#[test]
fn header_language_errors_and_preserves_missing_line_ending() {
    let mut no_header = parse_text(
        "plain.po",
        "msgid \"Hello\"\nmsgstr \"Hallo\"\n".to_string(),
    )
    .unwrap();
    assert!(
        set_header_language(&mut no_header, "de")
            .unwrap_err()
            .to_string()
            .contains("no header")
    );

    let mut header_without_msgstr = parse_text("plain.po", "msgid \"\"\n".to_string()).unwrap();
    assert!(
        set_header_language(&mut header_without_msgstr, "de")
            .unwrap_err()
            .to_string()
            .contains("no msgstr")
    );

    let mut no_header_newline = parse_text(
        "plain.po",
        "msgid \"\"\nmsgstr \"Language: en\"".to_string(),
    )
    .unwrap();
    set_header_language(&mut no_header_newline, "fr").unwrap();
    assert_eq!(
        write_document(&no_header_newline),
        "msgid \"\"\nmsgstr \"Language: fr\""
    );
}

#[test]
fn parser_covers_file_bytes_errors_obsolete_unknown_and_bad_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.po");
    fs::write(&path, b"#~ obsolete\nmsgid \"Gone\"\nmsgstr \"\"\n").unwrap();
    let doc = parse_document(&path).unwrap();
    assert!(doc.entries[0].obsolete);
    assert_eq!(
        doc.original_bytes,
        b"#~ obsolete\nmsgid \"Gone\"\nmsgstr \"\"\n"
    );

    let with_unknown = parse_text(
        "sample.po",
        "unexpected\nmsgid \"Known\"\nmsgstr \"\"\n".to_string(),
    )
    .unwrap();
    assert_eq!(with_unknown.entries[0].comments.unknown.len(), 1);

    let invalid = parse_text("bad.po", "msgstr \"orphan\"\n".to_string()).unwrap();
    assert!(
        invalid
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("entry has no msgid"))
    );

    let missing_bracket =
        parse_text("bad.po", "msgid \"A\"\nmsgstr[0 \"B\"\n".to_string()).unwrap();
    assert!(
        missing_bracket
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("missing ']'"))
    );

    let bad_index = parse_text("bad.po", "msgid \"A\"\nmsgstr[x] \"B\"\n".to_string()).unwrap();
    assert!(!bad_index.diagnostics.is_empty());

    let missing_quote = parse_text("bad.po", "msgid A\nmsgstr \"B\"\n".to_string()).unwrap();
    assert!(!missing_quote.diagnostics.is_empty());

    let bad_continuation = parse_text(
        "bad.po",
        "msgid \"A\"\n\"unterminated\nmsgstr \"B\"\n".to_string(),
    )
    .unwrap();
    assert!(!bad_continuation.diagnostics.is_empty());

    let bad_first_escape =
        parse_text("bad.po", "msgid \"A\\\"\nmsgstr \"B\"\n".to_string()).unwrap();
    assert!(!bad_first_escape.diagnostics.is_empty());

    let bad_continuation_escape =
        parse_text("bad.po", "msgid \"A\"\n\"B\\\"\nmsgstr \"C\"\n".to_string()).unwrap();
    assert!(!bad_continuation_escape.diagnostics.is_empty());

    let raw_bytes = vec![0xff, b'\n'];
    let lossy = parse_text_with_bytes(
        std::path::Path::new("bytes.po"),
        String::from_utf8_lossy(&raw_bytes).into(),
        raw_bytes.clone(),
    )
    .unwrap();
    assert_eq!(lossy.original_bytes, raw_bytes);
}

#[test]
fn writer_covers_comments_flags_crlf_plural_and_noop_paths() {
    let input = "# old\n#. extracted\n#: src/main.rs:1\n#, fuzzy, c-format\n#| msgid \"Old\"\nmsgid \"%d file\"\nmsgid_plural \"%d files\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n"
        .replace('\n', "\r\n");
    let mut doc = parse_text("plural.po", input.clone()).unwrap();
    assert_eq!(write_document_bytes(&doc), input.as_bytes());

    let id = doc.entries[0].id;
    set_translation(&mut doc, id, 0, String::new());
    assert!(!doc.dirty);

    doc.entries[0].edited.translator_comments = Some("new comment\n# already marked".to_string());
    doc.entries[0].edited.fuzzy = Some(false);
    set_translation(&mut doc, id, 1, "%d Dateien\n".to_string());

    let output = write_document(&doc);
    assert!(output.contains("# new comment\r\n# already marked\r\n"));
    assert!(
        output.contains("#. extracted\r\n#: src/main.rs:1\r\n#, c-format\r\n#| msgid \"Old\"\r\n")
    );
    assert!(output.contains("msgstr[1] \"\"\r\n\"%d Dateien\\n\"\r\n"));
    assert!(output.contains("\r\n"));
}

#[test]
fn writer_covers_no_final_newline_blank_comment_fuzzy_add_and_missing_targets() {
    let input = "# old\nmsgid \"Hello\"\nmsgstr \"Hallo\"".to_string();
    let mut doc = parse_text("single.po", input).unwrap();
    let id = doc.entries[0].id;

    set_translation(
        &mut doc,
        translater::po::EntryId(999),
        0,
        "Ignored".to_string(),
    );
    set_translation(&mut doc, id, 3, "Ignored".to_string());
    assert!(!doc.dirty);

    doc.entries[0].edited.translator_comments = Some("\nnew".to_string());
    doc.entries[0].edited.fuzzy = Some(true);
    set_translation(&mut doc, id, 0, "Guten Tag".to_string());

    let output = write_document(&doc);
    assert_eq!(
        output,
        "#\n# new\n#, fuzzy\nmsgid \"Hello\"\nmsgstr \"Guten Tag\""
    );
    assert_eq!(write_document_bytes(&doc), output.as_bytes());
}

#[test]
fn config_can_load_defaults_invalid_json_and_round_trip_custom_path() {
    let default = AppConfig::default();
    assert_eq!(default.translator_name, "Translator");
    assert_eq!(default.translator_email, "translator@local");

    let dir = tempfile::tempdir().unwrap();
    let missing_path = dir.path().join("missing").join("config.json");
    assert_eq!(
        AppConfig::load_from_path(&missing_path).translator_name,
        "Translator"
    );

    let invalid_path = dir.path().join("invalid.json");
    fs::write(&invalid_path, "{not json").unwrap();
    assert_eq!(
        AppConfig::load_from_path(&invalid_path).translator_email,
        "translator@local"
    );

    let custom = AppConfig {
        translator_name: "Ada".to_string(),
        translator_email: "ada@example.test".to_string(),
        ui_language: "en".to_string(),
    };
    let nested_path = dir.path().join("nested").join("config.json");
    custom.save_to_path(&nested_path).unwrap();
    let loaded = AppConfig::load_from_path(&nested_path);
    assert_eq!(loaded.translator_name, "Ada");
    assert_eq!(loaded.translator_email, "ada@example.test");

    let parent_file = dir.path().join("not-a-directory");
    fs::write(&parent_file, "blocks directory creation").unwrap();
    assert!(
        custom
            .save_to_path(&parent_file.join("config.json"))
            .is_err()
    );
    assert!(custom.save_to_path(&std::path::PathBuf::new()).is_err());

    assert!(AppConfig::load().translator_name.len() > 0);
}

#[test]
fn validation_accepts_matching_escaped_printf_and_reports_partial_plural() {
    let input = "#, c-format\nmsgid \"%1$s %% complete\"\nmsgstr \"%1$s %% fertig\"\n\nmsgid \"\"\nmsgstr \"Plural-Forms: nplurals=2; plural=n != 1;\\n\"\n\nmsgid \"%d file\"\nmsgid_plural \"%d files\"\nmsgstr[0] \"%d Datei\"\nmsgstr[1] \"\"\n"
        .to_string();
    let doc = parse_text("validate.po", input).unwrap();

    assert!(
        doc.entries[0]
            .diagnostics
            .iter()
            .all(|diag| !diag.message.contains("placeholder"))
    );
    assert!(doc.entries[2].diagnostics.iter().any(|diag| {
        diag.severity == DiagnosticSeverity::Warning
            && diag.message.contains("plural translation is incomplete")
    }));

    let unsupported_alpha = parse_text(
        "validate.po",
        "#, c-format\nmsgid \"%q value\"\nmsgstr \"%q Wert\"\n".to_string(),
    )
    .unwrap();
    assert!(
        unsupported_alpha.entries[0]
            .diagnostics
            .iter()
            .all(|diag| !diag.message.contains("placeholder"))
    );
}

#[test]
fn diff_apply_rejects_bad_context_and_unsupported_lines() {
    let patch = unified_diff("a\nb\n", "a\nc\n", "old", "new");
    assert_eq!(apply_unified_patch("a\nb\n", &patch).unwrap(), "a\nc\n");
    assert!(
        apply_unified_patch("x\nb\n", &patch)
            .unwrap_err()
            .to_string()
            .contains("context")
    );
    assert!(
        apply_unified_patch("a\n", "? unsupported\n")
            .unwrap_err()
            .to_string()
            .contains("unsupported")
    );
    assert_eq!(apply_unified_patch("a\n", "@@\n\n").unwrap(), "a\n");
    assert!(
        apply_unified_patch("a\n", "-missing\n")
            .unwrap_err()
            .to_string()
            .contains("deletion")
    );
}
