use anyhow::{Result, bail};

use crate::i18n::tr;

pub fn decode_po_string(raw: &str) -> Result<String> {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('b') => out.push('\u{0008}'),
            Some('f') => out.push('\u{000c}'),
            Some('a') => out.push('\u{0007}'),
            Some('v') => out.push('\u{000b}'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('?') => out.push('?'),
            Some('\'') => out.push('\''),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => bail!(tr("unterminated escape sequence").into_owned()),
        }
    }
    Ok(out)
}

pub fn encode_po_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\u{0007}' => out.push_str("\\a"),
            '\u{000b}' => out.push_str("\\v"),
            other => out.push(other),
        }
    }
    out
}

pub fn quoted_payload(line: &str) -> Option<&str> {
    let start = line.find('"')?;
    let end = line[start + 1..]
        .rfind('"')
        .map(|offset| start + 1 + offset)?;
    Some(&line[start + 1..end])
}

#[cfg(test)]
mod tests {
    use super::{decode_po_string, encode_po_string, quoted_payload};

    #[test]
    fn quoted_payload_rejects_missing_or_empty_quotes() {
        assert_eq!(quoted_payload("msgstr no quote"), None);
        assert_eq!(quoted_payload("msgstr \"unterminated"), None);
        assert_eq!(quoted_payload("msgstr \"\""), Some(""));
        assert_eq!(quoted_payload("msgstr \"translated\""), Some("translated"));
    }

    #[test]
    fn decodes_and_encodes_supported_escapes_and_errors_on_trailing_backslash() {
        assert_eq!(
            decode_po_string("\\n\\t\\r\\b\\f\\a\\v\\\\\\\"\\?\\'\\x").unwrap(),
            "\n\t\r\u{0008}\u{000c}\u{0007}\u{000b}\\\"?'\\x"
        );
        assert!(decode_po_string("abc\\").is_err());
        assert_eq!(
            encode_po_string("\n\t\r\u{0008}\u{000c}\u{0007}\u{000b}\\\""),
            "\\n\\t\\r\\b\\f\\a\\v\\\\\\\""
        );
    }
}
