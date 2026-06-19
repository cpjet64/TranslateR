use std::{fs, path::Path};

use pretty_assertions::assert_eq;
use translater::po::{parser::parse_text_with_bytes, writer::write_document_bytes};

#[test]
fn parses_all_sample_po_files() {
    for path in fixture_po_files() {
        let bytes = fs::read(&path)
            .unwrap_or_else(|err| panic!("could not read {}: {err}", path.display()));
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let doc = parse_text_with_bytes(&path, text, bytes).unwrap();
        let fatal = doc
            .diagnostics
            .iter()
            .filter(|d| d.severity == translater::po::DiagnosticSeverity::Error)
            .count();
        assert_eq!(fatal, 0, "fatal diagnostics in {}", path.display());
    }
}

#[test]
fn round_trips_all_sample_po_files_without_edits() {
    for path in fixture_po_files() {
        let bytes = fs::read(&path)
            .unwrap_or_else(|err| panic!("could not read {}: {err}", path.display()));
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let doc = parse_text_with_bytes(&path, text, bytes.clone()).unwrap();
        let output = write_document_bytes(&doc);
        assert_eq!(output, bytes, "round-trip changed {}", path.display());
    }
}

fn fixture_po_files() -> Vec<std::path::PathBuf> {
    let root = Path::new("tests/fixtures/gettext-po-samples/po");
    let mut files = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "po"))
        .collect::<Vec<_>>();
    files.sort();
    files
}
