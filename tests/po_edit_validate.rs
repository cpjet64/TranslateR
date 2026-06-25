use translater::po::{
    DiagnosticSeverity,
    header::{
        HEADER_LANGUAGE_TEAM, HEADER_LAST_TRANSLATOR, HEADER_REVISION_DATE, parse_header,
        set_header_field, set_header_language,
    },
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
    let output = write_document(&doc);
    assert_eq!(output, "# hi\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n");
}

#[test]
fn deleting_back_to_original_translation_clears_transient_edit() {
    let input = "msgid \"Small battery for storing excessive energy.\"\nmsgstr \"Small battery for storing excessive energy.\"\n"
        .to_string();
    let mut doc = parse_text("sample.po", input.clone()).unwrap();
    let id = doc.entries[0].id;

    set_translation(
        &mut doc,
        id,
        0,
        "Small battery for storing excessive energy.a".to_string(),
    );
    assert!(doc.dirty);
    assert_eq!(
        doc.entries[0].msgstr[0].value(),
        "Small battery for storing excessive energy.a"
    );

    set_translation(
        &mut doc,
        id,
        0,
        "Small battery for storing excessive energy.".to_string(),
    );

    assert!(!doc.dirty);
    assert_eq!(
        doc.entries[0].msgstr[0].edited_value.as_deref(),
        Some("Small battery for storing excessive energy.")
    );
    assert_eq!(
        doc.entries[0].msgstr[0].value(),
        "Small battery for storing excessive energy."
    );
    assert_eq!(write_document(&doc), input);
}

#[test]
fn terminal_exclamation_can_be_removed_and_retyped() {
    let input = "msgid \"Enemy Frigate inbound!\"\nmsgstr \"Enemy Frigate inbound!\"\n".to_string();
    let mut doc = parse_text("sample.po", input.clone()).unwrap();
    let id = doc.entries[0].id;

    set_translation(&mut doc, id, 0, "Enemy Frigate inbound".to_string());
    assert!(doc.dirty);
    assert_eq!(doc.entries[0].msgstr[0].value(), "Enemy Frigate inbound");

    set_translation(&mut doc, id, 0, "Enemy Frigate inbound!".to_string());
    assert!(!doc.dirty);
    assert_eq!(
        doc.entries[0].msgstr[0].edited_value.as_deref(),
        Some("Enemy Frigate inbound!")
    );
    assert_eq!(doc.entries[0].msgstr[0].value(), "Enemy Frigate inbound!");
    assert_eq!(write_document(&doc), input);

    set_translation(&mut doc, id, 0, "Enemy Frigate inbound?".to_string());
    assert!(doc.dirty);
    assert_eq!(doc.entries[0].msgstr[0].value(), "Enemy Frigate inbound?");

    set_translation(&mut doc, id, 0, "Enemy Frigate inbound!".to_string());
    assert!(!doc.dirty);
    assert_eq!(
        doc.entries[0].msgstr[0].edited_value.as_deref(),
        Some("Enemy Frigate inbound!")
    );
}

#[test]
fn terminal_question_mark_can_be_removed_and_retyped() {
    let input = "msgid \"Is the enemy inbound?\"\nmsgstr \"Is the enemy inbound?\"\n".to_string();
    let mut doc = parse_text("sample.po", input.clone()).unwrap();
    let id = doc.entries[0].id;

    set_translation(&mut doc, id, 0, "Is the enemy inbound".to_string());
    assert!(doc.dirty);
    assert_eq!(doc.entries[0].msgstr[0].value(), "Is the enemy inbound");

    set_translation(&mut doc, id, 0, "Is the enemy inbound?".to_string());
    assert!(!doc.dirty);
    assert_eq!(
        doc.entries[0].msgstr[0].edited_value.as_deref(),
        Some("Is the enemy inbound?")
    );
    assert_eq!(doc.entries[0].msgstr[0].value(), "Is the enemy inbound?");
    assert_eq!(write_document(&doc), input);

    set_translation(&mut doc, id, 0, "Is the enemy inbound!".to_string());
    assert!(doc.dirty);
    assert_eq!(doc.entries[0].msgstr[0].value(), "Is the enemy inbound!");
}

#[test]
fn edits_multiline_translation() {
    let input = "msgid \"Hello\"\nmsgstr \"\"\n".to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    let id = doc.entries[0].id;
    set_translation(&mut doc, id, 0, "Line 1\nLine 2".to_string());
    let output = write_document(&doc);
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
    let output = write_document(&doc);
    assert!(output.contains("\"Project-Id-Version: sample\\n\"\n"));
    assert!(output.contains("\"Language: fr_CA\\n\"\n"));
}

#[test]
fn edits_header_language_without_blank_separator() {
    let input = "msgid \"\"\nmsgstr \"\"\n\"Project-Id-Version: sample\\n\"\n\"Language: en\\n\"\nmsgctxt \"003D37A143C7B141BE4271B075763B5C\"\nmsgid \"\"\nmsgstr \"First unit\"\n\nmsgid \"Second\"\nmsgstr \"Second unit\"\n"
        .to_string();
    let mut doc = parse_text("sample.po", input).unwrap();

    assert_eq!(doc.entries.len(), 3);
    assert!(doc.entries[0].is_header());
    assert!(!doc.entries[1].is_header());

    set_header_language(&mut doc, "zh-Hans").unwrap();
    let output = write_document(&doc);

    assert!(output.contains("\"Language: zh-Hans\\n\"\n"));
    assert!(output.contains(
        "msgctxt \"003D37A143C7B141BE4271B075763B5C\"\nmsgid \"\"\nmsgstr \"First unit\"\n"
    ));
    assert!(output.contains("msgid \"Second\"\nmsgstr \"Second unit\"\n"));
}

#[test]
fn edits_header_language_keeps_realistic_context_entry() {
    let input = "# ILL Space_EN English translation.\n# Copyright Epic Games, Inc. All Rights Reserved.\n# \nmsgid \"\"\nmsgstr \"\"\n\"Project-Id-Version: ILL Space_EN\\n\"\n\"Language: en\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n#. Key:\t003D37A143C7B141BE4271B075763B5C\n#. SourceLocation:\t/Game/Data/TipsNTricksDataTable.TipsNTricksDataTable.NewRow_4.Text\n#: /Game/Data/TipsNTricksDataTable.TipsNTricksDataTable.NewRow_4.Text\nmsgctxt \",003D37A143C7B141BE4271B075763B5C\"\nmsgid \"Press and hold Space to engage retro-boosters and slow your ship to a stop.\"\nmsgstr \"Press and hold Space to engage retro-boosters and slow your ship to a stop.\"\n"
        .to_string();
    let mut doc = parse_text("sample.po", input).unwrap();

    set_header_language(&mut doc, "de").unwrap();
    let output = write_document(&doc);

    assert!(output.contains("\"Language: de\\n\"\n"));
    assert!(output.contains("msgctxt \",003D37A143C7B141BE4271B075763B5C\"\nmsgid \"Press and hold Space to engage retro-boosters and slow your ship to a stop.\"\nmsgstr \"Press and hold Space to engage retro-boosters and slow your ship to a stop.\"\n"));
}

#[test]
fn editing_entry_preserves_blank_separator_before_next_entry() {
    let input = "msgid \"One\"\nmsgstr \"\"\n\nmsgid \"Two\"\nmsgstr \"Two\"\n".to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    let id = doc.entries[0].id;
    set_translation(&mut doc, id, 0, "Uno".to_string());

    let output = write_document(&doc);
    assert_eq!(
        output,
        "msgid \"One\"\nmsgstr \"Uno\"\n\nmsgid \"Two\"\nmsgstr \"Two\"\n"
    );

    let reparsed = parse_text("sample.po", output).unwrap();
    assert_eq!(reparsed.entries.len(), 2);
    assert_eq!(reparsed.entries[0].msgstr.len(), 1);
    assert_eq!(reparsed.entries[1].msgstr.len(), 1);
    assert_eq!(reparsed.entries[1].msgid.decoded, "Two");
}

#[test]
fn editing_middle_context_entry_does_not_absorb_next_unit_as_form() {
    let input = "msgctxt \",003D37A143C7B141BE4271B075763B5C\"\nmsgid \"Press and hold Space to engage retro-boosters and slow your ship to a stop.\"\nmsgstr \"\"\n\nmsgctxt \",00B8E7AC472576DCA01289AAEEB59777\"\nmsgid \"Small battery for storing excessive energy.\"\nmsgstr \"\"\n\nmsgctxt \",build-solar-panels\"\nmsgid \"Build Solar Panels\"\nmsgstr \"Build Solar Panels\"\n"
        .to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    let id = doc.entries[1].id;
    set_translation(&mut doc, id, 0, "一个小型电池".to_string());

    let output = write_document(&doc);
    let reparsed = parse_text("sample.po", output).unwrap();

    assert_eq!(reparsed.entries.len(), 3);
    assert_eq!(reparsed.entries[1].msgstr.len(), 1);
    assert_eq!(reparsed.entries[1].msgstr[0].decoded, "一个小型电池");
    assert_eq!(reparsed.entries[2].msgid.decoded, "Build Solar Panels");
    assert_eq!(reparsed.entries[2].msgstr.len(), 1);
    assert_eq!(reparsed.entries[2].msgstr[0].decoded, "Build Solar Panels");
}

#[test]
fn appends_missing_header_language() {
    let input =
        "msgid \"\"\nmsgstr \"\"\n\"Project-Id-Version: sample\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n"
            .to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    set_header_language(&mut doc, "de").unwrap();

    assert_eq!(parse_header(&doc).language.as_deref(), Some("de"));
    let output = write_document(&doc);
    assert!(output.contains("\"Content-Type: text/plain; charset=UTF-8\\n\"\n"));
    assert!(output.contains("\"Language: de\\n\"\n"));
}

#[test]
fn edits_header_metadata_without_absorbing_next_entry() {
    let input = "msgid \"\"\nmsgstr \"\"\n\"Project-Id-Version: sample\\n\"\n\"Language: en\\n\"\n\"PO-Revision-Date: 2022-01-01 00:00+0000\\n\"\n\"Last-Translator: Old Name\\n\"\n\nmsgctxt \",first\"\nmsgid \"First\"\nmsgstr \"First translation\"\n\nmsgid \"Second\"\nmsgstr \"Second translation\"\n"
        .to_string();
    let mut doc = parse_text("sample.po", input).unwrap();

    set_header_field(&mut doc, HEADER_REVISION_DATE, "2026-06-24 09:30+0000").unwrap();
    set_header_field(&mut doc, HEADER_LAST_TRANSLATOR, "New Translator").unwrap();
    set_header_field(&mut doc, HEADER_LANGUAGE_TEAM, "French").unwrap();

    let header = parse_header(&doc);
    assert_eq!(
        header.revision_date.as_deref(),
        Some("2026-06-24 09:30+0000")
    );
    assert_eq!(header.last_translator.as_deref(), Some("New Translator"));
    assert_eq!(header.language_team.as_deref(), Some("French"));

    let output = write_document(&doc);
    let reparsed = parse_text("sample.po", output).unwrap();
    assert_eq!(reparsed.entries.len(), 3);
    assert_eq!(reparsed.entries[1].msgid.decoded, "First");
    assert_eq!(reparsed.entries[1].msgstr[0].decoded, "First translation");
    assert_eq!(reparsed.entries[2].msgid.decoded, "Second");
    assert_eq!(reparsed.entries[2].msgstr[0].decoded, "Second translation");
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
fn detects_repeated_c_format_placeholder_mismatch() {
    let input = "#, c-format\nmsgid \"%s %s\"\nmsgstr \"%s\"\n".to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    validate_document(&mut doc);

    assert!(doc.entries[0]
        .diagnostics
        .iter()
        .any(|d| d.message.contains("placeholder") && d.severity == DiagnosticSeverity::Warning));
}

#[test]
fn validation_does_not_duplicate_document_diagnostics() {
    let input = "#, fuzzy\nmsgid \"Hello\\n\"\nmsgstr \"Hallo\"\n".to_string();
    let mut doc = parse_text("sample.po", input).unwrap();
    let first = doc.diagnostics.clone();

    validate_document(&mut doc);
    validate_document(&mut doc);

    assert_eq!(doc.diagnostics, first);
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
