use std::{fs, path::Path};

use anyhow::{Result, anyhow};

use crate::i18n::{tr, tr_format};

use super::{
    Diagnostic, DiagnosticSeverity, EntryComments, EntryId, LineSpan, NewlineStyle, PoDocument,
    PoEntry, PoField, PoFieldKind, RawLine,
    escape::{decode_po_string, quoted_payload},
    validate::validate_document,
};

pub fn parse_document(path: &Path) -> Result<PoDocument> {
    let bytes = fs::read(path)?;
    let text = decode_po_bytes(&bytes)?;
    parse_text_with_bytes(path, text, bytes)
}

pub fn parse_text(path: impl AsRef<Path>, text: String) -> Result<PoDocument> {
    let bytes = text.as_bytes().to_vec();
    parse_text_with_bytes(path.as_ref(), text, bytes)
}

pub fn decode_po_bytes(bytes: &[u8]) -> Result<String> {
    let text = String::from_utf8(bytes.to_vec()).map_err(|_| {
        anyhow!(
            tr("PO file is not valid UTF-8; non-UTF-8 catalogs are not supported yet").into_owned()
        )
    })?;
    ensure_supported_charset(&text)?;
    Ok(text)
}

pub fn parse_text_with_bytes(path: &Path, text: String, bytes: Vec<u8>) -> Result<PoDocument> {
    ensure_supported_charset(&text)?;
    let newline = if text.contains("\r\n") {
        NewlineStyle::CrLf
    } else {
        NewlineStyle::Lf
    };
    let raw_lines = split_raw_lines(&text);
    let mut entries = Vec::new();
    let mut i = 0;
    let mut diagnostics = Vec::new();

    while i < raw_lines.len() {
        while i < raw_lines.len() && raw_lines[i].text_without_newline.trim().is_empty() {
            i += 1;
        }
        if i >= raw_lines.len() {
            break;
        }

        let start = i;
        i = find_entry_end(&raw_lines, start);
        let block = &raw_lines[start..i];
        match parse_entry(block, entries.len()) {
            Ok(entry) => entries.push(entry),
            Err(err) => diagnostics.push(Diagnostic {
                entry_id: None,
                severity: DiagnosticSeverity::Error,
                message: tr_format(
                    "parse error near line {line}: {error}",
                    &[
                        ("line", (start + 1).to_string()),
                        ("error", err.to_string()),
                    ],
                ),
            }),
        }
    }

    let mut doc = PoDocument {
        path: path.to_path_buf(),
        original_hash: crate::util::hashing::sha256_bytes(&bytes),
        original_bytes: bytes,
        original_text: text,
        newline,
        entries,
        trailing_raw: Vec::new(),
        dirty: false,
        diagnostics,
    };
    validate_document(&mut doc);
    Ok(doc)
}

fn ensure_supported_charset(text: &str) -> Result<()> {
    if let Some(charset) = declared_header_charset(text) {
        let normalized = charset
            .chars()
            .filter(|ch| *ch != '-' && *ch != '_')
            .flat_map(char::to_lowercase)
            .collect::<String>();
        if normalized != "utf8" {
            return Err(anyhow!(tr_format(
                "PO charset {charset} is not supported yet; save the catalog as UTF-8 first",
                &[("charset", charset)]
            )));
        }
    }
    Ok(())
}

fn declared_header_charset(text: &str) -> Option<String> {
    let mut lines = text.lines().skip_while(|line| line.trim().is_empty());
    let first = lines.next()?.trim_start();
    if first != "msgid \"\"" {
        return None;
    }

    for line in lines {
        if line.trim().is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        let Some(pos) = lower.find("charset=") else {
            continue;
        };
        let raw_value = &line[pos + "charset=".len()..];
        let charset = raw_value
            .chars()
            .take_while(|ch| !matches!(ch, ';' | '"' | '\'' | '\\') && !ch.is_whitespace())
            .collect::<String>();
        if !charset.is_empty() {
            return Some(charset);
        }
    }
    None
}

fn split_raw_lines(text: &str) -> Vec<RawLine> {
    let mut lines = Vec::new();
    for (idx, line) in text.split_inclusive('\n').enumerate() {
        let without = line
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .to_string();
        lines.push(RawLine {
            text_without_newline: without,
            line_no: idx,
        });
    }
    lines
}

fn find_entry_end(raw_lines: &[RawLine], start: usize) -> usize {
    let mut i = start;
    let mut saw_msgstr = false;

    while i < raw_lines.len() {
        let line = raw_lines[i].text_without_newline.trim_start();

        if line.trim().is_empty() {
            return i;
        }

        if i > start && saw_msgstr && starts_new_entry(line) {
            return i;
        }

        if line.starts_with("msgstr ") || line.starts_with("msgstr[") {
            saw_msgstr = true;
        }

        i += 1;
    }

    i
}

fn starts_new_entry(line: &str) -> bool {
    line.starts_with("msgctxt ") || line.starts_with("msgid ") || line.starts_with('#')
}

fn parse_entry(block: &[RawLine], ordinal: usize) -> Result<PoEntry> {
    let mut comments = EntryComments::default();
    let mut fields = Vec::<PoField>::new();
    let mut obsolete = false;
    let mut flags = Vec::<String>::new();
    let mut i = 0;

    while i < block.len() {
        let raw = &block[i];
        let line = raw.text_without_newline.as_str();
        if line.starts_with("#~") {
            obsolete = true;
            comments.unknown.push(raw.clone());
            i += 1;
        } else if line.starts_with("#.") {
            comments.extracted.push(raw.clone());
            i += 1;
        } else if line.starts_with("#:") {
            comments.reference.push(raw.clone());
            i += 1;
        } else if let Some(rest) = line.strip_prefix("#,") {
            comments.flags_raw.push(raw.clone());
            flags.extend(
                rest.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned),
            );
            i += 1;
        } else if line.starts_with("#|") {
            comments.previous.push(raw.clone());
            i += 1;
        } else if line.starts_with('#') {
            comments.translator.push(raw.clone());
            i += 1;
        } else if is_field_start(line) {
            let (field, next) = parse_field(block, i)?;
            fields.push(field);
            i = next;
        } else {
            comments.unknown.push(raw.clone());
            i += 1;
        }
    }

    let msgid_pos = fields
        .iter()
        .position(|f| f.kind == PoFieldKind::MsgId)
        .ok_or_else(|| anyhow!(tr("entry has no msgid").into_owned()))?;
    let msgid = fields.remove(msgid_pos);
    let msgctxt = take_field(&mut fields, PoFieldKind::MsgCtxt);
    let msgid_plural = take_field(&mut fields, PoFieldKind::MsgIdPlural);
    let msgstr = fields
        .into_iter()
        .filter(|f| f.kind == PoFieldKind::MsgStr)
        .collect::<Vec<_>>();

    Ok(PoEntry {
        id: EntryId(ordinal),
        ordinal,
        span: LineSpan {
            start: block.first().map(|l| l.line_no).unwrap_or(0),
            end: block.last().map(|l| l.line_no).unwrap_or(0),
        },
        obsolete,
        comments,
        flags,
        msgctxt,
        msgid,
        msgid_plural,
        msgstr,
        diagnostics: Vec::new(),
        edited: Default::default(),
    })
}

fn take_field(fields: &mut Vec<PoField>, kind: PoFieldKind) -> Option<PoField> {
    let pos = fields.iter().position(|f| f.kind == kind)?;
    Some(fields.remove(pos))
}

fn is_field_start(line: &str) -> bool {
    line.starts_with("msgctxt ")
        || line.starts_with("msgid ")
        || line.starts_with("msgid_plural ")
        || line.starts_with("msgstr ")
        || line.starts_with("msgstr[")
}

fn parse_field(block: &[RawLine], start: usize) -> Result<(PoField, usize)> {
    let first = &block[start];
    let line = first.text_without_newline.as_str();
    let (kind, index) = if line.starts_with("msgctxt ") {
        (PoFieldKind::MsgCtxt, None)
    } else if line.starts_with("msgid_plural ") {
        (PoFieldKind::MsgIdPlural, None)
    } else if line.starts_with("msgid ") {
        (PoFieldKind::MsgId, None)
    } else if line.starts_with("msgstr[") {
        let close = line
            .find(']')
            .ok_or_else(|| anyhow!(tr("msgstr plural index is missing ']'").into_owned()))?;
        let idx = line[7..close].parse::<usize>()?;
        (PoFieldKind::MsgStr, Some(idx))
    } else if line.starts_with("msgstr ") {
        (PoFieldKind::MsgStr, None)
    } else {
        return Err(anyhow!(tr("unknown field").into_owned()));
    };

    let mut raw_lines = vec![first.clone()];
    let mut decoded = String::new();
    let payload =
        quoted_payload(line).ok_or_else(|| anyhow!(tr("missing quoted string").into_owned()))?;
    decoded.push_str(&decode_po_string(payload)?);
    let mut i = start + 1;
    while i < block.len() {
        let continuation = block[i].text_without_newline.trim_start();
        if !continuation.starts_with('"') {
            break;
        }
        let payload = quoted_payload(continuation)
            .ok_or_else(|| anyhow!(tr("missing quoted string").into_owned()))?;
        decoded.push_str(&decode_po_string(payload)?);
        raw_lines.push(block[i].clone());
        i += 1;
    }

    Ok((
        PoField {
            kind,
            index,
            raw_lines,
            decoded,
            edited_value: None,
        },
        i,
    ))
}

#[cfg(test)]
mod tests {
    use super::{decode_po_bytes, parse_document, parse_field, parse_text};
    use crate::po::{RawLine, writer::write_document};

    #[test]
    fn round_trips_simple_entry() {
        let input = "# comment\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n".to_string();
        let doc = parse_text("sample.po", input.clone()).unwrap();
        assert_eq!(write_document(&doc), input);
    }

    #[test]
    fn parse_field_reports_private_error_branches() {
        let unknown = vec![RawLine {
            text_without_newline: "unknown".to_string(),
            line_no: 0,
        }];
        assert!(
            parse_field(&unknown, 0)
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );

        let bad_first = vec![RawLine {
            text_without_newline: "msgid noquote".to_string(),
            line_no: 0,
        }];
        assert!(
            parse_field(&bad_first, 0)
                .unwrap_err()
                .to_string()
                .contains("missing quoted string")
        );

        let bad_continuation = vec![
            RawLine {
                text_without_newline: "msgid \"A\"".to_string(),
                line_no: 0,
            },
            RawLine {
                text_without_newline: "\"unterminated".to_string(),
                line_no: 1,
            },
        ];
        assert!(
            parse_field(&bad_continuation, 0)
                .unwrap_err()
                .to_string()
                .contains("missing quoted string")
        );
    }

    #[test]
    fn parse_document_reports_file_read_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.po");
        assert!(parse_document(&missing).is_err());
    }

    #[test]
    fn decode_po_bytes_rejects_invalid_utf8_before_lossy_parse() {
        let err = decode_po_bytes(b"msgid \"caf\xe9\"\nmsgstr \"\"\n")
            .unwrap_err()
            .to_string();

        assert!(err.contains("not valid UTF-8"));
    }

    #[test]
    fn decode_po_bytes_rejects_declared_non_utf8_charset() {
        let err = decode_po_bytes(
            b"msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=ISO-8859-1\\n\"\n\nmsgid \"Hello\"\nmsgstr \"\"\n",
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("ISO-8859-1"));
        assert!(err.contains("not supported"));
    }

    #[test]
    fn decode_po_bytes_ignores_empty_declared_charset() {
        let text = decode_po_bytes(
            b"msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=\\n\"\n\nmsgid \"Hello\"\nmsgstr \"\"\n",
        )
        .unwrap();

        assert!(text.contains("charset="));
    }
}
