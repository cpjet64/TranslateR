use translater::po::{
    DiagnosticSeverity,
    header::{parse_header, set_header_language},
    parser::parse_text,
    validate::validate_document,
    writer::{set_translation, write_document},
};

#[test]
fn edits_singular_translation_only() {
    let input = "# hi\nmsgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    let id = doc.entries[0].id;
    set_translation(&mut doc, id, 0, "Hallo".to_string());
    let output = write_document(&doc).unwrap();
    assert_eq!(output, "# hi\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n");
}

#[test]
fn edits_multiline_translation() {
    let input = "msgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    let id = doc.entries[0].id;
    set_translation(&mut doc, id, 0, "Line 1\nLine 2".to_string());
    let output = write_document(&doc).unwrap();
    assert!(output.contains("msgstr \"\"\n\"Line 1\\n\"\n\"Line 2\"\n"));
}

#[test]
fn edits_header_language() {
    let input =
        "msgid \"\"\nmsgstr \"\"\n\"Project-Id-Version: sample\\n\"\n\"Language: ar\\n\"\n\n"
            .to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    set_header_language(&mut doc, "fr_CA").unwrap();

    assert_eq!(parse_header(&doc).language.as_deref(), Some("fr_CA"));
    let output = write_document(&doc).unwrap();
    assert!(output.contains("\"Project-Id-Version: sample\\n\"\n"));
    assert!(output.contains("\"Language: fr_CA\\n\"\n"));
}

#[test]
fn appends_missing_header_language() {
    let input =
        "msgid \"\"\nmsgstr \"\"\n\"Project-Id-Version: sample\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n"
            .to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    set_header_language(&mut doc, "de").unwrap();

    assert_eq!(parse_header(&doc).language.as_deref(), Some("de"));
    let output = write_document(&doc).unwrap();
    assert!(output.contains("\"Content-Type: text/plain; charset=UTF-8\\n\"\n"));
    assert!(output.contains("\"Language: de\\n\"\n"));
}

#[test]
fn rejects_invalid_header_language() {
    let input = "msgid \"\"\nmsgstr \"Language: ar\\n\"\n\n".to_string();
    let mut doc = parse_text("sample.po", input).unwrap();

    assert!(set_header_language(&mut doc, "").is_err());
    assert!(set_header_language(&mut doc, "fr\nCA").is_err());
}

#[test]
fn detects_c_format_mismatch() {
    let input = "#, c-format\nmsgid \"%d file\"\nmsgstr \"%s Datei\"\n".to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    validate_document(&mut doc);
    assert!(doc.entries[0]
        .diagnostics
        .iter()
        .any(|d| d.message.contains("placeholder") && d.severity == DiagnosticSeverity::Warning));
}

#[test]
fn detects_missing_plural_form() {
    let input = "msgid \"\"\nmsgstr \"Plural-Forms: nplurals=3; plural=n;\\n\"\n\nmsgid \"%d file\"\nmsgid_plural \"%d files\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n".to_string();
    let doc = parse_text("sample.po", input).unwrap();
    assert!(
        doc.entries[1]
            .diagnostics
            .iter()
            .any(|d| d.message.contains("plural entry has 2 of 3"))
    );
}

#[test]
fn detects_newline_mismatch_and_fuzzy() {
    let input = "#, fuzzy\nmsgid \"Hello\\n\"\nmsgstr \"Hallo\"\n".to_string();
    let doc = parse_text("sample.po", input).unwrap();
    let messages = doc.entries[0]
        .diagnostics
        .iter()
        .map(|d| d.message.as_str())
        .collect::<Vec<_>>();
    assert!(messages.iter().any(|m| m.contains("fuzzy")));
    assert!(messages.iter().any(|m| m.contains("newline")));
}
