use anyhow::{Result, anyhow};

use crate::i18n::tr;

use super::{PluralFormsHeader, PoDocument, PoHeader, writer::document_is_edited};

pub const HEADER_LANGUAGE: &str = "Language";
pub const HEADER_REVISION_DATE: &str = "PO-Revision-Date";
pub const HEADER_LAST_TRANSLATOR: &str = "Last-Translator";
pub const HEADER_LANGUAGE_TEAM: &str = "Language-Team";

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
                "PO-Revision-Date" if !value.is_empty() => header.revision_date = Some(value),
                "Last-Translator" if !value.is_empty() => header.last_translator = Some(value),
                "Language-Team" if !value.is_empty() => header.language_team = Some(value),
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
        if let Some(value) = trimmed.strip_prefix("nplurals=")
            && let Ok(nplurals) = value.trim().parse::<usize>()
        {
            return Some(PluralFormsHeader {
                nplurals,
                raw: raw.to_string(),
            });
        }
    }
    None
}

pub fn set_header_language(doc: &mut PoDocument, language: &str) -> Result<()> {
    set_header_field(doc, HEADER_LANGUAGE, language)
}

pub fn set_header_field(doc: &mut PoDocument, key: &str, value: &str) -> Result<()> {
    let key = canonical_header_key(key)?;
    let value = value.trim();
    if key == HEADER_LANGUAGE && value.is_empty() {
        return Err(anyhow!(tr("language code cannot be empty").into_owned()));
    }
    if value.contains(['\n', '\r']) {
        return Err(anyhow!(
            tr("header value must be a single line").into_owned()
        ));
    }

    let Some(entry) = doc.entries.iter_mut().find(|entry| entry.is_header()) else {
        return Err(anyhow!(
            tr("active PO file has no header entry").into_owned()
        ));
    };
    let Some(msgstr) = entry.msgstr.first_mut() else {
        return Err(anyhow!(tr("header entry has no msgstr field").into_owned()));
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
            .is_some_and(|(candidate, _)| candidate.trim() == key)
        {
            updated.push_str(key);
            updated.push_str(": ");
            updated.push_str(value);
            updated.push_str(line_ending);
            found = true;
        } else {
            updated.push_str(line);
        }
    }

    if !found {
        updated.push_str(key);
        updated.push_str(": ");
        updated.push_str(value);
        updated.push('\n');
    }

    msgstr.edited_value = if current == updated || updated == msgstr.decoded {
        None
    } else {
        Some(updated)
    };
    doc.dirty = document_is_edited(doc);
    Ok(())
}

fn canonical_header_key(key: &str) -> Result<&'static str> {
    match key.trim() {
        HEADER_LANGUAGE => Ok(HEADER_LANGUAGE),
        HEADER_REVISION_DATE => Ok(HEADER_REVISION_DATE),
        HEADER_LAST_TRANSLATOR => Ok(HEADER_LAST_TRANSLATOR),
        HEADER_LANGUAGE_TEAM => Ok(HEADER_LANGUAGE_TEAM),
        _ => Err(anyhow!(tr("unsupported header field").into_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HEADER_LANGUAGE_TEAM, HEADER_LAST_TRANSLATOR, HEADER_REVISION_DATE, parse_header,
        parse_plural_forms, set_header_field, set_header_language,
    };
    use crate::po::parser::parse_text;

    #[test]
    fn parse_header_handles_missing_header_or_msgstr_and_plural_failures() {
        let no_header =
            parse_text("sample.po", "msgid \"Hello\"\nmsgstr \"\"\n".to_string()).unwrap();
        assert!(parse_header(&no_header).language.is_none());
        assert!(parse_plural_forms("plural=(n != 1);").is_none());
        assert!(parse_plural_forms("nplurals=two; plural=(n != 1);").is_none());
    }

    #[test]
    fn set_header_language_rejects_invalid_or_missing_header_cases() {
        let mut no_header =
            parse_text("sample.po", "msgid \"Hello\"\nmsgstr \"\"\n".to_string()).unwrap();
        assert!(set_header_language(&mut no_header, "").is_err());
        assert!(set_header_language(&mut no_header, "fr\nCA").is_err());
        assert!(set_header_language(&mut no_header, "fr").is_err());

        let mut header_without_language = parse_text(
            "sample.po",
            "msgid \"\"\nmsgstr \"Content-Type: text/plain\\n\"\n".to_string(),
        )
        .unwrap();
        assert!(!header_without_language.dirty);
        set_header_language(&mut header_without_language, "fr").unwrap();
        assert!(header_without_language.dirty);
        assert!(
            header_without_language.entries[0].msgstr[0]
                .edited_value
                .as_ref()
                .unwrap()
                .contains("Language: fr\n")
        );

        let mut header_with_same_language = parse_text(
            "sample.po",
            "msgid \"\"\nmsgstr \"Language: fr\\n\"\n".to_string(),
        )
        .unwrap();
        set_header_language(&mut header_with_same_language, "fr").unwrap();
        assert!(!header_with_same_language.dirty);
        assert!(
            header_with_same_language.entries[0].msgstr[0]
                .edited_value
                .is_none()
        );

        let mut metadata = parse_text(
            "sample.po",
            "msgid \"\"\nmsgstr \"Language: en\\nLast-Translator: Old\\n\"\n".to_string(),
        )
        .unwrap();
        set_header_field(&mut metadata, HEADER_LAST_TRANSLATOR, "New Translator").unwrap();
        set_header_field(&mut metadata, HEADER_LANGUAGE_TEAM, "French").unwrap();
        set_header_field(&mut metadata, HEADER_REVISION_DATE, "2026-06-24 12:00+0000").unwrap();
        let header = parse_header(&metadata);
        assert_eq!(header.last_translator.as_deref(), Some("New Translator"));
        assert_eq!(header.language_team.as_deref(), Some("French"));
        assert_eq!(
            header.revision_date.as_deref(),
            Some("2026-06-24 12:00+0000")
        );
        assert!(set_header_field(&mut metadata, "Content-Type", "text/plain").is_err());
        assert!(set_header_field(&mut metadata, HEADER_LAST_TRANSLATOR, "A\nB").is_err());
    }
}
