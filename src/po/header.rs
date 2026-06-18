use anyhow::{Result, anyhow};

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

pub fn set_header_language(doc: &mut PoDocument, language: &str) -> Result<()> {
    let language = language.trim();
    if language.is_empty() {
        return Err(anyhow!("language code cannot be empty"));
    }
    if language.contains(['\n', '\r']) {
        return Err(anyhow!("language code must be a single line"));
    }

    let Some(entry) = doc.entries.iter_mut().find(|entry| entry.is_header()) else {
        return Err(anyhow!("active PO file has no header entry"));
    };
    let Some(msgstr) = entry.msgstr.first_mut() else {
        return Err(anyhow!("header entry has no msgstr field"));
    };

    let current = msgstr.value();
    let mut found = false;
    let mut updated = String::new();
    for line in current.split_inclusive('\n') {
        let (line_body, line_ending) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        if line_body
            .split_once(':')
            .is_some_and(|(key, _)| key.trim() == "Language")
        {
            updated.push_str("Language: ");
            updated.push_str(language);
            updated.push_str(line_ending);
            found = true;
        } else {
            updated.push_str(line);
        }
    }

    if !found {
        updated.push_str("Language: ");
        updated.push_str(language);
        updated.push('\n');
    }

    if current != updated {
        msgstr.edited_value = Some(updated);
        doc.dirty = true;
    }
    Ok(())
}
