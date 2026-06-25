use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlineStyle {
    Lf,
    CrLf,
}

impl NewlineStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntryId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLine {
    pub text_without_newline: String,
    pub line_no: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoFieldKind {
    MsgCtxt,
    MsgId,
    MsgIdPlural,
    MsgStr,
}

#[derive(Debug, Clone)]
pub struct PoField {
    pub kind: PoFieldKind,
    pub index: Option<usize>,
    pub raw_lines: Vec<RawLine>,
    pub decoded: String,
    pub edited_value: Option<String>,
}

impl PoField {
    pub fn value(&self) -> &str {
        self.edited_value.as_deref().unwrap_or(&self.decoded)
    }
}

#[derive(Debug, Clone, Default)]
pub struct EntryComments {
    pub translator: Vec<RawLine>,
    pub extracted: Vec<RawLine>,
    pub reference: Vec<RawLine>,
    pub flags_raw: Vec<RawLine>,
    pub previous: Vec<RawLine>,
    pub unknown: Vec<RawLine>,
}

#[derive(Debug, Clone, Default)]
pub struct EntryEditState {
    pub translator_comments: Option<String>,
    pub fuzzy: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PoEntry {
    pub id: EntryId,
    pub ordinal: usize,
    pub span: LineSpan,
    pub obsolete: bool,
    pub comments: EntryComments,
    pub flags: Vec<String>,
    pub msgctxt: Option<PoField>,
    pub msgid: PoField,
    pub msgid_plural: Option<PoField>,
    pub msgstr: Vec<PoField>,
    pub diagnostics: Vec<Diagnostic>,
    pub edited: EntryEditState,
}

impl PoEntry {
    pub fn is_header(&self) -> bool {
        self.msgid.value().is_empty() && self.msgctxt.is_none() && self.ordinal == 0
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }
}

#[derive(Debug, Clone)]
pub struct PoDocument {
    pub path: PathBuf,
    pub original_bytes: Vec<u8>,
    pub original_text: String,
    pub original_hash: String,
    pub newline: NewlineStyle,
    pub entries: Vec<PoEntry>,
    pub trailing_raw: Vec<RawLine>,
    pub dirty: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub entry_id: Option<EntryId>,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct PoHeader {
    pub language: Option<String>,
    pub revision_date: Option<String>,
    pub last_translator: Option<String>,
    pub language_team: Option<String>,
    pub plural_forms: Option<PluralFormsHeader>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluralFormsHeader {
    pub nplurals: usize,
    pub raw: String,
}
