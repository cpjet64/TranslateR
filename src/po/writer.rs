use anyhow::Result;

use super::{
    EntryId, PoDocument, PoEntry, PoField, PoFieldKind, RawLine, escape::encode_po_string,
};

pub fn write_document(doc: &PoDocument) -> Result<String> {
    if !doc.dirty && doc.entries.iter().all(|e| !entry_is_edited(e)) {
        return Ok(doc.original_text.clone());
    }

    let nl = doc.newline.as_str();
    let original_lines = doc
        .original_text
        .split_inclusive('\n')
        .map(|line| {
            line.trim_end_matches('\n')
                .trim_end_matches('\r')
                .to_string()
        })
        .collect::<Vec<_>>();
    let mut out = String::new();
    let mut cursor = 0usize;

    for entry in &doc.entries {
        let start = entry.span.start;
        if start > cursor {
            for line in &original_lines[cursor..start] {
                out.push_str(line);
                out.push_str(nl);
            }
        }

        if entry_is_edited(entry) {
            out.push_str(&write_entry(entry, nl));
        } else {
            for line in &original_lines[entry.span.start..=entry.span.end] {
                out.push_str(line);
                out.push_str(nl);
            }
        }
        cursor = entry.span.end + 1;
    }

    if cursor < original_lines.len() {
        for line in &original_lines[cursor..] {
            out.push_str(line);
            out.push_str(nl);
        }
    }

    if !doc.original_text.ends_with('\n') && out.ends_with(nl) {
        out.truncate(out.len() - nl.len());
    }
    Ok(out)
}

pub fn write_document_bytes(doc: &PoDocument) -> Result<Vec<u8>> {
    if !doc.dirty && doc.entries.iter().all(|e| !entry_is_edited(e)) {
        return Ok(doc.original_bytes.clone());
    }
    Ok(write_document(doc)?.into_bytes())
}

fn entry_is_edited(entry: &PoEntry) -> bool {
    entry.edited.translator_comments.is_some()
        || entry.edited.fuzzy.is_some()
        || entry.msgstr.iter().any(|f| f.edited_value.is_some())
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
        if field.edited_value.is_some() {
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
        if fuzzy && !flags.iter().any(|f| f == "fuzzy") {
            flags.push("fuzzy".to_string());
        }
        if !fuzzy {
            flags.retain(|f| f != "fuzzy");
        }
    }
    if flags == entry.flags && entry.edited.fuzzy.is_none() {
        write_raw_lines(&entry.comments.flags_raw, nl, out);
    } else if !flags.is_empty() {
        out.push_str("#, ");
        out.push_str(&flags.join(", "));
        out.push_str(nl);
    }
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
        let mut parts = value.split_inclusive('\n').peekable();
        while let Some(part) = parts.next() {
            out.push('"');
            out.push_str(&encode_po_string(part));
            out.push('"');
            out.push_str(nl);
        }
        if value.is_empty() || (!value.ends_with('\n') && parts.peek().is_none()) {
            // no-op; split_inclusive already emitted the final non-newline segment
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
    if let Some(entry) = doc.entries.iter_mut().find(|e| e.id == entry_id) {
        if let Some(field) = entry
            .msgstr
            .iter_mut()
            .find(|f| f.index.unwrap_or(0) == index)
        {
            if field.decoded != value {
                field.edited_value = Some(value);
                doc.dirty = true;
            }
        }
    }
}
