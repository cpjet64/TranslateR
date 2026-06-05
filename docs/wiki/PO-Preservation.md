# PO Preservation

TranslateR is designed around lossless `.po` handling. The editor reads
structure from the file, but the original file text remains the source of truth.

## Preservation Rules

TranslateR should preserve:

- Entry order.
- Translator comments.
- Extracted comments.
- Source references.
- Flags.
- Contexts.
- Plural entries.
- Header entries.
- Multiline strings.
- Escaped strings.
- Obsolete entries.
- Unrelated whitespace and layout where practical.

TranslateR should not sort entries, normalize the whole file, or rewrite
untouched fields.

## What Changes on Save

Only edited translation fields and supported edited metadata should change.

Examples:

- Editing `msgstr` should not rewrite neighboring comments.
- Editing `msgstr[2]` should not rewrite other plural forms.
- Opening and saving without edits should round-trip the file byte-for-byte.

## Regression Corpus

TranslateR tests against a pinned copy of:

```text
https://github.com/ergenius/gettext-po-samples
```

The corpus covers plural rules, contexts, flags, multiline strings, escaped
strings, headers, and many language codes.

Important test expectations:

- Every fixture `.po` file parses.
- No-edit parse/write round-trips fixture files byte-for-byte.
- Edits preserve unrelated content.
- Plural and multiline edits reparse correctly.

## Why This Matters

Many gettext maintainers review translation diffs in source control. If an
editor rewrites the entire `.po` file, it hides the translator's actual work and
makes review harder. TranslateR is built to keep diffs focused on translation
changes.
