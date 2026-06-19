use crate::{
    app::{MessageFilter, TranslateRApp, TranslationUnitSort},
    i18n::tr,
    po::{DiagnosticSeverity, PoEntry, stats::is_untranslated},
    ui::display::visible_po_text,
};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    let panel_width = 360.0;

    egui::Panel::left("message_list")
        .resizable(false)
        .default_size(panel_width)
        .show_inside(parent, |ui| {
            ui.set_width(panel_width);
            ui.heading(tr("Translation Units").as_ref());
            ui.add(
                egui::TextEdit::singleline(&mut app.ui.search)
                    .hint_text(tr("Search terms, for example: rookie +miner").as_ref()),
            );
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(
                    &mut app.ui.search_case_sensitive,
                    tr("Case sensitive").as_ref(),
                );
                sort_button(
                    ui,
                    &mut app.ui.sort,
                    TranslationUnitSort::FileOrder,
                    tr("File order").as_ref(),
                );
                sort_button(
                    ui,
                    &mut app.ui.sort,
                    TranslationUnitSort::FirstLetter,
                    tr("A-Z").as_ref(),
                );
            });
            ui.horizontal_wrapped(|ui| {
                filter_button(
                    ui,
                    &mut app.ui.filter,
                    MessageFilter::All,
                    tr("All").as_ref(),
                );
                filter_button(
                    ui,
                    &mut app.ui.filter,
                    MessageFilter::Untranslated,
                    tr("Untranslated").as_ref(),
                );
                filter_button(
                    ui,
                    &mut app.ui.filter,
                    MessageFilter::Fuzzy,
                    tr("Fuzzy").as_ref(),
                );
                filter_button(
                    ui,
                    &mut app.ui.filter,
                    MessageFilter::Warnings,
                    tr("Warnings").as_ref(),
                );
                filter_button(
                    ui,
                    &mut app.ui.filter,
                    MessageFilter::Plural,
                    tr("Plural").as_ref(),
                );
                filter_button(
                    ui,
                    &mut app.ui.filter,
                    MessageFilter::Context,
                    tr("Context").as_ref(),
                );
            });
            ui.horizontal_wrapped(|ui| {
                first_letter_button(
                    ui,
                    &mut app.ui.first_letter_filter,
                    None,
                    tr("Any letter").as_ref(),
                );
                first_letter_button(ui, &mut app.ui.first_letter_filter, Some('#'), "#");
                for letter in 'A'..='Z' {
                    let label = letter.to_string();
                    first_letter_button(ui, &mut app.ui.first_letter_filter, Some(letter), &label);
                }
            });
            ui.separator();

            let Some(doc) = &app.doc else {
                ui.label(tr("Open a .po file or folder.").as_ref());
                return;
            };
            let entries = visible_entries(
                &doc.entries,
                app.ui.filter,
                &app.ui.search,
                app.ui.search_case_sensitive,
                app.ui.first_letter_filter,
                app.ui.sort,
            );
            let selected = app.ui.selected_entry;
            let mut clicked_entry = None;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in entries {
                        let label = row_label(entry);
                        if ui
                            .selectable_label(selected == Some(entry.id), label)
                            .clicked()
                        {
                            clicked_entry = Some(entry.id);
                        }
                    }
                });
            if let Some(entry_id) = clicked_entry {
                app.select_entry(Some(entry_id));
            }
        });
}

fn filter_button(ui: &mut egui::Ui, filter: &mut MessageFilter, value: MessageFilter, label: &str) {
    if ui.selectable_label(*filter == value, label).clicked() {
        *filter = value;
    }
}

fn sort_button(
    ui: &mut egui::Ui,
    sort: &mut TranslationUnitSort,
    value: TranslationUnitSort,
    label: &str,
) {
    if ui.selectable_label(*sort == value, label).clicked() {
        *sort = value;
    }
}

fn first_letter_button(
    ui: &mut egui::Ui,
    first_letter_filter: &mut Option<char>,
    value: Option<char>,
    label: &str,
) {
    if ui
        .selectable_label(*first_letter_filter == value, label)
        .clicked()
    {
        *first_letter_filter = value;
    }
}

fn visible_entries<'a>(
    entries: &'a [PoEntry],
    filter: MessageFilter,
    search: &str,
    case_sensitive: bool,
    first_letter_filter: Option<char>,
    sort: TranslationUnitSort,
) -> Vec<&'a PoEntry> {
    let mut entries = entries
        .iter()
        .filter(|entry| {
            !entry.is_header()
                && matches_filter(entry, filter, search, case_sensitive)
                && matches_first_letter(entry, first_letter_filter)
        })
        .collect::<Vec<_>>();

    if sort == TranslationUnitSort::FirstLetter {
        entries.sort_by(|left, right| {
            first_letter_bucket(left)
                .cmp(&first_letter_bucket(right))
                .then_with(|| {
                    left.msgid
                        .value()
                        .to_lowercase()
                        .cmp(&right.msgid.value().to_lowercase())
                })
                .then_with(|| left.ordinal.cmp(&right.ordinal))
        });
    }

    entries
}

fn matches_filter(
    entry: &PoEntry,
    filter: MessageFilter,
    search: &str,
    case_sensitive: bool,
) -> bool {
    let filter_ok = match filter {
        MessageFilter::All => true,
        MessageFilter::Untranslated => is_untranslated(entry),
        MessageFilter::Fuzzy => entry.has_flag("fuzzy"),
        MessageFilter::Warnings => !entry.diagnostics.is_empty(),
        MessageFilter::Plural => entry.msgid_plural.is_some(),
        MessageFilter::Context => entry.msgctxt.is_some(),
    };
    if !filter_ok {
        return false;
    }
    let terms = search_terms(search, case_sensitive);
    if terms.is_empty() {
        return true;
    }
    let mut haystack = entry.msgid.value().to_string();
    if let Some(ctx) = &entry.msgctxt {
        haystack.push_str(ctx.value());
    }
    for field in &entry.msgstr {
        haystack.push_str(field.value());
    }
    if case_sensitive {
        return terms.iter().all(|term| haystack.contains(term));
    }
    let haystack = haystack.to_lowercase();
    terms.iter().all(|term| haystack.contains(term))
}

fn search_terms(search: &str, case_sensitive: bool) -> Vec<String> {
    search
        .split_whitespace()
        .filter_map(|term| {
            let term = term.trim_start_matches('+');
            if term.is_empty() {
                None
            } else if case_sensitive {
                Some(term.to_string())
            } else {
                Some(term.to_lowercase())
            }
        })
        .collect()
}

fn matches_first_letter(entry: &PoEntry, first_letter_filter: Option<char>) -> bool {
    first_letter_filter.is_none_or(|letter| first_letter_bucket(entry) == letter)
}

fn first_letter_bucket(entry: &PoEntry) -> char {
    entry
        .msgid
        .value()
        .trim_start()
        .chars()
        .next()
        .map(|letter| {
            let letter = letter.to_ascii_uppercase();
            if letter.is_ascii_alphabetic() {
                letter
            } else {
                '#'
            }
        })
        .unwrap_or('#')
}

fn row_label(entry: &PoEntry) -> String {
    let mut chips = String::new();
    if is_untranslated(entry) {
        chips.push_str("[U]");
    }
    if entry.has_flag("fuzzy") {
        chips.push_str("[F]");
    }
    if entry.msgid_plural.is_some() {
        chips.push_str("[P]");
    }
    if entry.msgctxt.is_some() {
        chips.push_str("[C]");
    }
    if entry.has_flag("c-format") {
        chips.push_str("[%]");
    }
    if entry
        .diagnostics
        .iter()
        .any(|d| d.severity != DiagnosticSeverity::Info)
    {
        chips.push_str("[!]");
    }
    let preview = visible_po_text(entry.msgid.value());
    format!("{chips} {preview}")
}

#[cfg(test)]
mod tests {
    use super::visible_entries;
    use crate::{
        app::{MessageFilter, TranslationUnitSort},
        po::parser::parse_text,
    };

    #[test]
    fn search_can_be_case_sensitive() {
        let doc = parse_text(
            "sample.po",
            "msgid \"\"\nmsgstr \"Language: en\\n\"\n\nmsgid \"ROOKIE THORIUM MINER\"\nmsgstr \"\"\n\nmsgid \"rookie miner\"\nmsgstr \"\"\n".to_string(),
        )
        .unwrap();

        let insensitive = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "miner",
            false,
            None,
            TranslationUnitSort::FileOrder,
        );
        assert_eq!(insensitive.len(), 2);

        let sensitive = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "miner",
            true,
            None,
            TranslationUnitSort::FileOrder,
        );
        assert_eq!(sensitive.len(), 1);
        assert_eq!(sensitive[0].msgid.value(), "rookie miner");
    }

    #[test]
    fn search_terms_match_all_words_and_accept_required_prefix() {
        let doc = parse_text(
            "sample.po",
            "msgid \"\"\nmsgstr \"Language: en\\n\"\n\nmsgid \"ROOKIE THORIUM MINER\"\nmsgstr \"\"\n\nmsgid \"ROOKIE ICE MINER\"\nmsgstr \"\"\n\nmsgid \"Rookie courier\"\nmsgstr \"\"\n\nmsgid \"Veteran miner\"\nmsgstr \"\"\n".to_string(),
        )
        .unwrap();

        let plain_terms = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "rookie miner",
            false,
            None,
            TranslationUnitSort::FileOrder,
        );
        assert_eq!(plain_terms.len(), 2);
        assert_eq!(plain_terms[0].msgid.value(), "ROOKIE THORIUM MINER");
        assert_eq!(plain_terms[1].msgid.value(), "ROOKIE ICE MINER");

        let required_terms = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "rookie +miner",
            false,
            None,
            TranslationUnitSort::FileOrder,
        );
        assert_eq!(required_terms.len(), 2);
        assert_eq!(required_terms[0].msgid.value(), "ROOKIE THORIUM MINER");
        assert_eq!(required_terms[1].msgid.value(), "ROOKIE ICE MINER");

        let case_sensitive = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "rookie miner",
            true,
            None,
            TranslationUnitSort::FileOrder,
        );
        assert!(case_sensitive.is_empty());

        let uppercase_case_sensitive = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "ROOKIE +MINER",
            true,
            None,
            TranslationUnitSort::FileOrder,
        );
        assert_eq!(uppercase_case_sensitive.len(), 2);
    }

    #[test]
    fn first_letter_filter_uses_source_text() {
        let doc = parse_text(
            "sample.po",
            "msgid \"\"\nmsgstr \"Language: en\\n\"\n\nmsgid \" Alpha\"\nmsgstr \"\"\n\nmsgid \"beta\"\nmsgstr \"\"\n\nmsgid \"1 numbered\"\nmsgstr \"\"\n".to_string(),
        )
        .unwrap();

        let a_entries = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "",
            false,
            Some('A'),
            TranslationUnitSort::FileOrder,
        );
        assert_eq!(a_entries.len(), 1);
        assert_eq!(a_entries[0].msgid.value(), " Alpha");

        let symbol_entries = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "",
            false,
            Some('#'),
            TranslationUnitSort::FileOrder,
        );
        assert_eq!(symbol_entries.len(), 1);
        assert_eq!(symbol_entries[0].msgid.value(), "1 numbered");
    }

    #[test]
    fn first_letter_sort_orders_without_changing_file_order_mode() {
        let doc = parse_text(
            "sample.po",
            "msgid \"\"\nmsgstr \"Language: en\\n\"\n\nmsgid \"beta\"\nmsgstr \"\"\n\nmsgid \"Alpha\"\nmsgstr \"\"\n".to_string(),
        )
        .unwrap();

        let file_order = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "",
            false,
            None,
            TranslationUnitSort::FileOrder,
        );
        assert_eq!(file_order[0].msgid.value(), "beta");

        let sorted = visible_entries(
            &doc.entries,
            MessageFilter::All,
            "",
            false,
            None,
            TranslationUnitSort::FirstLetter,
        );
        assert_eq!(sorted[0].msgid.value(), "Alpha");
        assert_eq!(sorted[1].msgid.value(), "beta");
    }
}
