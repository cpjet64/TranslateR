use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};

use crate::{
    i18n::{tr, tr_format},
    po::{
        EntryId, PoDocument,
        header::{HEADER_LANGUAGE, parse_header},
        parse_document,
        parser::{decode_po_bytes, parse_document as parse_po_document, parse_text_with_bytes},
        validate::validate_document,
        writer::{write_document, write_document_bytes},
    },
    project::{AppConfig, PoFileSummary, ProjectState, ThemeMode},
    update::UpdateState,
    util::{atomic_save::save_atomic_bytes, hashing::sha256_bytes},
    vcs::diff::unified_diff,
    workflow::{
        ActivePackage, EntryQuestion, VersionLogEntry, add_tpatch_metadata, change_summary,
        materialize_workflow_po, next_pack_version, parse_tpatch_metadata, read_trdraft,
        read_trpack, trdraft_from_document, trpack_from_document, version_log_entry, write_trdraft,
        write_trpack,
    },
};

pub struct TranslateRApp {
    pub mode: AppMode,
    pub project: ProjectState,
    pub doc: Option<PoDocument>,
    pub config: AppConfig,
    pub versions: Vec<VersionLogEntry>,
    pub ui: UiState,
    pub active_package: Option<ActivePackage>,
    pub active_draft_path: Option<PathBuf>,
    pub patch_base_text: Option<String>,
    pub updates: UpdateState,
    pub last_error: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Startup,
    Translator,
    Maintainer,
}

#[derive(Default)]
pub struct UiState {
    pub selected_entry: Option<EntryId>,
    pub search: String,
    pub search_case_sensitive: bool,
    pub filter: MessageFilter,
    pub sort: TranslationUnitSort,
    pub first_letter_filter: Option<char>,
    pub show_history: bool,
    pub diff_text: Option<String>,
    pub pending_patch: Option<Vec<u8>>,
    pub patch_folder: Option<PathBuf>,
    pub patch_files: Vec<PathBuf>,
    pub selected_patch: Option<usize>,
    pub header_editing_field: Option<String>,
    pub header_field_drafts: BTreeMap<String, String>,
    pub questions: BTreeMap<String, String>,
    pub translation_buffers: BTreeMap<String, String>,
    pub selected_history_version: Option<String>,
    pub pending_confirmation: Option<PendingFileOperation>,
    pub input_diagnostics: crate::ui::input_diagnostics::InputDiagnosticsState,
    pub undo_stack: Vec<UndoAction>,
    pub redo_stack: Vec<UndoAction>,
    pub show_close_confirmation: bool,
    pub close_confirmed: bool,
}

#[derive(Debug, Clone)]
pub struct PendingFileOperation {
    pub operation: FileOperation,
    pub action: ConfirmedAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    SavePo,
    SavePoAs,
    SaveTrDraft,
    SaveTrDraftAs,
    ExportTPatch,
    ExportTRPack,
    ApplyTPatch,
    ApplyAllTPatches,
    ApplyUpdate,
}

#[derive(Debug, Clone)]
pub enum ConfirmedAction {
    SaveActive,
    SaveActiveAs(PathBuf),
    SaveDraft(PathBuf),
    ExportPatch(PathBuf),
    ExportTrpack(PathBuf),
    ApplySelectedPatch,
    ApplyAllPatches,
    ApplyDownloadedUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoAction {
    Translation {
        entry_id: EntryId,
        index: usize,
        before: String,
        after: String,
    },
    HeaderLanguage {
        before: String,
        after: String,
    },
    Fuzzy {
        entry_id: EntryId,
        before: bool,
        after: bool,
    },
    TranslatorComments {
        entry_id: EntryId,
        before: String,
        after: String,
    },
}

impl ConfirmedAction {
    pub fn target_path(&self) -> Option<&Path> {
        match self {
            Self::SaveActiveAs(path)
            | Self::SaveDraft(path)
            | Self::ExportPatch(path)
            | Self::ExportTrpack(path) => Some(path),
            Self::SaveActive
            | Self::ApplySelectedPatch
            | Self::ApplyAllPatches
            | Self::ApplyDownloadedUpdate => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageFilter {
    #[default]
    All,
    Untranslated,
    Fuzzy,
    Warnings,
    Plural,
    Context,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TranslationUnitSort {
    #[default]
    FileOrder,
    FirstLetter,
}

impl TranslateRApp {
    pub fn request_confirmation(&mut self, operation: FileOperation, action: ConfirmedAction) {
        self.ui.pending_confirmation = Some(PendingFileOperation { operation, action });
    }

    pub fn cancel_pending_confirmation(&mut self) {
        self.ui.pending_confirmation = None;
    }

    pub fn confirm_pending_confirmation(&mut self) {
        let Some(pending) = self.ui.pending_confirmation.take() else {
            return;
        };
        if let Err(err) = self.run_confirmed_action(pending.action) {
            self.last_error = Some(err.to_string());
        }
    }

    fn run_confirmed_action(&mut self, action: ConfirmedAction) -> Result<()> {
        match action {
            ConfirmedAction::SaveActive => self.save_active(),
            ConfirmedAction::SaveActiveAs(path) => self.save_active_as(path),
            ConfirmedAction::SaveDraft(path) => self.save_draft(force_extension(path, "trdraft")),
            ConfirmedAction::ExportPatch(path) => {
                self.export_patch(force_extension(path, "tpatch"))
            }
            ConfirmedAction::ExportTrpack(path) => {
                self.export_trpack(force_extension(path, "trpack"))
            }
            ConfirmedAction::ApplySelectedPatch => self.apply_selected_patch(),
            ConfirmedAction::ApplyAllPatches => self.apply_all_patches(),
            ConfirmedAction::ApplyDownloadedUpdate => {
                self.apply_downloaded_update();
                Ok(())
            }
        }
    }

    pub fn open_file(&mut self, path: PathBuf) -> Result<()> {
        let bytes = fs::read(&path)?;
        let base_text = decode_po_bytes(&bytes)?;
        let doc = parse_text_with_bytes(&path, base_text.clone(), bytes)?;
        self.active_package = None;
        self.active_draft_path = None;
        self.patch_base_text = Some(base_text);
        self.ui.questions.clear();
        self.ui.translation_buffers.clear();
        self.ui.undo_stack.clear();
        self.ui.redo_stack.clear();
        self.ui.close_confirmed = false;
        self.ui.show_close_confirmation = false;
        self.project.root_dir = path.parent().map(PathBuf::from);
        self.project.files = vec![PoFileSummary::from_doc(&doc)];
        self.project.active_file = Some(0);
        self.select_entry(first_translatable_entry(&doc));
        self.status = tr_format("Opened {path}", &[("path", path.display().to_string())]);
        self.doc = Some(doc);
        self.refresh_version_history();
        Ok(())
    }

    pub fn start_translator(&mut self, path: PathBuf) -> Result<()> {
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("trpack"))
        {
            return self.start_translator_pack(path);
        }
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("trdraft"))
        {
            return self.start_translator_draft(path);
        }
        self.mode = AppMode::Translator;
        self.open_file(path)?;
        self.status = tr("Translator mode").into_owned();
        Ok(())
    }

    pub fn start_maintainer(&mut self, po_path: PathBuf, patch_folder: PathBuf) -> Result<()> {
        if po_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("trpack"))
        {
            return self.start_maintainer_pack(po_path, patch_folder);
        }
        self.mode = AppMode::Maintainer;
        self.open_file(po_path)?;
        self.load_patch_folder(patch_folder)?;
        self.status = tr("Maintainer mode").into_owned();
        Ok(())
    }

    fn start_maintainer_pack(&mut self, path: PathBuf, patch_folder: PathBuf) -> Result<()> {
        let pack = read_trpack(&path)?;
        let materialized =
            materialize_workflow_po(&pack.base_hash, &pack.po_filename, &pack.po_text)?;
        self.mode = AppMode::Maintainer;
        self.open_file(materialized)?;
        self.patch_base_text = Some(pack.po_text.clone());
        self.active_package = Some(ActivePackage::from_pack(path, &pack));
        self.load_patch_folder(patch_folder)?;
        self.refresh_version_history();
        self.status = tr_format(
            "Maintainer mode: {project} package {version}",
            &[("project", pack.project_id), ("version", pack.pack_version)],
        );
        Ok(())
    }

    fn start_translator_pack(&mut self, path: PathBuf) -> Result<()> {
        let pack = read_trpack(&path)?;
        let materialized =
            materialize_workflow_po(&pack.base_hash, &pack.po_filename, &pack.po_text)?;
        self.mode = AppMode::Translator;
        self.open_file(materialized)?;
        self.patch_base_text = Some(pack.po_text.clone());
        self.active_package = Some(ActivePackage::from_pack(path, &pack));
        self.active_draft_path = None;
        self.status = tr_format(
            "Translator mode: {project} package {version}",
            &[("project", pack.project_id), ("version", pack.pack_version)],
        );
        Ok(())
    }

    fn start_translator_draft(&mut self, path: PathBuf) -> Result<()> {
        let draft = read_trdraft(&path)?;
        let materialized =
            materialize_workflow_po(&draft.base_hash, &draft.po_filename, &draft.base_po_text)?;
        self.mode = AppMode::Translator;
        self.open_file(materialized.clone())?;
        self.patch_base_text = Some(draft.base_po_text.clone());
        save_atomic_bytes(&materialized, draft.po_text.as_bytes())?;
        let doc = parse_po_document(&materialized)?;
        self.project.files = vec![PoFileSummary::from_doc(&doc)];
        self.project.active_file = Some(0);
        self.select_entry(first_translatable_entry(&doc));
        self.doc = Some(doc);
        self.ui.translation_buffers.clear();
        self.active_package = Some(ActivePackage::from_draft(path.clone(), &draft));
        self.active_draft_path = Some(path);
        self.refresh_version_history();
        self.ui.questions = draft
            .questions
            .iter()
            .map(|question| {
                (
                    scoped_question_key(&question.entry_id, &question.scope),
                    question.question.clone(),
                )
            })
            .collect();
        self.status = tr_format(
            "Draft loaded: {project} package {version}",
            &[
                ("project", draft.project_id),
                ("version", draft.pack_version),
            ],
        );
        Ok(())
    }

    pub fn save_active(&mut self) -> Result<()> {
        let doc = self
            .doc
            .as_mut()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        if !doc.dirty {
            self.status = tr("No saved version created: no edits to save").into_owned();
            return Ok(());
        }
        match fs::read(&doc.path) {
            Ok(disk) if !disk.is_empty() && sha256_bytes(&disk) != doc.original_hash => {
                return Err(anyhow!(
                    tr("file changed on disk; reload before saving or overwrite intentionally")
                        .into_owned()
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(anyhow!(tr_format(
                    "could not check active PO for external changes: {error}",
                    &[("error", err.to_string())]
                )));
            }
        }
        validate_document(doc);
        let output_text = write_document(doc);
        let output = output_text.as_bytes().to_vec();
        save_atomic_bytes(&doc.path, &output)?;
        let reparsed = parse_document(&doc.path)?;
        *doc = reparsed;
        self.ui.translation_buffers.clear();
        if self.mode == AppMode::Maintainer {
            self.save_package_version(&output_text, "Save PO")?;
        }
        self.refresh_active_summary();
        if !matches!(self.mode, AppMode::Maintainer) || self.active_package.is_none() {
            self.status = tr("Saved PO").into_owned();
        }
        Ok(())
    }

    pub fn save_active_as(&mut self, path: PathBuf) -> Result<()> {
        let path = force_extension(path, "po");
        let doc = self
            .doc
            .as_mut()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        validate_document(doc);
        let output_text = write_document(doc);
        save_atomic_bytes(&path, output_text.as_bytes())?;
        let reparsed = parse_document(&path)?;
        self.project.root_dir = path.parent().map(PathBuf::from);
        self.project.files = vec![PoFileSummary::from_doc(&reparsed)];
        self.project.active_file = Some(0);
        self.doc = Some(reparsed);
        self.ui.translation_buffers.clear();
        self.refresh_active_summary();
        self.status = tr_format(
            "Saved PO as {path}",
            &[("path", path.display().to_string())],
        );
        Ok(())
    }

    pub fn export_patch(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let base = self.patch_base_text.as_ref().ok_or_else(|| {
            anyhow!(tr("no base PO text available for TPatch export").into_owned())
        })?;
        let current = write_document_bytes(doc);
        let current_text = String::from_utf8(current)
            .map_err(|_| anyhow!(tr("active PO output was not valid UTF-8").into_owned()))?;
        let patch = unified_diff(
            base,
            &current_text,
            "package-base",
            &doc.path.display().to_string(),
        );
        let questions = self.active_entry_questions();
        let patch = add_tpatch_metadata(patch, self.active_package.as_ref(), &questions);
        save_atomic_bytes(&path, patch.as_bytes())?;
        self.status = tr("TPatch exported").into_owned();
        Ok(())
    }

    pub fn export_trpack(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let po_text = write_document(doc);
        let mut pack = trpack_from_document(doc, po_text.clone(), None, None);
        if let Some(package) = &self.active_package {
            pack.project_id = package.project_id.clone();
            pack.pack_version = package.pack_version.clone();
            pack.history = package.history.clone();
            pack.contexts = package.contexts.clone();
            pack.answers = package.answers.clone();
            pack.screenshots = package.screenshots.clone();
        }
        let version = pack.pack_version.clone();
        write_trpack(&path, &pack)?;
        self.status = tr_format(
            "TRPack exported as version {version}",
            &[("version", version)],
        );
        Ok(())
    }

    pub fn save_draft(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let base = self.patch_base_text.as_ref().ok_or_else(|| {
            anyhow!(tr("no base package version available for this draft").into_owned())
        })?;
        let mut draft = trdraft_from_document(
            doc,
            write_document(doc),
            base.clone(),
            self.active_package.as_ref(),
        );
        draft.questions = self.active_entry_questions();
        if let Some(package) = &self.active_package {
            draft.history = package.history.clone();
        }
        write_trdraft(&path, &draft)?;
        self.active_draft_path = Some(path.clone());
        self.active_package = Some(ActivePackage::from_draft(path, &draft));
        self.status = tr_format(
            "Draft saved for package {project} version {version}",
            &[
                ("project", draft.project_id),
                ("version", draft.pack_version),
            ],
        );
        Ok(())
    }

    pub fn import_diff(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let imported = fs::read(&path)?;
        let mut diff = if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tpatch"))
        {
            String::from_utf8(imported.clone())
                .map_err(|_| anyhow!(tr("TPatch file was not valid UTF-8").into_owned()))?
        } else {
            let current = fs::read(&doc.path)?;
            let current_text = decode_po_bytes(&current)?;
            let imported_text = decode_po_bytes(&imported)?;
            unified_diff(
                &current_text,
                &imported_text,
                &doc.path.display().to_string(),
                &path.display().to_string(),
            )
        };
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tpatch"))
        {
            let metadata = parse_tpatch_metadata(&diff);
            if !metadata.questions.is_empty() {
                let mut question_summary = String::from("# TranslateR questions from translator\n");
                for question in &metadata.questions {
                    question_summary.push_str("# ");
                    question_summary.push_str(&question.scope);
                    question_summary.push_str(": ");
                    question_summary.push_str(&question.question.replace('\n', " "));
                    question_summary.push('\n');
                }
                diff = format!("{question_summary}{diff}");
            }
            if let Some(base_hash) = metadata.base_hash {
                let current_hash = sha256_bytes(&fs::read(&doc.path)?);
                if current_hash != base_hash {
                    diff = format!(
                        "# TranslateR warning: TPatch was exported from base {}, active PO is {}\n{}",
                        &base_hash[..12.min(base_hash.len())],
                        &current_hash[..12.min(current_hash.len())],
                        diff
                    );
                }
            }
        }
        self.ui.pending_patch = Some(imported);
        self.ui.diff_text = Some(diff);
        self.status = tr_format(
            "Imported TPatch {path}",
            &[("path", path.display().to_string())],
        );
        Ok(())
    }

    pub fn load_patch_folder(&mut self, path: PathBuf) -> Result<()> {
        let mut patches = fs::read_dir(&path)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("tpatch"))
            })
            .collect::<Vec<_>>();
        patches.sort();
        self.ui.patch_folder = Some(path);
        self.ui.patch_files = patches;
        self.ui.selected_patch = None;
        Ok(())
    }

    pub fn view_patch(&mut self, index: usize) -> Result<()> {
        let path = self
            .ui
            .patch_files
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!(tr("TPatch index out of range").into_owned()))?;
        self.ui.selected_patch = Some(index);
        self.import_diff(path)
    }

    pub fn apply_selected_patch(&mut self) -> Result<()> {
        self.apply_imported_patch()?;
        if let Some(index) = self.ui.selected_patch
            && let Some(path) = self.ui.patch_files.get(index)
        {
            self.status = tr_format("Applied {path}", &[("path", path.display().to_string())]);
        }
        Ok(())
    }

    pub fn apply_all_patches(&mut self) -> Result<()> {
        let patches = self.ui.patch_files.clone();
        if patches.is_empty() {
            self.status = tr("Applied all matching TPatches").into_owned();
            return Ok(());
        }
        let doc_path = self
            .doc
            .as_ref()
            .map(|doc| doc.path.clone())
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let mut merged = decode_po_bytes(&fs::read(&doc_path)?)?;
        for path in &patches {
            let patch = fs::read(path)?;
            let patch_text = String::from_utf8(patch)
                .map_err(|_| anyhow!(tr("TPatch file was not valid UTF-8").into_owned()))?;
            merged =
                crate::vcs::diff::apply_unified_patch(&merged, &patch_text).map_err(|err| {
                    anyhow!(tr_format(
                        "failed to apply TPatch {path}: {error}",
                        &[
                            ("path", path.display().to_string()),
                            ("error", err.to_string()),
                        ]
                    ))
                })?;
        }
        save_atomic_bytes(&doc_path, merged.as_bytes())?;
        let doc = parse_document(&doc_path)?;
        self.doc = Some(doc);
        self.ui.translation_buffers.clear();
        if self.mode == AppMode::Maintainer {
            self.save_package_version(&merged, "Apply All TPatches")?;
        }
        self.refresh_active_summary();
        self.ui.diff_text = None;
        self.ui.pending_patch = None;
        self.status = tr("Applied all matching TPatches").into_owned();
        Ok(())
    }

    pub fn apply_imported_patch(&mut self) -> Result<()> {
        let doc_path = self
            .doc
            .as_ref()
            .map(|d| d.path.clone())
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let patch = self
            .ui
            .pending_patch
            .as_ref()
            .ok_or_else(|| anyhow!(tr("no imported TPatch to apply").into_owned()))?;
        let current = decode_po_bytes(&fs::read(&doc_path)?)?;
        let patch_text = String::from_utf8(patch.clone())
            .map_err(|_| anyhow!(tr("TPatch file was not valid UTF-8").into_owned()))?;
        let merged = crate::vcs::diff::apply_unified_patch(&current, &patch_text)?;
        save_atomic_bytes(&doc_path, merged.as_bytes())?;
        let doc = parse_document(&doc_path)?;
        self.doc = Some(doc);
        self.ui.translation_buffers.clear();
        if self.mode == AppMode::Maintainer {
            self.save_package_version(&merged, "Apply TPatch")?;
        }
        self.refresh_active_summary();
        self.ui.diff_text = None;
        self.ui.pending_patch = None;
        if self.active_package.is_none() {
            self.status = tr("Applied TPatch").into_owned();
        }
        Ok(())
    }

    pub fn update_translation(&mut self, entry_id: EntryId, index: usize, value: String) {
        let before = self.translation_value(entry_id, index);
        if let Some(doc) = &mut self.doc {
            crate::po::writer::set_translation(doc, entry_id, index, value);
            validate_document(doc);
        }
        if let (Some(before), Some(after)) = (before, self.translation_value(entry_id, index))
            && before != after
        {
            self.record_undo(UndoAction::Translation {
                entry_id,
                index,
                before,
                after,
            });
        }
    }

    pub fn select_entry(&mut self, entry_id: Option<EntryId>) {
        self.ui.selected_entry = entry_id;
    }

    pub fn update_header_language(&mut self, language: String) -> Result<()> {
        self.update_header_field(HEADER_LANGUAGE, language.clone())?;
        self.status = tr_format(
            "Language set to {language}",
            &[("language", language.trim().to_string())],
        );
        Ok(())
    }

    pub fn update_header_field(&mut self, key: &str, value: String) -> Result<()> {
        let before = self.header_text_value();
        let doc = self
            .doc
            .as_mut()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        crate::po::header::set_header_field(doc, key, &value)?;
        validate_document(doc);
        self.refresh_active_summary();
        if let (Some(before), Some(after)) = (before, self.header_text_value())
            && before != after
        {
            self.record_undo(UndoAction::HeaderLanguage { before, after });
        }
        self.status = tr_format(
            "Header {field} set to {value}",
            &[
                ("field", key.to_string()),
                ("value", value.trim().to_string()),
            ],
        );
        Ok(())
    }

    pub fn update_fuzzy(&mut self, entry_id: EntryId, fuzzy: bool) {
        let before = self.fuzzy_value(entry_id);
        if let Some(doc) = &mut self.doc {
            crate::po::writer::set_fuzzy(doc, entry_id, fuzzy);
            validate_document(doc);
        }
        if let (Some(before), Some(after)) = (before, self.fuzzy_value(entry_id))
            && before != after
        {
            self.record_undo(UndoAction::Fuzzy {
                entry_id,
                before,
                after,
            });
        }
    }

    pub fn update_translator_comments(&mut self, entry_id: EntryId, comments: String) {
        let before = self.translator_comments_value(entry_id);
        if let Some(doc) = &mut self.doc {
            crate::po::writer::set_translator_comments(doc, entry_id, comments);
            validate_document(doc);
        }
        if let (Some(before), Some(after)) = (before, self.translator_comments_value(entry_id))
            && before != after
        {
            self.record_undo(UndoAction::TranslatorComments {
                entry_id,
                before,
                after,
            });
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.ui.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.ui.redo_stack.is_empty()
    }

    pub fn undo(&mut self) {
        let Some(action) = self.ui.undo_stack.pop() else {
            return;
        };
        if let Err(err) = self.apply_undo_action(&action, true) {
            self.last_error = Some(err.to_string());
            self.ui.undo_stack.push(action);
            return;
        }
        self.ui.redo_stack.push(action);
        self.status = tr("Undid last edit").into_owned();
    }

    pub fn redo(&mut self) {
        let Some(action) = self.ui.redo_stack.pop() else {
            return;
        };
        if let Err(err) = self.apply_undo_action(&action, false) {
            self.last_error = Some(err.to_string());
            self.ui.redo_stack.push(action);
            return;
        }
        self.ui.undo_stack.push(action);
        self.status = tr("Redid last edit").into_owned();
    }

    pub fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
            self.undo();
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)) {
            self.redo();
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            match self.mode {
                AppMode::Maintainer => {
                    self.request_confirmation(FileOperation::SavePo, ConfirmedAction::SaveActive);
                }
                AppMode::Translator => {
                    if let Some(path) = self.active_draft_path.clone() {
                        self.request_confirmation(
                            FileOperation::SaveTrDraft,
                            ConfirmedAction::SaveDraft(path),
                        );
                    }
                }
                AppMode::Startup => {}
            }
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.doc.as_ref().is_some_and(|doc| doc.dirty)
    }

    pub fn request_close_confirmation(&mut self) {
        self.ui.show_close_confirmation = true;
    }

    pub fn mark_close_confirmed(&mut self) {
        self.ui.close_confirmed = true;
        self.ui.show_close_confirmation = false;
    }

    pub fn question_value(&self, entry_id: EntryId, scope: &str) -> String {
        let Some(entry_key) = self.entry_question_id(entry_id) else {
            return String::new();
        };
        self.ui
            .questions
            .get(&scoped_question_key(&entry_key, scope))
            .cloned()
            .unwrap_or_default()
    }

    pub fn update_question(&mut self, entry_id: EntryId, scope: &str, value: String) {
        let Some(entry_key) = self.entry_question_id(entry_id) else {
            return;
        };
        let key = scoped_question_key(&entry_key, scope);
        if value.trim().is_empty() {
            self.ui.questions.remove(&key);
        } else {
            self.ui.questions.insert(key, value);
        }
    }

    fn entry_question_id(&self, entry_id: EntryId) -> Option<String> {
        self.doc.as_ref().and_then(|doc| {
            doc.entries
                .iter()
                .find(|entry| entry.id == entry_id)
                .map(crate::workflow::entry_key)
        })
    }

    fn active_entry_questions(&self) -> Vec<EntryQuestion> {
        self.ui
            .questions
            .iter()
            .filter_map(|(key, question)| {
                let (entry_id, scope) = key.split_once('|')?;
                let question = question.trim();
                (!question.is_empty()).then(|| EntryQuestion {
                    entry_id: entry_id.to_string(),
                    scope: scope.to_string(),
                    question: question.to_string(),
                    created_at: crate::workflow::now_rfc3339(),
                })
            })
            .collect()
    }

    fn record_undo(&mut self, action: UndoAction) {
        self.ui.undo_stack.push(action);
        self.ui.redo_stack.clear();
    }

    fn apply_undo_action(&mut self, action: &UndoAction, undo: bool) -> Result<()> {
        match action {
            UndoAction::Translation {
                entry_id,
                index,
                before,
                after,
            } => {
                let value = if undo { before } else { after }.clone();
                let doc = self
                    .doc
                    .as_mut()
                    .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
                crate::po::writer::set_translation(doc, *entry_id, *index, value.clone());
                validate_document(doc);
                self.ui
                    .translation_buffers
                    .insert(translation_buffer_key(*entry_id, *index), value);
            }
            UndoAction::HeaderLanguage { before, after } => {
                self.set_header_text(if undo { before } else { after }.clone())?;
            }
            UndoAction::Fuzzy {
                entry_id,
                before,
                after,
            } => {
                let doc = self
                    .doc
                    .as_mut()
                    .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
                crate::po::writer::set_fuzzy(doc, *entry_id, if undo { *before } else { *after });
                validate_document(doc);
            }
            UndoAction::TranslatorComments {
                entry_id,
                before,
                after,
            } => {
                let doc = self
                    .doc
                    .as_mut()
                    .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
                crate::po::writer::set_translator_comments(
                    doc,
                    *entry_id,
                    if undo { before } else { after }.clone(),
                );
                validate_document(doc);
            }
        }
        self.refresh_active_summary();
        Ok(())
    }

    fn translation_value(&self, entry_id: EntryId, index: usize) -> Option<String> {
        self.doc.as_ref().and_then(|doc| {
            doc.entries
                .iter()
                .find(|entry| entry.id == entry_id)?
                .msgstr
                .iter()
                .find(|field| field.index.unwrap_or(0) == index)
                .map(|field| field.value().to_string())
        })
    }

    fn fuzzy_value(&self, entry_id: EntryId) -> Option<bool> {
        self.doc.as_ref().and_then(|doc| {
            doc.entries
                .iter()
                .find(|entry| entry.id == entry_id)
                .map(crate::po::writer::effective_fuzzy)
        })
    }

    fn translator_comments_value(&self, entry_id: EntryId) -> Option<String> {
        self.doc.as_ref().and_then(|doc| {
            doc.entries
                .iter()
                .find(|entry| entry.id == entry_id)
                .map(crate::po::writer::translator_comments_text)
        })
    }

    fn header_text_value(&self) -> Option<String> {
        self.doc.as_ref().and_then(|doc| {
            doc.entries
                .iter()
                .find(|entry| entry.is_header())?
                .msgstr
                .first()
                .map(|field| field.value().to_string())
        })
    }

    fn set_header_text(&mut self, value: String) -> Result<()> {
        let doc = self
            .doc
            .as_mut()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let Some(entry) = doc.entries.iter_mut().find(|entry| entry.is_header()) else {
            return Err(anyhow!(
                tr("active PO file has no header entry").into_owned()
            ));
        };
        let Some(field) = entry.msgstr.first_mut() else {
            return Err(anyhow!(tr("header entry has no msgstr field").into_owned()));
        };
        field.edited_value = if value == field.decoded {
            None
        } else {
            Some(value)
        };
        doc.dirty = crate::po::writer::document_is_edited(doc);
        validate_document(doc);
        self.refresh_active_summary();
        Ok(())
    }

    fn refresh_active_summary(&mut self) {
        if let (Some(index), Some(doc)) = (self.project.active_file, &self.doc)
            && let Some(slot) = self.project.files.get_mut(index)
        {
            *slot = PoFileSummary::from_doc(doc);
        }
    }

    pub fn active_language(&self) -> Option<String> {
        self.doc.as_ref().and_then(|doc| parse_header(doc).language)
    }

    fn refresh_version_history(&mut self) {
        self.versions = self
            .active_package
            .as_ref()
            .map(|package| package.history.clone())
            .unwrap_or_default();
        if !self
            .ui
            .selected_history_version
            .as_ref()
            .is_some_and(|selected| {
                self.versions
                    .iter()
                    .any(|version| &version.version == selected)
            })
        {
            self.ui.selected_history_version =
                self.versions.last().map(|version| version.version.clone());
        }
    }

    fn save_package_version(&mut self, current_text: &str, note: &str) -> Result<()> {
        let Some(package) = self.active_package.as_mut() else {
            return Ok(());
        };
        if package.is_draft {
            return Ok(());
        }
        let previous_text = self.patch_base_text.clone().unwrap_or_default();
        if previous_text == current_text {
            self.status = tr("No package version created: no content changes").into_owned();
            return Ok(());
        }
        let version = next_pack_version(&package.pack_version);
        let summary = change_summary(&package.po_filename, &previous_text, current_text)?;
        let entry = version_log_entry(
            version.clone(),
            self.config.translator_name.clone(),
            note.to_string(),
            &previous_text,
            current_text,
            summary,
        );

        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let mut pack = trpack_from_document(
            doc,
            current_text.to_string(),
            Some(package.project_id.clone()),
            Some(version.clone()),
        );
        let mut history = package.history.clone();
        history.push(entry);
        pack.history = history.clone();
        pack.contexts = package.contexts.clone();
        pack.answers = package.answers.clone();
        pack.screenshots = package.screenshots.clone();
        write_trpack(&package.source_path, &pack)?;

        package.pack_version = version.clone();
        package.base_hash = pack.base_hash.clone();
        package.language = pack.language.clone();
        package.po_filename = pack.po_filename.clone();
        package.history = history;
        package.contexts = pack.contexts.clone();
        package.answers = pack.answers.clone();
        package.screenshots = pack.screenshots.clone();
        self.patch_base_text = Some(current_text.to_string());
        self.refresh_version_history();
        self.status = tr_format("Saved TRPack version {version}", &[("version", version)]);
        Ok(())
    }

    pub fn set_ui_language(&mut self, language: String) -> Result<()> {
        crate::i18n::set_language(&language);
        self.config.ui_language = crate::i18n::current_language();
        self.config.save()?;
        self.status = tr_format(
            "Interface language set to {language}",
            &[("language", self.config.ui_language.clone())],
        );
        Ok(())
    }

    pub fn set_theme_mode(&mut self, theme: ThemeMode, ctx: &egui::Context) -> Result<()> {
        ctx.set_theme(theme.egui_preference());
        self.config.theme = theme;
        self.config.save()?;
        let theme_name = match theme {
            ThemeMode::System => tr("System").into_owned(),
            ThemeMode::Light => tr("Light").into_owned(),
            ThemeMode::Dark => tr("Dark").into_owned(),
        };
        self.status = tr_format("Theme set to {theme}", &[("theme", theme_name)]);
        Ok(())
    }
}

fn scoped_question_key(entry_id: &str, scope: &str) -> String {
    format!("{entry_id}|{scope}")
}

fn translation_buffer_key(entry_id: EntryId, index: usize) -> String {
    format!("{}:{index}", entry_id.0)
}

pub(crate) fn first_translatable_entry(doc: &PoDocument) -> Option<EntryId> {
    doc.entries
        .iter()
        .find(|entry| !entry.is_header())
        .map(|entry| entry.id)
}

pub fn force_extension(mut path: PathBuf, extension: &str) -> PathBuf {
    let extension = extension.trim_start_matches('.');
    if !path
        .extension()
        .is_some_and(|existing| existing.eq_ignore_ascii_case(extension))
    {
        path.set_extension(extension);
    }
    path
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::{
        po::{parser::parse_text, writer::write_document},
        util::{
            hashing::sha256_bytes,
            paths::{set_app_config_dir_error_override, set_app_config_dir_override},
        },
        vcs::diff::unified_diff,
        workflow::{read_trdraft, read_trpack},
    };

    fn test_app() -> TranslateRApp {
        TranslateRApp {
            mode: AppMode::Startup,
            project: ProjectState::default(),
            doc: None,
            config: AppConfig::default(),
            versions: Vec::new(),
            ui: UiState::default(),
            active_package: None,
            active_draft_path: None,
            patch_base_text: None,
            updates: UpdateState::default(),
            last_error: None,
            status: "test".to_string(),
        }
    }

    fn sample_po(language: &str) -> String {
        format!(
            "msgid \"\"\nmsgstr \"\"\n\"Language: {language}\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\nmsgid \"Hello\"\nmsgstr \"\"\n\nmsgid \"Bye\"\nmsgstr \"\"\n"
        )
    }

    fn write_sample_po(dir: &tempfile::TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, sample_po("de")).unwrap();
        path
    }

    fn run_shortcut(app: &mut TranslateRApp, key: egui::Key) {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput {
            modifiers: egui::Modifiers::COMMAND,
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            ..Default::default()
        });
        app.handle_keyboard_shortcuts(&ctx);
        let _ = ctx.end_pass();
    }

    #[test]
    fn first_translatable_entry_skips_header() {
        let input =
            "msgid \"\"\nmsgstr \"Language: ar\\n\"\n\nmsgid \"Privacy Policy\"\nmsgstr \"\"\n"
                .to_string();
        let doc = parse_text("sample.po", input).unwrap();
        assert_eq!(first_translatable_entry(&doc), Some(doc.entries[1].id));
    }

    #[test]
    fn open_edit_save_and_questions_update_app_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_sample_po(&dir, "de.po");
        let mut app = test_app();

        app.open_file(path.clone()).unwrap();
        assert_eq!(app.project.root_dir.as_deref(), Some(dir.path()));
        assert_eq!(app.project.active_file, Some(0));
        assert_eq!(app.active_language().as_deref(), Some("de"));
        assert!(app.patch_base_text.as_ref().unwrap().contains("Hello"));
        let first = app.ui.selected_entry.unwrap();
        app.select_entry(Some(first));
        assert_eq!(app.ui.selected_entry, Some(first));
        let second = app.doc.as_ref().unwrap().entries[2].id;
        app.select_entry(Some(second));
        assert_eq!(app.ui.selected_entry, Some(second));

        app.update_question(first, "source", "Where is this shown?".to_string());
        assert_eq!(
            app.question_value(first, "source"),
            "Where is this shown?".to_string()
        );
        app.update_question(first, "source", "   ".to_string());
        assert!(app.question_value(first, "source").is_empty());
        app.update_question(EntryId(usize::MAX), "source", "ignored".to_string());
        assert!(app.ui.questions.is_empty());
        assert!(app.question_value(EntryId(usize::MAX), "source").is_empty());

        app.update_translation(first, 0, "Hallo".to_string());
        app.ui
            .translation_buffers
            .insert("buffer".to_string(), "stale".to_string());
        app.save_active().unwrap();
        assert!(app.ui.translation_buffers.is_empty());
        assert_eq!(app.status, "Saved PO");
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("msgstr \"Hallo\"")
        );

        app.save_active().unwrap();
        assert_eq!(app.status, "No saved version created: no edits to save");
    }

    #[test]
    fn force_extension_replaces_wrong_or_missing_extensions() {
        assert_eq!(
            force_extension(PathBuf::from("translation"), "po"),
            PathBuf::from("translation.po")
        );
        assert_eq!(
            force_extension(PathBuf::from("translation.txt"), ".tpatch"),
            PathBuf::from("translation.tpatch")
        );
        assert_eq!(
            force_extension(PathBuf::from("translation.PO"), "po"),
            PathBuf::from("translation.PO")
        );
    }

    #[test]
    fn save_active_as_writes_copy_and_switches_active_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_sample_po(&dir, "de.po");
        let original = fs::read_to_string(&path).unwrap();
        let save_as_path = dir.path().join("copy.txt");
        let expected_path = dir.path().join("copy.po");
        let mut app = test_app();

        app.open_file(path.clone()).unwrap();
        let first = app.ui.selected_entry.unwrap();
        app.update_translation(first, 0, "Hallo".to_string());
        app.save_active_as(save_as_path).unwrap();

        assert_eq!(app.doc.as_ref().unwrap().path, expected_path);
        assert!(
            fs::read_to_string(&expected_path)
                .unwrap()
                .contains("Hallo")
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(
            app.status,
            format!("Saved PO as {}", expected_path.display())
        );
    }

    #[test]
    fn confirmation_actions_cancel_or_run_file_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_sample_po(&dir, "de.po");
        let save_as_path = dir.path().join("confirmed.txt");
        let expected_save_as_path = dir.path().join("confirmed.po");
        let patch_path = dir.path().join("question.diff");
        let expected_patch_path = dir.path().join("question.tpatch");
        let mut app = test_app();

        app.open_file(path).unwrap();
        let first = app.ui.selected_entry.unwrap();
        app.update_translation(first, 0, "Hallo".to_string());

        app.request_confirmation(
            FileOperation::SavePoAs,
            ConfirmedAction::SaveActiveAs(save_as_path.clone()),
        );
        assert!(app.ui.pending_confirmation.is_some());
        app.cancel_pending_confirmation();
        assert!(app.ui.pending_confirmation.is_none());
        assert!(!expected_save_as_path.exists());

        app.request_confirmation(
            FileOperation::SavePoAs,
            ConfirmedAction::SaveActiveAs(save_as_path),
        );
        app.confirm_pending_confirmation();
        assert!(app.ui.pending_confirmation.is_none());
        assert!(expected_save_as_path.exists());
        assert!(app.last_error.is_none());

        app.update_question(first, "form:0", "Needs context?".to_string());
        app.request_confirmation(
            FileOperation::ExportTPatch,
            ConfirmedAction::ExportPatch(patch_path),
        );
        app.confirm_pending_confirmation();
        assert!(expected_patch_path.exists());
        assert!(
            fs::read_to_string(expected_patch_path)
                .unwrap()
                .contains("TranslateR-Questions-Json")
        );
    }

    #[test]
    fn confirmation_actions_cover_targets_and_remaining_command_branches() {
        let dir = tempfile::tempdir().unwrap();
        let po_path = write_sample_po(&dir, "de.po");
        let draft_path = dir.path().join("draft.txt");
        let expected_draft_path = dir.path().join("draft.trdraft");
        let pack_path = dir.path().join("pack.txt");
        let expected_pack_path = dir.path().join("pack.trpack");

        assert_eq!(
            ConfirmedAction::SaveActiveAs(PathBuf::from("copy.po")).target_path(),
            Some(Path::new("copy.po"))
        );
        assert_eq!(
            ConfirmedAction::SaveDraft(PathBuf::from("draft.trdraft")).target_path(),
            Some(Path::new("draft.trdraft"))
        );
        assert_eq!(
            ConfirmedAction::ExportPatch(PathBuf::from("change.tpatch")).target_path(),
            Some(Path::new("change.tpatch"))
        );
        assert_eq!(
            ConfirmedAction::ExportTrpack(PathBuf::from("pack.trpack")).target_path(),
            Some(Path::new("pack.trpack"))
        );
        assert!(ConfirmedAction::SaveActive.target_path().is_none());
        assert!(ConfirmedAction::ApplySelectedPatch.target_path().is_none());
        assert!(ConfirmedAction::ApplyAllPatches.target_path().is_none());
        assert!(
            ConfirmedAction::ApplyDownloadedUpdate
                .target_path()
                .is_none()
        );

        let mut empty_app = test_app();
        empty_app.confirm_pending_confirmation();
        assert!(empty_app.last_error.is_none());
        empty_app.request_confirmation(FileOperation::SavePo, ConfirmedAction::SaveActive);
        empty_app.confirm_pending_confirmation();
        assert!(
            empty_app
                .last_error
                .as_deref()
                .is_some_and(|err| err.contains("no active document"))
        );

        empty_app.last_error = None;
        empty_app.request_confirmation(
            FileOperation::ApplyUpdate,
            ConfirmedAction::ApplyDownloadedUpdate,
        );
        empty_app.confirm_pending_confirmation();
        assert!(
            empty_app
                .last_error
                .as_deref()
                .is_some_and(|err| err.contains("no downloaded update"))
        );

        let mut app = test_app();
        app.open_file(po_path).unwrap();
        app.request_confirmation(
            FileOperation::SaveTrDraftAs,
            ConfirmedAction::SaveDraft(draft_path),
        );
        app.confirm_pending_confirmation();
        assert!(expected_draft_path.exists());

        app.request_confirmation(
            FileOperation::ExportTRPack,
            ConfirmedAction::ExportTrpack(pack_path),
        );
        app.confirm_pending_confirmation();
        assert!(expected_pack_path.exists());

        app.request_confirmation(
            FileOperation::ApplyTPatch,
            ConfirmedAction::ApplySelectedPatch,
        );
        app.confirm_pending_confirmation();
        assert!(
            app.last_error
                .as_deref()
                .is_some_and(|err| err.contains("no imported TPatch"))
        );
        app.last_error = None;
        app.request_confirmation(
            FileOperation::ApplyAllTPatches,
            ConfirmedAction::ApplyAllPatches,
        );
        app.confirm_pending_confirmation();
        assert!(app.last_error.is_none());
        assert_eq!(app.status, "Applied all matching TPatches");
    }

    #[test]
    fn plain_translator_and_maintainer_modes_load_po_and_patch_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_sample_po(&dir, "de.po");
        let patch_dir = dir.path().join("patches");
        fs::create_dir_all(&patch_dir).unwrap();
        fs::write(patch_dir.join("b.tpatch"), "# TranslateR TPatch v1\n").unwrap();
        fs::write(patch_dir.join("a.tpatch"), "# TranslateR TPatch v1\n").unwrap();
        fs::write(patch_dir.join("ignored.txt"), "ignored").unwrap();

        let mut translator = test_app();
        translator.start_translator(path.clone()).unwrap();
        assert_eq!(translator.mode, AppMode::Translator);
        assert_eq!(translator.status, "Translator mode");

        let mut maintainer = test_app();
        maintainer
            .start_maintainer(path.clone(), patch_dir.clone())
            .unwrap();
        assert_eq!(maintainer.mode, AppMode::Maintainer);
        assert_eq!(maintainer.status, "Maintainer mode");
        assert_eq!(maintainer.ui.patch_files.len(), 2);
        assert!(
            maintainer.ui.patch_files[0]
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with('a')
        );

        assert!(
            maintainer
                .view_patch(usize::MAX)
                .unwrap_err()
                .to_string()
                .contains("index out of range")
        );
        maintainer.view_patch(0).unwrap();
        assert_eq!(maintainer.ui.selected_patch, Some(0));
        assert!(maintainer.ui.pending_patch.is_some());
        assert!(maintainer.ui.diff_text.as_ref().unwrap().contains("TPatch"));

        let mut bad_patch_folder = test_app();
        assert!(
            bad_patch_folder
                .start_maintainer(path, dir.path().join("missing-patches"))
                .is_err()
        );
    }

    #[test]
    fn patch_export_import_and_apply_cover_command_paths() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = write_sample_po(&dir, "source.po");
        let target_path = dir.path().join("target.po");
        fs::copy(&source_path, &target_path).unwrap();
        let patch_path = dir.path().join("hello.tpatch");

        let mut translator = test_app();
        translator.open_file(source_path.clone()).unwrap();
        let entry = translator.ui.selected_entry.unwrap();
        translator.update_translation(entry, 0, "Hallo".to_string());
        translator.update_question(entry, "form:0", "Should this be formal?".to_string());
        translator.export_patch(patch_path.clone()).unwrap();
        let patch = fs::read_to_string(&patch_path).unwrap();
        assert!(patch.contains("TranslateR-Questions-Json"));
        assert_eq!(translator.status, "TPatch exported");

        let mut maintainer = test_app();
        maintainer.open_file(target_path.clone()).unwrap();
        maintainer.import_diff(patch_path.clone()).unwrap();
        assert!(
            maintainer
                .ui
                .diff_text
                .as_ref()
                .unwrap()
                .contains("TranslateR questions")
        );
        maintainer.apply_imported_patch().unwrap();
        assert!(fs::read_to_string(&target_path).unwrap().contains("Hallo"));
        assert!(maintainer.ui.pending_patch.is_none());
        assert!(maintainer.ui.diff_text.is_none());
        assert_eq!(maintainer.status, "Applied TPatch");

        let edited_path = dir.path().join("edited.po");
        fs::write(
            &edited_path,
            fs::read_to_string(&target_path).unwrap().replacen(
                "msgstr \"\"",
                "msgstr \"Tschuess\"",
                1,
            ),
        )
        .unwrap();
        maintainer.import_diff(edited_path).unwrap();
        assert!(maintainer.ui.diff_text.as_ref().unwrap().contains("---"));
    }

    #[test]
    fn package_and_draft_modes_save_versions_and_restore_questions() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = tempfile::tempdir().unwrap();
        let _override = set_app_config_dir_override(config_dir.path().to_path_buf());
        let po_path = write_sample_po(&dir, "de.po");
        let pack_path = dir.path().join("de.trpack");
        let draft_path = dir.path().join("de.trdraft");
        let patch_dir = dir.path().join("patches");
        fs::create_dir_all(&patch_dir).unwrap();

        let mut maintainer = test_app();
        maintainer.open_file(po_path.clone()).unwrap();
        maintainer.export_trpack(pack_path.clone()).unwrap();
        let mut exported = read_trpack(&pack_path).unwrap();
        assert_eq!(exported.pack_version, "1");
        assert_eq!(maintainer.status, "TRPack exported as version 1");
        exported.contexts.push(crate::workflow::EntryContext {
            entry_id: "entry-1".to_string(),
            note: "Shown on the title screen".to_string(),
            screenshot_id: Some("screen-1".to_string()),
            tags: vec!["menu".to_string()],
        });
        exported.answers.push(crate::workflow::EntryAnswer {
            entry_id: "entry-1".to_string(),
            question: "Is this formal?".to_string(),
            answer: "Use a friendly tone.".to_string(),
            answered_at: "2026-06-18T00:00:00Z".to_string(),
        });
        exported.screenshots.push(crate::workflow::ScreenshotRef {
            id: "screen-1".to_string(),
            file_name: "title.png".to_string(),
            description: "Title screen".to_string(),
        });
        write_trpack(&pack_path, &exported).unwrap();

        let mut translator = test_app();
        translator.start_translator(pack_path.clone()).unwrap();
        assert_eq!(translator.mode, AppMode::Translator);
        assert!(
            translator
                .active_package
                .as_ref()
                .is_some_and(|p| !p.is_draft)
        );
        let entry = translator.ui.selected_entry.unwrap();
        translator.update_translation(entry, 0, "Hallo".to_string());
        translator.update_question(entry, "source", "Can this be casual?".to_string());
        translator.save_draft(draft_path.clone()).unwrap();
        let draft = read_trdraft(&draft_path).unwrap();
        assert_eq!(draft.questions.len(), 1);
        assert_eq!(
            translator.active_draft_path.as_deref(),
            Some(draft_path.as_path())
        );
        assert!(translator.active_package.as_ref().unwrap().is_draft);

        let mut reopened = test_app();
        reopened.start_translator(draft_path.clone()).unwrap();
        assert_eq!(reopened.mode, AppMode::Translator);
        assert_eq!(
            reopened.active_draft_path.as_deref(),
            Some(draft_path.as_path())
        );
        let reopened_entry = reopened.ui.selected_entry.unwrap();
        assert_eq!(
            reopened.question_value(reopened_entry, "source"),
            "Can this be casual?"
        );

        let mut package_maintainer = test_app();
        package_maintainer
            .start_maintainer(pack_path.clone(), patch_dir)
            .unwrap();
        let reexport_path = dir.path().join("reexport.trpack");
        package_maintainer
            .export_trpack(reexport_path.clone())
            .unwrap();
        let reexported = read_trpack(&reexport_path).unwrap();
        assert_eq!(reexported.project_id, exported.project_id);
        assert_eq!(reexported.pack_version, exported.pack_version);
        assert_eq!(reexported.contexts.len(), 1);
        assert_eq!(reexported.answers.len(), 1);
        assert_eq!(reexported.screenshots.len(), 1);
        let entry = package_maintainer.ui.selected_entry.unwrap();
        package_maintainer.update_translation(entry, 0, "Guten Tag".to_string());
        package_maintainer.save_active().unwrap();
        assert_eq!(
            package_maintainer
                .active_package
                .as_ref()
                .unwrap()
                .pack_version,
            "2"
        );
        assert_eq!(package_maintainer.versions.last().unwrap().version, "2");
        assert_eq!(package_maintainer.status, "Saved TRPack version 2");
        let saved_pack = read_trpack(&pack_path).unwrap();
        assert_eq!(saved_pack.pack_version, "2");
        assert_eq!(saved_pack.contexts.len(), 1);
        assert_eq!(saved_pack.answers.len(), 1);
        assert_eq!(saved_pack.screenshots.len(), 1);

        let current = package_maintainer.patch_base_text.clone().unwrap();
        package_maintainer
            .save_package_version(&current, "Noop")
            .unwrap();
        assert_eq!(
            package_maintainer.status,
            "No package version created: no content changes"
        );

        package_maintainer.active_package.as_mut().unwrap().is_draft = true;
        package_maintainer
            .save_package_version("changed", "Draft noop")
            .unwrap();
        assert_eq!(
            package_maintainer.status,
            "No package version created: no content changes"
        );
    }

    #[test]
    fn apply_selected_and_all_patches_cover_status_and_ordering() {
        let dir = tempfile::tempdir().unwrap();
        let po_path = write_sample_po(&dir, "de.po");
        let patch_dir = dir.path().join("patches");
        fs::create_dir_all(&patch_dir).unwrap();
        let base = fs::read_to_string(&po_path).unwrap();
        let one = base.replacen("msgstr \"\"", "msgstr \"Eins\"", 2);
        let two = one.replacen("msgstr \"\"", "msgstr \"Zwei\"", 1);
        fs::write(
            patch_dir.join("001.tpatch"),
            unified_diff(&base, &one, "base", "one"),
        )
        .unwrap();
        fs::write(
            patch_dir.join("002.tpatch"),
            unified_diff(&one, &two, "one", "two"),
        )
        .unwrap();

        let mut selected_app = test_app();
        selected_app.open_file(po_path.clone()).unwrap();
        selected_app.load_patch_folder(patch_dir.clone()).unwrap();
        selected_app.view_patch(0).unwrap();
        selected_app.apply_selected_patch().unwrap();
        assert!(selected_app.status.contains("001.tpatch"));
        assert!(fs::read_to_string(&po_path).unwrap().contains("Eins"));

        fs::write(&po_path, &base).unwrap();
        let mut missing_selected_app = test_app();
        missing_selected_app.open_file(po_path.clone()).unwrap();
        missing_selected_app
            .import_diff(patch_dir.join("001.tpatch"))
            .unwrap();
        missing_selected_app.ui.selected_patch = Some(usize::MAX);
        missing_selected_app.apply_selected_patch().unwrap();
        assert_eq!(missing_selected_app.status, "Applied TPatch");

        fs::write(&po_path, &base).unwrap();
        let mut all_app = test_app();
        all_app.open_file(po_path.clone()).unwrap();
        all_app.load_patch_folder(patch_dir.clone()).unwrap();
        all_app.apply_all_patches().unwrap();
        let merged = fs::read_to_string(&po_path).unwrap();
        assert!(merged.contains("Eins"));
        assert!(merged.contains("Zwei"));
        assert_eq!(all_app.status, "Applied all matching TPatches");

        let config_dir = tempfile::tempdir().unwrap();
        let _override = set_app_config_dir_override(config_dir.path().to_path_buf());
        let pack_path = dir.path().join("de.trpack");
        let mut pack_exporter = test_app();
        fs::write(&po_path, &base).unwrap();
        pack_exporter.open_file(po_path.clone()).unwrap();
        pack_exporter.export_trpack(pack_path.clone()).unwrap();
        let mut package_all_app = test_app();
        package_all_app
            .start_maintainer(pack_path.clone(), patch_dir.clone())
            .unwrap();
        package_all_app.apply_all_patches().unwrap();
        assert_eq!(
            package_all_app
                .active_package
                .as_ref()
                .unwrap()
                .pack_version,
            "2"
        );
        assert_eq!(read_trpack(&pack_path).unwrap().pack_version, "2");

        let bad_patch_dir = dir.path().join("bad-patches");
        fs::create_dir_all(&bad_patch_dir).unwrap();
        fs::write(
            bad_patch_dir.join("001.tpatch"),
            unified_diff(&base, &one, "base", "one"),
        )
        .unwrap();
        fs::write(bad_patch_dir.join("002.tpatch"), "not a unified diff").unwrap();
        fs::write(&po_path, &base).unwrap();
        let mut rollback_app = test_app();
        rollback_app.open_file(po_path.clone()).unwrap();
        rollback_app.load_patch_folder(bad_patch_dir).unwrap();
        assert!(
            rollback_app
                .apply_all_patches()
                .unwrap_err()
                .to_string()
                .contains("failed to apply TPatch")
        );
        assert_eq!(fs::read_to_string(&po_path).unwrap(), base);
    }

    #[test]
    fn app_command_edges_cover_summary_package_and_maintainer_apply_paths() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = tempfile::tempdir().unwrap();
        let _override = set_app_config_dir_override(config_dir.path().to_path_buf());
        let po_path = write_sample_po(&dir, "de.po");
        let patch_path = dir.path().join("update.tpatch");
        let patch_dir = dir.path().join("patches");
        fs::create_dir_all(&patch_dir).unwrap();

        let mut app = test_app();
        app.project.active_file = Some(0);
        app.refresh_active_summary();
        app.open_file(po_path.clone()).unwrap();
        app.project.active_file = Some(usize::MAX);
        app.refresh_active_summary();
        app.save_package_version("changed without package", "No package")
            .unwrap();

        let entry = app.ui.selected_entry.unwrap();
        app.update_translation(entry, 0, "Hallo".to_string());
        app.export_patch(patch_path.clone()).unwrap();

        let mut selected_without_index = test_app();
        selected_without_index.open_file(po_path.clone()).unwrap();
        selected_without_index
            .import_diff(patch_path.clone())
            .unwrap();
        selected_without_index.ui.selected_patch = None;
        selected_without_index.apply_selected_patch().unwrap();
        assert_eq!(selected_without_index.status, "Applied TPatch");

        fs::write(&po_path, sample_po("de")).unwrap();
        let pack_path = dir.path().join("de.trpack");
        let mut pack_exporter = test_app();
        pack_exporter.open_file(po_path.clone()).unwrap();
        pack_exporter.export_trpack(pack_path.clone()).unwrap();

        let mut package_maintainer = test_app();
        package_maintainer
            .start_maintainer(pack_path.clone(), patch_dir)
            .unwrap();
        package_maintainer.import_diff(patch_path).unwrap();
        package_maintainer.apply_imported_patch().unwrap();
        assert_eq!(
            package_maintainer
                .active_package
                .as_ref()
                .unwrap()
                .pack_version,
            "2"
        );
        assert_eq!(package_maintainer.status, "Saved TRPack version 2");
    }

    #[test]
    fn command_errors_are_reported_without_ui() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = test_app();
        assert!(app.open_file(dir.path().join("missing.po")).is_err());
        assert!(app.start_translator(dir.path().join("missing.po")).is_err());
        assert!(
            app.start_maintainer(dir.path().join("missing.po"), dir.path().to_path_buf())
                .is_err()
        );
        assert!(
            app.save_active()
                .unwrap_err()
                .to_string()
                .contains("no active")
        );
        assert!(
            app.export_patch(dir.path().join("x.tpatch"))
                .unwrap_err()
                .to_string()
                .contains("no active")
        );
        assert!(
            app.save_draft(dir.path().join("x.trdraft"))
                .unwrap_err()
                .to_string()
                .contains("no active")
        );
        assert!(
            app.export_trpack(dir.path().join("x.trpack"))
                .unwrap_err()
                .to_string()
                .contains("no active")
        );
        assert!(
            app.import_diff(dir.path().join("x.tpatch"))
                .unwrap_err()
                .to_string()
                .contains("no active")
        );
        assert!(
            app.apply_imported_patch()
                .unwrap_err()
                .to_string()
                .contains("no active")
        );
        assert!(app.load_patch_folder(dir.path().join("missing")).is_err());

        let path = write_sample_po(&dir, "de.po");
        let bad_pack = dir.path().join("bad.trpack");
        fs::write(&bad_pack, r#"{"format":"wrong"}"#).unwrap();
        assert!(app.start_translator(bad_pack.clone()).is_err());
        assert!(
            app.start_maintainer(bad_pack, dir.path().to_path_buf())
                .is_err()
        );

        let bad_draft = dir.path().join("bad.trdraft");
        fs::write(&bad_draft, r#"{"format":"wrong"}"#).unwrap();
        assert!(app.start_translator(bad_draft).is_err());

        app.open_file(path.clone()).unwrap();
        let bad_parent = dir.path().join("not-a-directory");
        fs::write(&bad_parent, "file").unwrap();
        let bad_destination = bad_parent.join("destination");
        assert!(
            app.export_patch(bad_destination.with_extension("tpatch"))
                .is_err()
        );
        assert!(
            app.export_trpack(bad_destination.with_extension("trpack"))
                .is_err()
        );
        assert!(
            app.save_draft(bad_destination.with_extension("trdraft"))
                .is_err()
        );
        assert!(app.import_diff(dir.path().join("missing.tpatch")).is_err());
        app.patch_base_text = None;
        assert!(
            app.export_patch(dir.path().join("x.tpatch"))
                .unwrap_err()
                .to_string()
                .contains("no base PO text")
        );
        assert!(
            app.save_draft(dir.path().join("x.trdraft"))
                .unwrap_err()
                .to_string()
                .contains("no base package")
        );
        assert!(
            app.apply_imported_patch()
                .unwrap_err()
                .to_string()
                .contains("no imported TPatch")
        );
        app.active_package = Some(ActivePackage {
            source_path: dir.path().join("de.trpack"),
            project_id: "game".to_string(),
            pack_version: "1".to_string(),
            language: Some("de".to_string()),
            base_hash: "base".to_string(),
            po_filename: "de.po".to_string(),
            is_draft: false,
            history: Vec::new(),
            contexts: Vec::new(),
            answers: Vec::new(),
            screenshots: Vec::new(),
        });
        app.doc = None;
        assert!(
            app.save_package_version("changed", "Save PO")
                .unwrap_err()
                .to_string()
                .contains("no active")
        );
        app.active_package = None;
        app.open_file(path.clone()).unwrap();

        let entry = app.ui.selected_entry.unwrap();
        app.update_translation(entry, 0, "Hallo".to_string());
        fs::write(&path, sample_po("fr")).unwrap();
        assert!(
            app.save_active()
                .unwrap_err()
                .to_string()
                .contains("file changed on disk")
        );

        let mut read_error_app = test_app();
        read_error_app.open_file(path.clone()).unwrap();
        let read_error_entry = read_error_app.ui.selected_entry.unwrap();
        read_error_app.update_translation(read_error_entry, 0, "Hallo".to_string());
        read_error_app.doc.as_mut().unwrap().path = dir.path().to_path_buf();
        assert!(
            read_error_app
                .save_active()
                .unwrap_err()
                .to_string()
                .contains("could not check active PO")
        );

        let mut save_error_app = test_app();
        save_error_app.open_file(path.clone()).unwrap();
        let save_entry = save_error_app.ui.selected_entry.unwrap();
        save_error_app.update_translation(save_entry, 0, "Hallo".to_string());
        let original_path = save_error_app.doc.as_ref().unwrap().path.clone();
        save_error_app.doc.as_mut().unwrap().path = bad_parent.join("active.po");
        assert!(save_error_app.save_active().is_err());
        save_error_app.doc.as_mut().unwrap().path = original_path;

        let mut apply_read_error = test_app();
        apply_read_error.open_file(path.clone()).unwrap();
        apply_read_error.doc.as_mut().unwrap().path = dir.path().join("missing-active.po");
        apply_read_error.ui.pending_patch = Some(Vec::new());
        assert!(apply_read_error.apply_imported_patch().is_err());

        let mut apply_bad_patch = test_app();
        apply_bad_patch.open_file(path).unwrap();
        apply_bad_patch.ui.pending_patch = Some(b"not a unified diff".to_vec());
        assert!(apply_bad_patch.apply_imported_patch().is_err());
    }

    #[test]
    fn workflow_package_start_reports_materialization_errors() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = tempfile::tempdir().unwrap();
        let patch_dir = dir.path().join("patches");
        fs::create_dir_all(&patch_dir).unwrap();
        let po_path = write_sample_po(&dir, "de.po");
        let pack_path = dir.path().join("de.trpack");
        let draft_path = dir.path().join("de.trdraft");

        {
            let _override = set_app_config_dir_override(config_dir.path().to_path_buf());
            let mut exporter = test_app();
            exporter.open_file(po_path).unwrap();
            exporter.export_trpack(pack_path.clone()).unwrap();
            exporter.save_draft(draft_path.clone()).unwrap();
        }

        let _override = set_app_config_dir_error_override();
        assert!(
            test_app()
                .start_translator(pack_path.clone())
                .unwrap_err()
                .to_string()
                .contains("could not resolve app config directory")
        );
        assert!(
            test_app()
                .start_maintainer(pack_path, patch_dir)
                .unwrap_err()
                .to_string()
                .contains("could not resolve app config directory")
        );
        assert!(
            test_app()
                .start_translator(draft_path)
                .unwrap_err()
                .to_string()
                .contains("could not resolve app config directory")
        );
    }

    #[test]
    fn draft_import_and_update_noop_edges_cover_clean_branches() {
        let dir = tempfile::tempdir().unwrap();
        let po_path = write_sample_po(&dir, "standalone.po");
        let draft_path = dir.path().join("standalone.trdraft");
        let patch_path = dir.path().join("matching.tpatch");

        let mut no_doc = test_app();
        no_doc.update_translation(EntryId(1), 0, "ignored".to_string());
        assert!(no_doc.doc.is_none());
        assert!(no_doc.active_language().is_none());

        let mut app = test_app();
        app.open_file(po_path.clone()).unwrap();
        let entry = app.ui.selected_entry.unwrap();
        let key = app.entry_question_id(entry).unwrap();
        assert_eq!(key.len(), 64);
        app.ui
            .questions
            .insert("malformed-key".to_string(), "ignored".to_string());
        app.ui.questions.insert(
            scoped_question_key(&key, "source"),
            "Needs context".to_string(),
        );
        let questions = app.active_entry_questions();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].question, "Needs context");
        app.update_translation(entry, 0, "Hallo".to_string());
        app.save_draft(draft_path.clone()).unwrap();
        let draft = read_trdraft(&draft_path).unwrap();
        assert_eq!(draft.project_id, "standalone");
        assert_eq!(draft.questions.len(), 1);
        assert!(draft.history.is_empty());

        let base = fs::read_to_string(&po_path).unwrap();
        let edited = base.replacen("msgstr \"Hallo\"", "msgstr \"Guten Tag\"", 1);
        let package = ActivePackage {
            source_path: patch_path.clone(),
            project_id: "standalone".to_string(),
            pack_version: "1".to_string(),
            language: Some("de".to_string()),
            base_hash: sha256_bytes(base.as_bytes()),
            po_filename: "standalone.po".to_string(),
            is_draft: false,
            history: Vec::new(),
            contexts: Vec::new(),
            answers: Vec::new(),
            screenshots: Vec::new(),
        };
        let patch = add_tpatch_metadata(
            unified_diff(&base, &edited, "base", "edited"),
            Some(&package),
            &[],
        );
        fs::write(&patch_path, patch).unwrap();

        app.import_diff(patch_path).unwrap();
        assert!(
            !app.ui
                .diff_text
                .as_ref()
                .unwrap()
                .contains("TranslateR warning")
        );
    }

    #[test]
    fn header_language_and_ui_language_commands_update_state() {
        let _i18n_guard = crate::test_support::i18n_runtime_guard();
        let dir = tempfile::tempdir().unwrap();
        let config_dir = tempfile::tempdir().unwrap();
        let _override = set_app_config_dir_override(config_dir.path().to_path_buf());
        let path = write_sample_po(&dir, "de.po");
        let mut app = test_app();

        assert!(app.update_header_language("fr".to_string()).is_err());
        app.open_file(path).unwrap();
        app.update_header_language("fr".to_string()).unwrap();
        assert_eq!(app.active_language().as_deref(), Some("fr"));
        assert_eq!(app.project.files[0].language.as_deref(), Some("fr"));
        assert_eq!(app.status, "Language set to fr");
        app.update_header_field(
            crate::po::header::HEADER_LAST_TRANSLATOR,
            "Test Translator".to_string(),
        )
        .unwrap();
        assert_eq!(
            parse_header(app.doc.as_ref().unwrap())
                .last_translator
                .as_deref(),
            Some("Test Translator")
        );
        app.undo();
        assert_ne!(
            parse_header(app.doc.as_ref().unwrap())
                .last_translator
                .as_deref(),
            Some("Test Translator")
        );
        app.undo();
        assert_eq!(app.active_language().as_deref(), Some("de"));
        app.redo();
        assert_eq!(app.active_language().as_deref(), Some("fr"));

        let mut header_error_app = test_app();
        header_error_app
            .ui
            .undo_stack
            .push(UndoAction::HeaderLanguage {
                before: "Language: de\n".to_string(),
                after: "Language: fr\n".to_string(),
            });
        header_error_app.undo();
        assert!(
            header_error_app
                .last_error
                .as_deref()
                .unwrap()
                .contains("no active")
        );

        let mut no_header_app = test_app();
        no_header_app.doc =
            Some(parse_text("no-header.po", "msgid \"Hello\"\nmsgstr \"\"\n".to_string()).unwrap());
        no_header_app
            .ui
            .undo_stack
            .push(UndoAction::HeaderLanguage {
                before: "Language: de\n".to_string(),
                after: "Language: fr\n".to_string(),
            });
        no_header_app.undo();
        assert!(
            no_header_app
                .last_error
                .as_deref()
                .unwrap()
                .contains("no header")
        );

        let mut no_msgstr_app = test_app();
        no_msgstr_app.doc = Some(parse_text("no-msgstr.po", "msgid \"\"\n".to_string()).unwrap());
        no_msgstr_app
            .ui
            .undo_stack
            .push(UndoAction::HeaderLanguage {
                before: "Language: de\n".to_string(),
                after: "Language: fr\n".to_string(),
            });
        no_msgstr_app.undo();
        assert!(
            no_msgstr_app
                .last_error
                .as_deref()
                .unwrap()
                .contains("no msgstr")
        );

        app.set_ui_language("en".to_string()).unwrap();
        assert_eq!(app.config.ui_language, "en");
        assert_eq!(crate::i18n::current_language(), "en");
        assert!(
            fs::read_to_string(config_dir.path().join("config.json"))
                .unwrap()
                .contains("\"ui_language\": \"en\"")
        );

        let ctx = egui::Context::default();
        app.set_theme_mode(ThemeMode::Light, &ctx).unwrap();
        assert_eq!(app.config.theme, ThemeMode::Light);
        assert_eq!(app.status, "Theme set to Light");
        app.set_theme_mode(ThemeMode::Dark, &ctx).unwrap();
        assert_eq!(app.config.theme, ThemeMode::Dark);
        assert_eq!(app.status, "Theme set to Dark");
        app.set_theme_mode(ThemeMode::System, &ctx).unwrap();
        assert_eq!(app.config.theme, ThemeMode::System);
        assert_eq!(app.status, "Theme set to System");

        drop(_override);
        let _override = set_app_config_dir_error_override();
        assert!(app.set_ui_language("en".to_string()).is_err());
        assert!(app.set_theme_mode(ThemeMode::Light, &ctx).is_err());
    }

    #[test]
    fn undo_redo_and_unsaved_close_state_track_editor_actions() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_sample_po(&dir, "de.po");

        let mut empty_app = test_app();
        empty_app.undo();
        empty_app.redo();
        empty_app.ui.undo_stack.push(UndoAction::Translation {
            entry_id: EntryId(1),
            index: 0,
            before: String::new(),
            after: "Hallo".to_string(),
        });
        empty_app.undo();
        assert!(
            empty_app
                .last_error
                .as_deref()
                .unwrap()
                .contains("no active")
        );
        assert!(empty_app.can_undo());
        empty_app.ui.redo_stack.push(UndoAction::Translation {
            entry_id: EntryId(1),
            index: 0,
            before: String::new(),
            after: "Hallo".to_string(),
        });
        empty_app.redo();
        assert!(empty_app.can_redo());

        let mut app = test_app();
        app.open_file(path).unwrap();

        let first = app.ui.selected_entry.unwrap();
        let second = app.doc.as_ref().unwrap().entries[2].id;
        app.update_translation(first, 0, "Hallo".to_string());
        app.update_translation(second, 0, "Tschuess".to_string());
        app.update_fuzzy(first, true);
        app.update_translator_comments(first, "Needs review".to_string());

        assert!(app.can_undo());
        assert!(!app.can_redo());
        assert!(app.has_unsaved_changes());
        app.request_close_confirmation();
        assert!(app.ui.show_close_confirmation);
        app.mark_close_confirmed();
        assert!(app.ui.close_confirmed);
        assert!(!app.ui.show_close_confirmation);

        app.undo();
        assert_eq!(
            app.doc.as_ref().unwrap().entries[1]
                .edited
                .translator_comments
                .as_deref(),
            None
        );
        app.undo();
        assert_eq!(app.doc.as_ref().unwrap().entries[1].edited.fuzzy, None);
        assert!(app.can_redo());
        app.undo();
        assert_eq!(app.doc.as_ref().unwrap().entries[2].msgstr[0].value(), "");
        app.undo();
        assert_eq!(app.doc.as_ref().unwrap().entries[1].msgstr[0].value(), "");
        assert!(!app.has_unsaved_changes());

        app.redo();
        assert_eq!(
            app.doc.as_ref().unwrap().entries[1].msgstr[0].value(),
            "Hallo"
        );
        app.redo();
        assert_eq!(
            app.doc.as_ref().unwrap().entries[2].msgstr[0].value(),
            "Tschuess"
        );
        app.redo();
        assert_eq!(
            app.doc.as_ref().unwrap().entries[1].edited.fuzzy,
            Some(true)
        );
        app.redo();
        assert_eq!(
            app.doc.as_ref().unwrap().entries[1]
                .edited
                .translator_comments
                .as_deref(),
            Some("Needs review")
        );
    }

    #[test]
    fn keyboard_shortcuts_route_to_mode_specific_commands() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_sample_po(&dir, "de.po");
        let mut app = test_app();
        app.open_file(path).unwrap();

        let first = app.ui.selected_entry.unwrap();
        app.update_translation(first, 0, "Hallo".to_string());
        run_shortcut(&mut app, egui::Key::Z);
        assert_eq!(app.doc.as_ref().unwrap().entries[1].msgstr[0].value(), "");
        run_shortcut(&mut app, egui::Key::Y);
        assert_eq!(
            app.doc.as_ref().unwrap().entries[1].msgstr[0].value(),
            "Hallo"
        );

        app.mode = AppMode::Maintainer;
        run_shortcut(&mut app, egui::Key::S);
        assert_eq!(
            app.ui.pending_confirmation.as_ref().unwrap().operation,
            FileOperation::SavePo
        );

        let draft_path = dir.path().join("work.trdraft");
        app.ui.pending_confirmation = None;
        app.mode = AppMode::Translator;
        app.active_draft_path = Some(draft_path.clone());
        run_shortcut(&mut app, egui::Key::S);
        let pending = app.ui.pending_confirmation.as_ref().unwrap();
        assert_eq!(pending.operation, FileOperation::SaveTrDraft);
        assert!(matches!(
            &pending.action,
            ConfirmedAction::SaveDraft(path) if path == &draft_path
        ));

        let mut startup = test_app();
        run_shortcut(&mut startup, egui::Key::S);
        assert!(startup.ui.pending_confirmation.is_none());
    }

    #[test]
    fn import_diff_warns_when_tpatch_base_hash_differs() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_sample_po(&dir, "de.po");
        let patch_path = dir.path().join("mismatch.tpatch");
        let base = fs::read_to_string(&path).unwrap();
        let edited = base.replacen("msgstr \"\"", "msgstr \"Hallo\"", 2);
        let mut patch = unified_diff(&base, &edited, "base", "edited");
        patch = add_tpatch_metadata(
            patch,
            Some(&ActivePackage {
                source_path: patch_path.clone(),
                project_id: "game".to_string(),
                pack_version: "1".to_string(),
                language: Some("de".to_string()),
                base_hash: sha256_bytes(b"different base"),
                po_filename: "de.po".to_string(),
                is_draft: false,
                history: Vec::new(),
                contexts: Vec::new(),
                answers: Vec::new(),
                screenshots: Vec::new(),
            }),
            &[],
        );
        fs::write(&patch_path, patch).unwrap();

        let mut app = test_app();
        app.open_file(path).unwrap();
        app.import_diff(patch_path).unwrap();
        assert!(
            app.ui
                .diff_text
                .as_ref()
                .unwrap()
                .contains("TranslateR warning")
        );
    }

    #[test]
    fn refresh_version_history_selects_existing_or_latest_version() {
        let dir = tempfile::tempdir().unwrap();
        let po = sample_po("de");
        let doc = parse_text(dir.path().join("de.po"), po.clone()).unwrap();
        let mut pack = trpack_from_document(
            &doc,
            po.clone(),
            Some("game".to_string()),
            Some("1".to_string()),
        );
        let edited = po.replacen("msgstr \"\"", "msgstr \"Hallo\"", 2);
        pack.history.push(version_log_entry(
            "2".to_string(),
            "Tester".to_string(),
            "Save PO".to_string(),
            &po,
            &edited,
            change_summary("de.po", &po, &edited).unwrap(),
        ));
        let mut app = test_app();
        app.active_package = Some(ActivePackage::from_pack(
            dir.path().join("de.trpack"),
            &pack,
        ));
        app.ui.selected_history_version = Some("missing".to_string());
        app.refresh_version_history();
        assert_eq!(app.ui.selected_history_version.as_deref(), Some("2"));
        app.ui.selected_history_version = Some("1".to_string());
        app.refresh_version_history();
        assert_eq!(app.ui.selected_history_version.as_deref(), Some("1"));

        let parsed = parse_text(dir.path().join("de.po"), edited.clone()).unwrap();
        let current = write_document(&parsed);
        assert_eq!(current, edited);
    }
}
