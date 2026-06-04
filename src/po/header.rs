use super::{PluralFormsHeader, PoDocument, PoHeader};

pub fn parse_header(doc: &PoDocument) -> PoHeader {
    let Some(entry) = doc.entries.first().filter(|e| e.is_header()) else {
        return PoHeader::default();
    };
    let Some(msgstr) = entry.msgstr.first() else {
        return PoHeader::default();
    };

    let mut header = PoHeader::default();
    for line in msgstr.value().lines() {
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim().to_string();
            match key.trim() {
                "Language" if !value.is_empty() => header.language = Some(value),
                "Content-Type" if !value.is_empty() => header.content_type = Some(value),
                "Plural-Forms" if !value.is_empty() => {
                    header.plural_forms = parse_plural_forms(&value);
                }
                _ => {}
            }
        }
    }
    header
}

pub fn parse_plural_forms(raw: &str) -> Option<PluralFormsHeader> {
    for part in raw.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix("nplurals=") {
            if let Ok(nplurals) = value.trim().parse::<usize>() {
                return Some(PluralFormsHeader {
                    nplurals,
                    raw: raw.to_string(),
                });
            }
        }
    }
    None
}
