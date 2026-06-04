use anyhow::{Result, bail};

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
            None => bail!("unterminated escape sequence"),
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
    let end = line.rfind('"')?;
    if end <= start {
        return None;
    }
    Some(&line[start + 1..end])
}
