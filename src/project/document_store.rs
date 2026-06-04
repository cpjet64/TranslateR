use std::path::PathBuf;

use crate::po::{
    PoDocument,
    header::parse_header,
    stats::{self, PoStats},
};

#[derive(Debug, Clone)]
pub struct PoFileSummary {
    pub path: PathBuf,
    pub language: Option<String>,
    pub stats: PoStats,
}

impl PoFileSummary {
    pub fn from_doc(doc: &PoDocument) -> Self {
        Self {
            path: doc.path.clone(),
            language: parse_header(doc).language,
            stats: stats::stats(doc),
        }
    }
}

#[derive(Debug, Default)]
pub struct ProjectState {
    pub root_dir: Option<PathBuf>,
    pub files: Vec<PoFileSummary>,
    pub active_file: Option<usize>,
}
