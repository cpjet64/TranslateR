use std::collections::BTreeSet;

use super::{
    Diagnostic, DiagnosticSeverity, EntryId, PoDocument, PoEntry, PoField, header::parse_header,
};

pub fn validate_document(doc: &mut PoDocument) {
    let header = parse_header(doc);
    let nplurals = header
        .plural_forms
        .as_ref()
        .map(|p| p.nplurals)
        .unwrap_or(2);

    let mut document_diags = doc.diagnostics.clone();
    for entry in &mut doc.entries {
        entry.diagnostics.clear();
        validate_entry(entry, nplurals);
        document_diags.extend(entry.diagnostics.clone());
    }
    doc.diagnostics = document_diags;
}

fn validate_entry(entry: &mut PoEntry, nplurals: usize) {
    if entry.is_header() {
        return;
    }

    if entry.has_flag("fuzzy") {
        push(entry, DiagnosticSeverity::Warning, "entry is fuzzy");
    }

    if entry.msgid_plural.is_some() {
        if entry.msgstr.len() < nplurals {
            push(
                entry,
                DiagnosticSeverity::Error,
                format!(
                    "plural entry has {} of {nplurals} forms",
                    entry.msgstr.len()
                ),
            );
        }
        if entry.msgstr.iter().all(|f| f.value().is_empty()) {
            push(
                entry,
                DiagnosticSeverity::Warning,
                "plural translation is empty",
            );
        } else if entry.msgstr.iter().any(|f| f.value().is_empty()) {
            push(
                entry,
                DiagnosticSeverity::Warning,
                "plural translation is incomplete",
            );
        }
    } else if entry.msgstr.first().is_none_or(|f| f.value().is_empty()) {
        push(entry, DiagnosticSeverity::Warning, "translation is empty");
    }

    validate_newline(entry);
    if entry.has_flag("c-format") {
        validate_c_format(entry);
    }
}

fn validate_newline(entry: &mut PoEntry) {
    let source_ends = entry.msgid.value().ends_with('\n');
    let translations = entry.msgstr.clone();
    for field in translations {
        if field.value().is_empty() {
            continue;
        }
        if field.value().ends_with('\n') != source_ends {
            push(
                entry,
                DiagnosticSeverity::Warning,
                "translation trailing newline differs from source",
            );
        }
    }
}

fn validate_c_format(entry: &mut PoEntry) {
    let mut expected = placeholders(&entry.msgid);
    if let Some(plural) = &entry.msgid_plural {
        expected.extend(placeholders(plural));
    }
    let translations = entry.msgstr.clone();
    for field in translations {
        let actual = placeholders(&field);
        if actual != expected {
            push(
                entry,
                DiagnosticSeverity::Warning,
                "printf placeholder mismatch",
            );
        }
    }
}

fn placeholders(field: &PoField) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let chars = field.value().chars().collect::<Vec<_>>();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        if i + 1 < chars.len() && chars[i + 1] == '%' {
            i += 2;
            continue;
        }
        let start = i;
        i += 1;
        while i < chars.len() {
            let ch = chars[i];
            if "diuoxXfFeEgGaAcspn".contains(ch) {
                set.insert(chars[start..=i].iter().collect());
                i += 1;
                break;
            }
            if ch.is_alphabetic() {
                i += 1;
                break;
            }
            i += 1;
        }
    }
    set
}

fn push(entry: &mut PoEntry, severity: DiagnosticSeverity, message: impl Into<String>) {
    entry.diagnostics.push(Diagnostic {
        entry_id: Some(EntryId(entry.id.0)),
        severity,
        message: message.into(),
    });
}
