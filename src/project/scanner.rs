use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub fn scan_po_files(root: &Path) -> Vec<PathBuf> {
    let mut files = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("po"))
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}
