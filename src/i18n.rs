use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    env, fs,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

use crate::po::parser::parse_text;

#[derive(Debug, Default)]
struct I18nState {
    language: String,
    catalog: Catalog,
}

#[derive(Debug, Default, Clone)]
pub struct Catalog {
    messages: HashMap<CatalogKey, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CatalogKey {
    context: Option<String>,
    msgid: String,
}

static STATE: OnceLock<RwLock<I18nState>> = OnceLock::new();

fn state() -> &'static RwLock<I18nState> {
    STATE.get_or_init(|| {
        RwLock::new(I18nState {
            language: "en".to_string(),
            catalog: Catalog::default(),
        })
    })
}

pub fn init(language: &str) {
    set_language(language);
}

pub fn set_language(language: &str) {
    let language = normalize_language(language);
    let catalog = if language == "en" {
        Catalog::default()
    } else {
        load_catalog_for_language(&language).unwrap_or_default()
    };
    if let Ok(mut guard) = state().write() {
        guard.language = language;
        guard.catalog = catalog;
    }
}

pub fn current_language() -> String {
    state()
        .read()
        .map(|guard| guard.language.clone())
        .unwrap_or_else(|_| "en".to_string())
}

pub fn available_languages() -> Vec<String> {
    let mut languages = BTreeSet::from(["en".to_string()]);
    for dir in i18n_dirs() {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("po"))
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            {
                languages.insert(stem.to_string());
            }
        }
    }
    languages.into_iter().collect()
}

pub fn tr(msgid: &'static str) -> Cow<'static, str> {
    lookup(None, msgid)
}

pub fn tr_ctx(context: &'static str, msgid: &'static str) -> Cow<'static, str> {
    lookup(Some(context), msgid)
}

pub fn tr_format(msgid: &'static str, args: &[(&str, String)]) -> String {
    format_message(tr(msgid).as_ref(), args)
}

pub fn tr_ctx_format(
    context: &'static str,
    msgid: &'static str,
    args: &[(&str, String)],
) -> String {
    format_message(tr_ctx(context, msgid).as_ref(), args)
}

pub fn format_message(template: &str, args: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

impl Catalog {
    pub fn from_po_text(path: impl AsRef<Path>, text: String) -> anyhow::Result<Self> {
        let doc =
            parse_text(path, text).expect("parse_text records PO parse issues as diagnostics");
        let mut messages = HashMap::new();
        for entry in doc.entries.iter().filter(|entry| !entry.is_header()) {
            if entry.has_flag("fuzzy") {
                continue;
            }
            let Some(msgstr) = entry.msgstr.first() else {
                continue;
            };
            let value = msgstr.value();
            if value.is_empty() {
                continue;
            }
            messages.insert(
                CatalogKey {
                    context: entry.msgctxt.as_ref().map(|ctx| ctx.value().to_string()),
                    msgid: entry.msgid.value().to_string(),
                },
                value.to_string(),
            );
        }
        Ok(Self { messages })
    }

    fn lookup(&self, context: Option<&str>, msgid: &str) -> Option<String> {
        self.messages
            .get(&CatalogKey {
                context: context.map(ToOwned::to_owned),
                msgid: msgid.to_string(),
            })
            .cloned()
    }
}

fn lookup(context: Option<&'static str>, msgid: &'static str) -> Cow<'static, str> {
    let Ok(guard) = state().read() else {
        return Cow::Borrowed(msgid);
    };
    guard
        .catalog
        .lookup(context, msgid)
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(msgid))
}

fn load_catalog_for_language(language: &str) -> Option<Catalog> {
    for dir in i18n_dirs() {
        let path = dir.join(format!("{language}.po"));
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        return Some(Catalog::from_po_text(&path, text).unwrap_or_default());
    }
    None
}

fn i18n_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.extend(executable_i18n_dirs());
    if let Ok(cwd) = env::current_dir() {
        dirs.push(cwd.join("i18n"));
    }
    if let Ok(config_dir) = crate::util::paths::app_config_dir() {
        dirs.push(config_dir.join("i18n"));
    }
    dirs
}

fn executable_i18n_dirs() -> Vec<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .map(|parent| {
            #[cfg(not(target_os = "macos"))]
            {
                vec![parent.join("i18n")]
            }
            #[cfg(target_os = "macos")]
            {
                let mut dirs = vec![parent.join("i18n")];
                if parent.file_name().and_then(|name| name.to_str()) == Some("MacOS") {
                    if let Some(contents) = parent.parent() {
                        dirs.push(contents.join("Resources").join("i18n"));
                    }
                }
                dirs
            }
        })
        .unwrap_or_default()
}

fn normalize_language(language: &str) -> String {
    let trimmed = language.trim();
    if trimmed.is_empty() {
        "en".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        Catalog, available_languages, current_language, format_message, init,
        load_catalog_for_language, set_language, state, tr, tr_ctx, tr_ctx_format,
    };
    use crate::{test_support::i18n_runtime_guard, util::paths::set_app_config_dir_override};

    #[test]
    fn loads_catalog_with_context_and_ignores_fuzzy_empty_entries() {
        let _guard = i18n_runtime_guard();
        let text = r#"msgid ""
msgstr "Language: zh-Hans\n"

msgctxt "button"
msgid "Open PO"
msgstr "打开 PO"

#, fuzzy
msgid "Save PO"
msgstr "保存 PO"

msgid "Empty"
msgstr ""

msgid "Missing msgstr"
"#
        .to_string();
        let catalog = Catalog::from_po_text("zh-Hans.po", text).unwrap();

        assert_eq!(
            catalog.lookup(Some("button"), "Open PO").as_deref(),
            Some("打开 PO")
        );
        assert_eq!(catalog.lookup(None, "Save PO"), None);
        assert_eq!(catalog.lookup(None, "Empty"), None);
        assert_eq!(catalog.lookup(None, "Missing msgstr"), None);
    }

    #[test]
    fn discovers_bundled_catalogs_and_init_loads_language() {
        let _guard = i18n_runtime_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let i18n_dir = temp_dir.path().join("i18n");
        fs::create_dir_all(&i18n_dir).unwrap();
        fs::write(
            i18n_dir.join("zz-Test.po"),
            "msgid \"\"\nmsgstr \"Language: zz-Test\\n\"\n\nmsgid \"Open PO\"\nmsgstr \"Open test\"\n",
        )
        .unwrap();
        let _override = set_app_config_dir_override(temp_dir.path().to_path_buf());

        let languages = available_languages();
        assert!(languages.contains(&"en".to_string()));
        assert!(languages.contains(&"zz-Test".to_string()));

        let catalog = load_catalog_for_language("en").unwrap();
        assert_eq!(catalog.lookup(None, "Open PO").as_deref(), Some("Open PO"));

        init("en");
        assert_eq!(current_language(), "en");
    }

    #[test]
    fn replaces_named_placeholders() {
        let _guard = i18n_runtime_guard();
        assert_eq!(
            format_message(
                "Saved {count} files for {language}",
                &[("count", "2".to_string()), ("language", "de".to_string())],
            ),
            "Saved 2 files for de"
        );
    }

    #[test]
    fn runtime_language_switch_tracks_language_and_falls_back_to_source_text() {
        let _guard = i18n_runtime_guard();
        const OPEN_PO: &str = "Open PO";

        set_language("  ");
        assert_eq!(current_language(), "en");
        assert_eq!(tr(OPEN_PO).as_ref(), OPEN_PO);

        set_language("zz-Test");
        assert_eq!(current_language(), "zz-Test");
        assert_eq!(tr(OPEN_PO).as_ref(), OPEN_PO);

        set_language("en");
        assert_eq!(current_language(), "en");
    }

    #[test]
    fn catalog_lookup_respects_context_and_formatting() {
        let _guard = i18n_runtime_guard();
        let text = r#"msgid ""
msgstr "Language: pirate\n"

msgctxt "toolbar"
msgid "Open"
msgstr "Open, matey"

msgid "Open"
msgstr "Open plain"

msgctxt "status"
msgid "Saved {path}"
msgstr "Stashed {path}"
"#
        .to_string();
        let catalog = Catalog::from_po_text("pirate.po", text).unwrap();

        assert_eq!(
            catalog.lookup(Some("toolbar"), "Open").as_deref(),
            Some("Open, matey")
        );
        assert_eq!(catalog.lookup(None, "Open").as_deref(), Some("Open plain"));
        assert_eq!(
            format_message(
                catalog
                    .lookup(Some("status"), "Saved {path}")
                    .unwrap()
                    .as_str(),
                &[("path", "file.po".to_string())],
            ),
            "Stashed file.po"
        );
    }

    #[test]
    fn context_formatting_falls_back_when_runtime_catalog_is_missing_message() {
        let _guard = i18n_runtime_guard();
        const CONTEXT: &str = "dialog";
        const CANCEL: &str = "Cancel";
        const STATUS: &str = "status";
        const SAVED_PATH: &str = "Saved {path}";

        set_language("zz-Missing");
        assert_eq!(tr_ctx(CONTEXT, CANCEL).as_ref(), CANCEL);
        assert_eq!(
            tr_ctx_format(
                STATUS,
                SAVED_PATH,
                &[("path", "translation.po".to_string())],
            ),
            "Saved translation.po"
        );
        set_language("en");
    }

    #[test]
    fn poisoned_runtime_state_falls_back_to_source_text() {
        let _guard = i18n_runtime_guard();
        let _ = std::panic::catch_unwind(|| {
            let _write_guard = state().write().unwrap();
            panic!("poison i18n runtime state for fallback coverage");
        });

        assert_eq!(tr("Open PO").as_ref(), "Open PO");
        assert_eq!(current_language(), "en");
        set_language("zz-Poisoned");
        assert_eq!(current_language(), "en");

        state().clear_poison();
        set_language("en");
    }
}
