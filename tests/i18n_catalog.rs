use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use translater::po::{PoDocument, parser::parse_text};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MessageKey {
    context: Option<String>,
    msgid: String,
}

#[test]
fn generated_english_catalog_has_fallback_for_every_message() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n/en.po");
    let text = fs::read_to_string(&path).expect("read generated English catalog");
    let doc = parse_text(&path, text).expect("parse generated English catalog");

    assert!(!doc.entries.is_empty());
    for entry in doc.entries.iter().filter(|entry| !entry.is_header()) {
        let source = entry.msgid.value();
        let translation = entry
            .msgstr
            .first()
            .map(|field| field.value())
            .unwrap_or_default();
        assert_eq!(
            translation, source,
            "missing English fallback for {source:?}"
        );
    }
}

#[test]
fn generated_template_covers_english_catalog_messages() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pot_text =
        fs::read_to_string(root.join("i18n/translater.pot")).expect("read generated POT");
    let en_text = fs::read_to_string(root.join("i18n/en.po")).expect("read generated English PO");
    let pot = parse_text("translater.pot", pot_text).expect("parse generated POT");
    let en = parse_text("en.po", en_text).expect("parse generated English PO");

    assert_eq!(catalog_message_keys(&pot), catalog_message_keys(&en));
}

#[test]
fn generated_template_covers_all_rust_i18n_call_sites() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pot_text =
        fs::read_to_string(root.join("i18n/translater.pot")).expect("read generated POT");
    let pot = parse_text("translater.pot", pot_text).expect("parse generated POT");

    let source_messages = rust_i18n_call_site_messages(&root.join("src"));
    let pot_messages = catalog_message_keys(&pot);

    assert!(
        source_messages.contains(&MessageKey {
            context: None,
            msgid: "Interface language set to {language}".to_string(),
        }),
        "source scan should include UI language status messages"
    );
    assert!(
        source_messages.contains(&MessageKey {
            context: None,
            msgid: "plural entry has {actual} of {expected} forms".to_string(),
        }),
        "source scan should include validation diagnostics"
    );
    assert!(
        source_messages.contains(&MessageKey {
            context: None,
            msgid: "unsupported TRPack format".to_string(),
        }),
        "source scan should include workflow errors"
    );

    assert_eq!(pot_messages, source_messages);
}

fn catalog_message_keys(doc: &PoDocument) -> BTreeSet<MessageKey> {
    doc.entries
        .iter()
        .filter(|entry| !entry.is_header())
        .map(|entry| MessageKey {
            context: entry
                .msgctxt
                .as_ref()
                .map(|field| field.value().to_string()),
            msgid: entry.msgid.value().to_string(),
        })
        .collect()
}

fn rust_i18n_call_site_messages(src_dir: &Path) -> BTreeSet<MessageKey> {
    let mut files = Vec::new();
    collect_rust_files(src_dir, &mut files);

    let mut messages = BTreeSet::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("read Rust source");
        for name in ["tr", "tr_format", "tr_ctx", "tr_ctx_format"] {
            let mut offset = 0;
            let needle = format!("{name}(");
            while let Some(relative) = text[offset..].find(&needle) {
                let start = offset + relative;
                if start > 0 && is_ident_byte(text.as_bytes()[start - 1]) {
                    offset = start + needle.len();
                    continue;
                }

                let mut index = skip_ws(&text, start + needle.len());
                let Some((first, next)) = rust_string_at(&text, index) else {
                    offset = start + needle.len();
                    continue;
                };
                index = next;

                let (context, msgid) = if name == "tr_ctx" || name == "tr_ctx_format" {
                    index = skip_ws(&text, index);
                    if text.as_bytes().get(index) != Some(&b',') {
                        offset = start + needle.len();
                        continue;
                    }
                    index = skip_ws(&text, index + 1);
                    let Some((second, _)) = rust_string_at(&text, index) else {
                        offset = start + needle.len();
                        continue;
                    };
                    (Some(first), second)
                } else {
                    (None, first)
                };

                messages.insert(MessageKey { context, msgid });
                offset = start + needle.len();
            }
        }
    }

    messages
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read source directory {}: {error}", dir.display()))
        .collect::<Result<Vec<_>, _>>()
        .expect("read source entries");
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn skip_ws(text: &str, mut index: usize) -> usize {
    while text
        .as_bytes()
        .get(index)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        index += 1;
    }
    index
}

fn rust_string_at(text: &str, start: usize) -> Option<(String, usize)> {
    if text.as_bytes().get(start) != Some(&b'"') {
        return None;
    }

    let mut escaped = false;
    for (relative, ch) in text[start + 1..].char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            let end = start + 1 + relative + ch.len_utf8();
            let literal = &text[start..end];
            let decoded = serde_json::from_str(literal).ok()?;
            return Some((decoded, end));
        }
    }
    None
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
