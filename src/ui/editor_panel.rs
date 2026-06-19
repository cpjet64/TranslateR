use crate::{
    app::{AppMode, TranslateRApp},
    po::{EntryId, header::parse_header},
    ui::display::visible_po_text,
};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    egui::CentralPanel::default().show_inside(parent, |ui| {
        let Some(doc) = &app.doc else {
            ui.heading("TranslateR");
            ui.label("Open a .po file or folder to begin.");
            return;
        };

        let header = parse_header(doc);
        let Some(entry_id) = app.ui.selected_entry else {
            ui.label("Select a message.");
            return;
        };
        let Some(entry) = doc.entries.iter().find(|e| e.id == entry_id).cloned() else {
            ui.label("Select a message.");
            return;
        };

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading("Header");
                egui::Grid::new("po_header_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Language");
                        if app.ui.header_language_editing {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut app.ui.header_language_draft);
                                if ui.button("Apply").clicked() {
                                    match app.update_header_language(
                                        app.ui.header_language_draft.clone(),
                                    ) {
                                        Ok(()) => app.ui.header_language_editing = false,
                                        Err(err) => app.last_error = Some(err.to_string()),
                                    }
                                }
                                if ui.button("Cancel").clicked() {
                                    app.ui.header_language_editing = false;
                                }
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(header.language.as_deref().unwrap_or("unknown"));
                                if ui.button("Edit").clicked() {
                                    app.ui.header_language_draft =
                                        header.language.clone().unwrap_or_default();
                                    app.ui.header_language_editing = true;
                                }
                            });
                        }
                        ui.end_row();

                        ui.label("Content-Type");
                        ui.label(header.content_type.as_deref().unwrap_or("unknown"));
                        ui.end_row();

                        ui.label("Plural forms");
                        let plural = header
                            .plural_forms
                            .as_ref()
                            .map(|p| format!("{} forms", p.nplurals))
                            .unwrap_or_else(|| "unknown".to_string());
                        ui.label(plural);
                        ui.end_row();
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_sized([120.0, 34.0], egui::Button::new("Previous"))
                        .clicked()
                    {
                        app.ui.selected_entry = previous_entry(app, entry_id);
                    }
                    if ui
                        .add_sized([120.0, 34.0], egui::Button::new("Next"))
                        .clicked()
                    {
                        app.ui.selected_entry = next_entry(app, entry_id);
                    }
                    if ui
                        .add_sized([180.0, 34.0], egui::Button::new("Next untranslated"))
                        .clicked()
                    {
                        app.ui.selected_entry = next_untranslated(app, entry_id);
                    }
                });

                ui.separator();
                ui.heading("Source");
                if let Some(ctx_field) = &entry.msgctxt {
                    ui.label(format!("Context: {}", ctx_field.value()));
                }
                source_with_question(app, ui, entry_id, "source", entry.msgid.value());
                if let Some(plural) = &entry.msgid_plural {
                    ui.label("Plural source");
                    source_with_question(app, ui, entry_id, "plural_source", plural.value());
                }

                ui.separator();
                ui.heading("Translation");
                let mut updates = Vec::new();
                for (idx, field) in entry.msgstr.iter().enumerate() {
                    let form = field.index.unwrap_or(idx);
                    ui.label(format!("Form {form}"));
                    let mut value = field.value().to_string();
                    let scope = format!("form:{form}");
                    ui.columns(2, |columns| {
                        if columns[0]
                            .add(egui::TextEdit::multiline(&mut value).desired_rows(5))
                            .changed()
                        {
                            updates.push((entry_id, form, value));
                        }
                        question_box(app, &mut columns[1], entry_id, &scope);
                    });
                }
                for (entry_id, form, value) in updates {
                    app.update_translation(entry_id, form, value);
                }

                ui.separator();
                ui.heading("Flags");
                ui.horizontal_wrapped(|ui| {
                    for flag in &entry.flags {
                        ui.label(format!("[{flag}]"));
                    }
                });

                if !entry.diagnostics.is_empty() {
                    ui.separator();
                    ui.heading("Validation");
                    for diag in &entry.diagnostics {
                        ui.label(format!("{:?}: {}", diag.severity, diag.message));
                    }
                }
            });
    });
}

fn source_with_question(
    app: &mut TranslateRApp,
    ui: &mut egui::Ui,
    entry_id: EntryId,
    scope: &str,
    text: &str,
) {
    ui.columns(2, |columns| {
        columns[0].add(
            egui::TextEdit::multiline(&mut visible_po_text(text))
                .desired_rows(3)
                .interactive(false),
        );
        question_box(app, &mut columns[1], entry_id, scope);
    });
}

fn question_box(app: &mut TranslateRApp, ui: &mut egui::Ui, entry_id: EntryId, scope: &str) {
    if app.mode != AppMode::Translator {
        return;
    }
    ui.label("Question for maintainer");
    let mut question = app.question_value(entry_id, scope);
    if ui
        .add(
            egui::TextEdit::multiline(&mut question)
                .hint_text("Ask about context, screenshots, tone, or where this text appears.")
                .desired_rows(3),
        )
        .changed()
    {
        app.update_question(entry_id, scope, question);
    }
}

fn previous_entry(app: &TranslateRApp, current: EntryId) -> Option<EntryId> {
    let doc = app.doc.as_ref()?;
    let pos = doc.entries.iter().position(|e| e.id == current)?;
    doc.entries
        .iter()
        .take(pos)
        .rev()
        .find(|entry| !entry.is_header())
        .map(|e| e.id)
}

fn next_entry(app: &TranslateRApp, current: EntryId) -> Option<EntryId> {
    let doc = app.doc.as_ref()?;
    let pos = doc.entries.iter().position(|e| e.id == current)?;
    doc.entries
        .iter()
        .skip(pos + 1)
        .find(|entry| !entry.is_header())
        .map(|e| e.id)
}

fn next_untranslated(app: &TranslateRApp, current: EntryId) -> Option<EntryId> {
    let doc = app.doc.as_ref()?;
    let pos = doc.entries.iter().position(|e| e.id == current)?;
    doc.entries
        .iter()
        .skip(pos + 1)
        .find(|e| !e.is_header() && crate::po::stats::is_untranslated(e))
        .map(|e| e.id)
}
