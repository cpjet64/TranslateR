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

pub(crate) fn search_terms(search: &str, case_sensitive: bool) -> Vec<String> {
    let normalized = if case_sensitive {
        search.trim().to_string()
    } else {
        search.trim().to_lowercase()
    };
    normalized
        .split(|ch: char| ch == '+' || ch.is_whitespace())
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn highlighted_visible_po_text(
    value: &str,
    search: &str,
    case_sensitive: bool,
    ui: &egui::Ui,
) -> egui::text::LayoutJob {
    highlighted_label_job(&visible_po_text(value), search, case_sensitive, ui)
}

pub(crate) fn highlighted_label_job(
    text: &str,
    search: &str,
    case_sensitive: bool,
    ui: &egui::Ui,
) -> egui::text::LayoutJob {
    let ranges = highlight_ranges(text, search, case_sensitive);
    let base = egui::TextFormat {
        font_id: egui::TextStyle::Body.resolve(ui.style()),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let mut highlight = base.clone();
    highlight.background = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(92, 76, 18)
    } else {
        egui::Color32::from_rgb(255, 238, 128)
    };

    let mut job = egui::text::LayoutJob::default();
    let mut cursor = 0;
    for (start, end) in ranges {
        if cursor < start {
            job.append(&text[cursor..start], 0.0, base.clone());
        }
        job.append(&text[start..end], 0.0, highlight.clone());
        cursor = end;
    }
    if cursor < text.len() {
        job.append(&text[cursor..], 0.0, base);
    }
    job
}

fn highlight_ranges(text: &str, search: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    let terms = search_terms(search, case_sensitive);
    if terms.is_empty() || text.is_empty() {
        return Vec::new();
    }

    let (haystack, byte_map) = normalized_with_byte_map(text, case_sensitive);
    let mut ranges = Vec::new();
    for term in terms {
        if term.is_empty() {
            continue;
        }
        for (start, _) in haystack.match_indices(&term) {
            let end = start + term.len();
            if start < byte_map.len() && end < byte_map.len() {
                ranges.push((byte_map[start], byte_map[end]));
            }
        }
    }
    merge_ranges(ranges)
}

fn normalized_with_byte_map(text: &str, case_sensitive: bool) -> (String, Vec<usize>) {
    if case_sensitive {
        return (text.to_string(), (0..=text.len()).collect());
    }

    let mut normalized = String::new();
    let mut byte_map = Vec::new();
    for (idx, ch) in text.char_indices() {
        for lower in ch.to_lowercase() {
            let mut encoded = [0_u8; 4];
            for _ in lower.encode_utf8(&mut encoded).as_bytes() {
                byte_map.push(idx);
            }
            normalized.push(lower);
        }
    }
    byte_map.push(text.len());
    (normalized, byte_map)
}

fn merge_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges {
        if start >= end {
            continue;
        }
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::{highlight_ranges, search_terms, visible_po_text};

    #[test]
    fn visible_po_text_escapes_control_characters() {
        assert_eq!(visible_po_text("a\nb\tc\\d"), "a\\nb\\tc\\\\d");
    }

    #[test]
    fn search_terms_support_plus_and_case_sensitivity() {
        assert_eq!(
            search_terms(" rookie + miner ", false),
            vec!["rookie".to_string(), "miner".to_string()]
        );
        assert_eq!(search_terms("ROOKIE", true), vec!["ROOKIE".to_string()]);
    }

    #[test]
    fn highlight_ranges_merge_overlapping_matches() {
        assert_eq!(
            highlight_ranges("ROOKIE miner", "rookie + miner", false),
            vec![(0, 6), (7, 12)]
        );
        assert_eq!(
            highlight_ranges("ROOKIE rookie", "rookie", true),
            vec![(7, 13)]
        );
    }
}
