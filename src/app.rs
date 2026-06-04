use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};

use crate::{
    history::{HistoryDb, HistoryState, VersionInfo},
    po::{
        EntryId, PoDocument, header::parse_header, parse_document, validate::validate_document,
        writer::write_document_bytes,
    },
    project::{AppConfig, PoFileSummary, ProjectState},
    util::{atomic_save::save_atomic_bytes, hashing::sha256_bytes},
    vcs::diff::unified_diff,
};

pub struct TranslateRApp {
    pub mode: AppMode,
    pub project: ProjectState,
    pub doc: Option<PoDocument>,
    pub config: AppConfig,
    pub history: HistoryDb,
    pub history_state: Option<HistoryState>,
    pub versions: Vec<VersionInfo>,
    pub ui: UiState,
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
    pub confirm_restore: bool,
    pub diff_text: Option<String>,
    pub pending_patch: Option<Vec<u8>>,
    pub patch_folder: Option<PathBuf>,
    pub patch_files: Vec<PathBuf>,
    pub selected_patch: Option<usize>,
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
            history: HistoryDb::open_default().expect("history database should open"),
            history_state: None,
            versions: Vec::new(),
            ui: UiState::default(),
            last_error: None,
            status: "Ready".to_string(),
        }
    }

    pub fn open_file(&mut self, path: PathBuf) -> Result<()> {
        let doc = parse_document(&path)?;
        self.project.root_dir = path.parent().map(PathBuf::from);
        self.project.files = vec![PoFileSummary::from_doc(&doc)];
        self.project.active_file = Some(0);
        self.ui.selected_entry = first_translatable_entry(&doc);
        self.status = format!("Opened {}", path.display());
        self.doc = Some(doc);
        self.refresh_history_state();
        Ok(())
    }

    pub fn start_translator(&mut self, path: PathBuf) -> Result<()> {
        self.mode = AppMode::Translator;
        self.open_file(path)?;
        self.record_version("Base PO")?;
        self.status = "Translator mode".to_string();
        Ok(())
    }

    pub fn start_maintainer(&mut self, po_path: PathBuf, patch_folder: PathBuf) -> Result<()> {
        self.mode = AppMode::Maintainer;
        self.open_file(po_path)?;
        self.record_version("Maintainer base PO")?;
        self.load_patch_folder(patch_folder)?;
        self.status = "Maintainer mode".to_string();
        Ok(())
    }

    pub fn save_active(&mut self) -> Result<()> {
        let doc = self
            .doc
            .as_mut()
            .ok_or_else(|| anyhow!("no active document"))?;
        let disk = fs::read(&doc.path).unwrap_or_default();
        if doc.dirty && !disk.is_empty() && sha256_bytes(&disk) != doc.original_hash {
            return Err(anyhow!(
                "file changed on disk; reload before saving or overwrite intentionally"
            ));
        }
        validate_document(doc);
        let output = write_document_bytes(doc)?;
        save_atomic_bytes(&doc.path, &output)?;
        let reparsed = parse_document(&doc.path)?;
        *doc = reparsed;
        self.record_version("Save")?;
        self.refresh_active_summary();
        self.status = "Saved version".to_string();
        Ok(())
    }

    pub fn record_version(&mut self, note: &str) -> Result<()> {
        let doc_path = self
            .doc
            .as_ref()
            .map(|d| d.path.clone())
            .ok_or_else(|| anyhow!("no active document"))?;
        let version = self.history.record_version(&doc_path, &self.config, note)?;
        self.refresh_history_state();
        self.status = format!("Saved version {version}");
        Ok(())
    }

    pub fn restore_latest(&mut self) -> Result<()> {
        let doc_path = self
            .doc
            .as_ref()
            .map(|d| d.path.clone())
            .ok_or_else(|| anyhow!("no active document"))?;
        self.history.restore_latest(&doc_path)?;
        let doc = parse_document(&doc_path)?;
        self.doc = Some(doc);
        self.refresh_active_summary();
        self.refresh_history_state();
        self.status = "Restored latest version".to_string();
        Ok(())
    }

    pub fn export_patch(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!("no active document"))?;
        let Some(base) = self.history.latest_bytes(&doc.path)? else {
            return Err(anyhow!(
                "save at least one version before exporting a patch"
            ));
        };
        let current = write_document_bytes(doc)?;
        let patch = unified_diff(
            &String::from_utf8_lossy(&base),
            &String::from_utf8_lossy(&current),
            "saved-version",
            &doc.path.display().to_string(),
        )?;
        fs::write(path, patch)?;
        self.status = "TPatch exported".to_string();
        Ok(())
    }

    pub fn import_diff(&mut self, path: PathBuf) -> Result<()> {
        let doc = self
            .doc
            .as_ref()
            .ok_or_else(|| anyhow!("no active document"))?;
        let imported = fs::read(&path)?;
        let diff = if path
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
            self.record_version(&format!("Apply {}", file_name(&path)))?;
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
        self.record_version("Apply TPatch")?;
        self.refresh_active_summary();
        self.ui.diff_text = None;
        self.ui.pending_patch = None;
        self.status = "Applied TPatch and saved version".to_string();
        Ok(())
    }

    pub fn update_translation(&mut self, entry_id: EntryId, index: usize, value: String) {
        if let Some(doc) = &mut self.doc {
            crate::po::writer::set_translation(doc, entry_id, index, value);
            validate_document(doc);
        }
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

    fn refresh_history_state(&mut self) {
        let Some(doc) = &self.doc else {
            self.history_state = None;
            self.versions.clear();
            return;
        };
        self.history_state = self.history.state_for_file(&doc.path).ok();
        self.versions = self.history.versions(&doc.path).unwrap_or_default();
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
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
