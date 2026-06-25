use super::{
    EntryId, PoDocument, PoEntry, PoField, PoFieldKind, RawLine, escape::encode_po_string,
};

pub fn write_document(doc: &PoDocument) -> String {
    if !doc.dirty && !document_is_edited(doc) {
        return doc.original_text.clone();
    }

    let nl = doc.newline.as_str();
    let original_lines = original_text_lines(&doc.original_text);
    let mut out = String::new();
    let mut cursor = 0usize;

    let mut entry_idx = 0usize;
    while entry_idx < doc.entries.len() {
        let entry = &doc.entries[entry_idx];
        let start = entry.span.start;
        if start > cursor {
            push_original_lines(&original_lines, cursor, start, nl, &mut out);
        }

        if entry_is_edited(entry) {
            out.push_str(&write_entry(entry, nl));
        } else {
            push_original_lines(
                &original_lines,
                entry.span.start,
                entry.span.end + 1,
                nl,
                &mut out,
            );
        }
        cursor = entry.span.end + 1;
        entry_idx += 1;
    }

    if cursor < original_lines.len() {
        for line in &original_lines[cursor..] {
            out.push_str(line);
            out.push_str(nl);
        }
    }

    if !doc.original_text.ends_with('\n') {
        trim_final_newline(&mut out, nl);
    }
    out
}

pub fn write_document_bytes(doc: &PoDocument) -> Vec<u8> {
    if !doc.dirty && !document_is_edited(doc) {
        return doc.original_bytes.clone();
    }
    write_document(doc).into_bytes()
}

fn entry_is_edited(entry: &PoEntry) -> bool {
    if entry.edited.translator_comments.is_some() {
        return true;
    }
    if entry.edited.fuzzy.is_some() {
        return true;
    }
    let mut idx = 0usize;
    while idx < entry.msgstr.len() {
        let field = &entry.msgstr[idx];
        if field_is_edited(field) {
            return true;
        }
        idx += 1;
    }
    false
}

fn field_is_edited(field: &PoField) -> bool {
    match field.edited_value.as_ref() {
        Some(value) => value != &field.decoded,
        None => false,
    }
}

pub(crate) fn document_is_edited(doc: &PoDocument) -> bool {
    let mut idx = 0usize;
    while idx < doc.entries.len() {
        if entry_is_edited(&doc.entries[idx]) {
            return true;
        }
        idx += 1;
    }
    false
}

fn original_text_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find('\n') {
        let line = &rest[..=pos];
        lines.push(
            line.trim_end_matches('\n')
                .trim_end_matches('\r')
                .to_string(),
        );
        rest = &rest[pos + 1..];
    }
    if !rest.is_empty() {
        lines.push(rest.trim_end_matches('\r').to_string());
    }
    lines
}

fn push_original_lines(
    original_lines: &[String],
    start: usize,
    end: usize,
    nl: &str,
    out: &mut String,
) {
    let mut idx = start;
    while idx < end {
        out.push_str(&original_lines[idx]);
        out.push_str(nl);
        idx += 1;
    }
}

fn trim_final_newline(out: &mut String, nl: &str) {
    if out.ends_with(nl) {
        let final_len = out.len() - nl.len();
        out.truncate(final_len);
    }
}

fn write_entry(entry: &PoEntry, nl: &str) -> String {
    let mut out = String::new();
    write_translator_comments(entry, nl, &mut out);
    write_raw_lines(&entry.comments.extracted, nl, &mut out);
    write_raw_lines(&entry.comments.reference, nl, &mut out);
    write_flags(entry, nl, &mut out);
    write_raw_lines(&entry.comments.previous, nl, &mut out);
    write_raw_lines(&entry.comments.unknown, nl, &mut out);

    if let Some(field) = &entry.msgctxt {
        write_raw_field(field, nl, &mut out);
    }
    write_raw_field(&entry.msgid, nl, &mut out);
    if let Some(field) = &entry.msgid_plural {
        write_raw_field(field, nl, &mut out);
    }
    for field in &entry.msgstr {
        if field_is_edited(field) {
            write_generated_field(field, nl, &mut out);
        } else {
            write_raw_field(field, nl, &mut out);
        }
    }
    out
}

fn write_translator_comments(entry: &PoEntry, nl: &str, out: &mut String) {
    if let Some(comments) = &entry.edited.translator_comments {
        for line in comments.lines() {
            if line.trim().is_empty() {
                out.push('#');
            } else {
                out.push_str("# ");
                out.push_str(line.trim_start_matches("# ").trim_start_matches('#'));
            }
            out.push_str(nl);
        }
    } else {
        write_raw_lines(&entry.comments.translator, nl, out);
    }
}

fn write_flags(entry: &PoEntry, nl: &str, out: &mut String) {
    let mut flags = entry.flags.clone();
    if let Some(fuzzy) = entry.edited.fuzzy {
        if fuzzy && !contains_flag(&flags, "fuzzy") {
            flags.push("fuzzy".to_string());
        }
        if !fuzzy {
            remove_flag(&mut flags, "fuzzy");
        }
    }
    if flags == entry.flags && entry.edited.fuzzy.is_none() {
        write_raw_lines(&entry.comments.flags_raw, nl, out);
        return;
    }

    if flags.is_empty() {
        return;
    }

    out.push_str("#, ");
    out.push_str(&flags.join(", "));
    out.push_str(nl);
}

fn contains_flag(flags: &[String], needle: &str) -> bool {
    let mut idx = 0usize;
    while idx < flags.len() {
        if flags[idx] == needle {
            return true;
        }
        idx += 1;
    }
    false
}

fn remove_flag(flags: &mut Vec<String>, needle: &str) {
    let mut kept = Vec::with_capacity(flags.len());
    let mut idx = 0usize;
    while idx < flags.len() {
        if flags[idx] != needle {
            kept.push(flags[idx].clone());
        }
        idx += 1;
    }
    *flags = kept;
}

fn write_raw_lines(lines: &[RawLine], nl: &str, out: &mut String) {
    for line in lines {
        out.push_str(&line.text_without_newline);
        out.push_str(nl);
    }
}

fn write_raw_field(field: &PoField, nl: &str, out: &mut String) {
    for line in &field.raw_lines {
        out.push_str(&line.text_without_newline);
        out.push_str(nl);
    }
}

fn write_generated_field(field: &PoField, nl: &str, out: &mut String) {
    let value = field.value();
    let prefix = match field.kind {
        PoFieldKind::MsgStr => match field.index {
            Some(index) => format!("msgstr[{index}]"),
            None => "msgstr".to_string(),
        },
        PoFieldKind::MsgCtxt => "msgctxt".to_string(),
        PoFieldKind::MsgId => "msgid".to_string(),
        PoFieldKind::MsgIdPlural => "msgid_plural".to_string(),
    };

    if value.contains('\n') {
        out.push_str(&prefix);
        out.push_str(" \"\"");
        out.push_str(nl);
        for part in value.split_inclusive('\n') {
            out.push('"');
            out.push_str(&encode_po_string(part));
            out.push('"');
            out.push_str(nl);
        }
    } else {
        out.push_str(&prefix);
        out.push_str(" \"");
        out.push_str(&encode_po_string(value));
        out.push('"');
        out.push_str(nl);
    }
}

pub fn set_translation(doc: &mut PoDocument, entry_id: EntryId, index: usize, value: String) {
    let mut entry_idx = 0usize;
    while entry_idx < doc.entries.len() {
        if doc.entries[entry_idx].id == entry_id {
            set_entry_translation(doc, entry_idx, index, value);
            return;
        }
        entry_idx += 1;
    }
}

fn set_entry_translation(doc: &mut PoDocument, entry_idx: usize, index: usize, value: String) {
    let mut field_idx = 0usize;
    while field_idx < doc.entries[entry_idx].msgstr.len() {
        let field_index = doc.entries[entry_idx].msgstr[field_idx].index.unwrap_or(0);
        if field_index == index {
            let field = &mut doc.entries[entry_idx].msgstr[field_idx];
            if field.edited_value.as_deref() == Some(value.as_str()) {
                return;
            }
            field.edited_value = Some(value);
            doc.dirty = document_is_edited(doc);
            return;
        }
        field_idx += 1;
    }
}

pub fn effective_fuzzy(entry: &PoEntry) -> bool {
    entry
        .edited
        .fuzzy
        .unwrap_or_else(|| contains_flag(&entry.flags, "fuzzy"))
}

pub fn translator_comments_text(entry: &PoEntry) -> String {
    if let Some(comments) = &entry.edited.translator_comments {
        return comments.clone();
    }

    raw_translator_comments_text(entry)
}

fn raw_translator_comments_text(entry: &PoEntry) -> String {
    let mut lines = Vec::new();
    for line in &entry.comments.translator {
        let raw = line.text_without_newline.as_str();
        let text = raw
            .strip_prefix("# ")
            .or_else(|| raw.strip_prefix('#'))
            .unwrap_or(raw);
        lines.push(text.to_string());
    }
    lines.join("\n")
}

pub fn set_fuzzy(doc: &mut PoDocument, entry_id: EntryId, fuzzy: bool) {
    let mut entry_idx = 0usize;
    while entry_idx < doc.entries.len() {
        if doc.entries[entry_idx].id == entry_id {
            let original = contains_flag(&doc.entries[entry_idx].flags, "fuzzy");
            doc.entries[entry_idx].edited.fuzzy =
                if fuzzy == original { None } else { Some(fuzzy) };
            doc.dirty = document_is_edited(doc);
            return;
        }
        entry_idx += 1;
    }
}

pub fn set_translator_comments(doc: &mut PoDocument, entry_id: EntryId, comments: String) {
    let mut entry_idx = 0usize;
    while entry_idx < doc.entries.len() {
        if doc.entries[entry_idx].id == entry_id {
            let original = raw_translator_comments_text(&doc.entries[entry_idx]);
            doc.entries[entry_idx].edited.translator_comments = if comments == original {
                None
            } else {
                Some(comments)
            };
            doc.dirty = document_is_edited(doc);
            return;
        }
        entry_idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        effective_fuzzy, set_fuzzy, set_translation, set_translator_comments,
        translator_comments_text, write_document, write_document_bytes, write_generated_field,
    };
    use crate::po::{PoField, PoFieldKind, parser::parse_text};

    fn field(kind: PoFieldKind, value: &str) -> PoField {
        PoField {
            kind,
            index: None,
            raw_lines: Vec::new(),
            decoded: "raw".to_string(),
            edited_value: Some(value.to_string()),
        }
    }

    #[test]
    fn generated_fields_cover_all_source_field_prefixes() {
        let mut out = String::new();
        write_generated_field(&field(PoFieldKind::MsgCtxt, "context"), "\n", &mut out);
        write_generated_field(&field(PoFieldKind::MsgId, "source"), "\n", &mut out);
        write_generated_field(&field(PoFieldKind::MsgIdPlural, "sources"), "\n", &mut out);

        assert!(out.contains("msgctxt \"context\"\n"));
        assert!(out.contains("msgid \"source\"\n"));
        assert!(out.contains("msgid_plural \"sources\"\n"));
    }

    #[test]
    fn fuzzy_clear_existing_fuzzy_and_noop_translation_paths() {
        let mut doc = parse_text(
            "flags.po",
            "#, fuzzy\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n".to_string(),
        )
        .unwrap();
        let id = doc.entries[0].id;

        doc.entries[0].edited.fuzzy = Some(false);
        assert_eq!(write_document(&doc), "msgid \"Hello\"\nmsgstr \"Hallo\"\n");

        set_translation(&mut doc, id, 0, "Hallo".to_string());
        assert!(doc.dirty);
        let edited = doc.entries[0].msgstr[0].edited_value.clone();
        set_translation(&mut doc, id, 0, "Hallo".to_string());
        assert_eq!(doc.entries[0].msgstr[0].edited_value, edited);
    }

    #[test]
    fn fuzzy_add_is_noop_when_flag_already_exists() {
        let mut doc = parse_text(
            "flags.po",
            "#, fuzzy, c-format\nmsgid \"%s\"\nmsgstr \"%s\"\n".to_string(),
        )
        .unwrap();
        doc.entries[0].edited.fuzzy = Some(true);

        let output = write_document(&doc);
        assert_eq!(output, "#, fuzzy, c-format\nmsgid \"%s\"\nmsgstr \"%s\"\n");
    }

    #[test]
    fn fuzzy_add_finds_existing_flag_after_other_flags() {
        let mut doc = parse_text(
            "flags.po",
            "#, c-format, fuzzy\nmsgid \"%s\"\nmsgstr \"%s\"\n".to_string(),
        )
        .unwrap();
        doc.entries[0].edited.fuzzy = Some(true);

        let output = write_document(&doc);
        assert_eq!(output, "#, c-format, fuzzy\nmsgid \"%s\"\nmsgstr \"%s\"\n");
    }

    #[test]
    fn fuzzy_setter_toggles_and_clears_to_original_state() {
        let mut doc = parse_text(
            "flags.po",
            "msgid \"Hello\"\nmsgstr \"Hallo\"\n".to_string(),
        )
        .unwrap();
        let id = doc.entries[0].id;

        assert!(!effective_fuzzy(&doc.entries[0]));
        set_fuzzy(&mut doc, id, true);
        assert!(doc.dirty);
        assert!(effective_fuzzy(&doc.entries[0]));
        assert_eq!(
            write_document(&doc),
            "#, fuzzy\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n"
        );

        set_fuzzy(&mut doc, id, false);
        assert!(!doc.dirty);
        assert!(!effective_fuzzy(&doc.entries[0]));
        assert_eq!(write_document(&doc), doc.original_text);
    }

    #[test]
    fn translator_comments_setter_rewrites_only_comment_block() {
        let mut doc = parse_text(
            "comments.po",
            "# Existing note\n#. Extracted\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n".to_string(),
        )
        .unwrap();
        let id = doc.entries[0].id;

        assert_eq!(translator_comments_text(&doc.entries[0]), "Existing note");
        set_translator_comments(&mut doc, id, "Needs context\nSecond line".to_string());

        assert!(doc.dirty);
        assert_eq!(
            write_document(&doc),
            "# Needs context\n# Second line\n#. Extracted\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n"
        );

        set_translator_comments(&mut doc, id, "Existing note".to_string());
        assert!(!doc.dirty);
        assert_eq!(write_document(&doc), doc.original_text);
    }

    #[test]
    fn write_document_bytes_returns_original_bytes_for_clean_document() {
        let input = b"msgid \"Hello\"\r\nmsgstr \"Hallo\"\r\n".to_vec();
        let text = String::from_utf8_lossy(&input).into_owned();
        let doc = crate::po::parser::parse_text_with_bytes(
            std::path::Path::new("crlf.po"),
            text,
            input.clone(),
        )
        .unwrap();

        assert_eq!(write_document_bytes(&doc), input);
    }

    #[test]
    fn write_document_bytes_detects_edits_even_when_dirty_flag_is_clear() {
        let mut doc = parse_text("bytes.po", "msgid \"Hello\"\nmsgstr \"\"\n".to_string()).unwrap();
        doc.entries[0].msgstr[0].edited_value = Some("Hallo".to_string());

        assert_eq!(
            String::from_utf8(write_document_bytes(&doc)).unwrap(),
            "msgid \"Hello\"\nmsgstr \"Hallo\"\n"
        );
    }

    #[test]
    fn generated_multiline_field_covers_final_segment_cases() {
        let mut out = String::new();
        write_generated_field(&field(PoFieldKind::MsgStr, "one\ntwo"), "\n", &mut out);
        assert_eq!(out, "msgstr \"\"\n\"one\\n\"\n\"two\"\n");
    }

    #[test]
    fn dirty_empty_document_writes_without_synthetic_newline() {
        let mut doc = parse_text("empty.po", String::new()).unwrap();
        doc.dirty = true;

        assert_eq!(write_document(&doc), "");
    }
}
