use anyhow::{Result, anyhow};

use crate::i18n::tr;

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
    let language = language.trim();
    if language.is_empty() {
        return Err(anyhow!(tr("language code cannot be empty").into_owned()));
    }
    if language.contains(['\n', '\r']) {
        return Err(anyhow!(
            tr("language code must be a single line").into_owned()
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

#[cfg(test)]
mod tests {
    use super::{parse_header, parse_plural_forms, set_header_language};
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
    }
}
