pub fn visible_po_text(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{0007}' => out.push_str("\\a"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\u{000b}' => out.push_str("\\v"),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}
