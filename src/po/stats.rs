use super::{DiagnosticSeverity, PoDocument, PoEntry};

#[derive(Debug, Clone, Copy, Default)]
pub struct PoStats {
    pub entries: usize,
    pub untranslated: usize,
    pub fuzzy: usize,
    pub warnings: usize,
}

pub fn stats(doc: &PoDocument) -> PoStats {
    let mut out = PoStats {
        entries: doc.entries.len(),
        ..Default::default()
    };
    for entry in &doc.entries {
        if is_untranslated(entry) {
            out.untranslated += 1;
        }
        if entry.has_flag("fuzzy") {
            out.fuzzy += 1;
        }
        if entry
            .diagnostics
            .iter()
            .any(|d| d.severity != DiagnosticSeverity::Info)
        {
            out.warnings += 1;
        }
    }
    out
}

pub fn is_untranslated(entry: &PoEntry) -> bool {
    !entry.is_header() && entry.msgstr.iter().all(|f| f.value().is_empty())
}
