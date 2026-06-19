use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Result, anyhow};

use crate::{
    po::{
        EntryId, PoDocument, header::parse_header, parse_document,
        parser::parse_document as parse_po_document, validate::validate_document,
        writer::write_document_bytes,
    },
    project::{AppConfig, PoFileSummary, ProjectState},
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
    pub filter: MessageFilter,
    pub show_history: bool,
    pub diff_text: Option<String>,
    pub pending_patch: Option<Vec<u8>>,
    pub patch_folder: Option<PathBuf>,
    pub patch_files: Vec<PathBuf>,
    pub selected_patch: Option<usize>,
    pub header_language_editing: bool,
    pub header_language_draft: String,
    pub questions: BTreeMap<String, String>,
    pub selected_history_version: Option<String>,
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

impl TranslateRApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        crate::ui::fonts::install_fonts(&_cc.egui_ctx);
        Self {
            mode: AppMode::Startup,
            project: ProjectState::default(),
            doc: None,
            config: AppConfig::load(),
            versions: Vec::new(),
            ui: UiState::default(),
            active_package: None,
            active_draft_path: None,
            patch_base_text: None,
            last_error: None,
            status: "Ready".to_string(),
        }
    }

    pub fn open_file(&mut self, path: PathBuf) -> Result<()> {
        let base_text = fs::read_to_string(&path)?;
        let doc = parse_document(&path)?;
        self.active_package = None;
        self.active_draft_path = None;
        self.patch_base_text = Some(base_text);
        self.ui.questions.clear();
        self.project.root_dir = path.parent().map(PathBuf::from);
        self.project.files = vec![PoFileSummary::from_doc(&doc)];
        self.project.active_file = Some(0);
        self.ui.selected_entry = first_translatable_entry(&doc);
        self.status = format!("Opened {}", path.display());
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
        self.status = "Translator mode".to_string();
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
        self.status = "Maintainer mode".to_string();
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
        self.status = format!(
            "Maintainer mode: {} package {}",
            pack.project_id, pack.pack_version
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
        self.status = format!(
            "Translator mode: {} package {}",
            pack.project_id, pack.pack_version
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
        self.ui.selected_entry = first_translatable_entry(&doc);
        self.doc = Some(doc);
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
        self.status = format!(
            "Draft loaded: {} package {}",
            draft.project_id, draft.pack_version
        );
        Ok(())
    }

    pub fn save_active(&mut self) -> Result<()> {
        let doc = self
            .doc
            .as_mut()
            .ok_or_else(|| anyhow!("no active document"))?;
        if !doc.dirty {
            self.status = "No saved version created: no edits to save".to_string();
            return Ok(());
        }
        let disk = fs::read(&doc.path).unwrap_or_default();
        if doc.dirty && !disk.is_empty() && sha256_bytes(&disk) != doc.original_hash {
            return Err(anyhow!(
                "file changed on disk; reload before saving or overwrite intentionally"
            ));
        }
        validate_document(doc);
        let output = write_document_bytes(doc)?;
        let output_text = String::from_utf8(output.clone())?;
        save_atomic_bytes(&doc.path, &output)?;
        let reparsed = parse_document(&doc.path)?;
        *doc = reparsed;
        if self.mode == AppMode::Maintainer {
            self.save_package_version(&output_text, "Save PO")?;
        }
        self.refresh_active_summary();
        if !matches!(self.mode, AppMode::Maintainer) || self.active_package.is_none() {
            self.status = "Saved PO".to_string();
        }
        Ok(())
    }

    pub fn export_patch(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!("no active document"))?;
        let base = self
            .patch_base_text
            .as_ref()
            .ok_or_else(|| anyhow!("no base PO text available for TPatch export"))?;
        let current = write_document_bytes(doc)?;
        let patch = unified_diff(
            base,
            &String::from_utf8_lossy(&current),
            "package-base",
            &doc.path.display().to_string(),
        )?;
        let questions = self.active_entry_questions();
        let patch = add_tpatch_metadata(patch, self.active_package.as_ref(), &questions);
        save_atomic_bytes(&path, patch.as_bytes())?;
        self.status = "TPatch exported".to_string();
        Ok(())
    }

    pub fn export_trpack(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!("no active document"))?;
        let current = write_document_bytes(doc)?;
        let po_text = String::from_utf8(current)?;
        let mut pack = trpack_from_document(doc, po_text.clone(), None, None);
        if let Some(package) = &self.active_package {
            pack.project_id = package.project_id.clone();
            pack.pack_version = package.pack_version.clone();
            pack.history = package.history.clone();
        }
        let version = pack.pack_version.clone();
        write_trpack(&path, &pack)?;
        self.status = format!("TRPack exported as version {version}");
        Ok(())
    }

    pub fn save_draft(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!("no active document"))?;
        let base = self
            .patch_base_text
            .as_ref()
            .ok_or_else(|| anyhow!("no base package version available for this draft"))?;
        let current = write_document_bytes(doc)?;
        let mut draft = trdraft_from_document(
            doc,
            String::from_utf8(current)?,
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
        self.status = format!(
            "Draft saved for package {} version {}",
            draft.project_id, draft.pack_version
        );
        Ok(())
    }

    pub fn import_diff(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!("no active document"))?;
        let imported = fs::read(&path)?;
        let mut diff = if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tpatch"))
        {
            String::from_utf8_lossy(&imported).into_owned()
        } else {
            let current = fs::read(&doc.path)?;
            unified_diff(
                &String::from_utf8_lossy(&current),
                &String::from_utf8_lossy(&imported),
                &doc.path.display().to_string(),
                &path.display().to_string(),
            )?
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
        self.status = format!("Imported TPatch {}", path.display());
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
            .ok_or_else(|| anyhow!("TPatch index out of range"))?;
        self.ui.selected_patch = Some(index);
        self.import_diff(path)
    }

    pub fn apply_selected_patch(&mut self) -> Result<()> {
        self.apply_imported_patch()?;
        if let Some(index) = self.ui.selected_patch {
            if let Some(path) = self.ui.patch_files.get(index) {
                self.status = format!("Applied {}", path.display());
            }
        }
        Ok(())
    }

    pub fn apply_all_patches(&mut self) -> Result<()> {
        let patches = self.ui.patch_files.clone();
        for path in patches {
            self.import_diff(path.clone())?;
            self.apply_imported_patch()?;
        }
        self.status = "Applied all matching TPatches".to_string();
        Ok(())
    }

    pub fn apply_imported_patch(&mut self) -> Result<()> {
        let doc_path = self
            .doc
            .as_ref()
            .map(|d| d.path.clone())
            .ok_or_else(|| anyhow!("no active document"))?;
        let patch = self
            .ui
            .pending_patch
            .as_ref()
            .ok_or_else(|| anyhow!("no imported TPatch to apply"))?;
        let current = fs::read_to_string(&doc_path)?;
        let patch_text = String::from_utf8_lossy(patch);
        let merged = crate::vcs::diff::apply_unified_patch(&current, &patch_text)?;
        save_atomic_bytes(&doc_path, merged.as_bytes())?;
        let doc = parse_document(&doc_path)?;
        self.doc = Some(doc);
        if self.mode == AppMode::Maintainer {
            self.save_package_version(&merged, "Apply TPatch")?;
        }
        self.refresh_active_summary();
        self.ui.diff_text = None;
        self.ui.pending_patch = None;
        if self.active_package.is_none() {
            self.status = "Applied TPatch".to_string();
        }
        Ok(())
    }

    pub fn update_translation(&mut self, entry_id: EntryId, index: usize, value: String) {
        if let Some(doc) = &mut self.doc {
            crate::po::writer::set_translation(doc, entry_id, index, value);
            validate_document(doc);
        }
    }

    pub fn update_header_language(&mut self, language: String) -> Result<()> {
        let doc = self
            .doc
            .as_mut()
            .ok_or_else(|| anyhow!("no active document"))?;
        crate::po::header::set_header_language(doc, &language)?;
        validate_document(doc);
        self.refresh_active_summary();
        self.status = format!("Language set to {}", language.trim());
        Ok(())
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

    fn refresh_active_summary(&mut self) {
        if let (Some(index), Some(doc)) = (self.project.active_file, &self.doc) {
            if let Some(slot) = self.project.files.get_mut(index) {
                *slot = PoFileSummary::from_doc(doc);
            }
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
            self.status = "No package version created: no content changes".to_string();
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
            .ok_or_else(|| anyhow!("no active document"))?;
        let mut pack = trpack_from_document(
            doc,
            current_text.to_string(),
            Some(package.project_id.clone()),
            Some(version.clone()),
        );
        let mut history = package.history.clone();
        history.push(entry);
        pack.history = history.clone();
        pack.contexts = Vec::new();
        pack.answers = Vec::new();
        pack.screenshots = Vec::new();
        write_trpack(&package.source_path, &pack)?;

        package.pack_version = version.clone();
        package.base_hash = pack.base_hash.clone();
        package.language = pack.language.clone();
        package.po_filename = pack.po_filename.clone();
        package.history = history;
        self.patch_base_text = Some(current_text.to_string());
        self.refresh_version_history();
        self.status = format!("Saved TRPack version {version}");
        Ok(())
    }
}

fn scoped_question_key(entry_id: &str, scope: &str) -> String {
    format!("{entry_id}|{scope}")
}

impl eframe::App for TranslateRApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::ui::draw(self, ui);
    }
}

pub(crate) fn first_translatable_entry(doc: &PoDocument) -> Option<EntryId> {
    doc.entries
        .iter()
        .find(|entry| !entry.is_header())
        .map(|entry| entry.id)
}

#[cfg(test)]
mod tests {
    use super::first_translatable_entry;
    use crate::po::parser::parse_text;

    #[test]
    fn first_translatable_entry_skips_header() {
        let input =
            "msgid \"\"\nmsgstr \"Language: ar\\n\"\n\nmsgid \"Privacy Policy\"\nmsgstr \"\"\n"
                .to_string();
        let doc = parse_text("sample.po", input).unwrap();
        assert_eq!(first_translatable_entry(&doc), Some(doc.entries[1].id));
    }
}
