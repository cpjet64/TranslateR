use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Result, anyhow};

use crate::{
    i18n::{tr, tr_format},
    po::{
        EntryId, PoDocument,
        header::parse_header,
        parse_document,
        parser::{parse_document as parse_po_document, parse_text_with_bytes},
        validate::validate_document,
        writer::{write_document, write_document_bytes},
    },
    project::{AppConfig, PoFileSummary, ProjectState},
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
    pub header_language_editing: bool,
    pub header_language_draft: String,
    pub questions: BTreeMap<String, String>,
    pub translation_buffers: BTreeMap<String, String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TranslationUnitSort {
    #[default]
    FileOrder,
    FirstLetter,
}

impl TranslateRApp {
    pub fn open_file(&mut self, path: PathBuf) -> Result<()> {
        let bytes = fs::read(&path)?;
        let base_text = String::from_utf8_lossy(&bytes).into_owned();
        let doc = parse_text_with_bytes(&path, base_text.clone(), bytes)?;
        self.active_package = None;
        self.active_draft_path = None;
        self.patch_base_text = Some(base_text);
        self.ui.questions.clear();
        self.ui.translation_buffers.clear();
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
        let disk = fs::read(&doc.path).unwrap_or_default();
        if doc.dirty && !disk.is_empty() && sha256_bytes(&disk) != doc.original_hash {
            return Err(anyhow!(
                tr("file changed on disk; reload before saving or overwrite intentionally")
                    .into_owned()
            ));
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

    pub fn export_patch(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        let base = self.patch_base_text.as_ref().ok_or_else(|| {
            anyhow!(tr("no base PO text available for TPatch export").into_owned())
        })?;
        let current = write_document_bytes(doc);
        let patch = unified_diff(
            base,
            &String::from_utf8_lossy(&current),
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
            String::from_utf8_lossy(&imported).into_owned()
        } else {
            let current = fs::read(&doc.path)?;
            unified_diff(
                &String::from_utf8_lossy(&current),
                &String::from_utf8_lossy(&imported),
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
        for path in patches {
            self.import_diff(path.clone())?;
            self.apply_imported_patch()?;
        }
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
        let current = fs::read_to_string(&doc_path)?;
        let patch_text = String::from_utf8_lossy(patch);
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
        if let Some(doc) = &mut self.doc {
            crate::po::writer::set_translation(doc, entry_id, index, value);
            validate_document(doc);
        }
    }

    pub fn select_entry(&mut self, entry_id: Option<EntryId>) {
        self.ui.selected_entry = entry_id;
    }

    pub fn update_header_language(&mut self, language: String) -> Result<()> {
        let doc = self
            .doc
            .as_mut()
            .ok_or_else(|| anyhow!(tr("no active document").into_owned()))?;
        crate::po::header::set_header_language(doc, &language)?;
        validate_document(doc);
        self.refresh_active_summary();
        self.status = tr_format(
            "Language set to {language}",
            &[("language", language.trim().to_string())],
        );
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
}

fn scoped_question_key(entry_id: &str, scope: &str) -> String {
    format!("{entry_id}|{scope}")
}

pub(crate) fn first_translatable_entry(doc: &PoDocument) -> Option<EntryId> {
    doc.entries
        .iter()
        .find(|entry| !entry.is_header())
        .map(|entry| entry.id)
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
        let exported = read_trpack(&pack_path).unwrap();
        assert_eq!(exported.pack_version, "1");
        assert_eq!(maintainer.status, "TRPack exported as version 1");

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
        assert_eq!(read_trpack(&pack_path).unwrap().pack_version, "2");

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
        all_app.load_patch_folder(patch_dir).unwrap();
        all_app.apply_all_patches().unwrap();
        let merged = fs::read_to_string(&po_path).unwrap();
        assert!(merged.contains("Eins"));
        assert!(merged.contains("Zwei"));
        assert_eq!(all_app.status, "Applied all matching TPatches");
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

        app.set_ui_language("en".to_string()).unwrap();
        assert_eq!(app.config.ui_language, "en");
        assert_eq!(crate::i18n::current_language(), "en");
        assert!(
            fs::read_to_string(config_dir.path().join("config.json"))
                .unwrap()
                .contains("\"ui_language\": \"en\"")
        );

        drop(_override);
        let _override = set_app_config_dir_error_override();
        assert!(app.set_ui_language("en".to_string()).is_err());
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
