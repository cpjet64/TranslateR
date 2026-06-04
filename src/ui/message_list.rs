use crate::{
    app::{MessageFilter, TranslateRApp},
    po::{DiagnosticSeverity, EntryId, PoEntry, stats::is_untranslated},
    ui::display::visible_po_text,
};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    let panel_width = 360.0;

    egui::Panel::left("message_list")
        .resizable(false)
        .default_size(panel_width)
        .show_inside(parent, |ui| {
            ui.set_width(panel_width);
            ui.heading("Translation Units");
            ui.text_edit_singleline(&mut app.ui.search);
            ui.horizontal_wrapped(|ui| {
                filter_button(ui, &mut app.ui.filter, MessageFilter::All, "All");
                filter_button(
                    ui,
                    &mut app.ui.filter,
                    MessageFilter::Untranslated,
                    "Untranslated",
                );
                filter_button(ui, &mut app.ui.filter, MessageFilter::Fuzzy, "Fuzzy");
                filter_button(ui, &mut app.ui.filter, MessageFilter::Warnings, "Warnings");
                filter_button(ui, &mut app.ui.filter, MessageFilter::Plural, "Plural");
                filter_button(ui, &mut app.ui.filter, MessageFilter::Context, "Context");
            });
            ui.separator();

            let Some(doc) = &app.doc else {
                ui.label("Open a .po file or folder.");
                return;
            };
            let search = app.ui.search.to_lowercase();
            let mut selected = app.ui.selected_entry;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in doc
                        .entries
                        .iter()
                        .filter(|e| !e.is_header() && matches_filter(e, app.ui.filter, &search))
                    {
                        let label = row_label(entry);
                        if ui
                            .selectable_label(selected == Some(entry.id), label)
                            .clicked()
                        {
                            selected = Some(EntryId(entry.id.0));
                        }
                    }
                });
            app.ui.selected_entry = selected;
        });
}

fn filter_button(ui: &mut egui::Ui, filter: &mut MessageFilter, value: MessageFilter, label: &str) {
    if ui.selectable_label(*filter == value, label).clicked() {
        *filter = value;
    }
}

fn matches_filter(entry: &PoEntry, filter: MessageFilter, search: &str) -> bool {
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
    if search.is_empty() {
        return true;
    }
    let mut haystack = entry.msgid.value().to_lowercase();
    if let Some(ctx) = &entry.msgctxt {
        haystack.push_str(&ctx.value().to_lowercase());
    }
    for field in &entry.msgstr {
        haystack.push_str(&field.value().to_lowercase());
    }
    haystack.contains(search)
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
