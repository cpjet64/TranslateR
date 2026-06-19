use crate::{
    app::{AppMode, TranslateRApp},
    i18n::{tr, tr_format},
    po::{EntryId, header::parse_header},
    ui::display::visible_po_text,
};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    egui::CentralPanel::default().show_inside(parent, |ui| {
        let Some(doc) = &app.doc else {
            ui.heading(tr("TranslateR").as_ref());
            ui.label(tr("Open a .po file or folder to begin.").as_ref());
            return;
        };

        let header = parse_header(doc);
        let Some(entry_id) = app.ui.selected_entry else {
            ui.label(tr("Select a message.").as_ref());
            return;
        };
        let Some(entry) = doc.entries.iter().find(|e| e.id == entry_id).cloned() else {
            ui.label(tr("Select a message.").as_ref());
            return;
        };

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading(tr("Header").as_ref());
                egui::Grid::new("po_header_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(tr("Language").as_ref());
                        if app.ui.header_language_editing {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut app.ui.header_language_draft);
                                if ui.button(tr("Apply").as_ref()).clicked() {
                                    match app.update_header_language(
                                        app.ui.header_language_draft.clone(),
                                    ) {
                                        Ok(()) => app.ui.header_language_editing = false,
                                        Err(err) => app.last_error = Some(err.to_string()),
                                    }
                                }
                                if ui.button(tr("Cancel").as_ref()).clicked() {
                                    app.ui.header_language_editing = false;
                                }
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(
                                    header.language.as_deref().unwrap_or(tr("unknown").as_ref()),
                                );
                                if ui.button(tr("Edit").as_ref()).clicked() {
                                    app.ui.header_language_draft =
                                        header.language.clone().unwrap_or_default();
                                    app.ui.header_language_editing = true;
                                }
                            });
                        }
                        ui.end_row();

                        ui.label(tr("Content-Type").as_ref());
                        ui.label(
                            header
                                .content_type
                                .as_deref()
                                .unwrap_or(tr("unknown").as_ref()),
                        );
                        ui.end_row();

                        ui.label(tr("Plural forms").as_ref());
                        let plural = header
                            .plural_forms
                            .as_ref()
                            .map(|p| {
                                tr_format("{count} forms", &[("count", p.nplurals.to_string())])
                            })
                            .unwrap_or_else(|| tr("unknown").into_owned());
                        ui.label(plural);
                        ui.end_row();
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_sized([120.0, 34.0], egui::Button::new(tr("Previous").as_ref()))
                        .clicked()
                    {
                        app.select_entry(previous_entry(app, entry_id));
                    }
                    if ui
                        .add_sized([120.0, 34.0], egui::Button::new(tr("Next").as_ref()))
                        .clicked()
                    {
                        app.select_entry(next_entry(app, entry_id));
                    }
                    if ui
                        .add_sized(
                            [180.0, 34.0],
                            egui::Button::new(tr("Next untranslated").as_ref()),
                        )
                        .clicked()
                    {
                        app.select_entry(next_untranslated(app, entry_id));
                    }
                });

                ui.separator();
                ui.heading(tr("Source").as_ref());
                if let Some(ctx_field) = &entry.msgctxt {
                    ui.label(tr_format(
                        "Context: {context}",
                        &[("context", ctx_field.value().to_string())],
                    ));
                }
                source_with_question(app, ui, entry_id, "source", entry.msgid.value());
                if let Some(plural) = &entry.msgid_plural {
                    ui.label(tr("Plural source").as_ref());
                    source_with_question(app, ui, entry_id, "plural_source", plural.value());
                }

                ui.separator();
                ui.heading(tr("Translation").as_ref());
                let mut updates = Vec::new();
                for (idx, field) in entry.msgstr.iter().enumerate() {
                    let form = field.index.unwrap_or(idx);
                    ui.label(tr_format("Form {form}", &[("form", form.to_string())]));
                    let field_value = field.value().to_string();
                    let buffer_key = translation_buffer_key(entry_id, form);
                    let mut value = app
                        .ui
                        .translation_buffers
                        .entry(buffer_key.clone())
                        .or_insert(field_value)
                        .clone();
                    let mut changed = false;
                    let scope = format!("form:{form}");
                    ui.columns(2, |columns| {
                        columns[0].push_id(("translation", entry_id.0, form), |ui| {
                            changed = ui
                                .add(
                                    egui::TextEdit::multiline(&mut value)
                                        .id_source(("translation_text", entry_id.0, form))
                                        .desired_rows(5),
                                )
                                .changed();
                        });
                        question_box(app, &mut columns[1], entry_id, &scope);
                    });
                    if changed {
                        app.ui.translation_buffers.insert(buffer_key, value.clone());
                        updates.push((entry_id, form, value));
                    }
                }
                for (entry_id, form, value) in updates {
                    app.update_translation(entry_id, form, value);
                }

                ui.separator();
                ui.heading(tr("Flags").as_ref());
                ui.horizontal_wrapped(|ui| {
                    for flag in &entry.flags {
                        ui.label(format!("[{flag}]"));
                    }
                });

                if !entry.diagnostics.is_empty() {
                    ui.separator();
                    ui.heading(tr("Validation").as_ref());
                    for diag in &entry.diagnostics {
                        ui.label(tr_format(
                            "{severity}: {message}",
                            &[
                                ("severity", format!("{:?}", diag.severity)),
                                ("message", diag.message.clone()),
                            ],
                        ));
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
        let text = visible_po_text(text);
        egui::Frame::default()
            .fill(columns[0].visuals().extreme_bg_color)
            .stroke(columns[0].visuals().widgets.noninteractive.bg_stroke)
            .inner_margin(egui::Margin::same(6))
            .show(&mut columns[0], |ui| {
                ui.set_min_height(76.0);
                ui.add(
                    egui::Label::new(text)
                        .selectable(true)
                        .wrap()
                        .halign(egui::Align::Min),
                );
            });
        question_box(app, &mut columns[1], entry_id, scope);
    });
}

fn question_box(app: &mut TranslateRApp, ui: &mut egui::Ui, entry_id: EntryId, scope: &str) {
    if app.mode != AppMode::Translator {
        return;
    }
    ui.label(tr("Question for maintainer").as_ref());
    let mut question = app.question_value(entry_id, scope);
    if ui
        .add(
            egui::TextEdit::multiline(&mut question)
                .id_source(("question_text", entry_id.0, scope))
                .hint_text(
                    tr("Ask about context, screenshots, tone, or where this text appears.")
                        .as_ref(),
                )
                .desired_rows(3),
        )
        .changed()
    {
        app.update_question(entry_id, scope, question);
    }
}

fn translation_buffer_key(entry_id: EntryId, form: usize) -> String {
    format!("{}:{form}", entry_id.0)
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
