use std::collections::BTreeMap;

use crate::i18n::{tr, tr_format};

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

    let mut document_diags = doc
        .diagnostics
        .iter()
        .filter(|diag| diag.entry_id.is_none())
        .cloned()
        .collect::<Vec<_>>();
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
        push(entry, DiagnosticSeverity::Warning, tr("entry is fuzzy"));
    }

    if entry.msgid_plural.is_some() {
        if entry.msgstr.len() < nplurals {
            push(
                entry,
                DiagnosticSeverity::Error,
                tr_format(
                    "plural entry has {actual} of {expected} forms",
                    &[
                        ("actual", entry.msgstr.len().to_string()),
                        ("expected", nplurals.to_string()),
                    ],
                ),
            );
        }
        if entry.msgstr.iter().all(|f| f.value().is_empty()) {
            push(
                entry,
                DiagnosticSeverity::Warning,
                tr("plural translation is empty"),
            );
        } else if entry.msgstr.iter().any(|f| f.value().is_empty()) {
            push(
                entry,
                DiagnosticSeverity::Warning,
                tr("plural translation is incomplete"),
            );
        }
    } else if entry.msgstr.first().is_none_or(|f| f.value().is_empty()) {
        push(
            entry,
            DiagnosticSeverity::Warning,
            tr("translation is empty"),
        );
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
                tr("translation trailing newline differs from source"),
            );
        }
    }
}

fn validate_c_format(entry: &mut PoEntry) {
    let mut expected = placeholders(&entry.msgid);
    if let Some(plural) = &entry.msgid_plural {
        merge_placeholders(&mut expected, placeholders(plural));
    }
    let translations = entry.msgstr.clone();
    for field in translations {
        let actual = placeholders(&field);
        if actual != expected {
            push(
                entry,
                DiagnosticSeverity::Warning,
                tr("printf placeholder mismatch"),
            );
        }
    }
}

fn placeholders(field: &PoField) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
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
                let token = chars[start..=i].iter().collect::<String>();
                *counts.entry(token).or_insert(0) += 1;
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
    counts
}

fn merge_placeholders(into: &mut BTreeMap<String, usize>, from: BTreeMap<String, usize>) {
    for (token, count) in from {
        let current = into.entry(token).or_insert(0);
        *current = (*current).max(count);
    }
}

fn push(entry: &mut PoEntry, severity: DiagnosticSeverity, message: impl Into<String>) {
    entry.diagnostics.push(Diagnostic {
        entry_id: Some(EntryId(entry.id.0)),
        severity,
        message: message.into(),
    });
}
